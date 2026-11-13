//! `MVM-Wasm` RV64IMAC 64-bit Linux MicroVM and `SpiralCore` `virtio-vsock` zero-NIC switch (`R4-37`).
//!
//! Implements a deterministic, zero-`unsafe` 64-bit RISC-V (`RV64IMAC` + `Zicsr`) processor,
//! multi-process Linux `riscv64` system call executive, 64 KB Copy-on-Write block storage
//! backed by the cross-compiled `mvm_initramfs.bin` (`UORMVM01`), in-sandbox C-to-ELF64
//! compilation (`mvm_cc`), zero-NIC guest socket isolation (`AF_VSOCK = 40` only), and
//! host secret-placeholder substitution that keeps raw credentials out of guest RAM.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

use sha2::{Digest, Sha256};

use crate::lexlean_generated::r4_connected_studio::{
    admit_bridge_transport, admit_git_secret_substitution, admit_package_install,
    BridgeTransportVerdict, GitSecretVerdict, McpPolicyMode, PackageInstallVerdict, TransportKind,
};
use crate::lexlean_generated::r4_spiralcore_vsock::{
    admit_guest_socket, admit_host_egress, admit_rv64_elf, admit_spiralcore_frame,
    EgressPolicyCheck, EgressVerdict, Elf64HeaderSummary, ElfAdmission, FrameVerdict,
    SocketAdmission, SpiralCoreFrame,
};
use crate::mvm_cc::{compile_c_to_rv64_elf, CCompileError};

/// Four-byte magic prefix for 16-byte `SpiralCore` `virtio-vsock` frames.
pub const SPIRALCORE_MAGIC: [u8; 4] = *b"SPVS";
/// Maximum payload bytes admitted in a single `SpiralCore` `virtio-vsock` frame.
pub const SPIRALCORE_MAX_PAYLOAD: usize = 65_536;
/// Copy-on-Write virtual block sector size (64 KB).
pub const MVM_COW_SECTOR_BYTES: usize = 65_536;
/// Flat guest physical memory size per process (8 MB).
const GUEST_RAM_BYTES: usize = 8 * 1024 * 1024;
/// Initial top of guest user stack.
const GUEST_STACK_TOP: u64 = 0x007E_0000;
/// Base virtual address for anonymous `mmap` allocations.
const GUEST_MMAP_BASE: u64 = 0x0040_0000;
/// Instruction fuel cap per command invocation.
const MAX_INSTRUCTION_FUEL: u64 = 25_000_000;

/// Embedded `UORMVM01` initramfs archive containing genuine cross-compiled `riscv64` ELF64 userland.
static INITRAMFS_IMAGE: &[u8] = include_bytes!("mvm_initramfs.bin");

/// Linux `EAFNOSUPPORT` (`97`) negated for `riscv64` kernel error return when NIC domains are denied.
const LINUX_EAFNOSUPPORT: i64 = -97;
/// Linux `ENOENT` (`2`) negated.
const LINUX_ENOENT: i64 = -2;
/// Linux `EBADF` (`9`) negated.
const LINUX_EBADF: i64 = -9;
/// Linux `EINVAL` (`22`) negated.
const LINUX_EINVAL: i64 = -22;
/// Linux `ENOSYS` (`38`) negated.
const LINUX_ENOSYS: i64 = -38;

/// Typed `SpiralCore` bridge and package-admission errors (`R4-39`, `R5`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeError {
    /// Outbound bridge blocked because MCP policy mode is `Deny`.
    PolicyDenied,
    /// Non-TLS `http://` or `ws://` remote transport rejected.
    InsecureTransportRejected,
    /// Source or destination `virtio-vsock` CID violated `CID >= 3 -> CID 2`.
    InvalidVsockCid,
    /// Raw secret bytes were detected inside `MvmGuest` RAM.
    GuestRamSecretContamination,
    /// Secret placeholder targeted a host outside `["github.com", "api.github.com"]`.
    HostNotAllowlistedForSecret,
    /// Secret substitution invoked without a `{{MVM_SECRET:...}}` placeholder.
    MissingSecretPlaceholder,
    /// Downloaded package bytes failed SHA-256 verification.
    PackageDigestMismatch,
    /// Downloaded package failed `ELF64` little-endian `EM_RISCV = 243` verification.
    PackageInvalidElf64,
    /// Downloaded package exceeded the 4 MB binary quota.
    PackageSizeExceedsQuota,
    /// Bridge resume or response JSON was malformed.
    InvalidBridgeJson(String),
}

impl BridgeError {
    /// Canonical machine-readable error code matching `R4-39` failure variants.
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::PolicyDenied => "PolicyDenied",
            Self::InsecureTransportRejected => "InsecureTransportRejected",
            Self::InvalidVsockCid => "InvalidVsockCid",
            Self::GuestRamSecretContamination => "GuestRamSecretContamination",
            Self::HostNotAllowlistedForSecret => "HostNotAllowlistedForSecret",
            Self::MissingSecretPlaceholder => "MissingSecretPlaceholder",
            Self::PackageDigestMismatch => "PackageDigestMismatch",
            Self::PackageInvalidElf64 => "PackageInvalidElf64",
            Self::PackageSizeExceedsQuota => "PackageSizeExceedsQuota",
            Self::InvalidBridgeJson(_) => "InvalidBridgeJson",
        }
    }
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PolicyDenied => write!(f, "SpiralCore bridge denied by active MCP policy"),
            Self::InsecureTransportRejected => {
                write!(f, "insecure plain HTTP/WS transport rejected by R4-39")
            }
            Self::InvalidVsockCid => write!(f, "invalid SpiralCore vsock CID pair"),
            Self::GuestRamSecretContamination => {
                write!(f, "secret contamination detected in MVM guest RAM")
            }
            Self::HostNotAllowlistedForSecret => {
                write!(f, "target host is not allowlisted for secret substitution")
            }
            Self::MissingSecretPlaceholder => {
                write!(f, "missing MVM secret placeholder in bridge request")
            }
            Self::PackageDigestMismatch => write!(f, "package SHA-256 digest mismatch"),
            Self::PackageInvalidElf64 => {
                write!(f, "package is not a valid 64-bit EM_RISCV (243) ELF binary")
            }
            Self::PackageSizeExceedsQuota => write!(f, "package exceeds 4 MB quota"),
            Self::InvalidBridgeJson(msg) => write!(f, "invalid bridge JSON: {msg}"),
        }
    }
}

impl std::error::Error for BridgeError {}

/// Typed runtime error returned by `MVM-Wasm` operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MvmError {
    /// Initramfs image header or SHA-256 entry integrity failed.
    CorruptInitramfs(String),
    /// ELF64 header failed `R4.SpiralCoreVsock.admitRv64Elf`.
    ElfRejected(ElfAdmission),
    /// `SpiralCore` 16-byte frame failed `R4.SpiralCoreVsock.admitSpiralCoreFrame`.
    FrameRejected(FrameVerdict),
    /// Guest execution exceeded deterministic instruction fuel or trapped on invalid memory/opcode.
    CpuTrap(String),
    /// In-sandbox C compilation failed.
    CompileError(CCompileError),
    /// Requested guest file or executable was not found.
    NotFound(String),
    /// `SpiralCore` `R4-39` connected bridge or package admission refused the operation.
    BridgeRejected(BridgeError),
}

impl std::fmt::Display for MvmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CorruptInitramfs(m) => write!(f, "corrupt initramfs: {m}"),
            Self::ElfRejected(v) => write!(f, "ELF64 rejected by R4-37 kernel: {v:?}"),
            Self::FrameRejected(v) => write!(f, "SpiralCore frame rejected: {v:?}"),
            Self::CpuTrap(m) => write!(f, "RV64IMAC trap: {m}"),
            Self::CompileError(e) => write!(f, "{e}"),
            Self::NotFound(p) => write!(f, "file not found: {p}"),
            Self::BridgeRejected(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for MvmError {}

/// Exact 16-byte `SpiralCore` `virtio-vsock` packet header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiralCoreHeader {
    /// Four-byte magic (`*b"SPVS"`).
    pub magic: [u8; 4],
    /// Source Context ID (`CID`, `2` = host switch, `>= 3` = MicroVM guest).
    pub src_cid: u16,
    /// Destination Context ID (`CID`).
    pub dst_cid: u16,
    /// `virtio-vsock` destination port.
    pub vsock_port: u16,
    /// Clifford `Cl(0,6)` spinor action (`0..63`).
    pub cl06_spin: u8,
    /// $E_8$ root orbit cell (`0..15`).
    pub e8_orbit: u8,
    /// Byte length of payload immediately following the 16-byte header.
    pub payload_len: u32,
}

impl SpiralCoreHeader {
    /// Encode header to its canonical 16-byte little-endian wire representation.
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[0..4].copy_from_slice(&self.magic);
        out[4..6].copy_from_slice(&self.src_cid.to_le_bytes());
        out[6..8].copy_from_slice(&self.dst_cid.to_le_bytes());
        out[8..10].copy_from_slice(&self.vsock_port.to_le_bytes());
        out[10] = self.cl06_spin;
        out[11] = self.e8_orbit;
        out[12..16].copy_from_slice(&self.payload_len.to_le_bytes());
        out
    }

    /// Decode a 16-byte `SpiralCore` header slice.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 16 {
            return None;
        }
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[0..4]);
        Some(Self {
            magic,
            src_cid: u16::from_le_bytes([bytes[4], bytes[5]]),
            dst_cid: u16::from_le_bytes([bytes[6], bytes[7]]),
            vsock_port: u16::from_le_bytes([bytes[8], bytes[9]]),
            cl06_spin: bytes[10],
            e8_orbit: bytes[11],
            payload_len: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
        })
    }
}

/// Complete `SpiralCore` `virtio-vsock` packet (16-byte header + payload).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VsockPacket {
    /// Verified 16-byte header.
    pub header: SpiralCoreHeader,
    /// Raw payload bytes.
    pub payload: Vec<u8>,
}

/// Result of a host egress secret-placeholder proxy check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EgressProxyOutcome {
    /// Formal `R4.SpiralCoreVsock.EgressVerdict`.
    pub verdict: EgressVerdict,
    /// Substituted outbound payload (present only when `verdict == ProxyAdmitted`).
    pub outbound_payload: Option<String>,
}

/// Copy-on-Write snapshot of the MicroVM filesystem and 64 KB dirty block overlay.
#[derive(Debug, Clone)]
pub struct MvmCowSnapshot {
    /// FIPS 180-4 SHA-256 hex digest of the logical filesystem and CoW blocks.
    pub state_sha256: String,
    inodes: BTreeMap<String, GuestInode>,
    dirs: BTreeSet<String>,
    cow_sectors: BTreeMap<u32, Vec<u8>>,
    dirty_sectors: BTreeSet<u32>,
}

/// Structured result of executing a command chain inside `MVM-Wasm`.
#[derive(Debug, Clone)]
pub struct MvmExecOutcome {
    /// Final process exit code (`0` = success, `127` = command not found).
    pub exit_code: i32,
    /// Captured standard output UTF-8 text.
    pub stdout: String,
    /// Captured standard error UTF-8 text.
    pub stderr: String,
    /// Total `RV64IMAC` instructions retired across spawned processes.
    pub instructions_retired: u64,
    /// `SpiralCore` `AF_VSOCK` packets emitted during execution.
    pub vsock_frames: Vec<VsockPacket>,
    /// `e_machine` of any ELF64 binary compiled during this invocation (`243` for `EM_RISCV`).
    pub compiled_elf_machine: Option<u16>,
    /// Updated SHA-256 digest of the CoW block state.
    pub cow_state_sha256: String,
}

/// Suspended `SpiralCore` (`CID 3 -> CID 2`) operation awaiting host Worker relay completion (`R4-39`).
#[derive(Debug, Clone)]
pub struct MvmBridgePending {
    /// Outbound `SpiralCore` bridge request JSON (`dns_query`, `http_request`, `pkg_fetch`, `git_smart_http`).
    pub bridge_req: serde_json::Value,
    /// Deterministic guest continuation state passed back into `complete_bridge`.
    pub resume_state: serde_json::Value,
    /// Verified 16-byte `SPVS` frames emitted for this request.
    pub vsock_frames: Vec<VsockPacket>,
}

/// Outcome of an interactive MicroVM shell turn (`Completed` locally or `NeedsBridge` over `AF_VSOCK = 40`).
#[derive(Debug, Clone)]
pub enum MvmInteractiveOutcome {
    /// Command chain completed synchronously inside `MVM-Wasm`.
    Completed(MvmExecOutcome),
    /// Command requires an asynchronous `SpiralCore` (`CID 3 -> CID 2`) bridge round-trip.
    NeedsBridge(MvmBridgePending),
}

/// Metadata for an admitted `ELF64` (`EM_RISCV = 243`) userland binary installed into `/usr/local/bin`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InstalledPackageMeta {
    /// Binary name under `/usr/local/bin/<name>`.
    pub name: String,
    /// Verified FIPS 180-4 SHA-256 hex digest.
    pub sha256: String,
    /// Binary size in bytes (`<= 4 MB`).
    pub byte_len: u32,
    /// Source URL or catalog identifier.
    pub source: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GitCommitRecord {
    oid: String,
    parent_oid: Option<String>,
    message: String,
    tree: BTreeMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GitRepoState {
    branch: String,
    remote_url: Option<String>,
    head_oid: Option<String>,
    staged: BTreeMap<String, Option<String>>,
    head_tree: BTreeMap<String, String>,
    commits: Vec<GitCommitRecord>,
}

/// Admit a guest `socket(domain, ...)` domain code via `R4.SpiralCoreVsock.admitGuestSocket`.
pub fn admit_guest_socket_domain(domain_code: u64) -> SocketAdmission {
    admit_guest_socket(domain_code)
}

/// Admit a 16-byte `SpiralCore` header via `R4.SpiralCoreVsock.admitSpiralCoreFrame`.
pub fn admit_spiralcore_header(hdr: &SpiralCoreHeader, max_payload_bytes: usize) -> FrameVerdict {
    if hdr.magic != SPIRALCORE_MAGIC {
        return FrameVerdict::RefusedInvalidCid;
    }
    admit_spiralcore_frame(SpiralCoreFrame {
        src_cid: hdr.src_cid as u64,
        dst_cid: hdr.dst_cid as u64,
        vsock_port: hdr.vsock_port as u64,
        cl06_spin: hdr.cl06_spin as u64,
        e8_orbit: hdr.e8_orbit as u64,
        payload_bytes: hdr.payload_len as u64,
        max_payload_bytes: max_payload_bytes as u64,
    })
}

/// Admit a 64-bit `EM_RISCV` ELF64 image via `R4.SpiralCoreVsock.admitRv64Elf`.
pub fn admit_elf64_riscv(bytes: &[u8]) -> ElfAdmission {
    if bytes.len() < 64 {
        return ElfAdmission::RefusedBadMagic;
    }
    let magic_valid = &bytes[0..4] == b"\x7fELF";
    let elf_class64 = bytes[4] == 2;
    let little_endian = bytes[5] == 1;
    let machine_code = u16::from_le_bytes([bytes[18], bytes[19]]) as u64;
    let entry = u64::from_le_bytes([
        bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30], bytes[31],
    ]);
    let phnum = u16::from_le_bytes([bytes[56], bytes[57]]) as usize;
    let phoff = u64::from_le_bytes([
        bytes[32], bytes[33], bytes[34], bytes[35], bytes[36], bytes[37], bytes[38], bytes[39],
    ]) as usize;
    let phentsize = u16::from_le_bytes([bytes[54], bytes[55]]) as usize;

    let mut load_segments = 0u64;
    if magic_valid && elf_class64 && little_endian && phentsize >= 56 {
        for i in 0..phnum {
            let off = phoff.saturating_add(i.saturating_mul(phentsize));
            if off + 56 <= bytes.len() {
                let p_type = u32::from_le_bytes([
                    bytes[off],
                    bytes[off + 1],
                    bytes[off + 2],
                    bytes[off + 3],
                ]);
                if p_type == 1 {
                    load_segments += 1;
                }
            }
        }
    }

    admit_rv64_elf(Elf64HeaderSummary {
        magic_valid,
        elf_class64,
        little_endian,
        machine_code,
        entry_aligned: entry >= 0x1000 && entry.is_multiple_of(2),
        segment_count: load_segments,
        max_segments: 16,
    })
}

#[derive(Debug, Clone)]
enum InodeKind {
    Regular { sector_start: u32, byte_len: usize },
    Symlink { target: String },
}

#[derive(Debug, Clone)]
struct GuestInode {
    mode: u32,
    kind: InodeKind,
}

#[derive(Debug, Clone)]
struct HostSecretRule {
    raw_secret: String,
    allowlisted_hosts: BTreeSet<String>,
}

#[derive(Clone)]
enum FdKind {
    StdinBuffer(Rc<RefCell<VecDeque<u8>>>),
    StdoutCapture(Rc<RefCell<Vec<u8>>>),
    StderrCapture(Rc<RefCell<Vec<u8>>>),
    PipeRead(Rc<RefCell<VecDeque<u8>>>),
    PipeWrite(Rc<RefCell<VecDeque<u8>>>),
    RegularFile {
        path: String,
        offset: usize,
        writable: bool,
        append: bool,
    },
    Directory {
        entries: Vec<(u8, String)>,
        index: usize,
    },
    VsockSocket {
        peer_cid: u16,
        peer_port: u16,
        rx_queue: VecDeque<Vec<u8>>,
    },
}

#[derive(Clone)]
struct Rv64Process {
    pid: i64,
    regs: [u64; 32],
    pc: u64,
    ram: Vec<u8>,
    brk_addr: u64,
    mmap_cursor: u64,
    lr_reservation: Option<u64>,
    fds: BTreeMap<i32, FdKind>,
    next_fd: i32,
    cwd: String,
    exited: Option<i32>,
}

impl Rv64Process {
    fn read_u8(&self, addr: u64) -> Result<u8, MvmError> {
        let idx = addr as usize;
        self.ram
            .get(idx)
            .copied()
            .ok_or_else(|| MvmError::CpuTrap(format!("load u8 fault at 0x{addr:x}")))
    }

    fn read_u16(&self, addr: u64) -> Result<u16, MvmError> {
        let idx = addr as usize;
        if idx + 2 > self.ram.len() {
            return Err(MvmError::CpuTrap(format!("load u16 fault at 0x{addr:x}")));
        }
        Ok(u16::from_le_bytes([self.ram[idx], self.ram[idx + 1]]))
    }

    fn read_u32(&self, addr: u64) -> Result<u32, MvmError> {
        let idx = addr as usize;
        if idx + 4 > self.ram.len() {
            return Err(MvmError::CpuTrap(format!("load u32 fault at 0x{addr:x}")));
        }
        Ok(u32::from_le_bytes([
            self.ram[idx],
            self.ram[idx + 1],
            self.ram[idx + 2],
            self.ram[idx + 3],
        ]))
    }

    fn read_u64(&self, addr: u64) -> Result<u64, MvmError> {
        let idx = addr as usize;
        if idx + 8 > self.ram.len() {
            return Err(MvmError::CpuTrap(format!("load u64 fault at 0x{addr:x}")));
        }
        let mut b = [0u8; 8];
        b.copy_from_slice(&self.ram[idx..idx + 8]);
        Ok(u64::from_le_bytes(b))
    }

    fn write_bytes(&mut self, addr: u64, data: &[u8]) -> Result<(), MvmError> {
        let idx = addr as usize;
        if idx.saturating_add(data.len()) > self.ram.len() {
            return Err(MvmError::CpuTrap(format!(
                "store fault at 0x{addr:x} len={}",
                data.len()
            )));
        }
        self.ram[idx..idx + data.len()].copy_from_slice(data);
        Ok(())
    }

    fn read_cstr(&self, addr: u64) -> Result<String, MvmError> {
        let mut idx = addr as usize;
        let mut bytes = Vec::new();
        while idx < self.ram.len() && bytes.len() < 4096 {
            let b = self.ram[idx];
            if b == 0 {
                return Ok(String::from_utf8_lossy(&bytes).into_owned());
            }
            bytes.push(b);
            idx += 1;
        }
        Err(MvmError::CpuTrap(format!(
            "unterminated cstr at 0x{addr:x}"
        )))
    }
}

/// Complete `MVM-Wasm` RV64IMAC Linux MicroVM instance with CoW storage and `SpiralCore` switch.
pub struct MvmSandbox {
    guest_cid: u16,
    inodes: BTreeMap<String, GuestInode>,
    dirs: BTreeSet<String>,
    cow_sectors: BTreeMap<u32, Vec<u8>>,
    dirty_sectors: BTreeSet<u32>,
    next_sector: u32,
    next_pid: i64,
    host_secrets: BTreeMap<String, HostSecretRule>,
    vsock_inbox: BTreeMap<u16, VecDeque<VsockPacket>>,
    last_guest_ram: Vec<u8>,
    current_cwd: String,
    policy_mode: McpPolicyMode,
    synced_workspace_paths: BTreeSet<String>,
    installed_packages: BTreeMap<String, InstalledPackageMeta>,
    next_bridge_seq: u64,
}

impl MvmSandbox {
    /// Boot a fresh `MVM-Wasm` MicroVM with default guest `CID = 3` from `mvm_initramfs.bin`.
    pub fn boot_default() -> Result<Self, MvmError> {
        Self::boot_with_cid(3)
    }

    /// Boot a fresh `MVM-Wasm` MicroVM assigned to `guest_cid` (`>= 3`).
    pub fn boot_with_cid(guest_cid: u16) -> Result<Self, MvmError> {
        if guest_cid < 3 {
            return Err(MvmError::FrameRejected(FrameVerdict::RefusedInvalidCid));
        }
        let mut dirs = BTreeSet::new();
        for d in [
            "/",
            "/bin",
            "/sbin",
            "/usr",
            "/usr/bin",
            "/usr/local",
            "/usr/local/bin",
            "/workspace",
            "/tmp",
            "/etc",
            "/etc/pkg",
        ] {
            dirs.insert(d.to_string());
        }
        let mut vm = Self {
            guest_cid,
            inodes: BTreeMap::new(),
            dirs,
            cow_sectors: BTreeMap::new(),
            dirty_sectors: BTreeSet::new(),
            next_sector: 0,
            next_pid: 100,
            host_secrets: BTreeMap::new(),
            vsock_inbox: BTreeMap::new(),
            last_guest_ram: Vec::new(),
            current_cwd: "/workspace".to_string(),
            policy_mode: McpPolicyMode::Safe,
            synced_workspace_paths: BTreeSet::new(),
            installed_packages: BTreeMap::new(),
            next_bridge_seq: 1,
        };
        vm.mount_initramfs(INITRAMFS_IMAGE)?;
        vm.dirty_sectors.clear();
        Ok(vm)
    }

    /// Active interactive shell working directory inside the guest filesystem.
    pub fn current_cwd(&self) -> &str {
        &self.current_cwd
    }

    /// Active `R4-39` MCP bridge policy mode (`Safe`, `Confirm`, or `Deny`).
    pub fn policy_mode(&self) -> McpPolicyMode {
        self.policy_mode
    }

    /// Update the active `R4-39` MCP bridge policy mode.
    pub fn set_policy_mode(&mut self, mode: McpPolicyMode) {
        self.policy_mode = mode;
    }

    /// Admitted `ELF64` (`EM_RISCV = 243`) packages installed under `/usr/local/bin`.
    pub fn installed_packages(&self) -> &BTreeMap<String, InstalledPackageMeta> {
        &self.installed_packages
    }

    /// Guest `virtio-vsock` Context ID (`CID`).
    pub fn guest_cid(&self) -> u16 {
        self.guest_cid
    }

    /// Canonical guest ISA identifier.
    pub fn architecture(&self) -> &'static str {
        "riscv64-rv64imac"
    }

    /// Count of 64 KB Copy-on-Write sectors dirtied since boot or last snapshot restore.
    pub fn dirty_cow_sector_count(&self) -> usize {
        self.dirty_sectors.len()
    }

    /// Export dirty 64 KB CoW sectors `(sector_id, bytes)` for Worker OPFS persistence.
    pub fn export_dirty_cow_sectors(&self) -> Vec<(u32, Vec<u8>)> {
        self.dirty_sectors
            .iter()
            .filter_map(|id| self.cow_sectors.get(id).map(|b| (*id, b.clone())))
            .collect()
    }

    /// Hydrate 64 KB CoW sectors loaded from OPFS `FileSystemSyncAccessHandle`.
    pub fn import_cow_sectors(&mut self, sectors: &[(u32, Vec<u8>)]) {
        for (id, data) in sectors {
            if data.len() == MVM_COW_SECTOR_BYTES {
                self.cow_sectors.insert(*id, data.clone());
                self.dirty_sectors.insert(*id);
                if *id >= self.next_sector {
                    self.next_sector = *id + 1;
                }
            }
        }
    }

    /// Compute the FIPS 180-4 SHA-256 hex digest of the logical filesystem and CoW state.
    pub fn cow_state_sha256(&self) -> String {
        let mut h = Sha256::new();
        h.update(b"UOR_MVM_COW_V1:");
        for d in &self.dirs {
            h.update(b"D:");
            h.update(d.as_bytes());
            h.update([0]);
        }
        for (path, inode) in &self.inodes {
            h.update(b"F:");
            h.update(path.as_bytes());
            h.update(inode.mode.to_le_bytes());
            match &inode.kind {
                InodeKind::Symlink { target } => {
                    h.update(b"L:");
                    h.update(target.as_bytes());
                }
                InodeKind::Regular { .. } => {
                    if let Some(bytes) = self.read_guest_file(path) {
                        h.update((bytes.len() as u64).to_le_bytes());
                        h.update(&bytes);
                    }
                }
            }
        }
        let digest = h.finalize();
        let mut out = String::with_capacity(64);
        for b in digest {
            use std::fmt::Write;
            let _ = write!(&mut out, "{b:02x}");
        }
        out
    }

    /// Capture an O(1)-metadata Copy-on-Write snapshot of the MicroVM storage layer.
    pub fn snapshot_cow(&self) -> MvmCowSnapshot {
        MvmCowSnapshot {
            state_sha256: self.cow_state_sha256(),
            inodes: self.inodes.clone(),
            dirs: self.dirs.clone(),
            cow_sectors: self.cow_sectors.clone(),
            dirty_sectors: self.dirty_sectors.clone(),
        }
    }

    /// Roll back the MicroVM storage layer to a previously captured CoW snapshot.
    pub fn restore_cow(&mut self, snap: &MvmCowSnapshot) {
        self.inodes = snap.inodes.clone();
        self.dirs = snap.dirs.clone();
        self.cow_sectors = snap.cow_sectors.clone();
        self.dirty_sectors = snap.dirty_sectors.clone();
    }

    fn mount_initramfs(&mut self, img: &[u8]) -> Result<(), MvmError> {
        if img.len() < 12 || &img[0..8] != b"UORMVM01" {
            return Err(MvmError::CorruptInitramfs("bad magic".to_string()));
        }
        let count = u32::from_le_bytes([img[8], img[9], img[10], img[11]]) as usize;
        let mut cur = 12usize;
        for _ in 0..count {
            if cur + 42 > img.len() {
                return Err(MvmError::CorruptInitramfs(
                    "truncated entry header".to_string(),
                ));
            }
            let path_len = u16::from_le_bytes([img[cur], img[cur + 1]]) as usize;
            let mode = u32::from_le_bytes([img[cur + 2], img[cur + 3], img[cur + 4], img[cur + 5]]);
            let data_len =
                u32::from_le_bytes([img[cur + 6], img[cur + 7], img[cur + 8], img[cur + 9]])
                    as usize;
            let expected_sha = &img[cur + 10..cur + 42];
            cur += 42;
            if cur + path_len + data_len > img.len() {
                return Err(MvmError::CorruptInitramfs(
                    "truncated entry payload".to_string(),
                ));
            }
            let path = std::str::from_utf8(&img[cur..cur + path_len])
                .map_err(|_| MvmError::CorruptInitramfs("invalid utf8 path".to_string()))?
                .to_string();
            cur += path_len;
            let data = &img[cur..cur + data_len];
            cur += data_len;

            let mut h = Sha256::new();
            h.update(data);
            if h.finalize().as_slice() != expected_sha {
                return Err(MvmError::CorruptInitramfs(format!(
                    "SHA-256 mismatch for {path}"
                )));
            }

            if (mode & 0o170000) == 0o120000 {
                let target = String::from_utf8_lossy(data).into_owned();
                self.inodes.insert(
                    path,
                    GuestInode {
                        mode,
                        kind: InodeKind::Symlink { target },
                    },
                );
            } else {
                self.write_inode_with_mode(&path, data, mode)?;
            }
        }
        Ok(())
    }

    fn normalize_guest_path(cwd: &str, raw: &str) -> String {
        let joined = if raw.starts_with('/') {
            raw.to_string()
        } else if cwd == "/" {
            format!("/{raw}")
        } else {
            format!("{cwd}/{raw}")
        };
        let mut stack: Vec<&str> = Vec::new();
        for seg in joined.split('/') {
            if seg.is_empty() || seg == "." {
                continue;
            }
            if seg == ".." {
                stack.pop();
            } else {
                stack.push(seg);
            }
        }
        if stack.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", stack.join("/"))
        }
    }

    fn ensure_parent_dirs(&mut self, norm_path: &str) {
        let mut prefix = String::new();
        let parts: Vec<&str> = norm_path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() <= 1 {
            return;
        }
        for seg in &parts[..parts.len() - 1] {
            prefix.push('/');
            prefix.push_str(seg);
            self.dirs.insert(prefix.clone());
        }
    }

    fn write_inode_with_mode(
        &mut self,
        path: &str,
        data: &[u8],
        mode: u32,
    ) -> Result<(), MvmError> {
        let norm = Self::normalize_guest_path("/", path);
        self.ensure_parent_dirs(&norm);
        let needed_sectors = data.len().div_ceil(MVM_COW_SECTOR_BYTES).max(1);
        let start = self.next_sector;
        for i in 0..needed_sectors {
            let sec_id = start + (i as u32);
            let mut sector = vec![0u8; MVM_COW_SECTOR_BYTES];
            let chunk_off = i * MVM_COW_SECTOR_BYTES;
            if chunk_off < data.len() {
                let end = (chunk_off + MVM_COW_SECTOR_BYTES).min(data.len());
                sector[0..(end - chunk_off)].copy_from_slice(&data[chunk_off..end]);
            }
            self.cow_sectors.insert(sec_id, sector);
            self.dirty_sectors.insert(sec_id);
        }
        self.next_sector = start + (needed_sectors as u32);
        self.inodes.insert(
            norm,
            GuestInode {
                mode,
                kind: InodeKind::Regular {
                    sector_start: start,
                    byte_len: data.len(),
                },
            },
        );
        Ok(())
    }

    /// Write a regular file into the guest CoW filesystem (`0o100644` default permissions).
    pub fn write_guest_file(&mut self, path: &str, data: &[u8]) -> Result<(), MvmError> {
        let mode = if data.starts_with(b"\x7fELF") {
            0o100755
        } else {
            0o100644
        };
        self.write_inode_with_mode(path, data, mode)
    }

    /// Read a file (resolving up to 8 symlink hops) from the guest CoW filesystem.
    pub fn read_guest_file(&self, path: &str) -> Option<Vec<u8>> {
        let mut cur = Self::normalize_guest_path("/", path);
        for _ in 0..8 {
            let inode = self.inodes.get(&cur)?;
            match &inode.kind {
                InodeKind::Symlink { target } => {
                    cur = Self::normalize_guest_path("/", target);
                }
                InodeKind::Regular {
                    sector_start,
                    byte_len,
                } => {
                    let mut out = Vec::with_capacity(*byte_len);
                    let mut rem = *byte_len;
                    let mut sec = *sector_start;
                    while rem > 0 {
                        let sector = self.cow_sectors.get(&sec)?;
                        let take = rem.min(MVM_COW_SECTOR_BYTES);
                        out.extend_from_slice(&sector[0..take]);
                        rem -= take;
                        sec += 1;
                    }
                    return Some(out);
                }
            }
        }
        None
    }

    /// Synchronize host Studio `/workspace` text files into the guest `/workspace` directory,
    /// propagating Studio deletions without disturbing `.git` metadata or compiled ELF binaries.
    pub fn sync_from_workspace_map(&mut self, workspace_files: &BTreeMap<String, String>) {
        let mut clean_map: BTreeMap<String, &String> = BTreeMap::new();
        for (rel, content) in workspace_files {
            let clean = rel.trim_start_matches('/').to_string();
            if clean.is_empty() || is_git_internal_rel_path(&clean) {
                continue;
            }
            clean_map.insert(clean, content);
        }

        let removed: Vec<String> = self
            .synced_workspace_paths
            .iter()
            .filter(|old| !clean_map.contains_key(*old))
            .cloned()
            .collect();
        for old_rel in removed {
            let guest_path = format!("/workspace/{old_rel}");
            self.inodes.remove(&guest_path);
        }

        for (clean, content) in &clean_map {
            let guest_path = format!("/workspace/{clean}");
            let unchanged = self
                .read_guest_file(&guest_path)
                .map(|cur| cur == content.as_bytes())
                .unwrap_or(false);
            if !unchanged {
                let _ = self.write_guest_file(&guest_path, content.as_bytes());
            }
        }

        self.synced_workspace_paths = clean_map.keys().cloned().collect();
    }

    /// Export UTF-8 files under guest `/workspace` (excluding `.git` internals and ELF binaries)
    /// back to Studio's workspace map and update deletion tracking.
    pub fn export_workspace_text_files(&mut self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        let paths: Vec<String> = self.inodes.keys().cloned().collect();
        for path in paths {
            if let Some(rel) = path.strip_prefix("/workspace/") {
                if rel.is_empty() || is_git_internal_rel_path(rel) {
                    continue;
                }
                if let Some(bytes) = self.read_guest_file(&path) {
                    if !bytes.starts_with(b"\x7fELF") {
                        if let Ok(text) = String::from_utf8(bytes) {
                            out.insert(rel.to_string(), text);
                        }
                    }
                }
            }
        }
        self.synced_workspace_paths = out.keys().cloned().collect();
        out
    }

    fn remove_guest_path(&mut self, norm_path: &str, recursive: bool) -> Result<usize, String> {
        if norm_path == "/" || norm_path == "/bin" || norm_path == "/sbin" {
            return Err(format!("rm: refusing to remove protected path {norm_path}"));
        }
        let mut removed = 0usize;
        if self.inodes.remove(norm_path).is_some() {
            removed += 1;
        }
        if recursive {
            let prefix = format!("{norm_path}/");
            let child_files: Vec<String> = self
                .inodes
                .keys()
                .filter(|k| k.starts_with(&prefix))
                .cloned()
                .collect();
            for k in child_files {
                self.inodes.remove(&k);
                removed += 1;
            }
            let child_dirs: Vec<String> = self
                .dirs
                .iter()
                .filter(|d| *d == norm_path || d.starts_with(&prefix))
                .cloned()
                .collect();
            for d in child_dirs {
                if d != "/workspace" {
                    self.dirs.remove(&d);
                    removed += 1;
                }
            }
        } else if removed == 0 && self.dirs.contains(norm_path) {
            return Err(format!("rm: cannot remove '{norm_path}': Is a directory"));
        }
        Ok(removed)
    }

    /// Register a host-only credential placeholder (`{{MVM_SECRET:<name>}}`) restricted to `allowlisted_hosts`.
    /// Passing an empty `raw_secret` removes any previously registered credential entry so cleared secrets
    /// neither linger in `host_secrets` nor trigger `ensure_guest_ram_uncontaminated` on empty strings.
    pub fn register_host_secret(
        &mut self,
        name: &str,
        raw_secret: &str,
        allowlisted_hosts: &[&str],
    ) {
        if raw_secret.trim().is_empty() {
            self.host_secrets.remove(name);
            return;
        }
        let hosts = allowlisted_hosts.iter().map(|s| (*s).to_string()).collect();
        self.host_secrets.insert(
            name.to_string(),
            HostSecretRule {
                raw_secret: raw_secret.to_string(),
                allowlisted_hosts: hosts,
            },
        );
    }

    /// Check whether `needle` bytes appear anywhere in the most recently executed guest RAM image.
    pub fn guest_ram_contains(&self, needle: &[u8]) -> bool {
        if needle.is_empty() {
            return false;
        }
        self.last_guest_ram
            .windows(needle.len())
            .any(|w| w == needle)
    }

    /// Evaluate and perform host-side `SpiralCore` egress secret-placeholder substitution (`R4-37`).
    pub fn proxy_vsock_egress(
        &self,
        target_host: &str,
        request_template: &str,
    ) -> EgressProxyOutcome {
        let mut secret_in_guest = false;
        for rule in self.host_secrets.values() {
            if !rule.raw_secret.is_empty() && self.guest_ram_contains(rule.raw_secret.as_bytes()) {
                secret_in_guest = true;
                break;
            }
        }

        let mut matched_rule: Option<(&String, &HostSecretRule)> = None;
        for (name, rule) in &self.host_secrets {
            let marker = format!("{{{{MVM_SECRET:{name}}}}}");
            if request_template.contains(&marker) {
                matched_rule = Some((name, rule));
                break;
            }
        }

        let placeholder_matched = matched_rule.is_some();
        let host_allowlisted = matched_rule
            .map(|(_, r)| r.allowlisted_hosts.contains(target_host))
            .unwrap_or(false);

        let verdict = admit_host_egress(EgressPolicyCheck {
            host_allowlisted,
            placeholder_matched,
            secret_in_guest_ram: secret_in_guest,
        });

        if verdict == EgressVerdict::ProxyAdmitted {
            if let Some((name, rule)) = matched_rule {
                let marker = format!("{{{{MVM_SECRET:{name}}}}}");
                return EgressProxyOutcome {
                    verdict,
                    outbound_payload: Some(request_template.replace(&marker, &rule.raw_secret)),
                };
            }
        }
        EgressProxyOutcome {
            verdict,
            outbound_payload: None,
        }
    }

    /// Construct and validate a wire-encoded 16-byte `SpiralCore` `virtio-vsock` packet from this MicroVM.
    pub fn build_vsock_packet(
        &self,
        dst_cid: u16,
        vsock_port: u16,
        cl06_spin: u8,
        e8_orbit: u8,
        payload: &[u8],
    ) -> Result<Vec<u8>, MvmError> {
        let hdr = SpiralCoreHeader {
            magic: SPIRALCORE_MAGIC,
            src_cid: self.guest_cid,
            dst_cid,
            vsock_port,
            cl06_spin,
            e8_orbit,
            payload_len: payload.len() as u32,
        };
        let verdict = admit_spiralcore_header(&hdr, SPIRALCORE_MAX_PAYLOAD);
        if verdict != FrameVerdict::FrameAdmitted {
            return Err(MvmError::FrameRejected(verdict));
        }
        let mut wire = Vec::with_capacity(16 + payload.len());
        wire.extend_from_slice(&hdr.to_bytes());
        wire.extend_from_slice(payload);
        Ok(wire)
    }

    /// Deliver a wire-encoded 16-byte `SpiralCore` `virtio-vsock` packet into this MicroVM's port inbox.
    pub fn deliver_vsock_packet(&mut self, wire: &[u8]) -> Result<(), MvmError> {
        let Some(hdr) = SpiralCoreHeader::from_bytes(wire) else {
            return Err(MvmError::FrameRejected(FrameVerdict::RefusedInvalidCid));
        };
        let verdict = admit_spiralcore_header(&hdr, SPIRALCORE_MAX_PAYLOAD);
        if verdict != FrameVerdict::FrameAdmitted {
            return Err(MvmError::FrameRejected(verdict));
        }
        if hdr.dst_cid != self.guest_cid {
            return Err(MvmError::FrameRejected(FrameVerdict::RefusedInvalidCid));
        }
        let end = 16usize.saturating_add(hdr.payload_len as usize);
        if wire.len() < end {
            return Err(MvmError::FrameRejected(
                FrameVerdict::RefusedPayloadTooLarge,
            ));
        }
        let pkt = VsockPacket {
            header: hdr,
            payload: wire[16..end].to_vec(),
        };
        self.vsock_inbox
            .entry(hdr.vsock_port)
            .or_default()
            .push_back(pkt);
        Ok(())
    }

    /// Pop the next queued `SpiralCore` packet on `vsock_port`.
    pub fn pop_vsock_inbox(&mut self, vsock_port: u16) -> Option<VsockPacket> {
        self.vsock_inbox.get_mut(&vsock_port)?.pop_front()
    }

    /// Execute an interactive command or `&&`/`||`/`;` chain, persisting `cwd` across `cd`
    /// and suspending cleanly into `MvmInteractiveOutcome::NeedsBridge` when an outbound
    /// `SpiralCore` (`CID 3 -> CID 2`) `nslookup`/`dig`, `curl`/`wget`, `pkg install`, or
    /// `git clone`/`pull`/`push` operation is invoked (`R4-39`).
    pub fn exec_interactive(
        &mut self,
        cmd_line: &str,
        explicit_cwd: Option<&str>,
    ) -> Result<MvmInteractiveOutcome, MvmError> {
        let mut working_cwd = match explicit_cwd {
            Some(c) if !c.trim().is_empty() => Self::normalize_guest_path(&self.current_cwd, c),
            _ => self.current_cwd.clone(),
        };
        let segments = split_command_chain(cmd_line);
        self.run_interactive_segments(
            &segments,
            0,
            &mut working_cwd,
            String::new(),
            String::new(),
            0,
            0,
            Vec::new(),
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn run_interactive_segments(
        &mut self,
        segments: &[(ChainOp, String)],
        start_idx: usize,
        working_cwd: &mut String,
        mut total_stdout: String,
        mut total_stderr: String,
        mut last_exit: i32,
        mut total_insts: u64,
        mut all_frames: Vec<VsockPacket>,
        mut compiled_machine: Option<u16>,
    ) -> Result<MvmInteractiveOutcome, MvmError> {
        for idx in start_idx..segments.len() {
            let (op, seg) = &segments[idx];
            if *op == ChainOp::And && last_exit != 0 {
                continue;
            }
            if *op == ChainOp::Or && last_exit == 0 {
                continue;
            }
            match self.exec_interactive_segment(seg, working_cwd)? {
                MvmInteractiveOutcome::Completed(single) => {
                    total_stdout.push_str(&single.stdout);
                    total_stderr.push_str(&single.stderr);
                    last_exit = single.exit_code;
                    total_insts = total_insts.saturating_add(single.instructions_retired);
                    all_frames.extend(single.vsock_frames);
                    if single.compiled_elf_machine.is_some() {
                        compiled_machine = single.compiled_elf_machine;
                    }
                }
                MvmInteractiveOutcome::NeedsBridge(mut pending) => {
                    self.current_cwd = working_cwd.clone();
                    let remaining: Vec<serde_json::Value> = segments[idx + 1..]
                        .iter()
                        .map(|(cop, cseg)| {
                            let op_str = match cop {
                                ChainOp::First => "first",
                                ChainOp::Semi => "semi",
                                ChainOp::And => "and",
                                ChainOp::Or => "or",
                            };
                            serde_json::json!({ "op": op_str, "seg": cseg })
                        })
                        .collect();
                    if let Some(obj) = pending.resume_state.as_object_mut() {
                        obj.insert(
                            "working_cwd".to_string(),
                            serde_json::Value::String(working_cwd.clone()),
                        );
                        obj.insert(
                            "stdout_prefix".to_string(),
                            serde_json::Value::String(total_stdout),
                        );
                        obj.insert(
                            "stderr_prefix".to_string(),
                            serde_json::Value::String(total_stderr),
                        );
                        obj.insert(
                            "instructions_prefix".to_string(),
                            serde_json::Value::Number(total_insts.into()),
                        );
                        obj.insert(
                            "remaining_segments".to_string(),
                            serde_json::Value::Array(remaining),
                        );
                    }
                    all_frames.extend(pending.vsock_frames.clone());
                    pending.vsock_frames = all_frames;
                    return Ok(MvmInteractiveOutcome::NeedsBridge(pending));
                }
            }
        }
        self.current_cwd = working_cwd.clone();
        Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
            exit_code: last_exit,
            stdout: total_stdout,
            stderr: total_stderr,
            instructions_retired: total_insts,
            vsock_frames: all_frames,
            compiled_elf_machine: compiled_machine,
            cow_state_sha256: self.cow_state_sha256(),
        }))
    }

    /// Resume a suspended `NeedsBridge` command once the host Worker delivers `bridge_resp` (`R4-39`).
    pub fn complete_bridge(
        &mut self,
        resume_state: &serde_json::Value,
        bridge_resp: &serde_json::Value,
    ) -> Result<MvmInteractiveOutcome, MvmError> {
        self.ensure_guest_ram_uncontaminated()?;

        let bridge_kind = resume_state
            .get("bridge_kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                MvmError::BridgeRejected(BridgeError::InvalidBridgeJson(
                    "missing resume_state.bridge_kind".to_string(),
                ))
            })?;
        let mut working_cwd = resume_state
            .get("working_cwd")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| self.current_cwd.clone());
        let mut total_stdout = resume_state
            .get("stdout_prefix")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let mut total_stderr = resume_state
            .get("stderr_prefix")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let total_insts = resume_state
            .get("instructions_prefix")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let redirect = resume_state
            .get("redirect_path")
            .and_then(|v| v.as_str())
            .map(|p| {
                let append = resume_state
                    .get("redirect_append")
                    .and_then(|a| a.as_bool())
                    .unwrap_or(false);
                (p.to_string(), append)
            });

        if bridge_resp.get("ok") == Some(&serde_json::Value::Bool(false)) {
            let code = bridge_resp
                .get("error_code")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match code {
                "PolicyDenied" => return Err(MvmError::BridgeRejected(BridgeError::PolicyDenied)),
                "InsecureTransportRejected" => {
                    return Err(MvmError::BridgeRejected(
                        BridgeError::InsecureTransportRejected,
                    ))
                }
                "InvalidVsockCid" => {
                    return Err(MvmError::BridgeRejected(BridgeError::InvalidVsockCid))
                }
                "GuestRamSecretContamination" => {
                    return Err(MvmError::BridgeRejected(
                        BridgeError::GuestRamSecretContamination,
                    ))
                }
                "HostNotAllowlistedForSecret" => {
                    return Err(MvmError::BridgeRejected(
                        BridgeError::HostNotAllowlistedForSecret,
                    ))
                }
                "MissingSecretPlaceholder" => {
                    return Err(MvmError::BridgeRejected(
                        BridgeError::MissingSecretPlaceholder,
                    ))
                }
                "PackageDigestMismatch" | "DigestMismatch" => {
                    return Err(MvmError::BridgeRejected(BridgeError::PackageDigestMismatch))
                }
                "PackageInvalidElf64" | "InvalidElf64Riscv" => {
                    return Err(MvmError::BridgeRejected(BridgeError::PackageInvalidElf64))
                }
                "PackageSizeExceedsQuota" | "SizeExceedsQuota" => {
                    return Err(MvmError::BridgeRejected(
                        BridgeError::PackageSizeExceedsQuota,
                    ))
                }
                _ => {
                    let err_msg = bridge_resp
                        .get("error_message")
                        .or_else(|| bridge_resp.get("message"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("SpiralCore relay returned error");
                    total_stderr.push_str(err_msg);
                    if !total_stderr.ends_with('\n') {
                        total_stderr.push('\n');
                    }
                    self.current_cwd = working_cwd;
                    return Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                        exit_code: 1,
                        stdout: total_stdout,
                        stderr: total_stderr,
                        instructions_retired: total_insts,
                        vsock_frames: Vec::new(),
                        compiled_elf_machine: None,
                        cow_state_sha256: self.cow_state_sha256(),
                    }));
                }
            }
        }

        let data = bridge_resp
            .get("data")
            .filter(|d| d.is_object())
            .unwrap_or(bridge_resp);

        let (seg_exit, seg_stdout, seg_stderr) = match bridge_kind {
            "dns_query" => {
                let qname = resume_state
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let mut out = format!("Server:\t\t1.1.1.1 (SpiralCore DoH)\nName:\t{qname}\n");
                if let Some(answers) = data.get("answers").and_then(|v| v.as_array()) {
                    for ans in answers {
                        let rdata = ans
                            .get("data")
                            .and_then(|v| v.as_str())
                            .unwrap_or("0.0.0.0");
                        let rtype = ans.get("type_name").and_then(|v| v.as_str()).unwrap_or("A");
                        out.push_str(&format!("Address ({rtype}): {rdata}\n"));
                    }
                } else if let Some(ip) = data.get("address").and_then(|v| v.as_str()) {
                    out.push_str(&format!("Address: {ip}\n"));
                }
                (0, out, String::new())
            }
            "http_request" => {
                let raw_bytes = extract_bridge_payload_bytes(data, bridge_resp);
                if let Some(out_file) = resume_state.get("output_file").and_then(|v| v.as_str()) {
                    let norm_out = Self::normalize_guest_path(&working_cwd, out_file);
                    self.write_guest_file(&norm_out, &raw_bytes)?;
                    (
                        0,
                        format!("Saved {} bytes to {norm_out}\n", raw_bytes.len()),
                        String::new(),
                    )
                } else {
                    let mut body_str = String::from_utf8_lossy(&raw_bytes).into_owned();
                    if !body_str.is_empty() && !body_str.ends_with('\n') {
                        body_str.push('\n');
                    }
                    (0, body_str, String::new())
                }
            }
            "pkg_fetch" => {
                let pkg_name = resume_state
                    .get("pkg_name")
                    .and_then(|v| v.as_str())
                    .or_else(|| data.get("pkg_name").and_then(|v| v.as_str()))
                    .or_else(|| data.get("name").and_then(|v| v.as_str()))
                    .or_else(|| bridge_resp.get("name").and_then(|v| v.as_str()))
                    .unwrap_or("pkg-bin");
                let expected_sha = resume_state
                    .get("expected_sha256")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .or_else(|| data.get("expected_sha256").and_then(|v| v.as_str()))
                    .or_else(|| bridge_resp.get("expected_sha256").and_then(|v| v.as_str()))
                    .or_else(|| data.get("sha256").and_then(|v| v.as_str()))
                    .or_else(|| bridge_resp.get("sha256").and_then(|v| v.as_str()))
                    .unwrap_or("");
                let elf_bytes = extract_bridge_elf_bytes(data, bridge_resp)?;
                let actual_sha = sha256_hex_bytes(&elf_bytes);
                let sha_matched =
                    !expected_sha.is_empty() && actual_sha.eq_ignore_ascii_case(expected_sha);
                let elf_magic_ok = elf_bytes.len() >= 64 && &elf_bytes[0..4] == b"\x7fELF";
                let elf_class_64 = elf_bytes.len() >= 64 && elf_bytes[4] == 2 && elf_bytes[5] == 1;
                let machine_code = if elf_bytes.len() >= 20 {
                    u16::from_le_bytes([elf_bytes[18], elf_bytes[19]])
                } else {
                    0
                };
                let byte_len = elf_bytes.len().min(u32::MAX as usize) as u32;
                let verdict = admit_package_install(
                    sha_matched,
                    elf_magic_ok,
                    elf_class_64,
                    machine_code,
                    byte_len,
                    4 * 1024 * 1024,
                );
                match verdict {
                    PackageInstallVerdict::DigestMismatch => {
                        return Err(MvmError::BridgeRejected(BridgeError::PackageDigestMismatch))
                    }
                    PackageInstallVerdict::InvalidElf64Riscv => {
                        return Err(MvmError::BridgeRejected(BridgeError::PackageInvalidElf64))
                    }
                    PackageInstallVerdict::SizeExceedsQuota => {
                        return Err(MvmError::BridgeRejected(
                            BridgeError::PackageSizeExceedsQuota,
                        ))
                    }
                    PackageInstallVerdict::PackageAdmitted => {}
                }
                if admit_elf64_riscv(&elf_bytes) != ElfAdmission::ElfAdmitted {
                    return Err(MvmError::BridgeRejected(BridgeError::PackageInvalidElf64));
                }
                let bin_path = format!("/usr/local/bin/{pkg_name}");
                self.write_inode_with_mode(&bin_path, &elf_bytes, 0o100755)?;
                let source = resume_state
                    .get("source_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("spiralcore-pkg")
                    .to_string();
                self.installed_packages.insert(
                    pkg_name.to_string(),
                    InstalledPackageMeta {
                        name: pkg_name.to_string(),
                        sha256: actual_sha.clone(),
                        byte_len,
                        source,
                    },
                );
                if let Ok(cat_bytes) = serde_json::to_vec(&self.installed_packages) {
                    let _ = self.write_guest_file("/etc/pkg/installed.json", &cat_bytes);
                }
                (
                    0,
                    format!(
                        "Installed {pkg_name} ({byte_len} bytes, sha256:{}) -> {bin_path}\n",
                        &actual_sha[..12]
                    ),
                    String::new(),
                )
            }
            "git_smart_http" => {
                let git_action = resume_state
                    .get("git_action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("clone");
                let repo_url = resume_state
                    .get("repo_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let target_dir = resume_state
                    .get("target_dir")
                    .and_then(|v| v.as_str())
                    .unwrap_or("/workspace");
                self.complete_git_bridge(git_action, repo_url, target_dir, data, bridge_resp)?
            }
            other => {
                return Err(MvmError::BridgeRejected(BridgeError::InvalidBridgeJson(
                    format!("unsupported bridge_kind `{other}`"),
                )))
            }
        };

        self.ensure_guest_ram_uncontaminated()?;

        if let Some((redir_path, append)) = redirect {
            self.write_redirected_stdout(&working_cwd, &redir_path, &seg_stdout, append)?;
        } else {
            total_stdout.push_str(&seg_stdout);
        }
        total_stderr.push_str(&seg_stderr);

        let mut remaining_segments: Vec<(ChainOp, String)> = Vec::new();
        if let Some(arr) = resume_state
            .get("remaining_segments")
            .and_then(|v| v.as_array())
        {
            for item in arr {
                let op_s = item.get("op").and_then(|v| v.as_str()).unwrap_or("semi");
                let seg_s = item.get("seg").and_then(|v| v.as_str()).unwrap_or("");
                let cop = match op_s {
                    "and" => ChainOp::And,
                    "or" => ChainOp::Or,
                    "first" => ChainOp::First,
                    _ => ChainOp::Semi,
                };
                remaining_segments.push((cop, seg_s.to_string()));
            }
        }

        self.run_interactive_segments(
            &remaining_segments,
            0,
            &mut working_cwd,
            total_stdout,
            total_stderr,
            seg_exit,
            total_insts,
            Vec::new(),
            None,
        )
    }

    /// Execute a command or `&&`/`||`/`;` chain inside the `MVM-Wasm` RV64IMAC Linux MicroVM.
    pub fn exec_command(&mut self, cmd_line: &str, cwd: &str) -> Result<MvmExecOutcome, MvmError> {
        let mut working_cwd = Self::normalize_guest_path("/workspace", cwd);
        let segments = split_command_chain(cmd_line);
        let mut total_stdout = String::new();
        let mut total_stderr = String::new();
        let mut last_exit = 0i32;
        let mut total_insts = 0u64;
        let mut all_frames = Vec::new();
        let mut compiled_machine = None;

        for (op, seg) in segments {
            if op == ChainOp::And && last_exit != 0 {
                continue;
            }
            if op == ChainOp::Or && last_exit == 0 {
                continue;
            }
            let single = self.exec_single_segment_mut_cwd(&seg, &mut working_cwd)?;
            total_stdout.push_str(&single.stdout);
            total_stderr.push_str(&single.stderr);
            last_exit = single.exit_code;
            total_insts = total_insts.saturating_add(single.instructions_retired);
            all_frames.extend(single.vsock_frames);
            if single.compiled_elf_machine.is_some() {
                compiled_machine = single.compiled_elf_machine;
            }
        }

        Ok(MvmExecOutcome {
            exit_code: last_exit,
            stdout: total_stdout,
            stderr: total_stderr,
            instructions_retired: total_insts,
            vsock_frames: all_frames,
            compiled_elf_machine: compiled_machine,
            cow_state_sha256: self.cow_state_sha256(),
        })
    }

    fn exec_interactive_segment(
        &mut self,
        seg: &str,
        working_cwd: &mut String,
    ) -> Result<MvmInteractiveOutcome, MvmError> {
        let trimmed = seg.trim();
        if trimmed.is_empty() {
            return Ok(MvmInteractiveOutcome::Completed(self.empty_outcome()));
        }
        if !contains_unquoted_pipe(trimmed) {
            let (words, redirect) = split_shell_words_with_redirect(trimmed);
            if !words.is_empty() {
                match words[0].as_str() {
                    "nslookup" | "dig" => {
                        let pending = self.prepare_dns_bridge(&words, working_cwd, redirect)?;
                        return Ok(MvmInteractiveOutcome::NeedsBridge(pending));
                    }
                    "curl" | "wget" => {
                        let pending = self.prepare_http_bridge(&words, working_cwd, redirect)?;
                        return Ok(MvmInteractiveOutcome::NeedsBridge(pending));
                    }
                    "pkg" => {
                        return self.exec_pkg_command(&words, working_cwd, redirect);
                    }
                    "git" => {
                        return self.exec_git_command(&words, working_cwd, redirect);
                    }
                    _ => {}
                }
            }
        }
        let outcome = self.exec_single_segment_mut_cwd(seg, working_cwd)?;
        Ok(MvmInteractiveOutcome::Completed(outcome))
    }

    fn empty_outcome(&self) -> MvmExecOutcome {
        MvmExecOutcome {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            instructions_retired: 0,
            vsock_frames: Vec::new(),
            compiled_elf_machine: None,
            cow_state_sha256: self.cow_state_sha256(),
        }
    }

    fn write_redirected_stdout(
        &mut self,
        cwd: &str,
        raw_target: &str,
        stdout: &str,
        append: bool,
    ) -> Result<(), MvmError> {
        let target_path = Self::normalize_guest_path(cwd, raw_target);
        if append {
            let mut buf = self.read_guest_file(&target_path).unwrap_or_default();
            buf.extend_from_slice(stdout.as_bytes());
            self.write_guest_file(&target_path, &buf)
        } else {
            self.write_guest_file(&target_path, stdout.as_bytes())
        }
    }

    fn exec_single_segment_mut_cwd(
        &mut self,
        seg: &str,
        working_cwd: &mut String,
    ) -> Result<MvmExecOutcome, MvmError> {
        let trimmed = seg.trim();
        if trimmed.is_empty() {
            return Ok(self.empty_outcome());
        }

        // If pipeline `|`, execute sequentially when any stage resolves outside built-in `/bin` applets
        // (e.g. `/usr/local/bin/<pkg>` installed via `pkg install`), otherwise use `/bin/mvmbox sh -c`.
        if contains_unquoted_pipe(trimmed) {
            let stages = split_unquoted_pipes(trimmed);
            let needs_sandbox_pipe = stages.len() >= 2
                && stages.iter().any(|st| {
                    let (st_words, _) = split_shell_words_with_redirect(st.trim());
                    let Some(prog) = st_words.first().map(String::as_str) else {
                        return false;
                    };
                    if matches!(prog, "cd" | "rm" | "cc" | "gcc" | "pkg" | "git") {
                        return true;
                    }
                    match self.resolve_guest_exec_path(working_cwd, prog) {
                        Some(p) => {
                            !p.starts_with("/bin/")
                                && !p.starts_with("/sbin/")
                                && !p.starts_with("/usr/bin/")
                        }
                        None => true,
                    }
                });
            if needs_sandbox_pipe {
                let mut piped_stdin: Vec<u8> = Vec::new();
                let mut total_stderr = String::new();
                let mut total_insts = 0u64;
                let mut all_frames = Vec::new();
                let mut last_exit = 0i32;
                let mut compiled_machine = None;

                for (idx, stage_raw) in stages.iter().enumerate() {
                    let stage_trimmed = stage_raw.trim();
                    if stage_trimmed.is_empty() {
                        continue;
                    }
                    let stage_out = if idx == 0 {
                        self.exec_single_segment_mut_cwd(stage_trimmed, working_cwd)?
                    } else {
                        let (st_words, st_redirect) =
                            split_shell_words_with_redirect(stage_trimmed);
                        if st_words.is_empty() {
                            continue;
                        }
                        let prog = &st_words[0];
                        let Some(exec_path) = self.resolve_guest_exec_path(working_cwd, prog)
                        else {
                            total_stderr.push_str(&format!("mvm-sh: command not found: {prog}\n"));
                            last_exit = 127;
                            piped_stdin.clear();
                            continue;
                        };
                        let mut out = self.run_elf_process_with_stdin(
                            &exec_path,
                            &st_words,
                            working_cwd,
                            &piped_stdin,
                        )?;
                        if let Some((redir_target, append)) = st_redirect {
                            self.write_redirected_stdout(
                                working_cwd,
                                &redir_target,
                                &out.stdout,
                                append,
                            )?;
                            out.stdout.clear();
                        }
                        out
                    };

                    total_stderr.push_str(&stage_out.stderr);
                    total_insts = total_insts.saturating_add(stage_out.instructions_retired);
                    all_frames.extend(stage_out.vsock_frames);
                    last_exit = stage_out.exit_code;
                    if stage_out.compiled_elf_machine.is_some() {
                        compiled_machine = stage_out.compiled_elf_machine;
                    }
                    piped_stdin = stage_out.stdout.into_bytes();
                }

                return Ok(MvmExecOutcome {
                    exit_code: last_exit,
                    stdout: String::from_utf8_lossy(&piped_stdin).into_owned(),
                    stderr: total_stderr,
                    instructions_retired: total_insts,
                    vsock_frames: all_frames,
                    compiled_elf_machine: compiled_machine,
                    cow_state_sha256: self.cow_state_sha256(),
                });
            }

            let argv = vec!["/bin/sh".to_string(), "-c".to_string(), trimmed.to_string()];
            return self.run_elf_process("/bin/sh", &argv, working_cwd);
        }

        let (words, redirect) = split_shell_words_with_redirect(trimmed);
        if words.is_empty() {
            return Ok(self.empty_outcome());
        }

        // Shell built-in `cd`
        if words[0] == "cd" {
            let target = words.get(1).map(String::as_str).unwrap_or("/workspace");
            let next_dir = Self::normalize_guest_path(working_cwd, target);
            let dir_prefix = if next_dir == "/" {
                "/".to_string()
            } else {
                format!("{next_dir}/")
            };
            let exists = self.dirs.contains(&next_dir)
                || self.inodes.keys().any(|k| k.starts_with(&dir_prefix));
            if exists {
                *working_cwd = next_dir;
                return Ok(self.empty_outcome());
            }
            return Ok(MvmExecOutcome {
                exit_code: 1,
                stdout: String::new(),
                stderr: format!("cd: no such file or directory: {target}\n"),
                instructions_retired: 0,
                vsock_frames: Vec::new(),
                compiled_elf_machine: None,
                cow_state_sha256: self.cow_state_sha256(),
            });
        }

        // Built-in `rm` supporting `-r`, `-f`, `-rf` for files and directories
        if words[0] == "rm" {
            let mut recursive = false;
            let mut force = false;
            let mut targets = Vec::new();
            for arg in &words[1..] {
                if arg.starts_with('-') && arg.len() > 1 {
                    for ch in arg[1..].chars() {
                        match ch {
                            'r' | 'R' => recursive = true,
                            'f' => force = true,
                            _ => {}
                        }
                    }
                } else {
                    targets.push(arg.as_str());
                }
            }
            if targets.is_empty() {
                return Ok(MvmExecOutcome {
                    exit_code: if force { 0 } else { 1 },
                    stdout: String::new(),
                    stderr: if force {
                        String::new()
                    } else {
                        "rm: missing operand\n".to_string()
                    },
                    instructions_retired: 0,
                    vsock_frames: Vec::new(),
                    compiled_elf_machine: None,
                    cow_state_sha256: self.cow_state_sha256(),
                });
            }
            let mut err_out = String::new();
            let mut code = 0i32;
            for raw_t in targets {
                let norm = Self::normalize_guest_path(working_cwd, raw_t);
                match self.remove_guest_path(&norm, recursive) {
                    Ok(0) if !force => {
                        code = 1;
                        err_out.push_str(&format!(
                            "rm: cannot remove '{raw_t}': No such file or directory\n"
                        ));
                    }
                    Ok(_) => {}
                    Err(msg) => {
                        code = 1;
                        err_out.push_str(&msg);
                        err_out.push('\n');
                    }
                }
            }
            return Ok(MvmExecOutcome {
                exit_code: code,
                stdout: String::new(),
                stderr: err_out,
                instructions_retired: 0,
                vsock_frames: Vec::new(),
                compiled_elf_machine: None,
                cow_state_sha256: self.cow_state_sha256(),
            });
        }

        // Built-in C-to-RV64IM ELF64 compiler (`cc` / `gcc`)
        if words[0] == "cc" || words[0] == "gcc" || words[0] == "/usr/bin/cc" {
            let mut src_arg: Option<String> = None;
            let mut out_arg = "a.out".to_string();
            let mut i = 1usize;
            while i < words.len() {
                if words[i] == "-o" && i + 1 < words.len() {
                    out_arg = words[i + 1].clone();
                    i += 2;
                    continue;
                }
                if !words[i].starts_with('-') && src_arg.is_none() {
                    src_arg = Some(words[i].clone());
                }
                i += 1;
            }
            let Some(raw_src) = src_arg else {
                return Ok(MvmExecOutcome {
                    exit_code: 1,
                    stdout: String::new(),
                    stderr: "cc: missing input C source file\n".to_string(),
                    instructions_retired: 0,
                    vsock_frames: Vec::new(),
                    compiled_elf_machine: None,
                    cow_state_sha256: self.cow_state_sha256(),
                });
            };
            let src_path = Self::normalize_guest_path(working_cwd, &raw_src);
            let out_path = Self::normalize_guest_path(working_cwd, &out_arg);
            let Some(src_bytes) = self.read_guest_file(&src_path) else {
                return Ok(MvmExecOutcome {
                    exit_code: 1,
                    stdout: String::new(),
                    stderr: format!("cc: file not found: {src_path}\n"),
                    instructions_retired: 0,
                    vsock_frames: Vec::new(),
                    compiled_elf_machine: None,
                    cow_state_sha256: self.cow_state_sha256(),
                });
            };
            let src_text = String::from_utf8_lossy(&src_bytes);
            match compile_c_to_rv64_elf(&src_text) {
                Ok(elf_bytes) => {
                    let adm = admit_elf64_riscv(&elf_bytes);
                    if adm != ElfAdmission::ElfAdmitted {
                        return Err(MvmError::ElfRejected(adm));
                    }
                    let machine = u16::from_le_bytes([elf_bytes[18], elf_bytes[19]]);
                    self.write_inode_with_mode(&out_path, &elf_bytes, 0o100755)?;
                    return Ok(MvmExecOutcome {
                        exit_code: 0,
                        stdout: String::new(),
                        stderr: String::new(),
                        instructions_retired: 0,
                        vsock_frames: Vec::new(),
                        compiled_elf_machine: Some(machine),
                        cow_state_sha256: self.cow_state_sha256(),
                    });
                }
                Err(diag) => {
                    return Ok(MvmExecOutcome {
                        exit_code: 1,
                        stdout: String::new(),
                        stderr: format!("{diag}\n"),
                        instructions_retired: 0,
                        vsock_frames: Vec::new(),
                        compiled_elf_machine: None,
                        cow_state_sha256: self.cow_state_sha256(),
                    });
                }
            }
        }

        // Local synchronous `pkg` / `git` invocations via `exec_command`
        if words[0] == "pkg" || words[0] == "git" {
            let res = if words[0] == "pkg" {
                self.exec_pkg_command(&words, working_cwd, redirect)?
            } else {
                self.exec_git_command(&words, working_cwd, redirect)?
            };
            if let MvmInteractiveOutcome::Completed(done) = res {
                return Ok(done);
            }
            return Ok(MvmExecOutcome {
                exit_code: 1,
                stdout: String::new(),
                stderr: format!(
                    "{}: remote operation requires interactive SpiralCore bridge\n",
                    words[0]
                ),
                instructions_retired: 0,
                vsock_frames: Vec::new(),
                compiled_elf_machine: None,
                cow_state_sha256: self.cow_state_sha256(),
            });
        }

        // Resolve executable path in guest filesystem (including `/usr/local/bin`)
        let prog = &words[0];
        let Some(exec_path) = self.resolve_guest_exec_path(working_cwd, prog) else {
            return Ok(MvmExecOutcome {
                exit_code: 127,
                stdout: String::new(),
                stderr: format!("mvm-sh: command not found: {prog}\n"),
                instructions_retired: 0,
                vsock_frames: Vec::new(),
                compiled_elf_machine: None,
                cow_state_sha256: self.cow_state_sha256(),
            });
        };

        let mut out = self.run_elf_process(&exec_path, &words, working_cwd)?;
        if let Some((redir_target, append)) = redirect {
            self.write_redirected_stdout(working_cwd, &redir_target, &out.stdout, append)?;
            out.stdout.clear();
            out.cow_state_sha256 = self.cow_state_sha256();
        }
        Ok(out)
    }

    fn resolve_guest_exec_path(&self, working_cwd: &str, prog: &str) -> Option<String> {
        if prog.starts_with('/') || prog.starts_with("./") || prog.starts_with("../") {
            let p = Self::normalize_guest_path(working_cwd, prog);
            self.inodes.contains_key(&p).then_some(p)
        } else {
            [
                format!("/usr/local/bin/{prog}"),
                format!("/bin/{prog}"),
                format!("/sbin/{prog}"),
                format!("/usr/bin/{prog}"),
                format!("/workspace/{prog}"),
            ]
            .into_iter()
            .find(|cand| self.inodes.contains_key(cand))
        }
    }

    fn load_elf_into_process(
        &mut self,
        proc_state: &mut Rv64Process,
        exec_path: &str,
        argv: &[String],
        envp: &[String],
    ) -> Result<(), MvmError> {
        let Some(elf) = self.read_guest_file(exec_path) else {
            return Err(MvmError::NotFound(exec_path.to_string()));
        };
        let adm = admit_elf64_riscv(&elf);
        if adm != ElfAdmission::ElfAdmitted {
            return Err(MvmError::ElfRejected(adm));
        }

        proc_state.ram.fill(0);
        proc_state.regs = [0u64; 32];
        proc_state.lr_reservation = None;

        let entry = u64::from_le_bytes([
            elf[24], elf[25], elf[26], elf[27], elf[28], elf[29], elf[30], elf[31],
        ]);
        let phoff = u64::from_le_bytes([
            elf[32], elf[33], elf[34], elf[35], elf[36], elf[37], elf[38], elf[39],
        ]) as usize;
        let phentsize = u16::from_le_bytes([elf[54], elf[55]]) as usize;
        let phnum = u16::from_le_bytes([elf[56], elf[57]]) as usize;

        let mut max_end = 0x20000u64;
        for i in 0..phnum {
            let off = phoff + i * phentsize;
            let p_type = u32::from_le_bytes([elf[off], elf[off + 1], elf[off + 2], elf[off + 3]]);
            if p_type != 1 {
                continue;
            }
            let p_offset = u64::from_le_bytes([
                elf[off + 8],
                elf[off + 9],
                elf[off + 10],
                elf[off + 11],
                elf[off + 12],
                elf[off + 13],
                elf[off + 14],
                elf[off + 15],
            ]) as usize;
            let p_vaddr = u64::from_le_bytes([
                elf[off + 16],
                elf[off + 17],
                elf[off + 18],
                elf[off + 19],
                elf[off + 20],
                elf[off + 21],
                elf[off + 22],
                elf[off + 23],
            ]);
            let p_filesz = u64::from_le_bytes([
                elf[off + 32],
                elf[off + 33],
                elf[off + 34],
                elf[off + 35],
                elf[off + 36],
                elf[off + 37],
                elf[off + 38],
                elf[off + 39],
            ]) as usize;
            let p_memsz = u64::from_le_bytes([
                elf[off + 40],
                elf[off + 41],
                elf[off + 42],
                elf[off + 43],
                elf[off + 44],
                elf[off + 45],
                elf[off + 46],
                elf[off + 47],
            ]);

            if p_offset.saturating_add(p_filesz) > elf.len() {
                return Err(MvmError::ElfRejected(ElfAdmission::RefusedSegmentOverflow));
            }
            proc_state.write_bytes(p_vaddr, &elf[p_offset..p_offset + p_filesz])?;
            let seg_end = p_vaddr.saturating_add(p_memsz);
            if seg_end > max_end {
                max_end = seg_end;
            }
        }

        proc_state.brk_addr = (max_end + 4095) & !4095;
        proc_state.mmap_cursor = GUEST_MMAP_BASE;

        // Build SysV RISC-V initial user stack: [argc, argv..., 0, envp..., 0, auxv_null(0,0)]
        let mut sp = GUEST_STACK_TOP;
        let mut env_ptrs = Vec::with_capacity(envp.len());
        for e in envp {
            let bytes = e.as_bytes();
            sp -= (bytes.len() as u64) + 1;
            proc_state.write_bytes(sp, bytes)?;
            proc_state.write_bytes(sp + (bytes.len() as u64), &[0])?;
            env_ptrs.push(sp);
        }
        let mut arg_ptrs = Vec::with_capacity(argv.len());
        for a in argv {
            let bytes = a.as_bytes();
            sp -= (bytes.len() as u64) + 1;
            proc_state.write_bytes(sp, bytes)?;
            proc_state.write_bytes(sp + (bytes.len() as u64), &[0])?;
            arg_ptrs.push(sp);
        }
        sp &= !15;
        let table_words = 1 + arg_ptrs.len() + 1 + env_ptrs.len() + 1 + 2;
        if !table_words.is_multiple_of(2) {
            sp -= 8;
        }
        // AT_NULL (0, 0)
        sp -= 16;
        proc_state.write_bytes(sp, &0u64.to_le_bytes())?;
        proc_state.write_bytes(sp + 8, &0u64.to_le_bytes())?;
        // envp NULL terminator + pointers
        sp -= 8;
        proc_state.write_bytes(sp, &0u64.to_le_bytes())?;
        for &ep in env_ptrs.iter().rev() {
            sp -= 8;
            proc_state.write_bytes(sp, &ep.to_le_bytes())?;
        }
        // argv NULL terminator + pointers
        sp -= 8;
        proc_state.write_bytes(sp, &0u64.to_le_bytes())?;
        for &ap in arg_ptrs.iter().rev() {
            sp -= 8;
            proc_state.write_bytes(sp, &ap.to_le_bytes())?;
        }
        // argc
        sp -= 8;
        proc_state.write_bytes(sp, &(argv.len() as u64).to_le_bytes())?;

        proc_state.regs[2] = sp; // sp
        proc_state.pc = entry;
        Ok(())
    }

    fn run_elf_process(
        &mut self,
        exec_path: &str,
        argv: &[String],
        cwd: &str,
    ) -> Result<MvmExecOutcome, MvmError> {
        self.run_elf_process_with_stdin(exec_path, argv, cwd, &[])
    }

    fn run_elf_process_with_stdin(
        &mut self,
        exec_path: &str,
        argv: &[String],
        cwd: &str,
        stdin_bytes: &[u8],
    ) -> Result<MvmExecOutcome, MvmError> {
        let stdout_buf = Rc::new(RefCell::new(Vec::new()));
        let stderr_buf = Rc::new(RefCell::new(Vec::new()));
        let stdin_buf = Rc::new(RefCell::new(VecDeque::from(stdin_bytes.to_vec())));

        let pid = self.next_pid;
        self.next_pid += 1;

        let mut fds = BTreeMap::new();
        fds.insert(0, FdKind::StdinBuffer(stdin_buf));
        fds.insert(1, FdKind::StdoutCapture(stdout_buf.clone()));
        fds.insert(2, FdKind::StderrCapture(stderr_buf.clone()));

        let mut root_proc = Rv64Process {
            pid,
            regs: [0u64; 32],
            pc: 0,
            ram: vec![0u8; GUEST_RAM_BYTES],
            brk_addr: 0x20000,
            mmap_cursor: GUEST_MMAP_BASE,
            lr_reservation: None,
            fds,
            next_fd: 3,
            cwd: cwd.to_string(),
            exited: None,
        };

        let envp = vec![
            "PATH=/usr/local/bin:/bin:/sbin:/usr/bin:/workspace".to_string(),
            "USER=lexlean".to_string(),
            "HOME=/workspace".to_string(),
        ];
        self.load_elf_into_process(&mut root_proc, exec_path, argv, &envp)?;

        let mut procs: Vec<Rv64Process> = vec![root_proc];
        let mut active_idx = 0usize;
        let mut fuel_used = 0u64;
        let mut vsock_frames = Vec::new();

        while fuel_used < MAX_INSTRUCTION_FUEL {
            if procs[active_idx].exited.is_some() {
                if let Some(next_runnable) = procs.iter().position(|p| p.exited.is_none()) {
                    active_idx = next_runnable;
                } else {
                    break;
                }
            }

            let trap_or_sys = self.step_process(&mut procs[active_idx], &mut fuel_used)?;
            if let Some(sys_event) = trap_or_sys {
                match sys_event {
                    StepEvent::CloneChild => {
                        let mut child = procs[active_idx].clone();
                        let child_pid = self.next_pid;
                        self.next_pid += 1;
                        child.pid = child_pid;
                        child.regs[10] = 0; // child sees 0
                        procs[active_idx].regs[10] = child_pid as u64;
                        let child_idx = procs.len();
                        procs.push(child);
                        // Run child first so pipeline writers/execve populate pipes before parent wait4
                        active_idx = child_idx;
                    }
                    StepEvent::Wait4 {
                        target_pid,
                        status_ptr,
                    } => {
                        let found_idx = procs.iter().position(|p| {
                            p.pid != procs[active_idx].pid
                                && (target_pid <= 0 || p.pid == target_pid)
                        });
                        if let Some(cidx) = found_idx {
                            if let Some(code) = procs[cidx].exited {
                                let cpid = procs[cidx].pid;
                                if status_ptr != 0 {
                                    let encoded = ((code & 0xff) << 8) as u32;
                                    procs[active_idx]
                                        .write_bytes(status_ptr, &encoded.to_le_bytes())?;
                                }
                                procs.remove(cidx);
                                if active_idx > cidx {
                                    active_idx -= 1;
                                }
                                procs[active_idx].regs[10] = cpid as u64;
                            } else {
                                // Suspend parent at wait4 ecall and switch to runnable child
                                procs[active_idx].pc -= 4;
                                active_idx = cidx;
                            }
                        } else {
                            procs[active_idx].regs[10] = (-10i64) as u64; // -ECHILD
                        }
                    }
                    StepEvent::VsockPacketEmitted(pkt) => {
                        vsock_frames.push(pkt);
                    }
                }
            }
        }

        if fuel_used >= MAX_INSTRUCTION_FUEL {
            return Err(MvmError::CpuTrap(
                "instruction fuel budget exceeded".to_string(),
            ));
        }

        self.last_guest_ram = procs[0].ram.clone();
        let exit_code = procs[0].exited.unwrap_or(0);
        let stdout = String::from_utf8_lossy(&stdout_buf.borrow()).into_owned();
        let stderr = String::from_utf8_lossy(&stderr_buf.borrow()).into_owned();

        Ok(MvmExecOutcome {
            exit_code,
            stdout,
            stderr,
            instructions_retired: fuel_used,
            vsock_frames,
            compiled_elf_machine: None,
            cow_state_sha256: self.cow_state_sha256(),
        })
    }

    fn step_process(
        &mut self,
        p: &mut Rv64Process,
        fuel_used: &mut u64,
    ) -> Result<Option<StepEvent>, MvmError> {
        *fuel_used += 1;
        p.regs[0] = 0;

        let half = p.read_u16(p.pc)?;
        let (inst, step_bytes) = if (half & 0b11) != 0b11 {
            (expand_rv64c(half)?, 2u64)
        } else {
            (p.read_u32(p.pc)?, 4u64)
        };

        let pc = p.pc;
        let mut next_pc = pc.wrapping_add(step_bytes);

        let opcode = inst & 0x7f;
        let rd = ((inst >> 7) & 0x1f) as usize;
        let funct3 = (inst >> 12) & 0x7;
        let rs1 = ((inst >> 15) & 0x1f) as usize;
        let rs2 = ((inst >> 20) & 0x1f) as usize;
        let funct7 = (inst >> 25) & 0x7f;

        match opcode {
            0x37 => {
                // LUI
                let imm = ((inst & 0xffff_f000) as i32) as i64 as u64;
                if rd != 0 {
                    p.regs[rd] = imm;
                }
            }
            0x17 => {
                // AUIPC
                let imm = ((inst & 0xffff_f000) as i32) as i64 as u64;
                if rd != 0 {
                    p.regs[rd] = pc.wrapping_add(imm);
                }
            }
            0x6f => {
                // JAL
                let b20 = ((inst >> 31) & 1) << 20;
                let b10_1 = ((inst >> 21) & 0x3ff) << 1;
                let b11 = ((inst >> 20) & 1) << 11;
                let b19_12 = ((inst >> 12) & 0xff) << 12;
                let raw = (b20 | b19_12 | b11 | b10_1) as i32;
                let off = ((raw << 11) >> 11) as i64 as u64;
                if rd != 0 {
                    p.regs[rd] = next_pc;
                }
                next_pc = pc.wrapping_add(off);
            }
            0x67 => {
                // JALR
                let imm = ((inst as i32) >> 20) as i64 as u64;
                let target = p.regs[rs1].wrapping_add(imm) & !1;
                if rd != 0 {
                    p.regs[rd] = next_pc;
                }
                next_pc = target;
            }
            0x63 => {
                // BRANCH
                let b12 = ((inst >> 31) & 1) << 12;
                let b11 = ((inst >> 7) & 1) << 11;
                let b10_5 = ((inst >> 25) & 0x3f) << 5;
                let b4_1 = ((inst >> 8) & 0xf) << 1;
                let raw = (b12 | b11 | b10_5 | b4_1) as i32;
                let off = ((raw << 19) >> 19) as i64 as u64;
                let a = p.regs[rs1];
                let b = p.regs[rs2];
                let take = match funct3 {
                    0 => a == b,
                    1 => a != b,
                    4 => (a as i64) < (b as i64),
                    5 => (a as i64) >= (b as i64),
                    6 => a < b,
                    7 => a >= b,
                    _ => return Err(MvmError::CpuTrap(format!("bad branch funct3={funct3}"))),
                };
                if take {
                    next_pc = pc.wrapping_add(off);
                }
            }
            0x03 => {
                // LOAD
                let imm = ((inst as i32) >> 20) as i64 as u64;
                let addr = p.regs[rs1].wrapping_add(imm);
                let val = match funct3 {
                    0 => (p.read_u8(addr)? as i8) as i64 as u64,
                    1 => (p.read_u16(addr)? as i16) as i64 as u64,
                    2 => (p.read_u32(addr)? as i32) as i64 as u64,
                    3 => p.read_u64(addr)?,
                    4 => p.read_u8(addr)? as u64,
                    5 => p.read_u16(addr)? as u64,
                    6 => p.read_u32(addr)? as u64,
                    _ => return Err(MvmError::CpuTrap(format!("bad load funct3={funct3}"))),
                };
                if rd != 0 {
                    p.regs[rd] = val;
                }
            }
            0x23 => {
                // STORE
                let imm_lo = (inst >> 7) & 0x1f;
                let imm_hi = (inst >> 25) & 0x7f;
                let raw = ((imm_hi << 5) | imm_lo) as i32;
                let imm = ((raw << 20) >> 20) as i64 as u64;
                let addr = p.regs[rs1].wrapping_add(imm);
                let val = p.regs[rs2];
                match funct3 {
                    0 => p.write_bytes(addr, &[val as u8])?,
                    1 => p.write_bytes(addr, &(val as u16).to_le_bytes())?,
                    2 => p.write_bytes(addr, &(val as u32).to_le_bytes())?,
                    3 => p.write_bytes(addr, &val.to_le_bytes())?,
                    _ => return Err(MvmError::CpuTrap(format!("bad store funct3={funct3}"))),
                }
            }
            0x13 => {
                // OP-IMM (64-bit)
                let imm = ((inst as i32) >> 20) as i64;
                let src = p.regs[rs1];
                let shamt = (inst >> 20) & 0x3f;
                let res = match funct3 {
                    0 => src.wrapping_add(imm as u64),
                    2 => u64::from((src as i64) < imm),
                    3 => u64::from(src < (imm as u64)),
                    4 => src ^ (imm as u64),
                    6 => src | (imm as u64),
                    7 => src & (imm as u64),
                    1 => src << shamt,
                    5 => {
                        if ((inst >> 30) & 1) == 1 {
                            ((src as i64) >> shamt) as u64
                        } else {
                            src >> shamt
                        }
                    }
                    _ => return Err(MvmError::CpuTrap(format!("bad op-imm funct3={funct3}"))),
                };
                if rd != 0 {
                    p.regs[rd] = res;
                }
            }
            0x1b => {
                // OP-IMM-32
                let imm = (inst as i32) >> 20;
                let src32 = p.regs[rs1] as u32;
                let shamt = (inst >> 20) & 0x1f;
                let res32: i32 = match funct3 {
                    0 => (src32 as i32).wrapping_add(imm),
                    1 => (src32 << shamt) as i32,
                    5 => {
                        if ((inst >> 30) & 1) == 1 {
                            (src32 as i32) >> shamt
                        } else {
                            (src32 >> shamt) as i32
                        }
                    }
                    _ => return Err(MvmError::CpuTrap(format!("bad op-imm-32 funct3={funct3}"))),
                };
                if rd != 0 {
                    p.regs[rd] = (res32 as i64) as u64;
                }
            }
            0x33 => {
                // OP (64-bit + RV64M)
                let a = p.regs[rs1];
                let b = p.regs[rs2];
                let res = if funct7 == 0x01 {
                    match funct3 {
                        0 => a.wrapping_mul(b),
                        1 => (((a as i64 as i128) * (b as i64 as i128)) >> 64) as u64,
                        2 => (((a as i64 as i128) * (b as u128 as i128)) >> 64) as u64,
                        3 => (((a as u128) * (b as u128)) >> 64) as u64,
                        4 => {
                            if b == 0 {
                                u64::MAX
                            } else if (a as i64) == i64::MIN && (b as i64) == -1 {
                                i64::MIN as u64
                            } else {
                                ((a as i64) / (b as i64)) as u64
                            }
                        }
                        5 => a.checked_div(b).unwrap_or(u64::MAX),
                        6 => {
                            if b == 0 {
                                a
                            } else if (a as i64) == i64::MIN && (b as i64) == -1 {
                                0
                            } else {
                                ((a as i64) % (b as i64)) as u64
                            }
                        }
                        7 => {
                            if b == 0 {
                                a
                            } else {
                                a % b
                            }
                        }
                        _ => unreachable!(),
                    }
                } else {
                    match funct3 {
                        0 => {
                            if funct7 == 0x20 {
                                a.wrapping_sub(b)
                            } else {
                                a.wrapping_add(b)
                            }
                        }
                        1 => a << (b & 0x3f),
                        2 => u64::from((a as i64) < (b as i64)),
                        3 => u64::from(a < b),
                        4 => a ^ b,
                        5 => {
                            if funct7 == 0x20 {
                                ((a as i64) >> (b & 0x3f)) as u64
                            } else {
                                a >> (b & 0x3f)
                            }
                        }
                        6 => a | b,
                        7 => a & b,
                        _ => unreachable!(),
                    }
                };
                if rd != 0 {
                    p.regs[rd] = res;
                }
            }
            0x3b => {
                // OP-32 + RV64M-32
                let a = p.regs[rs1] as u32;
                let b = p.regs[rs2] as u32;
                let res32: i32 = if funct7 == 0x01 {
                    match funct3 {
                        0 => (a as i32).wrapping_mul(b as i32),
                        4 => {
                            if b == 0 {
                                -1
                            } else if (a as i32) == i32::MIN && (b as i32) == -1 {
                                i32::MIN
                            } else {
                                (a as i32) / (b as i32)
                            }
                        }
                        5 => a.checked_div(b).map_or(-1, |q| q as i32),
                        6 => {
                            if b == 0 {
                                a as i32
                            } else if (a as i32) == i32::MIN && (b as i32) == -1 {
                                0
                            } else {
                                (a as i32) % (b as i32)
                            }
                        }
                        7 => {
                            if b == 0 {
                                a as i32
                            } else {
                                (a % b) as i32
                            }
                        }
                        _ => return Err(MvmError::CpuTrap(format!("bad op-32M funct3={funct3}"))),
                    }
                } else {
                    match funct3 {
                        0 => {
                            if funct7 == 0x20 {
                                (a as i32).wrapping_sub(b as i32)
                            } else {
                                (a as i32).wrapping_add(b as i32)
                            }
                        }
                        1 => (a << (b & 0x1f)) as i32,
                        5 => {
                            if funct7 == 0x20 {
                                (a as i32) >> (b & 0x1f)
                            } else {
                                (a >> (b & 0x1f)) as i32
                            }
                        }
                        _ => return Err(MvmError::CpuTrap(format!("bad op-32 funct3={funct3}"))),
                    }
                };
                if rd != 0 {
                    p.regs[rd] = (res32 as i64) as u64;
                }
            }
            0x2f => {
                // RV64A Atomics
                let funct5 = funct7 >> 2;
                let addr = p.regs[rs1];
                if funct3 == 2 {
                    let old = p.read_u32(addr)? as i32;
                    let src = p.regs[rs2] as i32;
                    match funct5 {
                        0x02 => {
                            p.lr_reservation = Some(addr);
                            if rd != 0 {
                                p.regs[rd] = (old as i64) as u64;
                            }
                        }
                        0x03 => {
                            if p.lr_reservation == Some(addr) {
                                p.write_bytes(addr, &(src as u32).to_le_bytes())?;
                                if rd != 0 {
                                    p.regs[rd] = 0;
                                }
                            } else if rd != 0 {
                                p.regs[rd] = 1;
                            }
                            p.lr_reservation = None;
                        }
                        _ => {
                            let new_v = match funct5 {
                                0x01 => src,
                                0x00 => old.wrapping_add(src),
                                0x04 => old ^ src,
                                0x0c => old & src,
                                0x08 => old | src,
                                0x10 => old.min(src),
                                0x14 => old.max(src),
                                0x18 => ((old as u32).min(src as u32)) as i32,
                                0x1c => ((old as u32).max(src as u32)) as i32,
                                _ => return Err(MvmError::CpuTrap("bad amo.w".to_string())),
                            };
                            p.write_bytes(addr, &(new_v as u32).to_le_bytes())?;
                            if rd != 0 {
                                p.regs[rd] = (old as i64) as u64;
                            }
                        }
                    }
                } else if funct3 == 3 {
                    let old = p.read_u64(addr)?;
                    let src = p.regs[rs2];
                    match funct5 {
                        0x02 => {
                            p.lr_reservation = Some(addr);
                            if rd != 0 {
                                p.regs[rd] = old;
                            }
                        }
                        0x03 => {
                            if p.lr_reservation == Some(addr) {
                                p.write_bytes(addr, &src.to_le_bytes())?;
                                if rd != 0 {
                                    p.regs[rd] = 0;
                                }
                            } else if rd != 0 {
                                p.regs[rd] = 1;
                            }
                            p.lr_reservation = None;
                        }
                        _ => {
                            let new_v = match funct5 {
                                0x01 => src,
                                0x00 => old.wrapping_add(src),
                                0x04 => old ^ src,
                                0x0c => old & src,
                                0x08 => old | src,
                                0x10 => ((old as i64).min(src as i64)) as u64,
                                0x14 => ((old as i64).max(src as i64)) as u64,
                                0x18 => old.min(src),
                                0x1c => old.max(src),
                                _ => return Err(MvmError::CpuTrap("bad amo.d".to_string())),
                            };
                            p.write_bytes(addr, &new_v.to_le_bytes())?;
                            if rd != 0 {
                                p.regs[rd] = old;
                            }
                        }
                    }
                }
            }
            0x0f => {
                // FENCE / FENCE.I (deterministic single-hart memory is sequentially consistent)
            }
            0x73 => {
                // SYSTEM / ECALL / EBREAK / Zicsr
                if funct3 == 0 {
                    let sys_imm = inst >> 20;
                    if sys_imm == 0 {
                        p.pc = next_pc;
                        return self.handle_syscall(p, *fuel_used);
                    } else if sys_imm == 1 {
                        p.exited = Some(0);
                    }
                } else if rd != 0 {
                    // Read-only deterministic cycle/time/instret CSRs
                    p.regs[rd] = *fuel_used;
                }
            }
            _ => {
                return Err(MvmError::CpuTrap(format!(
                    "unsupported RV64 opcode 0x{opcode:02x} at pc=0x{pc:x} (inst=0x{inst:08x})"
                )));
            }
        }

        p.regs[0] = 0;
        p.pc = next_pc;
        Ok(None)
    }

    fn handle_syscall(
        &mut self,
        p: &mut Rv64Process,
        fuel_used: u64,
    ) -> Result<Option<StepEvent>, MvmError> {
        let nr = p.regs[17];
        let a0 = p.regs[10];
        let a1 = p.regs[11];
        let a2 = p.regs[12];

        match nr {
            17 => {
                // SYS_getcwd(buf, size)
                let bytes = p.cwd.as_bytes().to_vec();
                if (a1 as usize) <= bytes.len() {
                    p.regs[10] = (-34i64) as u64; // -ERANGE
                } else {
                    p.write_bytes(a0, &bytes)?;
                    p.write_bytes(a0 + (bytes.len() as u64), &[0])?;
                    p.regs[10] = a0;
                }
            }
            24 => {
                // SYS_dup3(oldfd, newfd, flags)
                let oldfd = a0 as i32;
                let newfd = a1 as i32;
                if let Some(kind) = p.fds.get(&oldfd).cloned() {
                    p.fds.insert(newfd, kind);
                    p.regs[10] = newfd as u64;
                } else {
                    p.regs[10] = LINUX_EBADF as u64;
                }
            }
            34 => {
                // SYS_mkdirat(dirfd, path, mode)
                let raw = p.read_cstr(a1)?;
                let norm = Self::normalize_guest_path(&p.cwd, &raw);
                self.ensure_parent_dirs(&norm);
                self.dirs.insert(norm);
                p.regs[10] = 0;
            }
            35 => {
                // SYS_unlinkat(dirfd, path, flags)
                let raw = p.read_cstr(a1)?;
                let norm = Self::normalize_guest_path(&p.cwd, &raw);
                if self.inodes.remove(&norm).is_some() || self.dirs.remove(&norm) {
                    p.regs[10] = 0;
                } else {
                    p.regs[10] = LINUX_ENOENT as u64;
                }
            }
            49 => {
                // SYS_chdir(path)
                let raw = p.read_cstr(a0)?;
                let norm = Self::normalize_guest_path(&p.cwd, &raw);
                if self.dirs.contains(&norm) {
                    p.cwd = norm;
                    p.regs[10] = 0;
                } else {
                    p.regs[10] = LINUX_ENOENT as u64;
                }
            }
            56 => {
                // SYS_openat(dirfd, pathname, flags, mode)
                let raw = p.read_cstr(a1)?;
                let norm = Self::normalize_guest_path(&p.cwd, &raw);
                let flags = a2 as u32;
                let is_dir_flag = (flags & 0o200000) != 0;
                let o_creat = (flags & 0o100) != 0;
                let o_trunc = (flags & 0o1000) != 0;
                let o_append = (flags & 0o2000) != 0;
                let writable = (flags & 0b11) != 0;

                if is_dir_flag || self.dirs.contains(&norm) {
                    if !self.dirs.contains(&norm) {
                        p.regs[10] = LINUX_ENOENT as u64;
                        return Ok(None);
                    }
                    let entries = self.list_dir_entries(&norm);
                    let fd = p.next_fd;
                    p.next_fd += 1;
                    p.fds.insert(fd, FdKind::Directory { entries, index: 0 });
                    p.regs[10] = fd as u64;
                } else {
                    if !self.inodes.contains_key(&norm) {
                        if o_creat {
                            self.write_guest_file(&norm, &[])?;
                        } else {
                            p.regs[10] = LINUX_ENOENT as u64;
                            return Ok(None);
                        }
                    } else if o_trunc && writable {
                        self.write_guest_file(&norm, &[])?;
                    }
                    let fd = p.next_fd;
                    p.next_fd += 1;
                    p.fds.insert(
                        fd,
                        FdKind::RegularFile {
                            path: norm,
                            offset: 0,
                            writable,
                            append: o_append,
                        },
                    );
                    p.regs[10] = fd as u64;
                }
            }
            57 => {
                // SYS_close(fd)
                let fd = a0 as i32;
                if p.fds.remove(&fd).is_some() {
                    p.regs[10] = 0;
                } else {
                    p.regs[10] = LINUX_EBADF as u64;
                }
            }
            59 => {
                // SYS_pipe2(pipefd[2], flags)
                let shared = Rc::new(RefCell::new(VecDeque::new()));
                let rfd = p.next_fd;
                let wfd = p.next_fd + 1;
                p.next_fd += 2;
                p.fds.insert(rfd, FdKind::PipeRead(shared.clone()));
                p.fds.insert(wfd, FdKind::PipeWrite(shared));
                p.write_bytes(a0, &rfd.to_le_bytes())?;
                p.write_bytes(a0 + 4, &wfd.to_le_bytes())?;
                p.regs[10] = 0;
            }
            61 => {
                // SYS_getdents64(fd, dirp, count)
                let fd = a0 as i32;
                let max_bytes = a2 as usize;
                let mut out_buf: Vec<u8> = Vec::new();
                {
                    let Some(FdKind::Directory { entries, index }) = p.fds.get_mut(&fd) else {
                        p.regs[10] = LINUX_EBADF as u64;
                        return Ok(None);
                    };
                    while *index < entries.len() {
                        let (dtype, ref name) = entries[*index];
                        let name_bytes = name.as_bytes();
                        let reclen = (19 + name_bytes.len() + 1 + 7) & !7;
                        if out_buf.len() + reclen > max_bytes {
                            break;
                        }
                        let mut rec = vec![0u8; reclen];
                        let ino = (*index as u64) + 1000;
                        let next_off = (*index + 1) as i64;
                        rec[0..8].copy_from_slice(&ino.to_le_bytes());
                        rec[8..16].copy_from_slice(&next_off.to_le_bytes());
                        rec[16..18].copy_from_slice(&(reclen as u16).to_le_bytes());
                        rec[18] = dtype;
                        rec[19..19 + name_bytes.len()].copy_from_slice(name_bytes);
                        out_buf.extend_from_slice(&rec);
                        *index += 1;
                    }
                }
                if !out_buf.is_empty() {
                    p.write_bytes(a1, &out_buf)?;
                }
                p.regs[10] = out_buf.len() as u64;
            }
            62 => {
                // SYS_lseek(fd, offset, whence)
                let fd = a0 as i32;
                let off = a1 as i64;
                let whence = a2 as u32;
                let file_len = match p.fds.get(&fd) {
                    Some(FdKind::RegularFile { path, .. }) => {
                        self.read_guest_file(path).map(|b| b.len()).unwrap_or(0)
                    }
                    _ => {
                        p.regs[10] = LINUX_EBADF as u64;
                        return Ok(None);
                    }
                };
                if let Some(FdKind::RegularFile { offset, .. }) = p.fds.get_mut(&fd) {
                    let base = match whence {
                        0 => 0i64,
                        1 => *offset as i64,
                        2 => file_len as i64,
                        _ => {
                            p.regs[10] = LINUX_EINVAL as u64;
                            return Ok(None);
                        }
                    };
                    let next = (base + off).max(0) as usize;
                    *offset = next;
                    p.regs[10] = next as u64;
                }
            }
            63 => {
                // SYS_read(fd, buf, count)
                let fd = a0 as i32;
                let count = a2 as usize;
                let kind_clone = p.fds.get(&fd).cloned();
                match kind_clone {
                    Some(FdKind::StdinBuffer(q)) | Some(FdKind::PipeRead(q)) => {
                        let mut guard = q.borrow_mut();
                        let take = count.min(guard.len());
                        let mut tmp = Vec::with_capacity(take);
                        for _ in 0..take {
                            if let Some(b) = guard.pop_front() {
                                tmp.push(b);
                            }
                        }
                        drop(guard);
                        p.write_bytes(a1, &tmp)?;
                        p.regs[10] = tmp.len() as u64;
                    }
                    Some(FdKind::RegularFile { path, offset, .. }) => {
                        let Some(data) = self.read_guest_file(&path) else {
                            p.regs[10] = LINUX_ENOENT as u64;
                            return Ok(None);
                        };
                        let avail = data.len().saturating_sub(offset);
                        let take = count.min(avail);
                        if take > 0 {
                            p.write_bytes(a1, &data[offset..offset + take])?;
                            if let Some(FdKind::RegularFile {
                                offset: off_mut, ..
                            }) = p.fds.get_mut(&fd)
                            {
                                *off_mut += take;
                            }
                        }
                        p.regs[10] = take as u64;
                    }
                    _ => p.regs[10] = LINUX_EBADF as u64,
                }
            }
            64 => {
                // SYS_write(fd, buf, count)
                let fd = a0 as i32;
                let count = a2 as usize;
                let idx = a1 as usize;
                if idx.saturating_add(count) > p.ram.len() {
                    return Err(MvmError::CpuTrap("SYS_write buffer fault".to_string()));
                }
                let data = p.ram[idx..idx + count].to_vec();
                let kind_clone = p.fds.get(&fd).cloned();
                match kind_clone {
                    Some(FdKind::StdoutCapture(out)) => {
                        out.borrow_mut().extend_from_slice(&data);
                        p.regs[10] = count as u64;
                    }
                    Some(FdKind::StderrCapture(err)) => {
                        err.borrow_mut().extend_from_slice(&data);
                        p.regs[10] = count as u64;
                    }
                    Some(FdKind::PipeWrite(q)) => {
                        q.borrow_mut().extend(data.iter().copied());
                        p.regs[10] = count as u64;
                    }
                    Some(FdKind::RegularFile {
                        path,
                        offset,
                        writable,
                        append,
                    }) => {
                        if !writable {
                            p.regs[10] = LINUX_EBADF as u64;
                            return Ok(None);
                        }
                        let mut cur = self.read_guest_file(&path).unwrap_or_default();
                        let pos = if append { cur.len() } else { offset };
                        if pos + data.len() > cur.len() {
                            cur.resize(pos + data.len(), 0);
                        }
                        cur[pos..pos + data.len()].copy_from_slice(&data);
                        self.write_guest_file(&path, &cur)?;
                        if let Some(FdKind::RegularFile {
                            offset: off_mut, ..
                        }) = p.fds.get_mut(&fd)
                        {
                            *off_mut = pos + data.len();
                        }
                        p.regs[10] = count as u64;
                    }
                    _ => p.regs[10] = LINUX_EBADF as u64,
                }
            }
            79 | 80 => {
                // SYS_fstatat / SYS_fstat (minimal 128-byte zeroed stat struct with st_size)
                let stat_ptr = if nr == 80 { a1 } else { a2 };
                let zeros = [0u8; 128];
                p.write_bytes(stat_ptr, &zeros)?;
                p.regs[10] = 0;
            }
            93 | 94 => {
                // SYS_exit / SYS_exit_group
                p.exited = Some((a0 as i32) & 0xff);
            }
            113 => {
                // SYS_clock_gettime(clk_id, tp)
                let secs = 1_794_332_000u64 + (fuel_used / 1_000_000);
                let nsecs = (fuel_used % 1_000_000) * 1_000;
                if a1 != 0 {
                    p.write_bytes(a1, &secs.to_le_bytes())?;
                    p.write_bytes(a1 + 8, &nsecs.to_le_bytes())?;
                }
                p.regs[10] = 0;
            }
            160 => {
                // SYS_uname(buf) — 6 * 65-byte fields
                let mut u = [0u8; 390];
                let fields = [
                    "Linux",
                    "mvm-wasm",
                    "6.6.0-uor-rv64imac",
                    "#1 SMP PREEMPT_DYNAMIC",
                    "riscv64",
                    "uor.local",
                ];
                for (i, f) in fields.iter().enumerate() {
                    let b = f.as_bytes();
                    let take = b.len().min(64);
                    u[i * 65..i * 65 + take].copy_from_slice(&b[..take]);
                }
                p.write_bytes(a0, &u)?;
                p.regs[10] = 0;
            }
            172 => p.regs[10] = p.pid as u64, // SYS_getpid
            174 | 175 => p.regs[10] = 1000,   // SYS_getuid / SYS_geteuid
            198 => {
                // SYS_socket(domain, type, protocol) — Zero-NIC enforcement (`R4-37`)
                match admit_guest_socket_domain(a0) {
                    SocketAdmission::AdmittedVsock => {
                        let fd = p.next_fd;
                        p.next_fd += 1;
                        p.fds.insert(
                            fd,
                            FdKind::VsockSocket {
                                peer_cid: 0,
                                peer_port: 0,
                                rx_queue: VecDeque::new(),
                            },
                        );
                        p.regs[10] = fd as u64;
                    }
                    SocketAdmission::RefusedNicDisabled
                    | SocketAdmission::RefusedUnsupportedDomain => {
                        p.regs[10] = LINUX_EAFNOSUPPORT as u64;
                    }
                }
            }
            200 | 201 | 242 => {
                // SYS_bind / SYS_listen / SYS_accept4
                p.regs[10] = 0;
            }
            203 => {
                // SYS_connect(sockfd, sockaddr_vm*, addrlen)
                let fd = a0 as i32;
                if a2 < 16 {
                    p.regs[10] = LINUX_EINVAL as u64;
                    return Ok(None);
                }
                let family = p.read_u16(a1)?;
                let port = p.read_u32(a1 + 4)? as u16;
                let cid = p.read_u32(a1 + 8)? as u16;
                if admit_guest_socket_domain(family as u64) != SocketAdmission::AdmittedVsock {
                    p.regs[10] = LINUX_EAFNOSUPPORT as u64;
                    return Ok(None);
                }
                if let Some(FdKind::VsockSocket {
                    peer_cid,
                    peer_port,
                    ..
                }) = p.fds.get_mut(&fd)
                {
                    *peer_cid = cid;
                    *peer_port = port;
                    p.regs[10] = 0;
                } else {
                    p.regs[10] = LINUX_EBADF as u64;
                }
            }
            206 => {
                // SYS_sendto(sockfd, buf, len, flags, dest_addr, addrlen)
                let fd = a0 as i32;
                let len = a2 as usize;
                let idx = a1 as usize;
                if idx.saturating_add(len) > p.ram.len() {
                    return Err(MvmError::CpuTrap("SYS_sendto fault".to_string()));
                }
                let payload = p.ram[idx..idx + len].to_vec();
                let Some(FdKind::VsockSocket {
                    peer_cid,
                    peer_port,
                    rx_queue,
                }) = p.fds.get_mut(&fd)
                else {
                    p.regs[10] = LINUX_EBADF as u64;
                    return Ok(None);
                };
                let dst_cid = *peer_cid;
                let port = *peer_port;

                // Derive deterministic Cl(0,6) spin (0..63) and E8 orbit (0..15) from payload SHA-256
                let mut h = Sha256::new();
                h.update(&payload);
                let dig = h.finalize();
                let hdr = SpiralCoreHeader {
                    magic: SPIRALCORE_MAGIC,
                    src_cid: self.guest_cid,
                    dst_cid,
                    vsock_port: port,
                    cl06_spin: dig[0] & 63,
                    e8_orbit: dig[1] & 15,
                    payload_len: payload.len() as u32,
                };
                let verdict = admit_spiralcore_header(&hdr, SPIRALCORE_MAX_PAYLOAD);
                if verdict != FrameVerdict::FrameAdmitted {
                    p.regs[10] = LINUX_EINVAL as u64;
                    return Ok(None);
                }

                // Host CID 2 control plane immediate reply
                if dst_cid == 2 {
                    rx_queue.push_back(b"SPIRALCORE_HOST_ACK".to_vec());
                }
                p.regs[10] = len as u64;
                return Ok(Some(StepEvent::VsockPacketEmitted(VsockPacket {
                    header: hdr,
                    payload,
                })));
            }
            207 => {
                // SYS_recvfrom(sockfd, buf, len, ...)
                let fd = a0 as i32;
                let max_len = a2 as usize;
                let msg_opt = match p.fds.get_mut(&fd) {
                    Some(FdKind::VsockSocket { rx_queue, .. }) => rx_queue.pop_front(),
                    _ => {
                        p.regs[10] = LINUX_EBADF as u64;
                        return Ok(None);
                    }
                };
                if let Some(msg) = msg_opt {
                    let take = max_len.min(msg.len());
                    p.write_bytes(a1, &msg[..take])?;
                    p.regs[10] = take as u64;
                } else {
                    p.regs[10] = 0;
                }
            }
            214 => {
                // SYS_brk(addr)
                if a0 >= p.brk_addr && (a0 as usize) < GUEST_MMAP_BASE as usize {
                    p.brk_addr = a0;
                }
                p.regs[10] = p.brk_addr;
            }
            215 => p.regs[10] = 0, // SYS_munmap
            220 => {
                // SYS_clone
                return Ok(Some(StepEvent::CloneChild));
            }
            221 => {
                // SYS_execve(filename, argv, envp)
                let raw_path = p.read_cstr(a0)?;
                let norm = Self::normalize_guest_path(&p.cwd, &raw_path);
                let mut argv_vec = Vec::new();
                if a1 != 0 {
                    let mut cur = a1;
                    for _ in 0..64 {
                        let ptr = p.read_u64(cur)?;
                        if ptr == 0 {
                            break;
                        }
                        argv_vec.push(p.read_cstr(ptr)?);
                        cur += 8;
                    }
                }
                let mut envp_vec = Vec::new();
                if a2 != 0 {
                    let mut cur = a2;
                    for _ in 0..64 {
                        let ptr = p.read_u64(cur)?;
                        if ptr == 0 {
                            break;
                        }
                        envp_vec.push(p.read_cstr(ptr)?);
                        cur += 8;
                    }
                }
                let exec_target = if self.inodes.contains_key(&norm) {
                    norm
                } else if let Some(base) = norm.rsplit('/').next() {
                    let alt = format!("/usr/local/bin/{base}");
                    if self.inodes.contains_key(&alt) {
                        alt
                    } else {
                        norm
                    }
                } else {
                    norm
                };
                if self
                    .load_elf_into_process(p, &exec_target, &argv_vec, &envp_vec)
                    .is_err()
                {
                    p.regs[10] = LINUX_ENOENT as u64;
                }
            }
            222 => {
                // SYS_mmap(addr, length, ...)
                let aligned = (a1.max(4096) + 4095) & !4095;
                let out = p.mmap_cursor;
                if (out.saturating_add(aligned)) >= GUEST_STACK_TOP - 65536 {
                    p.regs[10] = (-12i64) as u64; // -ENOMEM
                } else {
                    p.mmap_cursor += aligned;
                    p.regs[10] = out;
                }
            }
            260 => {
                // SYS_wait4(pid, wstatus, options, rusage)
                return Ok(Some(StepEvent::Wait4 {
                    target_pid: a0 as i64,
                    status_ptr: a1,
                }));
            }
            _ => {
                p.regs[10] = LINUX_ENOSYS as u64;
            }
        }
        Ok(None)
    }

    fn list_dir_entries(&self, norm_dir: &str) -> Vec<(u8, String)> {
        let mut seen = BTreeMap::new();
        seen.insert(".".to_string(), 4u8);
        seen.insert("..".to_string(), 4u8);
        let prefix = if norm_dir == "/" {
            "/".to_string()
        } else {
            format!("{norm_dir}/")
        };
        for d in &self.dirs {
            if let Some(rest) = d.strip_prefix(&prefix) {
                if !rest.is_empty() && !rest.contains('/') {
                    seen.insert(rest.to_string(), 4u8);
                }
            }
        }
        for (f, inode) in &self.inodes {
            if let Some(rest) = f.strip_prefix(&prefix) {
                if !rest.is_empty() && !rest.contains('/') {
                    let dtype = match inode.kind {
                        InodeKind::Symlink { .. } => 10u8,
                        InodeKind::Regular { .. } => 8u8,
                    };
                    seen.insert(rest.to_string(), dtype);
                }
            }
        }
        seen.into_iter().map(|(k, v)| (v, k)).collect()
    }

    fn ensure_guest_ram_uncontaminated(&self) -> Result<(), MvmError> {
        for rule in self.host_secrets.values() {
            if !rule.raw_secret.is_empty() && self.guest_ram_contains(rule.raw_secret.as_bytes()) {
                return Err(MvmError::BridgeRejected(
                    BridgeError::GuestRamSecretContamination,
                ));
            }
        }
        Ok(())
    }

    fn admit_outbound_transport(&self, transport: TransportKind) -> Result<(), MvmError> {
        match admit_bridge_transport(self.guest_cid, 2, self.policy_mode, transport) {
            BridgeTransportVerdict::TransportAdmitted => Ok(()),
            BridgeTransportVerdict::PolicyDenied => {
                Err(MvmError::BridgeRejected(BridgeError::PolicyDenied))
            }
            BridgeTransportVerdict::InsecureTransportRejected => Err(MvmError::BridgeRejected(
                BridgeError::InsecureTransportRejected,
            )),
            BridgeTransportVerdict::InvalidVsockCid => {
                Err(MvmError::BridgeRejected(BridgeError::InvalidVsockCid))
            }
        }
    }

    fn verify_secret_placeholders_for_host(
        &self,
        target_host: &str,
        text_to_scan: &str,
    ) -> Result<bool, MvmError> {
        let mut secret_in_guest = false;
        for rule in self.host_secrets.values() {
            if !rule.raw_secret.is_empty() && self.guest_ram_contains(rule.raw_secret.as_bytes()) {
                secret_in_guest = true;
                break;
            }
        }
        if secret_in_guest {
            return Err(MvmError::BridgeRejected(
                BridgeError::GuestRamSecretContamination,
            ));
        }

        let github_domain_ok = matches!(
            target_host.to_ascii_lowercase().as_str(),
            "github.com" | "api.github.com"
        );
        let mut cursor = text_to_scan;
        let mut any_verified = false;
        while let Some(start) = cursor.find("{{MVM_SECRET:") {
            let after = &cursor[start + "{{MVM_SECRET:".len()..];
            let Some(end) = after.find("}}") else {
                return Err(MvmError::BridgeRejected(
                    BridgeError::MissingSecretPlaceholder,
                ));
            };
            let secret_name = &after[..end];
            let placeholder_present = !secret_name.is_empty();
            let rule_host_ok = self
                .host_secrets
                .get(secret_name)
                .map(|r| r.allowlisted_hosts.contains(target_host))
                .unwrap_or(true);
            let is_allowlisted = github_domain_ok && rule_host_ok;

            match admit_git_secret_substitution(
                secret_in_guest,
                placeholder_present,
                is_allowlisted,
            ) {
                GitSecretVerdict::SecretSubstitutionAdmitted => {
                    any_verified = true;
                }
                GitSecretVerdict::GuestRamSecretContamination => {
                    return Err(MvmError::BridgeRejected(
                        BridgeError::GuestRamSecretContamination,
                    ));
                }
                GitSecretVerdict::MissingSecretPlaceholder => {
                    return Err(MvmError::BridgeRejected(
                        BridgeError::MissingSecretPlaceholder,
                    ));
                }
                GitSecretVerdict::HostNotAllowlistedForSecret => {
                    return Err(MvmError::BridgeRejected(
                        BridgeError::HostNotAllowlistedForSecret,
                    ));
                }
            }
            cursor = &after[end + 2..];
        }
        Ok(any_verified)
    }

    fn next_bridge_req_id(&mut self, prefix: &str) -> String {
        let seq = self.next_bridge_seq;
        self.next_bridge_seq = self.next_bridge_seq.saturating_add(1);
        format!("{prefix}-{seq}")
    }

    fn policy_str(&self) -> &'static str {
        match self.policy_mode {
            McpPolicyMode::Safe => "safe",
            McpPolicyMode::Confirm => "confirm",
            McpPolicyMode::Deny => "deny",
        }
    }

    fn emit_bridge_vsock_frame(&self, op_json: &[u8]) -> Result<VsockPacket, MvmError> {
        let mut h = Sha256::new();
        h.update(op_json);
        let dig = h.finalize();
        let clamped = if op_json.len() > SPIRALCORE_MAX_PAYLOAD {
            &op_json[..SPIRALCORE_MAX_PAYLOAD]
        } else {
            op_json
        };
        let wire = self.build_vsock_packet(2, 2048, dig[0] & 63, dig[1] & 15, clamped)?;
        let hdr = SpiralCoreHeader::from_bytes(&wire)
            .ok_or(MvmError::FrameRejected(FrameVerdict::RefusedInvalidCid))?;
        Ok(VsockPacket {
            header: hdr,
            payload: wire[16..].to_vec(),
        })
    }

    fn prepare_dns_bridge(
        &mut self,
        words: &[String],
        working_cwd: &str,
        redirect: Option<(String, bool)>,
    ) -> Result<MvmBridgePending, MvmError> {
        self.admit_outbound_transport(TransportKind::WssRelay)?;
        self.ensure_guest_ram_uncontaminated()?;

        let mut domain = String::new();
        let mut rtype = "A".to_string();
        for arg in &words[1..] {
            if arg.starts_with('-') || arg.starts_with('@') || arg.starts_with('+') {
                continue;
            }
            let up = arg.to_ascii_uppercase();
            if matches!(up.as_str(), "A" | "AAAA" | "TXT" | "MX" | "CNAME" | "NS")
                && !domain.is_empty()
            {
                rtype = up;
            } else if domain.is_empty() {
                domain = arg.clone();
            }
        }
        if domain.is_empty() {
            domain = "github.com".to_string();
        }

        let req_id = self.next_bridge_req_id("mvm-dns");
        let bridge_req = serde_json::json!({
            "req_id": req_id,
            "spvs_magic": "SPVS",
            "src_cid": self.guest_cid,
            "dst_cid": 2,
            "policy": self.policy_str(),
            "op": "dns_query",
            "name": domain,
            "rr_type": rtype,
            "type": rtype,
            "params": {
                "name": domain,
                "type": rtype,
            }
        });
        let frame = self.emit_bridge_vsock_frame(bridge_req.to_string().as_bytes())?;
        let resume_state = serde_json::json!({
            "bridge_kind": "dns_query",
            "name": domain,
            "rtype": rtype,
            "working_cwd": working_cwd,
            "redirect_path": redirect.as_ref().map(|(p, _)| p.clone()),
            "redirect_append": redirect.as_ref().map(|(_, a)| *a).unwrap_or(false),
        });
        Ok(MvmBridgePending {
            bridge_req,
            resume_state,
            vsock_frames: vec![frame],
        })
    }

    fn prepare_http_bridge(
        &mut self,
        words: &[String],
        working_cwd: &str,
        redirect: Option<(String, bool)>,
    ) -> Result<MvmBridgePending, MvmError> {
        let mut method = "GET".to_string();
        let mut url = String::new();
        let mut headers = serde_json::Map::new();
        let mut body: Option<String> = None;
        let mut output_file: Option<String> = None;

        let mut i = 1usize;
        while i < words.len() {
            match words[i].as_str() {
                "-X" | "--request" if i + 1 < words.len() => {
                    method = words[i + 1].to_ascii_uppercase();
                    i += 2;
                }
                "-H" | "--header" if i + 1 < words.len() => {
                    if let Some((k, v)) = words[i + 1].split_once(':') {
                        headers.insert(
                            k.trim().to_string(),
                            serde_json::Value::String(v.trim().to_string()),
                        );
                    }
                    i += 2;
                }
                "-d" | "--data" | "--data-raw" if i + 1 < words.len() => {
                    body = Some(words[i + 1].clone());
                    if method == "GET" {
                        method = "POST".to_string();
                    }
                    i += 2;
                }
                "-o" | "-O" | "--output" if i + 1 < words.len() => {
                    output_file = Some(words[i + 1].clone());
                    i += 2;
                }
                "-I" | "--head" => {
                    method = "HEAD".to_string();
                    i += 1;
                }
                arg if !arg.starts_with('-') && url.is_empty() => {
                    url = arg.to_string();
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }

        let (scheme, host) = parse_url_scheme_and_host(&url);
        let transport = match scheme.as_str() {
            "https" => TransportKind::HttpsDirect,
            "wss" => TransportKind::WssRelay,
            "ws" => TransportKind::PlainWs,
            _ => TransportKind::PlainHttp,
        };
        self.admit_outbound_transport(transport)?;

        let combined_scan = format!(
            "{} {} {}",
            url,
            serde_json::Value::Object(headers.clone()),
            body.as_deref().unwrap_or("")
        );
        let secret_verified = self.verify_secret_placeholders_for_host(&host, &combined_scan)?;

        let req_id = self.next_bridge_req_id("mvm-http");
        let body_b64 = body.as_ref().map(|b| encode_base64_bytes(b.as_bytes()));
        let bridge_req = serde_json::json!({
            "req_id": req_id,
            "spvs_magic": "SPVS",
            "src_cid": self.guest_cid,
            "dst_cid": 2,
            "policy": self.policy_str(),
            "op": "http_request",
            "method": method,
            "url": url,
            "headers": headers,
            "body_base64": body_b64,
            "secret_placeholder_verified": secret_verified,
            "params": {
                "method": method,
                "url": url,
                "headers": headers,
                "body_b64": body_b64,
            }
        });
        let frame = self.emit_bridge_vsock_frame(bridge_req.to_string().as_bytes())?;
        let resume_state = serde_json::json!({
            "bridge_kind": "http_request",
            "url": url,
            "output_file": output_file,
            "working_cwd": working_cwd,
            "redirect_path": redirect.as_ref().map(|(p, _)| p.clone()),
            "redirect_append": redirect.as_ref().map(|(_, a)| *a).unwrap_or(false),
        });
        Ok(MvmBridgePending {
            bridge_req,
            resume_state,
            vsock_frames: vec![frame],
        })
    }

    /// Admit and build a `SpiralCore` (`CID 3 -> CID 2`) `web_search` bridge request (`R4-39`).
    pub(crate) fn prepare_web_search_bridge(
        &mut self,
        query: &str,
    ) -> Result<serde_json::Value, MvmError> {
        self.admit_outbound_transport(TransportKind::HttpsDirect)?;
        let secret_verified =
            self.verify_secret_placeholders_for_host("search.uor.foundation", query)?;
        let req_id = self.next_bridge_req_id("chat-web-search");
        let bridge_req = serde_json::json!({
            "req_id": req_id,
            "spvs_magic": "SPVS",
            "src_cid": self.guest_cid,
            "dst_cid": 2,
            "policy": self.policy_str(),
            "op": "web_search",
            "query": query,
            "limit": 5,
            "secret_placeholder_verified": secret_verified,
            "params": { "query": query, "limit": 5 }
        });
        let _ = self.emit_bridge_vsock_frame(bridge_req.to_string().as_bytes())?;
        Ok(bridge_req)
    }

    /// Admit and build a `SpiralCore` (`CID 3 -> CID 2`) `web_fetch` (`http_request`) bridge request (`R4-39`).
    pub(crate) fn prepare_web_fetch_bridge(
        &mut self,
        url: &str,
    ) -> Result<serde_json::Value, MvmError> {
        let (scheme, host) = parse_url_scheme_and_host(url);
        let transport = match scheme.as_str() {
            "https" => TransportKind::HttpsDirect,
            "wss" => TransportKind::WssRelay,
            "ws" => TransportKind::PlainWs,
            _ => TransportKind::PlainHttp,
        };
        self.admit_outbound_transport(transport)?;
        let secret_verified = self.verify_secret_placeholders_for_host(&host, url)?;
        let req_id = self.next_bridge_req_id("chat-web-fetch");
        let bridge_req = serde_json::json!({
            "req_id": req_id,
            "spvs_magic": "SPVS",
            "src_cid": self.guest_cid,
            "dst_cid": 2,
            "policy": self.policy_str(),
            "op": "http_request",
            "method": "GET",
            "url": url,
            "secret_placeholder_verified": secret_verified,
            "params": { "method": "GET", "url": url }
        });
        let _ = self.emit_bridge_vsock_frame(bridge_req.to_string().as_bytes())?;
        Ok(bridge_req)
    }

    fn exec_pkg_command(
        &mut self,
        words: &[String],
        working_cwd: &str,
        redirect: Option<(String, bool)>,
    ) -> Result<MvmInteractiveOutcome, MvmError> {
        let sub = words.get(1).map(String::as_str).unwrap_or("list");
        match sub {
            "list" => {
                let mut out = String::from("Installed RV64 ELF64 packages (/usr/local/bin):\n");
                if self.installed_packages.is_empty() {
                    out.push_str("  (none installed; base busybox applets active in /bin)\n");
                } else {
                    for meta in self.installed_packages.values() {
                        out.push_str(&format!(
                            "  {} ({} bytes, sha256:{})\n",
                            meta.name,
                            meta.byte_len,
                            &meta.sha256[..12.min(meta.sha256.len())]
                        ));
                    }
                }
                if let Some((redir, append)) = redirect {
                    self.write_redirected_stdout(working_cwd, &redir, &out, append)?;
                    out.clear();
                }
                Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                    exit_code: 0,
                    stdout: out,
                    stderr: String::new(),
                    instructions_retired: 0,
                    vsock_frames: Vec::new(),
                    compiled_elf_machine: None,
                    cow_state_sha256: self.cow_state_sha256(),
                }))
            }
            "search" | "update" => {
                let query = words
                    .get(2)
                    .map(|s| s.to_ascii_lowercase())
                    .unwrap_or_default();
                let catalog = [
                    (
                        "jq",
                        "JSON parser, pretty-printer, and path filter (static ELF64 riscv64)",
                    ),
                    (
                        "rg",
                        "Recursive substring file search across directories (static ELF64 riscv64)",
                    ),
                    (
                        "tree",
                        "Recursive ASCII directory hierarchy viewer (static ELF64 riscv64)",
                    ),
                    (
                        "xxd",
                        "Canonical 16-byte hexadecimal and ASCII dump (static ELF64 riscv64)",
                    ),
                    (
                        "sha256sum",
                        "FIPS 180-4 SHA-256 digest utility (static ELF64 riscv64)",
                    ),
                    (
                        "bc",
                        "64-bit integer arithmetic and variable calculator (static ELF64 riscv64)",
                    ),
                    ("wc", "Line, word, and byte counter (static ELF64 riscv64)"),
                    (
                        "head",
                        "Output leading lines or bytes of files (static ELF64 riscv64)",
                    ),
                    (
                        "tail",
                        "Output trailing lines or bytes of files (static ELF64 riscv64)",
                    ),
                    (
                        "http",
                        "Zero-NIC AF_VSOCK SpiralCore HTTP egress client (static ELF64 riscv64)",
                    ),
                ];
                let mut out = String::new();
                for (name, desc) in catalog {
                    if query.is_empty()
                        || name.contains(&query)
                        || desc.to_ascii_lowercase().contains(&query)
                    {
                        out.push_str(&format!("{name}\t-\t{desc}\n"));
                    }
                }
                if let Some((redir, append)) = redirect {
                    self.write_redirected_stdout(working_cwd, &redir, &out, append)?;
                    out.clear();
                }
                Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                    exit_code: 0,
                    stdout: out,
                    stderr: String::new(),
                    instructions_retired: 0,
                    vsock_frames: Vec::new(),
                    compiled_elf_machine: None,
                    cow_state_sha256: self.cow_state_sha256(),
                }))
            }
            "install" => {
                let target = words.get(2).map(String::as_str).unwrap_or("");
                if target.is_empty() {
                    return Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                        exit_code: 1,
                        stdout: String::new(),
                        stderr: "pkg install: missing package name or HTTPS URL\n".to_string(),
                        instructions_retired: 0,
                        vsock_frames: Vec::new(),
                        compiled_elf_machine: None,
                        cow_state_sha256: self.cow_state_sha256(),
                    }));
                }
                let mut expected_sha = String::new();
                let mut custom_name: Option<String> = None;
                let mut idx = 3usize;
                while idx < words.len() {
                    if words[idx] == "--sha256" && idx + 1 < words.len() {
                        expected_sha = words[idx + 1].clone();
                        idx += 2;
                    } else if words[idx] == "--name" && idx + 1 < words.len() {
                        custom_name = Some(words[idx + 1].clone());
                        idx += 2;
                    } else {
                        idx += 1;
                    }
                }

                let (source_url, pkg_name, transport) = if target.contains("://") {
                    let (scheme, _) = parse_url_scheme_and_host(target);
                    let tr = if scheme == "https" {
                        TransportKind::HttpsDirect
                    } else {
                        TransportKind::PlainHttp
                    };
                    let inferred = custom_name.unwrap_or_else(|| {
                        target
                            .rsplit('/')
                            .next()
                            .unwrap_or("pkg-bin")
                            .split('?')
                            .next()
                            .unwrap_or("pkg-bin")
                            .to_string()
                    });
                    (target.to_string(), inferred, tr)
                } else {
                    let name = custom_name.unwrap_or_else(|| target.to_string());
                    (
                        format!("https://pkg.uor.foundation/riscv64/{name}.elf"),
                        name,
                        TransportKind::HttpsDirect,
                    )
                };

                self.admit_outbound_transport(transport)?;
                self.ensure_guest_ram_uncontaminated()?;

                let req_id = self.next_bridge_req_id("mvm-pkg");
                let bridge_req = serde_json::json!({
                    "req_id": req_id,
                    "spvs_magic": "SPVS",
                    "src_cid": self.guest_cid,
                    "dst_cid": 2,
                    "policy": self.policy_str(),
                    "op": "pkg_fetch",
                    "subcommand": "install",
                    "name": pkg_name,
                    "pkg_name": pkg_name,
                    "expected_sha256": expected_sha,
                    "params": {
                        "pkg_name": pkg_name,
                        "url": source_url,
                        "expected_sha256": expected_sha,
                        "max_bytes": 4 * 1024 * 1024,
                    }
                });
                let frame = self.emit_bridge_vsock_frame(bridge_req.to_string().as_bytes())?;
                let resume_state = serde_json::json!({
                    "bridge_kind": "pkg_fetch",
                    "pkg_name": pkg_name,
                    "source_url": source_url,
                    "expected_sha256": expected_sha,
                    "working_cwd": working_cwd,
                    "redirect_path": redirect.as_ref().map(|(p, _)| p.clone()),
                    "redirect_append": redirect.as_ref().map(|(_, a)| *a).unwrap_or(false),
                });
                Ok(MvmInteractiveOutcome::NeedsBridge(MvmBridgePending {
                    bridge_req,
                    resume_state,
                    vsock_frames: vec![frame],
                }))
            }
            other => Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                exit_code: 1,
                stdout: String::new(),
                stderr: format!("pkg: unknown subcommand '{other}'\n"),
                instructions_retired: 0,
                vsock_frames: Vec::new(),
                compiled_elf_machine: None,
                cow_state_sha256: self.cow_state_sha256(),
            })),
        }
    }

    fn find_git_root(&self, cwd: &str) -> Option<String> {
        let mut cur = cwd.to_string();
        loop {
            let state_path = if cur == "/" {
                "/.git/uor_state.json".to_string()
            } else {
                format!("{cur}/.git/uor_state.json")
            };
            if self.inodes.contains_key(&state_path) {
                return Some(cur);
            }
            if cur == "/" || cur.is_empty() {
                return None;
            }
            let pos = cur.rfind('/')?;
            cur = if pos == 0 {
                "/".to_string()
            } else {
                cur[..pos].to_string()
            };
        }
    }

    fn load_git_state(&self, repo_root: &str) -> Option<GitRepoState> {
        let p = format!("{repo_root}/.git/uor_state.json");
        let bytes = self.read_guest_file(&p)?;
        serde_json::from_slice(&bytes).ok()
    }

    fn save_git_state(&mut self, repo_root: &str, state: &GitRepoState) -> Result<(), MvmError> {
        let p = format!("{repo_root}/.git/uor_state.json");
        let bytes = serde_json::to_vec(state).unwrap_or_default();
        self.write_guest_file(&p, &bytes)?;
        let head_ref = format!("ref: refs/heads/{}\n", state.branch);
        self.write_guest_file(&format!("{repo_root}/.git/HEAD"), head_ref.as_bytes())?;
        if let Some(ref oid) = state.head_oid {
            self.write_guest_file(
                &format!("{repo_root}/.git/refs/heads/{}", state.branch),
                format!("{oid}\n").as_bytes(),
            )?;
        }
        Ok(())
    }

    fn collect_git_working_tree(&self, repo_root: &str) -> BTreeMap<String, String> {
        let mut tree = BTreeMap::new();
        let prefix = if repo_root == "/" {
            "/".to_string()
        } else {
            format!("{repo_root}/")
        };
        for path in self.inodes.keys() {
            if let Some(rel) = path.strip_prefix(&prefix) {
                if rel.is_empty() || is_git_internal_rel_path(rel) {
                    continue;
                }
                if let Some(bytes) = self.read_guest_file(path) {
                    if !bytes.starts_with(b"\x7fELF") {
                        if let Ok(s) = String::from_utf8(bytes) {
                            tree.insert(rel.to_string(), s);
                        }
                    }
                }
            }
        }
        tree
    }

    fn exec_git_command(
        &mut self,
        words: &[String],
        working_cwd: &mut String,
        redirect: Option<(String, bool)>,
    ) -> Result<MvmInteractiveOutcome, MvmError> {
        let sub = words.get(1).map(String::as_str).unwrap_or("status");
        match sub {
            "init" => {
                let target_dir = words
                    .get(2)
                    .filter(|s| !s.starts_with('-'))
                    .map(|d| Self::normalize_guest_path(working_cwd, d))
                    .unwrap_or_else(|| working_cwd.clone());
                let state = GitRepoState {
                    branch: "main".to_string(),
                    remote_url: None,
                    head_oid: None,
                    staged: BTreeMap::new(),
                    head_tree: BTreeMap::new(),
                    commits: Vec::new(),
                };
                self.save_git_state(&target_dir, &state)?;
                let mut out = format!("Initialized empty Git repository in {target_dir}/.git/\n");
                if let Some((redir, append)) = redirect {
                    self.write_redirected_stdout(working_cwd, &redir, &out, append)?;
                    out.clear();
                }
                Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                    exit_code: 0,
                    stdout: out,
                    stderr: String::new(),
                    instructions_retired: 0,
                    vsock_frames: Vec::new(),
                    compiled_elf_machine: None,
                    cow_state_sha256: self.cow_state_sha256(),
                }))
            }
            "status" => {
                let Some(repo_root) = self.find_git_root(working_cwd) else {
                    return Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                        exit_code: 128,
                        stdout: String::new(),
                        stderr:
                            "fatal: not a git repository (or any of the parent directories): .git\n"
                                .to_string(),
                        instructions_retired: 0,
                        vsock_frames: Vec::new(),
                        compiled_elf_machine: None,
                        cow_state_sha256: self.cow_state_sha256(),
                    }));
                };
                let state = self.load_git_state(&repo_root).unwrap();
                let wt = self.collect_git_working_tree(&repo_root);
                let mut out = format!("On branch {}\n", state.branch);
                if !state.staged.is_empty() {
                    out.push_str("Changes to be committed:\n");
                    for (k, v) in &state.staged {
                        let status_lbl = match (state.head_tree.contains_key(k), v.is_some()) {
                            (false, true) => "new file",
                            (true, true) => "modified",
                            (_, false) => "deleted",
                        };
                        out.push_str(&format!("\t{status_lbl}:   {k}\n"));
                    }
                }

                let mut effective_index = state.head_tree.clone();
                for (k, v) in &state.staged {
                    match v {
                        Some(c) => {
                            effective_index.insert(k.clone(), c.clone());
                        }
                        None => {
                            effective_index.remove(k);
                        }
                    }
                }

                let mut unstaged = Vec::new();
                for (k, idx_content) in &effective_index {
                    match wt.get(k) {
                        Some(cur) if cur != idx_content => {
                            unstaged.push(format!("\tmodified:   {k}\n"))
                        }
                        None => unstaged.push(format!("\tdeleted:    {k}\n")),
                        _ => {}
                    }
                }
                if !unstaged.is_empty() {
                    out.push_str("Changes not staged for commit:\n");
                    for line in &unstaged {
                        out.push_str(line);
                    }
                }

                let mut untracked = Vec::new();
                for k in wt.keys() {
                    if !effective_index.contains_key(k) {
                        untracked.push(format!("\t{k}\n"));
                    }
                }
                if !untracked.is_empty() {
                    out.push_str("Untracked files:\n");
                    for line in &untracked {
                        out.push_str(line);
                    }
                }
                if state.staged.is_empty() && unstaged.is_empty() && untracked.is_empty() {
                    out.push_str("nothing to commit, working tree clean\n");
                }
                if let Some((redir, append)) = redirect {
                    self.write_redirected_stdout(working_cwd, &redir, &out, append)?;
                    out.clear();
                }
                Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                    exit_code: 0,
                    stdout: out,
                    stderr: String::new(),
                    instructions_retired: 0,
                    vsock_frames: Vec::new(),
                    compiled_elf_machine: None,
                    cow_state_sha256: self.cow_state_sha256(),
                }))
            }
            "add" => {
                let Some(repo_root) = self.find_git_root(working_cwd) else {
                    return Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                        exit_code: 128,
                        stdout: String::new(),
                        stderr: "fatal: not a git repository\n".to_string(),
                        instructions_retired: 0,
                        vsock_frames: Vec::new(),
                        compiled_elf_machine: None,
                        cow_state_sha256: self.cow_state_sha256(),
                    }));
                };
                let mut state = self.load_git_state(&repo_root).unwrap();
                let wt = self.collect_git_working_tree(&repo_root);
                let specs = &words[2..];
                let stage_all = specs
                    .iter()
                    .any(|s| s == "." || s == "-A" || s == "--all" || s == "-a");
                if stage_all {
                    for (k, v) in &wt {
                        if state.head_tree.get(k) != Some(v) {
                            state.staged.insert(k.clone(), Some(v.clone()));
                        } else {
                            state.staged.remove(k);
                        }
                    }
                    for k in state.head_tree.keys() {
                        if !wt.contains_key(k) {
                            state.staged.insert(k.clone(), None);
                        }
                    }
                } else {
                    let prefix = format!("{repo_root}/");
                    for raw_spec in specs {
                        if raw_spec.starts_with('-') {
                            continue;
                        }
                        let norm = Self::normalize_guest_path(working_cwd, raw_spec);
                        if let Some(rel) = norm.strip_prefix(&prefix) {
                            if let Some(v) = wt.get(rel) {
                                state.staged.insert(rel.to_string(), Some(v.clone()));
                            } else if state.head_tree.contains_key(rel) {
                                state.staged.insert(rel.to_string(), None);
                            }
                        }
                    }
                }
                self.save_git_state(&repo_root, &state)?;
                Ok(MvmInteractiveOutcome::Completed(self.empty_outcome()))
            }
            "commit" => {
                let Some(repo_root) = self.find_git_root(working_cwd) else {
                    return Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                        exit_code: 128,
                        stdout: String::new(),
                        stderr: "fatal: not a git repository\n".to_string(),
                        instructions_retired: 0,
                        vsock_frames: Vec::new(),
                        compiled_elf_machine: None,
                        cow_state_sha256: self.cow_state_sha256(),
                    }));
                };
                let mut state = self.load_git_state(&repo_root).unwrap();
                let mut msg = String::new();
                let mut auto_add = false;
                let mut idx = 2usize;
                while idx < words.len() {
                    if (words[idx] == "-m" || words[idx] == "-am") && idx + 1 < words.len() {
                        if words[idx] == "-am" {
                            auto_add = true;
                        }
                        msg = words[idx + 1].clone();
                        idx += 2;
                    } else if words[idx] == "-a" {
                        auto_add = true;
                        idx += 1;
                    } else {
                        idx += 1;
                    }
                }
                if auto_add {
                    let wt = self.collect_git_working_tree(&repo_root);
                    for (k, c) in &wt {
                        if state.head_tree.contains_key(k) && state.head_tree.get(k) != Some(c) {
                            state.staged.insert(k.clone(), Some(c.clone()));
                        }
                    }
                }
                if state.staged.is_empty() {
                    return Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                        exit_code: 1,
                        stdout: "nothing to commit, working tree clean\n".to_string(),
                        stderr: String::new(),
                        instructions_retired: 0,
                        vsock_frames: Vec::new(),
                        compiled_elf_machine: None,
                        cow_state_sha256: self.cow_state_sha256(),
                    }));
                }
                let changed_count = state.staged.len();
                for (k, v) in std::mem::take(&mut state.staged) {
                    match v {
                        Some(c) => {
                            state.head_tree.insert(k, c);
                        }
                        None => {
                            state.head_tree.remove(&k);
                        }
                    }
                }
                let commit_seed = format!(
                    "commit|parent={:?}|msg={}|tree={:?}|n={}",
                    state.head_oid,
                    msg,
                    state.head_tree,
                    state.commits.len()
                );
                let full_sha = sha256_hex_bytes(commit_seed.as_bytes());
                let oid = full_sha[..40].to_string();
                let rec = GitCommitRecord {
                    oid: oid.clone(),
                    parent_oid: state.head_oid.clone(),
                    message: msg.clone(),
                    tree: state.head_tree.clone(),
                };
                state.head_oid = Some(oid.clone());
                state.commits.push(rec);
                self.save_git_state(&repo_root, &state)?;
                let mut out = format!(
                    "[{} {}] {}\n {} file(s) changed\n",
                    state.branch,
                    &oid[..7],
                    msg,
                    changed_count
                );
                if let Some((redir, append)) = redirect {
                    self.write_redirected_stdout(working_cwd, &redir, &out, append)?;
                    out.clear();
                }
                Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                    exit_code: 0,
                    stdout: out,
                    stderr: String::new(),
                    instructions_retired: 0,
                    vsock_frames: Vec::new(),
                    compiled_elf_machine: None,
                    cow_state_sha256: self.cow_state_sha256(),
                }))
            }
            "log" => {
                let Some(repo_root) = self.find_git_root(working_cwd) else {
                    return Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                        exit_code: 128,
                        stdout: String::new(),
                        stderr: "fatal: not a git repository\n".to_string(),
                        instructions_retired: 0,
                        vsock_frames: Vec::new(),
                        compiled_elf_machine: None,
                        cow_state_sha256: self.cow_state_sha256(),
                    }));
                };
                let state = self.load_git_state(&repo_root).unwrap();
                let mut out = String::new();
                for rec in state.commits.iter().rev() {
                    out.push_str(&format!("commit {}\n\n    {}\n\n", rec.oid, rec.message));
                }
                if let Some((redir, append)) = redirect {
                    self.write_redirected_stdout(working_cwd, &redir, &out, append)?;
                    out.clear();
                }
                Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                    exit_code: 0,
                    stdout: out,
                    stderr: String::new(),
                    instructions_retired: 0,
                    vsock_frames: Vec::new(),
                    compiled_elf_machine: None,
                    cow_state_sha256: self.cow_state_sha256(),
                }))
            }
            "diff" => {
                let Some(repo_root) = self.find_git_root(working_cwd) else {
                    return Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                        exit_code: 128,
                        stdout: String::new(),
                        stderr: "fatal: not a git repository\n".to_string(),
                        instructions_retired: 0,
                        vsock_frames: Vec::new(),
                        compiled_elf_machine: None,
                        cow_state_sha256: self.cow_state_sha256(),
                    }));
                };
                let state = self.load_git_state(&repo_root).unwrap();
                let wt = self.collect_git_working_tree(&repo_root);
                let mut base = state.head_tree.clone();
                for (k, v) in &state.staged {
                    match v {
                        Some(c) => {
                            base.insert(k.clone(), c.clone());
                        }
                        None => {
                            base.remove(k);
                        }
                    }
                }
                let mut out = String::new();
                for (k, old_c) in &base {
                    match wt.get(k) {
                        Some(new_c) if new_c != old_c => {
                            out.push_str(&compute_simple_git_diff(k, old_c, new_c));
                        }
                        None => {
                            out.push_str(&compute_simple_git_diff(k, old_c, ""));
                        }
                        _ => {}
                    }
                }
                if let Some((redir, append)) = redirect {
                    self.write_redirected_stdout(working_cwd, &redir, &out, append)?;
                    out.clear();
                }
                Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                    exit_code: 0,
                    stdout: out,
                    stderr: String::new(),
                    instructions_retired: 0,
                    vsock_frames: Vec::new(),
                    compiled_elf_machine: None,
                    cow_state_sha256: self.cow_state_sha256(),
                }))
            }
            "remote" => {
                let Some(repo_root) = self.find_git_root(working_cwd) else {
                    return Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                        exit_code: 128,
                        stdout: String::new(),
                        stderr: "fatal: not a git repository\n".to_string(),
                        instructions_retired: 0,
                        vsock_frames: Vec::new(),
                        compiled_elf_machine: None,
                        cow_state_sha256: self.cow_state_sha256(),
                    }));
                };
                let mut state = self.load_git_state(&repo_root).unwrap();
                if words.get(2).map(String::as_str) == Some("add") && words.len() >= 5 {
                    state.remote_url = Some(words[4].clone());
                    self.save_git_state(&repo_root, &state)?;
                    return Ok(MvmInteractiveOutcome::Completed(self.empty_outcome()));
                }
                let out = match state.remote_url {
                    Some(u) => format!("origin\t{u} (fetch)\norigin\t{u} (push)\n"),
                    None => String::new(),
                };
                Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                    exit_code: 0,
                    stdout: out,
                    stderr: String::new(),
                    instructions_retired: 0,
                    vsock_frames: Vec::new(),
                    compiled_elf_machine: None,
                    cow_state_sha256: self.cow_state_sha256(),
                }))
            }
            "clone" | "pull" | "push" => {
                let (repo_url, target_dir) = if sub == "clone" {
                    let url = words
                        .iter()
                        .skip(2)
                        .find(|a| !a.starts_with('-'))
                        .cloned()
                        .unwrap_or_default();
                    let explicit_dir = words
                        .iter()
                        .skip(2)
                        .filter(|a| !a.starts_with('-'))
                        .nth(1)
                        .cloned();
                    let dir_name = explicit_dir.unwrap_or_else(|| {
                        url.trim_end_matches('/')
                            .rsplit('/')
                            .next()
                            .unwrap_or("repo")
                            .trim_end_matches(".git")
                            .to_string()
                    });
                    let norm_dir = Self::normalize_guest_path(working_cwd, &dir_name);
                    (url, norm_dir)
                } else {
                    let Some(repo_root) = self.find_git_root(working_cwd) else {
                        return Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                            exit_code: 128,
                            stdout: String::new(),
                            stderr: "fatal: not a git repository\n".to_string(),
                            instructions_retired: 0,
                            vsock_frames: Vec::new(),
                            compiled_elf_machine: None,
                            cow_state_sha256: self.cow_state_sha256(),
                        }));
                    };
                    let state = self.load_git_state(&repo_root).unwrap();
                    let explicit_url = words.iter().skip(2).find(|a| a.contains("://")).cloned();
                    let url = explicit_url.or(state.remote_url).unwrap_or_else(|| {
                        "https://github.com/uor-foundation/repo.git".to_string()
                    });
                    (url, repo_root)
                };

                let (scheme, host) = parse_url_scheme_and_host(&repo_url);
                let transport = if scheme == "https" {
                    TransportKind::HttpsDirect
                } else {
                    TransportKind::PlainHttp
                };
                self.admit_outbound_transport(transport)?;

                let has_github_secret = self.host_secrets.contains_key("GITHUB_TOKEN");
                let auth_placeholder = if has_github_secret {
                    Some("{{MVM_SECRET:GITHUB_TOKEN}}".to_string())
                } else if repo_url.contains("{{MVM_SECRET:") {
                    Some(repo_url.clone())
                } else {
                    None
                };
                let secret_verified = if let Some(ref ph) = auth_placeholder {
                    self.verify_secret_placeholders_for_host(&host, ph)?
                } else {
                    self.ensure_guest_ram_uncontaminated()?;
                    false
                };

                let (git_branch, push_files, push_msg) = if sub == "push" {
                    if let Some(st) = self.load_git_state(&target_dir) {
                        let tree = self.collect_git_working_tree(&target_dir);
                        let files_arr: Vec<serde_json::Value> = tree
                            .iter()
                            .map(|(p, c)| serde_json::json!({ "path": p, "content": c }))
                            .collect();
                        let last_msg = st
                            .commits
                            .last()
                            .map(|c| c.message.clone())
                            .unwrap_or_else(|| "Update from UOR-R4 Connected Studio".to_string());
                        (st.branch, Some(files_arr), Some(last_msg))
                    } else {
                        ("main".to_string(), None, None)
                    }
                } else {
                    let b = self
                        .load_git_state(&target_dir)
                        .map(|s| s.branch)
                        .unwrap_or_else(|| "main".to_string());
                    (b, None, None)
                };
                let service_q = if sub == "push" {
                    "info/refs?service=git-receive-pack"
                } else {
                    "info/refs?service=git-upload-pack"
                };
                let req_id = self.next_bridge_req_id("mvm-git");
                let bridge_req = serde_json::json!({
                    "req_id": req_id,
                    "spvs_magic": "SPVS",
                    "src_cid": self.guest_cid,
                    "dst_cid": 2,
                    "policy": self.policy_str(),
                    "op": "git_smart_http",
                    "subcommand": sub,
                    "git_action": sub,
                    "repo_url": repo_url,
                    "branch": git_branch,
                    "files": push_files,
                    "message": push_msg,
                    "target_dir": target_dir,
                    "secret_placeholder_verified": secret_verified,
                    "params": {
                        "repo_url": repo_url,
                        "path_and_query": service_q,
                        "method": "GET",
                        "auth_bearer": auth_placeholder,
                    }
                });
                let frame = self.emit_bridge_vsock_frame(bridge_req.to_string().as_bytes())?;
                let resume_state = serde_json::json!({
                    "bridge_kind": "git_smart_http",
                    "git_action": sub,
                    "repo_url": repo_url,
                    "target_dir": target_dir,
                    "working_cwd": working_cwd,
                    "redirect_path": redirect.as_ref().map(|(p, _)| p.clone()),
                    "redirect_append": redirect.as_ref().map(|(_, a)| *a).unwrap_or(false),
                });
                Ok(MvmInteractiveOutcome::NeedsBridge(MvmBridgePending {
                    bridge_req,
                    resume_state,
                    vsock_frames: vec![frame],
                }))
            }
            other => Ok(MvmInteractiveOutcome::Completed(MvmExecOutcome {
                exit_code: 1,
                stdout: String::new(),
                stderr: format!("git: '{other}' is not a git command.\n"),
                instructions_retired: 0,
                vsock_frames: Vec::new(),
                compiled_elf_machine: None,
                cow_state_sha256: self.cow_state_sha256(),
            })),
        }
    }

    fn complete_git_bridge(
        &mut self,
        git_action: &str,
        repo_url: &str,
        target_dir: &str,
        data: &serde_json::Value,
        bridge_resp: &serde_json::Value,
    ) -> Result<(i32, String, String), MvmError> {
        let mut branch = data
            .get("branch")
            .or_else(|| bridge_resp.get("branch"))
            .and_then(|v| v.as_str())
            .unwrap_or("main")
            .to_string();
        let mut head_oid = data
            .get("head_oid")
            .or_else(|| data.get("head_sha"))
            .or_else(|| data.get("commit_sha"))
            .or_else(|| bridge_resp.get("head_oid"))
            .or_else(|| bridge_resp.get("head_sha"))
            .or_else(|| bridge_resp.get("commit_sha"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        // Parse Git smart-HTTP pkt-line advertisement when `body_b64` is returned directly by relay
        if let Some(b64) = data
            .get("body_b64")
            .or_else(|| bridge_resp.get("body_b64"))
            .and_then(|v| v.as_str())
        {
            if let Ok(raw) = decode_base64_bytes(b64) {
                let adv = String::from_utf8_lossy(&raw);
                let _ =
                    self.write_guest_file(&format!("{target_dir}/.git/FETCH_HEAD"), adv.as_bytes());
                for line in adv.lines() {
                    let trimmed = line.trim();
                    let body = if trimmed.len() > 4
                        && trimmed[..4].chars().all(|c| c.is_ascii_hexdigit())
                    {
                        &trimmed[4..]
                    } else {
                        trimmed
                    };
                    if body.len() >= 41 && body[..40].chars().all(|c| c.is_ascii_hexdigit()) {
                        let oid_cand = &body[..40];
                        let rest = body[40..].split('\0').next().unwrap_or("").trim();
                        if rest == "HEAD" && head_oid.is_none() {
                            head_oid = Some(oid_cand.to_string());
                        } else if let Some(bname) = rest.strip_prefix("refs/heads/") {
                            if head_oid.as_deref() == Some(oid_cand) || head_oid.is_none() {
                                branch = bname.to_string();
                                if head_oid.is_none() {
                                    head_oid = Some(oid_cand.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        let resolved_oid = head_oid.unwrap_or_else(|| {
            let h = sha256_hex_bytes(format!("git|{repo_url}|{branch}").as_bytes());
            h[..40].to_string()
        });

        let mut state = self
            .load_git_state(target_dir)
            .unwrap_or_else(|| GitRepoState {
                branch: branch.clone(),
                remote_url: Some(repo_url.to_string()),
                head_oid: Some(resolved_oid.clone()),
                staged: BTreeMap::new(),
                head_tree: BTreeMap::new(),
                commits: Vec::new(),
            });
        state.branch = branch.clone();
        if !repo_url.is_empty() {
            state.remote_url = Some(repo_url.to_string());
        }
        state.head_oid = Some(resolved_oid.clone());

        let mut written_files = 0usize;
        let files_val = data.get("files").or_else(|| bridge_resp.get("files"));
        if let Some(files_obj) = files_val.and_then(|v| v.as_object()) {
            for (rel, val) in files_obj {
                let clean = rel.trim_start_matches('/');
                if clean.is_empty() || is_git_internal_rel_path(clean) {
                    continue;
                }
                if let Some(content) = val.as_str() {
                    let guest_path = format!("{target_dir}/{clean}");
                    self.write_guest_file(&guest_path, content.as_bytes())?;
                    state
                        .head_tree
                        .insert(clean.to_string(), content.to_string());
                    written_files += 1;
                }
            }
        } else if let Some(files_arr) = files_val.and_then(|v| v.as_array()) {
            for entry in files_arr {
                let Some(rel) = entry.get("path").and_then(|v| v.as_str()) else {
                    continue;
                };
                let clean = rel.trim_start_matches('/');
                if clean.is_empty() || is_git_internal_rel_path(clean) {
                    continue;
                }
                if let Some(content) = entry.get("content").and_then(|v| v.as_str()) {
                    let guest_path = format!("{target_dir}/{clean}");
                    self.write_guest_file(&guest_path, content.as_bytes())?;
                    state
                        .head_tree
                        .insert(clean.to_string(), content.to_string());
                    written_files += 1;
                }
            }
        }

        if state.commits.is_empty() {
            state.commits.push(GitCommitRecord {
                oid: resolved_oid.clone(),
                parent_oid: None,
                message: format!("Imported upstream {branch}"),
                tree: state.head_tree.clone(),
            });
        }
        self.save_git_state(target_dir, &state)?;

        let short_oid = &resolved_oid[..7.min(resolved_oid.len())];
        let msg = match git_action {
            "push" => format!("To {repo_url}\n   {short_oid}..{short_oid}  {branch} -> {branch}\n"),
            "pull" => format!("From {repo_url}\n * branch {branch} -> FETCH_HEAD ({short_oid}, {written_files} file(s) updated)\n"),
            _ => format!("Cloning into '{target_dir}'...\nChecked out '{branch}' at {short_oid} ({written_files} file(s) materialized).\n"),
        };
        Ok((0, msg, String::new()))
    }
}

fn is_git_internal_rel_path(rel: &str) -> bool {
    rel == ".git" || rel.starts_with(".git/") || rel.contains("/.git/") || rel.ends_with("/.git")
}

fn split_shell_words_with_redirect(seg: &str) -> (Vec<String>, Option<(String, bool)>) {
    let raw_tokens = split_shell_words_preserving_redir(seg);
    let mut words = Vec::new();
    let mut redirect = None;
    let mut i = 0usize;
    while i < raw_tokens.len() {
        let (tok, was_quoted) = &raw_tokens[i];
        if !*was_quoted {
            if tok == ">>" && i + 1 < raw_tokens.len() {
                redirect = Some((raw_tokens[i + 1].0.clone(), true));
                i += 2;
                continue;
            }
            if tok == ">" && i + 1 < raw_tokens.len() {
                redirect = Some((raw_tokens[i + 1].0.clone(), false));
                i += 2;
                continue;
            }
            if let Some(rest) = tok.strip_prefix(">>") {
                if !rest.is_empty() {
                    redirect = Some((rest.to_string(), true));
                    i += 1;
                    continue;
                }
            }
            if let Some(rest) = tok.strip_prefix('>') {
                if !rest.is_empty() {
                    redirect = Some((rest.to_string(), false));
                    i += 1;
                    continue;
                }
            }
        }
        words.push(tok.clone());
        i += 1;
    }
    (words, redirect)
}

fn split_shell_words_preserving_redir(seg: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut cur_quoted = false;
    let mut q = 0u8;
    let bytes = seg.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if q != 0 {
            if b == b'\\' && q == b'"' && i + 1 < bytes.len() {
                cur.push(bytes[i + 1] as char);
                i += 2;
                continue;
            }
            if b == q {
                q = 0;
            } else {
                cur.push(b as char);
            }
            i += 1;
            continue;
        }
        if b == b'\'' || b == b'"' {
            q = b;
            cur_quoted = true;
            i += 1;
            continue;
        }
        if b.is_ascii_whitespace() {
            if !cur.is_empty() || cur_quoted {
                out.push((std::mem::take(&mut cur), cur_quoted));
                cur_quoted = false;
            }
            i += 1;
            continue;
        }
        if b == b'>' {
            if !cur.is_empty() || cur_quoted {
                out.push((std::mem::take(&mut cur), cur_quoted));
                cur_quoted = false;
            }
            if i + 1 < bytes.len() && bytes[i + 1] == b'>' {
                out.push((">>".to_string(), false));
                i += 2;
            } else {
                out.push((">".to_string(), false));
                i += 1;
            }
            continue;
        }
        cur.push(b as char);
        i += 1;
    }
    if !cur.is_empty() || cur_quoted {
        out.push((cur, cur_quoted));
    }
    out
}

fn parse_url_scheme_and_host(url: &str) -> (String, String) {
    let trimmed = url.trim();
    let Some((scheme_raw, after_scheme)) = trimmed.split_once("://") else {
        return ("http".to_string(), trimmed.to_string());
    };
    let authority = after_scheme.split(['/', '?', '#']).next().unwrap_or("");
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    let host = host_port.split(':').next().unwrap_or(host_port);
    (scheme_raw.to_ascii_lowercase(), host.to_ascii_lowercase())
}

fn sha256_hex_bytes(bytes: &[u8]) -> String {
    let dig = Sha256::digest(bytes);
    let mut s = String::with_capacity(64);
    for b in dig {
        use std::fmt::Write;
        let _ = write!(&mut s, "{b:02x}");
    }
    s
}

fn decode_hex_bytes(hex_str: &str) -> Result<Vec<u8>, BridgeError> {
    let clean: Vec<u8> = hex_str
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    if !clean.len().is_multiple_of(2) {
        return Err(BridgeError::InvalidBridgeJson(
            "odd-length hex payload".to_string(),
        ));
    }
    let mut out = Vec::with_capacity(clean.len() / 2);
    for pair in clean.chunks_exact(2) {
        let hi = match pair[0] {
            b'0'..=b'9' => pair[0] - b'0',
            b'a'..=b'f' => pair[0] - b'a' + 10,
            b'A'..=b'F' => pair[0] - b'A' + 10,
            _ => {
                return Err(BridgeError::InvalidBridgeJson(
                    "invalid hex digit".to_string(),
                ))
            }
        };
        let lo = match pair[1] {
            b'0'..=b'9' => pair[1] - b'0',
            b'a'..=b'f' => pair[1] - b'a' + 10,
            b'A'..=b'F' => pair[1] - b'A' + 10,
            _ => {
                return Err(BridgeError::InvalidBridgeJson(
                    "invalid hex digit".to_string(),
                ))
            }
        };
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn encode_base64_bytes(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn decode_base64_bytes(b64: &str) -> Result<Vec<u8>, BridgeError> {
    let mut vals = Vec::with_capacity(b64.len());
    for ch in b64.bytes() {
        if ch.is_ascii_whitespace() || ch == b'=' {
            continue;
        }
        let v = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'a'..=b'z' => ch - b'a' + 26,
            b'0'..=b'9' => ch - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => {
                return Err(BridgeError::InvalidBridgeJson(
                    "invalid base64 byte".to_string(),
                ))
            }
        };
        vals.push(v);
    }
    let mut out = Vec::with_capacity((vals.len() * 3) / 4);
    let mut i = 0usize;
    while i + 1 < vals.len() {
        out.push((vals[i] << 2) | (vals[i + 1] >> 4));
        if i + 2 < vals.len() {
            out.push((vals[i + 1] << 4) | (vals[i + 2] >> 2));
            if i + 3 < vals.len() {
                out.push((vals[i + 2] << 6) | vals[i + 3]);
            }
        }
        i += 4;
    }
    Ok(out)
}

fn extract_bridge_payload_bytes(
    data: &serde_json::Value,
    bridge_resp: &serde_json::Value,
) -> Vec<u8> {
    for src in [data, bridge_resp] {
        if let Some(s) = src
            .get("body_utf8")
            .or_else(|| src.get("body_text"))
            .and_then(|v| v.as_str())
        {
            return s.as_bytes().to_vec();
        }
        if let Some(b64) = src
            .get("body_b64")
            .or_else(|| src.get("body_base64"))
            .and_then(|v| v.as_str())
        {
            if let Ok(raw) = decode_base64_bytes(b64) {
                return raw;
            }
        }
        if let Some(ex) = src
            .get("text_excerpt")
            .or_else(|| src.get("excerpt"))
            .and_then(|v| v.as_str())
        {
            return ex.as_bytes().to_vec();
        }
    }
    Vec::new()
}

fn extract_bridge_elf_bytes(
    data: &serde_json::Value,
    bridge_resp: &serde_json::Value,
) -> Result<Vec<u8>, MvmError> {
    for src in [data, bridge_resp] {
        if let Some(hex_s) = src.get("elf_hex").and_then(|v| v.as_str()) {
            return decode_hex_bytes(hex_s).map_err(MvmError::BridgeRejected);
        }
        if let Some(b64) = src
            .get("elf_b64")
            .or_else(|| src.get("elf_base64"))
            .or_else(|| src.get("body_b64"))
            .or_else(|| src.get("body_base64"))
            .and_then(|v| v.as_str())
        {
            return decode_base64_bytes(b64).map_err(MvmError::BridgeRejected);
        }
    }
    Err(MvmError::BridgeRejected(BridgeError::InvalidBridgeJson(
        "missing elf_hex, elf_base64, or body_b64 in pkg_fetch bridge response".to_string(),
    )))
}

fn compute_simple_git_diff(rel: &str, old_text: &str, new_text: &str) -> String {
    let old_lines: Vec<&str> = old_text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();
    let mut out = format!(
        "diff --git a/{rel} b/{rel}\n--- a/{rel}\n+++ b/{rel}\n@@ -1,{} +1,{} @@\n",
        old_lines.len(),
        new_lines.len()
    );
    let max_len = old_lines.len().max(new_lines.len());
    for idx in 0..max_len {
        match (old_lines.get(idx), new_lines.get(idx)) {
            (Some(a), Some(b)) if a == b => {
                out.push_str(&format!(" {a}\n"));
            }
            (Some(a), Some(b)) => {
                out.push_str(&format!("-{a}\n+{b}\n"));
            }
            (Some(a), None) => {
                out.push_str(&format!("-{a}\n"));
            }
            (None, Some(b)) => {
                out.push_str(&format!("+{b}\n"));
            }
            (None, None) => {}
        }
    }
    out
}

enum StepEvent {
    CloneChild,
    Wait4 { target_pid: i64, status_ptr: u64 },
    VsockPacketEmitted(VsockPacket),
}

#[derive(PartialEq, Eq)]
enum ChainOp {
    First,
    Semi,
    And,
    Or,
}

fn split_command_chain(line: &str) -> Vec<(ChainOp, String)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut op = ChainOp::First;
    let bytes = line.as_bytes();
    let mut i = 0usize;
    let mut quote = 0u8;
    while i < bytes.len() {
        let c = bytes[i];
        if (c == b'\'' || c == b'"') && (i == 0 || bytes[i - 1] != b'\\') {
            if quote == c {
                quote = 0;
            } else if quote == 0 {
                quote = c;
            }
            cur.push(c as char);
            i += 1;
            continue;
        }
        if quote == 0 {
            if c == b'&' && i + 1 < bytes.len() && bytes[i + 1] == b'&' {
                out.push((op, cur.trim().to_string()));
                cur.clear();
                op = ChainOp::And;
                i += 2;
                continue;
            }
            if c == b'|' && i + 1 < bytes.len() && bytes[i + 1] == b'|' {
                out.push((op, cur.trim().to_string()));
                cur.clear();
                op = ChainOp::Or;
                i += 2;
                continue;
            }
            if c == b';' {
                out.push((op, cur.trim().to_string()));
                cur.clear();
                op = ChainOp::Semi;
                i += 1;
                continue;
            }
        }
        cur.push(c as char);
        i += 1;
    }
    if !cur.trim().is_empty() {
        out.push((op, cur.trim().to_string()));
    }
    out
}

fn contains_unquoted_pipe(seg: &str) -> bool {
    let b = seg.as_bytes();
    let mut q = 0u8;
    let mut i = 0usize;
    while i < b.len() {
        if q == b'"' && b[i] == b'\\' && i + 1 < b.len() {
            i += 2;
            continue;
        }
        if b[i] == b'\'' || b[i] == b'"' {
            if q == b[i] {
                q = 0;
            } else if q == 0 {
                q = b[i];
            }
        } else if q == 0
            && b[i] == b'|'
            && (i == 0 || b[i - 1] != b'|')
            && (i + 1 >= b.len() || b[i + 1] != b'|')
        {
            return true;
        }
        i += 1;
    }
    false
}

fn split_unquoted_pipes(seg: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let b = seg.as_bytes();
    let mut q = 0u8;
    let mut i = 0usize;
    while i < b.len() {
        if q == b'"' && b[i] == b'\\' && i + 1 < b.len() {
            cur.push('\\');
            cur.push(b[i + 1] as char);
            i += 2;
            continue;
        }
        if b[i] == b'\'' || b[i] == b'"' {
            if q == b[i] {
                q = 0;
            } else if q == 0 {
                q = b[i];
            }
            cur.push(b[i] as char);
            i += 1;
            continue;
        }
        if q == 0
            && b[i] == b'|'
            && (i == 0 || b[i - 1] != b'|')
            && (i + 1 >= b.len() || b[i + 1] != b'|')
        {
            out.push(cur.trim().to_string());
            cur.clear();
            i += 1;
            continue;
        }
        cur.push(b[i] as char);
        i += 1;
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// Expand a 16-bit RV64C compressed instruction into its exact 32-bit RV64I equivalent.
fn expand_rv64c(c: u16) -> Result<u32, MvmError> {
    let c32 = c as u32;
    let op = c32 & 0b11;
    let funct3 = (c32 >> 13) & 0x7;
    let r_prime = |bits: u32| 8 + (bits & 0x7);

    let sign_ext = |val: u32, bits: u32| -> i32 {
        let shift = 32 - bits;
        ((val << shift) as i32) >> shift
    };
    let enc_i = |imm: i32, rs1: u32, f3: u32, rd: u32, opc: u32| -> u32 {
        (((imm as u32) & 0xfff) << 20) | (rs1 << 15) | (f3 << 12) | (rd << 7) | opc
    };
    let enc_s = |imm: i32, rs2: u32, rs1: u32, f3: u32, opc: u32| -> u32 {
        let u = (imm as u32) & 0xfff;
        ((u >> 5) << 25) | (rs2 << 20) | (rs1 << 15) | (f3 << 12) | ((u & 0x1f) << 7) | opc
    };
    let enc_r = |f7: u32, rs2: u32, rs1: u32, f3: u32, rd: u32, opc: u32| -> u32 {
        (f7 << 25) | (rs2 << 20) | (rs1 << 15) | (f3 << 12) | (rd << 7) | opc
    };
    let enc_b = |off: i32, rs2: u32, rs1: u32, f3: u32| -> u32 {
        let u = off as u32;
        (((u >> 12) & 1) << 31)
            | (((u >> 5) & 0x3f) << 25)
            | (rs2 << 20)
            | (rs1 << 15)
            | (f3 << 12)
            | (((u >> 1) & 0xf) << 8)
            | (((u >> 11) & 1) << 7)
            | 0x63
    };
    let enc_j = |off: i32, rd: u32| -> u32 {
        let u = off as u32;
        (((u >> 20) & 1) << 31)
            | (((u >> 1) & 0x3ff) << 21)
            | (((u >> 11) & 1) << 20)
            | (((u >> 12) & 0xff) << 12)
            | (rd << 7)
            | 0x6f
    };

    match (op, funct3) {
        (0b00, 0b000) => {
            // C.ADDI4SPN
            let rd = r_prime(c32 >> 2);
            let imm = (((c32 >> 5) & 1) << 3)
                | (((c32 >> 6) & 1) << 2)
                | (((c32 >> 7) & 0xf) << 6)
                | (((c32 >> 11) & 0x3) << 4);
            if imm == 0 {
                return Err(MvmError::CpuTrap(
                    "illegal C instruction 0x0000".to_string(),
                ));
            }
            Ok(enc_i(imm as i32, 2, 0, rd, 0x13))
        }
        (0b00, 0b010) => {
            // C.LW
            let rd = r_prime(c32 >> 2);
            let rs1 = r_prime(c32 >> 7);
            let imm =
                (((c32 >> 5) & 1) << 6) | (((c32 >> 6) & 1) << 2) | (((c32 >> 10) & 0x7) << 3);
            Ok(enc_i(imm as i32, rs1, 2, rd, 0x03))
        }
        (0b00, 0b011) => {
            // C.LD
            let rd = r_prime(c32 >> 2);
            let rs1 = r_prime(c32 >> 7);
            let imm = (((c32 >> 5) & 0x3) << 6) | (((c32 >> 10) & 0x7) << 3);
            Ok(enc_i(imm as i32, rs1, 3, rd, 0x03))
        }
        (0b00, 0b110) => {
            // C.SW
            let rs2 = r_prime(c32 >> 2);
            let rs1 = r_prime(c32 >> 7);
            let imm =
                (((c32 >> 5) & 1) << 6) | (((c32 >> 6) & 1) << 2) | (((c32 >> 10) & 0x7) << 3);
            Ok(enc_s(imm as i32, rs2, rs1, 2, 0x23))
        }
        (0b00, 0b111) => {
            // C.SD
            let rs2 = r_prime(c32 >> 2);
            let rs1 = r_prime(c32 >> 7);
            let imm = (((c32 >> 5) & 0x3) << 6) | (((c32 >> 10) & 0x7) << 3);
            Ok(enc_s(imm as i32, rs2, rs1, 3, 0x23))
        }
        (0b01, 0b000) => {
            // C.NOP / C.ADDI
            let rd = (c32 >> 7) & 0x1f;
            let raw = (((c32 >> 12) & 1) << 5) | ((c32 >> 2) & 0x1f);
            let imm = sign_ext(raw, 6);
            Ok(enc_i(imm, rd, 0, rd, 0x13))
        }
        (0b01, 0b001) => {
            // C.ADDIW
            let rd = (c32 >> 7) & 0x1f;
            let raw = (((c32 >> 12) & 1) << 5) | ((c32 >> 2) & 0x1f);
            let imm = sign_ext(raw, 6);
            Ok(enc_i(imm, rd, 0, rd, 0x1b))
        }
        (0b01, 0b010) => {
            // C.LI
            let rd = (c32 >> 7) & 0x1f;
            let raw = (((c32 >> 12) & 1) << 5) | ((c32 >> 2) & 0x1f);
            let imm = sign_ext(raw, 6);
            Ok(enc_i(imm, 0, 0, rd, 0x13))
        }
        (0b01, 0b011) => {
            // C.ADDI16SP / C.LUI
            let rd = (c32 >> 7) & 0x1f;
            if rd == 2 {
                let raw = (((c32 >> 12) & 1) << 9)
                    | (((c32 >> 3) & 0x3) << 7)
                    | (((c32 >> 5) & 1) << 6)
                    | (((c32 >> 2) & 1) << 5)
                    | (((c32 >> 6) & 1) << 4);
                let imm = sign_ext(raw, 10);
                Ok(enc_i(imm, 2, 0, 2, 0x13))
            } else {
                let raw = (((c32 >> 12) & 1) << 5) | ((c32 >> 2) & 0x1f);
                let imm = sign_ext(raw, 6);
                Ok((((imm as u32) & 0xfffff) << 12) | (rd << 7) | 0x37)
            }
        }
        (0b01, 0b100) => {
            let rd = r_prime(c32 >> 7);
            let sub_op = (c32 >> 10) & 0x3;
            let shamt = (((c32 >> 12) & 1) << 5) | ((c32 >> 2) & 0x1f);
            match sub_op {
                0 => Ok(enc_i(shamt as i32, rd, 5, rd, 0x13)), // C.SRLI
                1 => Ok(enc_i((shamt | (1 << 10)) as i32, rd, 5, rd, 0x13)), // C.SRAI
                2 => {
                    let imm = sign_ext(shamt, 6);
                    Ok(enc_i(imm, rd, 7, rd, 0x13)) // C.ANDI
                }
                3 => {
                    let rs2 = r_prime(c32 >> 2);
                    let f = (((c32 >> 12) & 1) << 2) | ((c32 >> 5) & 0x3);
                    match f {
                        0b000 => Ok(enc_r(0x20, rs2, rd, 0, rd, 0x33)), // C.SUB
                        0b001 => Ok(enc_r(0x00, rs2, rd, 4, rd, 0x33)), // C.XOR
                        0b010 => Ok(enc_r(0x00, rs2, rd, 6, rd, 0x33)), // C.OR
                        0b011 => Ok(enc_r(0x00, rs2, rd, 7, rd, 0x33)), // C.AND
                        0b100 => Ok(enc_r(0x20, rs2, rd, 0, rd, 0x3b)), // C.SUBW
                        0b101 => Ok(enc_r(0x00, rs2, rd, 0, rd, 0x3b)), // C.ADDW
                        _ => Err(MvmError::CpuTrap("bad C.ALU".to_string())),
                    }
                }
                _ => unreachable!(),
            }
        }
        (0b01, 0b101) => {
            // C.J
            let raw = (((c32 >> 12) & 1) << 11)
                | (((c32 >> 8) & 1) << 10)
                | (((c32 >> 9) & 0x3) << 8)
                | (((c32 >> 6) & 1) << 7)
                | (((c32 >> 7) & 1) << 6)
                | (((c32 >> 2) & 1) << 5)
                | (((c32 >> 11) & 1) << 4)
                | (((c32 >> 3) & 0x7) << 1);
            Ok(enc_j(sign_ext(raw, 12), 0))
        }
        (0b01, 0b110) | (0b01, 0b111) => {
            // C.BEQZ / C.BNEZ
            let rs1 = r_prime(c32 >> 7);
            let raw = (((c32 >> 12) & 1) << 8)
                | (((c32 >> 5) & 0x3) << 6)
                | (((c32 >> 2) & 1) << 5)
                | (((c32 >> 10) & 0x3) << 3)
                | (((c32 >> 3) & 0x3) << 1);
            let f3 = if funct3 == 0b110 { 0 } else { 1 };
            Ok(enc_b(sign_ext(raw, 9), 0, rs1, f3))
        }
        (0b10, 0b000) => {
            // C.SLLI
            let rd = (c32 >> 7) & 0x1f;
            let shamt = (((c32 >> 12) & 1) << 5) | ((c32 >> 2) & 0x1f);
            Ok(enc_i(shamt as i32, rd, 1, rd, 0x13))
        }
        (0b10, 0b010) => {
            // C.LWSP
            let rd = (c32 >> 7) & 0x1f;
            let imm =
                (((c32 >> 2) & 0x3) << 6) | (((c32 >> 12) & 1) << 5) | (((c32 >> 4) & 0x7) << 2);
            Ok(enc_i(imm as i32, 2, 2, rd, 0x03))
        }
        (0b10, 0b011) => {
            // C.LDSP
            let rd = (c32 >> 7) & 0x1f;
            let imm =
                (((c32 >> 2) & 0x7) << 6) | (((c32 >> 12) & 1) << 5) | (((c32 >> 5) & 0x3) << 3);
            Ok(enc_i(imm as i32, 2, 3, rd, 0x03))
        }
        (0b10, 0b100) => {
            let bit12 = (c32 >> 12) & 1;
            let rd = (c32 >> 7) & 0x1f;
            let rs2 = (c32 >> 2) & 0x1f;
            match (bit12, rs2) {
                (0, 0) => Ok(enc_i(0, rd, 0, 0, 0x67)),      // C.JR
                (0, _) => Ok(enc_r(0, rs2, 0, 0, rd, 0x33)), // C.MV
                (1, 0) => {
                    if rd == 0 {
                        Ok(0x0010_0073) // C.EBREAK
                    } else {
                        Ok(enc_i(0, rd, 0, 1, 0x67)) // C.JALR
                    }
                }
                (1, _) => Ok(enc_r(0, rs2, rd, 0, rd, 0x33)), // C.ADD
                _ => unreachable!(),
            }
        }
        (0b10, 0b110) => {
            // C.SWSP
            let rs2 = (c32 >> 2) & 0x1f;
            let imm = (((c32 >> 7) & 0x3) << 6) | (((c32 >> 9) & 0xf) << 2);
            Ok(enc_s(imm as i32, rs2, 2, 2, 0x23))
        }
        (0b10, 0b111) => {
            // C.SDSP
            let rs2 = (c32 >> 2) & 0x1f;
            let imm = (((c32 >> 7) & 0x7) << 6) | (((c32 >> 10) & 0x7) << 3);
            Ok(enc_s(imm as i32, rs2, 2, 3, 0x23))
        }
        _ => Err(MvmError::CpuTrap(format!(
            "unsupported RV64C instruction 0x{c:04x}"
        ))),
    }
}
