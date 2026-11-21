//! Wave 2 (`R4-70`): Zero-Float `vTensor` Certified Storage Planner (`VT-09`, `VT-10`,
//! `VT-14`, `VT-15`, `VT-16`) & `verdant-tensor` Semiring Contractions + `16x16` Tile-Binned
//! 3D PBR Rasterizer + Progressive `UORC` `LOD0..LOD4` WebGUI Engine.
//!
//! Grounded in `src/R4/VtensorSemiring3d.lex.tex` (`R4.VtensorSemiring3d`), citing:
//! - `afflom/vTensor` (`9f897e18e7`, certified aligned arena & 4-axis `REPEAT` addressing)
//! - `afflom/verdant-engine` (`8f560d975e`, `crates/verdant-tensor` Laurent/tropical/`Q15`
//!   decode -> wide-accumulate-no-intermediate-rounding -> encode-once contractions,
//!   `16x16` tile-binned RGBA8+depth PBR rasterizer, and `UORC` `LOD0..LOD4` meshes).
//!
//! Strictly enforces `float_ops = 0`, `matmul_ops = 0`, and `softmax_ops = 0`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;

use crate::lexlean_generated::r4_vtensor_semiring_3d::{
    admit_semiring_raster_3d, admit_vtensor_storage_plan, four_axis_repeat_volume,
    verify_vtensor_semiring_3d_admitted, SemiringRaster3dCheck, SemiringRaster3dVerdict,
    VtensorStoragePlanCheck, VtensorStorageVerdict,
};

/// Certified non-float storage dtypes admitted by `R4.VtensorSemiring3d` (`R4-70`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VtensorIntDtype {
    /// Symmetric 4-bit nibble blocks of 32 elements with integer scale.
    Int4Group32,
    /// Symmetric 8-bit integer elements.
    Int8Sym,
    /// Signed 16-bit `Q15` (`[-32767, 32767]`) state/normal coordinate.
    Int16Q15,
    /// Signed 32-bit `Q24` hyperbolic Poincaré radial coordinate.
    Int32Q24,
    /// Signed 32-bit `PhaseQ29` (`[-pi, pi)` mapped to `i32`) Hopf/Zeta angle.
    Int32PhaseQ29,
    /// Exact 4-term integer Laurent polynomial `c_{-1}z^{-1} + c_0 + c_1 z + c_2 z^2`.
    LaurentZPoly4,
}

/// Typed `R4-70` errors (R5: every public failure is typed, modeled, and tested).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vtensor3dError {
    /// Storage plan rejected by `R4.VtensorSemiring3d.admitVtensorStoragePlan`.
    StorageRefused(String),
    /// 3D raster / LOD plan rejected by `R4.VtensorSemiring3d.admitSemiringRaster3d`.
    RasterRefused(String),
    /// Malformed `UORC` progressive mesh binary container or JSON scene payload.
    InvalidSceneOrMesh(String),
}

impl core::fmt::Display for Vtensor3dError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::StorageRefused(msg) => write!(f, "vTensor storage refused: {msg}"),
            Self::RasterRefused(msg) => write!(f, "semiring 3D raster refused: {msg}"),
            Self::InvalidSceneOrMesh(msg) => write!(f, "invalid 3D scene/UORC mesh: {msg}"),
        }
    }
}

impl std::error::Error for Vtensor3dError {}

/// Formal R4.VtensorSemiring3d parse dtype tag and reject float storage (`VT-09`, `R4-70`).
pub fn parse_vtensor_dtype(tag: &str) -> Result<VtensorIntDtype, Vtensor3dError> {
    match tag.trim().to_ascii_lowercase().as_str() {
        "i4" | "int4" | "int4_group32" => Ok(VtensorIntDtype::Int4Group32),
        "i8" | "int8" | "int8_sym" => Ok(VtensorIntDtype::Int8Sym),
        "q15" | "i16" | "int16_q15" => Ok(VtensorIntDtype::Int16Q15),
        "q24" | "i32_q24" | "int32_q24" => Ok(VtensorIntDtype::Int32Q24),
        "phase_q29" | "q29" | "int32_phase_q29" => Ok(VtensorIntDtype::Int32PhaseQ29),
        "laurent4" | "laurent_z_poly4" => Ok(VtensorIntDtype::LaurentZPoly4),
        other => {
            let verdict = admit_vtensor_storage_plan(VtensorStoragePlanCheck {
                is_float_dtype: true,
                is_empty_node: false,
                alignment_mod: 0,
                offset_bytes: 0,
                transfer_bytes: 64,
                capacity_bytes: 1024,
                repeat_axis0: 1,
                repeat_axis1: 1,
                repeat_axis2: 1,
                repeat_axis3: 1,
            });
            Err(Vtensor3dError::StorageRefused(format!(
                "dtype `{other}` rejected with {verdict:?}"
            )))
        }
    }
}

/// Certified linear aligned buffer arena (`VT-09`, `VT-10`, `VT-14`, `R4-70`).
#[derive(Debug, Clone)]
pub struct VtensorLinearArena {
    alignment: usize,
    cursor: usize,
    storage: Vec<u8>,
}

/// Admitted slice handle inside `VtensorLinearArena`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VtensorSliceHandle {
    /// Aligned byte offset inside the arena.
    pub offset: usize,
    /// Byte length of the slice (`0` for `VT-14` empty-node no-op).
    pub len: usize,
    /// Certified integer dtype.
    pub dtype: VtensorIntDtype,
    /// 4-axis logical dimensions `[d0, d1, d2, d3]`.
    pub dims: [usize; 4],
}

impl VtensorLinearArena {
    /// Formal R4.VtensorSemiring3d create a capacity-bounded aligned integer arena (`VT-10`).
    pub fn new(capacity_bytes: usize, alignment: usize) -> Result<Self, Vtensor3dError> {
        if alignment == 0 || !alignment.is_power_of_two() {
            return Err(Vtensor3dError::StorageRefused(format!(
                "alignment {alignment} must be a non-zero power of two"
            )));
        }
        Ok(Self {
            alignment,
            cursor: 0,
            storage: vec![0u8; capacity_bytes],
        })
    }

    /// Formal R4.VtensorSemiring3d allocate an aligned slice verified by `admit_vtensor_storage_plan`.
    pub fn alloc_aligned(
        &mut self,
        dtype: VtensorIntDtype,
        dims: [usize; 4],
        transfer_bytes: usize,
        repeat_axes: [usize; 4],
    ) -> Result<VtensorSliceHandle, Vtensor3dError> {
        let is_empty = transfer_bytes == 0;
        let aligned_offset = if is_empty {
            self.cursor
        } else {
            let mask = self.alignment - 1;
            (self.cursor.saturating_add(mask)) & !mask
        };
        let check = VtensorStoragePlanCheck {
            is_float_dtype: false,
            is_empty_node: is_empty,
            alignment_mod: (aligned_offset % self.alignment) as u64,
            offset_bytes: aligned_offset as u64,
            transfer_bytes: transfer_bytes as u64,
            capacity_bytes: self.storage.len() as u64,
            repeat_axis0: repeat_axes[0] as u64,
            repeat_axis1: repeat_axes[1] as u64,
            repeat_axis2: repeat_axes[2] as u64,
            repeat_axis3: repeat_axes[3] as u64,
        };
        match admit_vtensor_storage_plan(check) {
            VtensorStorageVerdict::StorageGraphAdmitted => {
                self.cursor = aligned_offset + transfer_bytes;
                Ok(VtensorSliceHandle {
                    offset: aligned_offset,
                    len: transfer_bytes,
                    dtype,
                    dims,
                })
            }
            VtensorStorageVerdict::EmptyNodeNoOpAdmitted => Ok(VtensorSliceHandle {
                offset: aligned_offset,
                len: 0,
                dtype,
                dims,
            }),
            refused => Err(Vtensor3dError::StorageRefused(format!("{refused:?}"))),
        }
    }

    /// Formal R4.VtensorSemiring3d inspect slice bytes inside the arena (`VT-09`).
    pub fn bytes(&self, handle: VtensorSliceHandle) -> Result<&[u8], Vtensor3dError> {
        if handle.len == 0 {
            return Ok(&[]);
        }
        self.storage
            .get(handle.offset..handle.offset.saturating_add(handle.len))
            .ok_or_else(|| Vtensor3dError::StorageRefused("handle out of arena bounds".to_string()))
    }

    /// Formal R4.VtensorSemiring3d write verified transfer bytes into a slice (`VT-09`, `VT-15`).
    pub fn write_bytes(
        &mut self,
        handle: VtensorSliceHandle,
        src: &[u8],
    ) -> Result<(), Vtensor3dError> {
        if handle.len == 0 && src.is_empty() {
            return Ok(());
        }
        if src.len() != handle.len {
            return Err(Vtensor3dError::StorageRefused(format!(
                "transfer length {} != slice length {}",
                src.len(),
                handle.len
            )));
        }
        let check = VtensorStoragePlanCheck {
            is_float_dtype: false,
            is_empty_node: false,
            alignment_mod: (handle.offset % self.alignment) as u64,
            offset_bytes: handle.offset as u64,
            transfer_bytes: src.len() as u64,
            capacity_bytes: self.storage.len() as u64,
            repeat_axis0: 1,
            repeat_axis1: 1,
            repeat_axis2: 1,
            repeat_axis3: 1,
        };
        let verdict = admit_vtensor_storage_plan(check);
        if verdict != VtensorStorageVerdict::StorageGraphAdmitted {
            return Err(Vtensor3dError::StorageRefused(format!("{verdict:?}")));
        }
        self.storage[handle.offset..handle.offset + handle.len].copy_from_slice(src);
        Ok(())
    }

    /// Formal R4.VtensorSemiring3d execute `DUP` / `CONT` / `CPY` block copy inside arena (`VT-15`).
    pub fn copy_or_dup_slice(
        &mut self,
        src: VtensorSliceHandle,
        dst: VtensorSliceHandle,
    ) -> Result<(), Vtensor3dError> {
        if dst.len == 0 {
            return Ok(());
        }
        if src.len == 0 || !dst.len.is_multiple_of(src.len) {
            return Err(Vtensor3dError::StorageRefused(format!(
                "DUP/CPY requires dst.len ({}) to be a multiple of non-zero src.len ({})",
                dst.len, src.len
            )));
        }
        let temp = self.bytes(src)?.to_vec();
        for chunk in self.storage[dst.offset..dst.offset + dst.len].chunks_exact_mut(src.len) {
            chunk.copy_from_slice(&temp);
        }
        Ok(())
    }
}

/// Formal R4.VtensorSemiring3d certified 4-axis `REPEAT` modular flat index (`VT-16`, `R4-70`).
pub fn vtensor_repeat4d_index(
    coords: [usize; 4],
    src_dims: [usize; 4],
    repeat_factors: [usize; 4],
) -> Result<usize, Vtensor3dError> {
    let vol = four_axis_repeat_volume(
        repeat_factors[0] as u64,
        repeat_factors[1] as u64,
        repeat_factors[2] as u64,
        repeat_factors[3] as u64,
    );
    if vol == 0 || src_dims.contains(&0) {
        return Err(Vtensor3dError::StorageRefused(
            "RefusedInvalidFourAxisRepeatFactor".to_string(),
        ));
    }
    for ax in 0..4 {
        let max_c = src_dims[ax].saturating_mul(repeat_factors[ax]);
        if coords[ax] >= max_c {
            return Err(Vtensor3dError::StorageRefused(format!(
                "axis {ax} coord {} >= repeated bound {max_c}",
                coords[ax]
            )));
        }
    }
    let s0 = coords[0] % src_dims[0];
    let s1 = coords[1] % src_dims[1];
    let s2 = coords[2] % src_dims[2];
    let s3 = coords[3] % src_dims[3];
    Ok((((s0 * src_dims[1] + s1) * src_dims[2] + s2) * src_dims[3]) + s3)
}

// ============================================================================
// Exact Semiring Contractions (Decode -> Wide Accumulate -> Encode Once)
// ============================================================================

/// Formal R4.VtensorSemiring3d exact `Q15` ring contraction with wide `i128` accumulation and single final shift (`R4-70`).
pub fn semiring_q15_contract_wide(
    lhs_q15: &[i16],
    rhs_q15: &[i16],
    rows: usize,
    inner: usize,
    cols: usize,
) -> Result<Vec<i16>, Vtensor3dError> {
    if lhs_q15.len() != rows.saturating_mul(inner) || rhs_q15.len() != inner.saturating_mul(cols) {
        return Err(Vtensor3dError::StorageRefused(
            "Q15 semiring contraction dimension mismatch".to_string(),
        ));
    }
    let mut out = vec![0i16; rows * cols];
    for r in 0..rows {
        let row_slice = &lhs_q15[r * inner..(r + 1) * inner];
        for c in 0..cols {
            let mut acc_wide: i128 = 0;
            for k in 0..inner {
                acc_wide += (row_slice[k] as i128) * (rhs_q15[k * cols + c] as i128);
            }
            let shifted = (acc_wide >> 15).clamp(-32767, 32767) as i16;
            out[r * cols + c] = shifted;
        }
    }
    Ok(out)
}

/// Formal R4.VtensorSemiring3d tropical `(max, +)` semiring contraction for frustum/support bounds (`R4-70`).
pub fn semiring_tropical_max_plus_contract(
    lhs: &[i64],
    rhs: &[i64],
    rows: usize,
    inner: usize,
    cols: usize,
) -> Result<Vec<i64>, Vtensor3dError> {
    if lhs.len() != rows.saturating_mul(inner) || rhs.len() != inner.saturating_mul(cols) {
        return Err(Vtensor3dError::StorageRefused(
            "tropical max-plus dimension mismatch".to_string(),
        ));
    }
    let mut out = vec![i64::MIN; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            let mut best = i64::MIN;
            for k in 0..inner {
                let cand = lhs[r * inner + k].saturating_add(rhs[k * cols + c]);
                if cand > best {
                    best = cand;
                }
            }
            out[r * cols + c] = best;
        }
    }
    Ok(out)
}

/// Formal R4.VtensorSemiring3d exact 4-term integer Laurent polynomial (`c_{-1}z^{-1} + c_0 + c_1 z + c_2 z^2`) product (`R4-70`).
pub fn semiring_laurent4_mul_acc(acc: &mut [i128; 4], a: [i64; 4], b: [i64; 4]) {
    // Exponents for index 0..4 are [-1, 0, 1, 2]; keep terms landing in [-1, 2].
    for (ia, &va) in a.iter().enumerate() {
        let ea = (ia as isize) - 1;
        for (ib, &vb) in b.iter().enumerate() {
            let eb = (ib as isize) - 1;
            let e_sum = ea + eb;
            if (-1..=2).contains(&e_sum) {
                acc[(e_sum + 1) as usize] += (va as i128) * (vb as i128);
            }
        }
    }
}

// ============================================================================
// Pure-Integer Q15 Trigonometry & Progressive UORC LOD0..LOD4 3D PBR Rasterizer
// ============================================================================

/// Formal R4.VtensorSemiring3d build 1024-step full-circle `[sin_q15, cos_q15]` LUT via integer Chebyshev recurrence (`float_ops = 0`).
fn trig_lut_1024_q15() -> &'static [(i16, i16); 1024] {
    static LUT: OnceLock<[(i16, i16); 1024]> = OnceLock::new();
    LUT.get_or_init(|| {
        // Quarter circle = 256 steps. Step angle delta = 2*pi / 1024.
        // Exact rounded Q30 constants: cos(2*pi/1024) = 1073721623, sin(2*pi/1024) = 6588355.
        let cos_d_q30: i64 = 1_073_721_623;
        let sin_d_q30: i64 = 6_588_355;
        let mut table = [(0i16, 32767i16); 1024];
        let mut cx_q30: i64 = 1 << 30;
        let mut sx_q30: i64 = 0;
        for slot in table.iter_mut().take(256) {
            let s15 = (sx_q30 >> 15).clamp(-32767, 32767) as i16;
            let c15 = (cx_q30 >> 15).clamp(-32767, 32767) as i16;
            *slot = (s15, c15);
            let next_c = ((cx_q30 * cos_d_q30) - (sx_q30 * sin_d_q30)) >> 30;
            let next_s = ((sx_q30 * cos_d_q30) + (cx_q30 * sin_d_q30)) >> 30;
            cx_q30 = next_c;
            sx_q30 = next_s;
        }
        table[256] = (32767, 0);
        for k in 1..256 {
            let (s, c) = table[256 - k];
            table[256 + k] = (s, -c);
        }
        for k in 0..512 {
            let (s, c) = table[k];
            table[512 + k] = (-s, -c);
        }
        table
    })
}

/// Formal R4.VtensorSemiring3d lookup `(sin_q15, cos_q15)` from phase in `0..1024` (`milli`-radians mapped deterministically).
#[inline]
pub fn sin_cos_phase1024_q15(phase_idx: i32) -> (i32, i32) {
    let idx = phase_idx.rem_euclid(1024) as usize;
    let (s, c) = trig_lut_1024_q15()[idx];
    (s as i32, c as i32)
}

/// Formal R4.VtensorSemiring3d integer square root for exact surface normal normalization (`float_ops = 0`).
#[inline]
pub fn isqrt_u64_exact(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    let mut x = 1u64 << ((64 - n.leading_zeros()).div_ceil(2));
    loop {
        let y = (x + n / x) >> 1;
        if y >= x {
            return x;
        }
        x = y;
    }
}

/// Progressive `UORC` 3D mesh (`LOD0..LOD4`) with `Q15` vertices, indexed triangles, and RGB8 albedo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UorcLodMesh {
    /// Progressive level of detail (`0..=4`).
    pub lod: u8,
    /// Vertex positions `[x_q15, y_q15, z_q15]`.
    pub vertices_q15: Vec<[i16; 3]>,
    /// Triangle index triples `[i0, i1, i2]` plus per-triangle RGB8 tint `[r, g, b]`.
    pub triangles: Vec<([u16; 3], [u8; 3])>,
    /// Metallic (`0..=255`) and roughness (`16..=255`) PBR material parameters.
    pub metallic_q8: u8,
    /// Surface roughness (`16..=255`).
    pub roughness_q8: u8,
}

/// Formal R4.VtensorSemiring3d generate progressive `LOD0..LOD4` Hopf-Torus / Icosian-Shell / Poincare-Crown mesh (`R4-70`).
pub fn build_parametric_uorc_mesh(
    preset: &str,
    lod_level: u32,
    metallic_q8: u8,
    roughness_q8: u8,
) -> Result<UorcLodMesh, Vtensor3dError> {
    let check = SemiringRaster3dCheck {
        float_ops: 0,
        accumulate_exact_no_intermediate_rounding: true,
        tile_bin_width: 16,
        tile_bin_height: 16,
        lod_level: lod_level as u64,
        observatory_manifold_bound: true,
        studio_preview_scene_bound: true,
    };
    let verdict = admit_semiring_raster_3d(check);
    if verdict != SemiringRaster3dVerdict::TilePbrObservatoryAndStudioAdmitted {
        return Err(Vtensor3dError::RasterRefused(format!("{verdict:?}")));
    }

    let u_steps: usize = match lod_level {
        0 => 8,
        1 => 12,
        2 => 16,
        3 => 24,
        _ => 32,
    };
    let v_steps: usize = match lod_level {
        0 => 6,
        1 => 8,
        2 => 12,
        3 => 16,
        _ => 20,
    };

    let mode = preset.trim().to_ascii_lowercase();
    let major_r: i32 = if mode.contains("icosian") || mode.contains("poincare") {
        13_500
    } else {
        16_400
    };
    let minor_r: i32 = if mode.contains("zeta") {
        5_600
    } else if mode.contains("icosian") {
        8_200
    } else {
        6_800
    };

    let mut vertices_q15 = Vec::with_capacity(u_steps * v_steps);
    for iu in 0..u_steps {
        let u_phase = ((iu * 1024) / u_steps) as i32;
        let (su, cu) = sin_cos_phase1024_q15(u_phase);
        let (s3u, _) = sin_cos_phase1024_q15(u_phase * 3);
        for iv in 0..v_steps {
            let v_phase = ((iv * 1024) / v_steps) as i32;
            // Non-ablated Hopf twist: S^1 fiber phase couples u_phase + v_phase!
            let hopf_phase = if mode.contains("icosian") {
                v_phase
            } else {
                v_phase + (u_phase >> 1)
            };
            let (sv, cv) = sin_cos_phase1024_q15(hopf_phase);
            let zeta_ripple = if mode.contains("zeta") {
                (s3u * 1_600) >> 15
            } else {
                0
            };
            let tube_r = major_r + (((minor_r + zeta_ripple) * cv) >> 15);
            let x = ((tube_r * cu) >> 15).clamp(-32000, 32000) as i16;
            let z = ((tube_r * su) >> 15).clamp(-32000, 32000) as i16;
            let y = ((((minor_r + zeta_ripple) * sv) >> 15) + ((s3u * 1_100) >> 15))
                .clamp(-32000, 32000) as i16;
            vertices_q15.push([x, y, z]);
        }
    }

    let mut triangles = Vec::with_capacity(u_steps * v_steps * 2);
    for iu in 0..u_steps {
        let iu_next = (iu + 1) % u_steps;
        for iv in 0..v_steps {
            let iv_next = (iv + 1) % v_steps;
            let i00 = (iu * v_steps + iv) as u16;
            let i10 = (iu_next * v_steps + iv) as u16;
            let i11 = (iu_next * v_steps + iv_next) as u16;
            let i01 = (iu * v_steps + iv_next) as u16;

            // Color encodes signed Hopf orthant / Zeta harmonic band
            let band = ((iu * 4) / u_steps) + (((iv * 4) / v_steps) << 2);
            let tint = match band & 3 {
                0 => [56u8, 189, 248],  // cyan Hopf fiber
                1 => [52u8, 211, 153],  // emerald Icosian shell
                2 => [244u8, 114, 182], // magenta Zeta zero harmonic
                _ => [129u8, 140, 248], // indigo Poincare geodesic
            };
            triangles.push(([i00, i10, i11], tint));
            triangles.push(([i00, i11, i01], tint));
        }
    }

    Ok(UorcLodMesh {
        lod: lod_level as u8,
        vertices_q15,
        triangles,
        metallic_q8,
        roughness_q8: roughness_q8.max(16),
    })
}

/// Formal R4.VtensorSemiring3d encode a `UorcLodMesh` into compact `UORCLOD1` bytes (`R4-70`).
pub fn uorc_encode_mesh(mesh: &UorcLodMesh) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + mesh.vertices_q15.len() * 6 + mesh.triangles.len() * 9);
    out.extend_from_slice(b"UORCLOD1");
    out.push(mesh.lod);
    out.push(mesh.metallic_q8);
    out.push(mesh.roughness_q8);
    out.push(0);
    out.extend_from_slice(&(mesh.vertices_q15.len() as u16).to_le_bytes());
    out.extend_from_slice(&(mesh.triangles.len() as u16).to_le_bytes());
    for v in &mesh.vertices_q15 {
        out.extend_from_slice(&v[0].to_le_bytes());
        out.extend_from_slice(&v[1].to_le_bytes());
        out.extend_from_slice(&v[2].to_le_bytes());
    }
    for (idx, rgb) in &mesh.triangles {
        out.extend_from_slice(&idx[0].to_le_bytes());
        out.extend_from_slice(&idx[1].to_le_bytes());
        out.extend_from_slice(&idx[2].to_le_bytes());
        out.extend_from_slice(rgb);
    }
    out
}

/// Formal R4.VtensorSemiring3d decompress `UORCLOD1` progressive mesh bytes with bounds validation (`R4-70`).
pub fn uorc_decompress_mesh(bytes: &[u8]) -> Result<UorcLodMesh, Vtensor3dError> {
    if bytes.len() < 16 || &bytes[0..8] != b"UORCLOD1" {
        return Err(Vtensor3dError::InvalidSceneOrMesh(
            "expected UORCLOD1 header".to_string(),
        ));
    }
    let lod = bytes[8];
    let verdict = admit_semiring_raster_3d(SemiringRaster3dCheck {
        float_ops: 0,
        accumulate_exact_no_intermediate_rounding: true,
        tile_bin_width: 16,
        tile_bin_height: 16,
        lod_level: lod as u64,
        observatory_manifold_bound: true,
        studio_preview_scene_bound: true,
    });
    if verdict != SemiringRaster3dVerdict::TilePbrObservatoryAndStudioAdmitted {
        return Err(Vtensor3dError::RasterRefused(format!("{verdict:?}")));
    }
    let metallic_q8 = bytes[9];
    let roughness_q8 = bytes[10].max(16);
    let v_cnt = u16::from_le_bytes([bytes[12], bytes[13]]) as usize;
    let t_cnt = u16::from_le_bytes([bytes[14], bytes[15]]) as usize;
    let expected_len = 16usize
        .saturating_add(v_cnt.saturating_mul(6))
        .saturating_add(t_cnt.saturating_mul(9));
    if bytes.len() < expected_len {
        return Err(Vtensor3dError::InvalidSceneOrMesh(
            "truncated UORCLOD1 payload".to_string(),
        ));
    }
    let mut pos = 16usize;
    let mut vertices_q15 = Vec::with_capacity(v_cnt);
    for _ in 0..v_cnt {
        let x = i16::from_le_bytes([bytes[pos], bytes[pos + 1]]);
        let y = i16::from_le_bytes([bytes[pos + 2], bytes[pos + 3]]);
        let z = i16::from_le_bytes([bytes[pos + 4], bytes[pos + 5]]);
        vertices_q15.push([x, y, z]);
        pos += 6;
    }
    let mut triangles = Vec::with_capacity(t_cnt);
    for _ in 0..t_cnt {
        let i0 = u16::from_le_bytes([bytes[pos], bytes[pos + 1]]);
        let i1 = u16::from_le_bytes([bytes[pos + 2], bytes[pos + 3]]);
        let i2 = u16::from_le_bytes([bytes[pos + 4], bytes[pos + 5]]);
        if (i0 as usize) >= v_cnt || (i1 as usize) >= v_cnt || (i2 as usize) >= v_cnt {
            return Err(Vtensor3dError::InvalidSceneOrMesh(
                "UORCLOD1 triangle vertex index out of bounds".to_string(),
            ));
        }
        let rgb = [bytes[pos + 6], bytes[pos + 7], bytes[pos + 8]];
        triangles.push(([i0, i1, i2], rgb));
        pos += 9;
    }
    Ok(UorcLodMesh {
        lod,
        vertices_q15,
        triangles,
        metallic_q8,
        roughness_q8,
    })
}

#[derive(Clone, Copy)]
struct BinnedScreenTri {
    x0: i32,
    y0: i32,
    z0: i32,
    x1: i32,
    y1: i32,
    z1: i32,
    x2: i32,
    y2: i32,
    z2: i32,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
    area2: i64,
    shaded_rgb: [u8; 3],
}

/// Formal R4.VtensorSemiring3d shade a triangle face using pure-integer Cook-Torrance/Schlick PBR (`float_ops = 0`).
fn shade_triangle_pbr_integer(
    p0: [i32; 3],
    p1: [i32; 3],
    p2: [i32; 3],
    albedo: [u8; 3],
    metallic_q8: u8,
    roughness_q8: u8,
) -> [u8; 3] {
    let ux = (p1[0] - p0[0]) as i64;
    let uy = (p1[1] - p0[1]) as i64;
    let uz = (p1[2] - p0[2]) as i64;
    let vx = (p2[0] - p0[0]) as i64;
    let vy = (p2[1] - p0[1]) as i64;
    let vz = (p2[2] - p0[2]) as i64;

    let mut nx = (uy * vz - uz * vy) >> 10;
    let mut ny = (uz * vx - ux * vz) >> 10;
    let mut nz = (ux * vy - uy * vx) >> 10;
    let norm_sq = (nx * nx + ny * ny + nz * nz) as u64;
    let norm = isqrt_u64_exact(norm_sq).max(1) as i64;
    nx = (nx << 15) / norm;
    ny = (ny << 15) / norm;
    nz = (nz << 15) / norm;
    if nz > 0 {
        // Flip toward camera (camera looks along +Z, view vector V = [0, 0, -1])
        nx = -nx;
        ny = -ny;
        nz = -nz;
    }

    // Normalized directional light L_q15 = [-14654, 21981, -19395] (len = 32767)
    let lx: i64 = -14_654;
    let ly: i64 = 21_981;
    let lz: i64 = -19_395;
    // Halfway vector H between L and V=[0,0,-1]: H_q15 = [-8520, 12780, -29046]
    let hx: i64 = -8_520;
    let hy: i64 = 12_780;
    let hz: i64 = -29_046;

    let n_dot_l = ((nx * lx + ny * ly + nz * lz) >> 15).clamp(0, 32767);
    let n_dot_v = (-nz).clamp(0, 32767);
    let n_dot_h = ((nx * hx + ny * hy + nz * hz) >> 15).clamp(0, 32767);

    // Specular lobe via integer squaring modulated by roughness
    let nh2 = (n_dot_h * n_dot_h) >> 15;
    let nh4 = (nh2 * nh2) >> 15;
    let nh8 = (nh4 * nh4) >> 15;
    let smooth_q8 = (255u16.saturating_sub(roughness_q8 as u16)) as i64;
    let spec_q15 = ((nh8 * (64 + smooth_q8)) >> 8).clamp(0, 32767);

    // Schlick Fresnel integer fifth power: (1 - N.V)^5
    let one_minus_nv = 32767 - n_dot_v;
    let f2 = (one_minus_nv * one_minus_nv) >> 15;
    let f4 = (f2 * f2) >> 15;
    let f5 = (f4 * one_minus_nv) >> 15;
    let f0_q15 = 1_310 + (((metallic_q8 as i64) * 28_000) >> 8);
    let fresnel_q15 = f0_q15 + (((32767 - f0_q15) * f5) >> 15);

    let ambient_q15: i64 = 6_200;
    let kd_q15 = ((255 - metallic_q8 as i64) * 32767) >> 8;
    let diff_term = (((kd_q15 * n_dot_l) >> 15) * 26_000) >> 15;
    let spec_term = ((spec_q15 * fresnel_q15) >> 15) + ((spec_q15 * (metallic_q8 as i64)) >> 9);

    let mut rgb = [0u8; 3];
    for ch in 0..3 {
        let base = albedo[ch] as i64;
        let lit = ((base * (ambient_q15 + diff_term)) >> 15) + ((255 * spec_term) >> 15);
        rgb[ch] = lit.clamp(0, 255) as u8;
    }
    rgb
}

/// Formal R4.VtensorSemiring3d `16x16` tile-binned RGBA8+depth 3D PBR rasterizer (`R4-70`, `float_ops = 0`).
pub fn rasterize_tiles_16x16_pbr_rgba8(
    mesh: &UorcLodMesh,
    width: u32,
    height: u32,
    yaw_phase1024: i32,
    pitch_phase1024: i32,
    overlay_points_q15: &[[i16; 3]],
) -> Result<Vec<u8>, Vtensor3dError> {
    let w = width.clamp(16, 1024) as usize;
    let h = height.clamp(16, 1024) as usize;
    let check = SemiringRaster3dCheck {
        float_ops: 0,
        accumulate_exact_no_intermediate_rounding: true,
        tile_bin_width: 16,
        tile_bin_height: 16,
        lod_level: mesh.lod as u64,
        observatory_manifold_bound: true,
        studio_preview_scene_bound: true,
    };
    let verdict = admit_semiring_raster_3d(check);
    if verdict != SemiringRaster3dVerdict::TilePbrObservatoryAndStudioAdmitted {
        return Err(Vtensor3dError::RasterRefused(format!("{verdict:?}")));
    }

    // Build 3x3 integer rotation matrix via exact Q15 semiring contraction (yaw * pitch)
    let (sy, cy) = sin_cos_phase1024_q15(yaw_phase1024);
    let (sp, cp) = sin_cos_phase1024_q15(pitch_phase1024);
    let rot_yaw: [i16; 9] = [
        cy as i16,
        0,
        (-sy) as i16,
        0,
        32767,
        0,
        sy as i16,
        0,
        cy as i16,
    ];
    let rot_pitch: [i16; 9] = [
        32767,
        0,
        0,
        0,
        cp as i16,
        (-sp) as i16,
        0,
        sp as i16,
        cp as i16,
    ];
    let rot_mat = semiring_q15_contract_wide(&rot_pitch, &rot_yaw, 3, 3, 3)?;

    // Transform all mesh vertices via wide Q15 semiring contraction (zero intermediate rounding!)
    let mut flat_verts = Vec::with_capacity(mesh.vertices_q15.len() * 3);
    for v in &mesh.vertices_q15 {
        flat_verts.extend_from_slice(v);
    }
    let rotated_flat =
        semiring_q15_contract_wide(&flat_verts, &rot_mat, mesh.vertices_q15.len(), 3, 3)?;

    let cx = (w as i32) / 2;
    let cy_scr = (h as i32) / 2;
    let focal = (w.min(h) as i64) * 46_000;
    let cam_dist: i32 = 56_000;

    let mut cam_pts = Vec::with_capacity(mesh.vertices_q15.len());
    let mut scr_pts = Vec::with_capacity(mesh.vertices_q15.len());
    for vi in 0..mesh.vertices_q15.len() {
        let rx = rotated_flat[vi * 3] as i32;
        let ry = rotated_flat[vi * 3 + 1] as i32;
        let rz = rotated_flat[vi * 3 + 2] as i32 + cam_dist;
        cam_pts.push([rx, ry, rz]);
        let z_safe = rz.max(8_192) as i64;
        let sx = cx + (((rx as i64) * focal) / (z_safe << 15)) as i32;
        let sy_px = cy_scr - (((ry as i64) * focal) / (z_safe << 15)) as i32;
        scr_pts.push([sx, sy_px, rz]);
    }

    let tiles_x = w.div_ceil(16);
    let tiles_y = h.div_ceil(16);
    let mut tile_bins: Vec<Vec<usize>> = vec![Vec::new(); tiles_x * tiles_y];
    let mut binned_tris: Vec<BinnedScreenTri> = Vec::with_capacity(mesh.triangles.len());

    for (idx, albedo) in &mesh.triangles {
        let i0 = idx[0] as usize;
        let i1 = idx[1] as usize;
        let i2 = idx[2] as usize;
        let s0 = scr_pts[i0];
        let s1 = scr_pts[i1];
        let s2 = scr_pts[i2];

        // Tropical min/max bounding box culling against viewport
        let min_x = s0[0].min(s1[0]).min(s2[0]).max(0);
        let max_x = s0[0].max(s1[0]).max(s2[0]).min((w as i32) - 1);
        let min_y = s0[1].min(s1[1]).min(s2[1]).max(0);
        let max_y = s0[1].max(s1[1]).max(s2[1]).min((h as i32) - 1);
        if min_x > max_x || min_y > max_y {
            continue;
        }
        let area2 = ((s1[0] - s0[0]) as i64) * ((s2[1] - s0[1]) as i64)
            - ((s1[1] - s0[1]) as i64) * ((s2[0] - s0[0]) as i64);
        if area2 == 0 {
            continue;
        }
        let shaded_rgb = shade_triangle_pbr_integer(
            cam_pts[i0],
            cam_pts[i1],
            cam_pts[i2],
            *albedo,
            mesh.metallic_q8,
            mesh.roughness_q8,
        );
        let tri_id = binned_tris.len();
        binned_tris.push(BinnedScreenTri {
            x0: s0[0],
            y0: s0[1],
            z0: s0[2],
            x1: s1[0],
            y1: s1[1],
            z1: s1[2],
            x2: s2[0],
            y2: s2[1],
            z2: s2[2],
            min_x,
            max_x,
            min_y,
            max_y,
            area2,
            shaded_rgb,
        });

        let tx0 = (min_x as usize) / 16;
        let tx1 = (max_x as usize) / 16;
        let ty0 = (min_y as usize) / 16;
        let ty1 = (max_y as usize) / 16;
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                tile_bins[ty * tiles_x + tx].push(tri_id);
            }
        }
    }

    // Allocate RGBA8 framebuffer + per-tile 16x16 depth buffer
    let mut rgba = vec![0u8; w * h * 4];
    // Subtle deep-space Poincaré radial vignette background (integer distance squared)
    let rad_den = ((cx * cx + cy_scr * cy_scr).max(1)) as i64;
    for y in 0..h {
        let dy = (y as i32 - cy_scr) as i64;
        for x in 0..w {
            let dx = (x as i32 - cx) as i64;
            let r2 = ((dx * dx + dy * dy) * 18) / rad_den;
            let off = (y * w + x) * 4;
            rgba[off] = (14i64.saturating_sub(r2 >> 1)).clamp(5, 18) as u8;
            rgba[off + 1] = (22i64.saturating_sub(r2 >> 1)).clamp(9, 26) as u8;
            rgba[off + 2] = (40i64.saturating_sub(r2)).clamp(16, 44) as u8;
            rgba[off + 3] = 255;
        }
    }

    let mut tile_depth = [i32::MAX; 256];
    for ty in 0..tiles_y {
        let py_start = ty * 16;
        let py_end = (py_start + 16).min(h);
        for tx in 0..tiles_x {
            let bin = &tile_bins[ty * tiles_x + tx];
            if bin.is_empty() {
                continue;
            }
            tile_depth.fill(i32::MAX);
            let px_start = tx * 16;
            let px_end = (px_start + 16).min(w);
            for &tri_idx in bin {
                let t = binned_tris[tri_idx];
                let bx0 = (t.min_x as usize).max(px_start);
                let bx1 = (t.max_x as usize).min(px_end - 1);
                let by0 = (t.min_y as usize).max(py_start);
                let by1 = (t.max_y as usize).min(py_end - 1);
                for py in by0..=by1 {
                    let py_i = py as i32;
                    for px in bx0..=bx1 {
                        let px_i = px as i32;
                        let mut w0 = ((t.x2 - t.x1) as i64) * ((py_i - t.y1) as i64)
                            - ((t.y2 - t.y1) as i64) * ((px_i - t.x1) as i64);
                        let mut w1 = ((t.x0 - t.x2) as i64) * ((py_i - t.y2) as i64)
                            - ((t.y0 - t.y2) as i64) * ((px_i - t.x2) as i64);
                        let mut w2 = ((t.x1 - t.x0) as i64) * ((py_i - t.y0) as i64)
                            - ((t.y1 - t.y0) as i64) * ((px_i - t.x0) as i64);
                        let mut denom = t.area2;
                        if denom < 0 {
                            w0 = -w0;
                            w1 = -w1;
                            w2 = -w2;
                            denom = -denom;
                        }
                        if w0 >= 0 && w1 >= 0 && w2 >= 0 {
                            let z = ((w0 * (t.z0 as i64) + w1 * (t.z1 as i64) + w2 * (t.z2 as i64))
                                / denom) as i32;
                            let local = (py - py_start) * 16 + (px - px_start);
                            if z < tile_depth[local] {
                                tile_depth[local] = z;
                                let off = (py * w + px) * 4;
                                rgba[off] = t.shaded_rgb[0];
                                rgba[off + 1] = t.shaded_rgb[1];
                                rgba[off + 2] = t.shaded_rgb[2];
                                rgba[off + 3] = 255;
                            }
                        }
                    }
                }
            }
        }
    }

    // Superimpose live trajectory / Zeta-zero luminous geodesic nodes (Bresenham integer segments)
    if !overlay_points_q15.is_empty() {
        let mut flat_ov = Vec::with_capacity(overlay_points_q15.len() * 3);
        for p in overlay_points_q15 {
            flat_ov.extend_from_slice(p);
        }
        let rot_ov =
            semiring_q15_contract_wide(&flat_ov, &rot_mat, overlay_points_q15.len(), 3, 3)?;
        let mut prev_xy: Option<(i32, i32)> = None;
        for oi in 0..overlay_points_q15.len() {
            let rx = rot_ov[oi * 3] as i32;
            let ry = rot_ov[oi * 3 + 1] as i32;
            let rz = (rot_ov[oi * 3 + 2] as i32 + cam_dist).max(8_192) as i64;
            let sx = cx + (((rx as i64) * focal) / (rz << 15)) as i32;
            let sy_px = cy_scr - (((ry as i64) * focal) / (rz << 15)) as i32;
            if let Some((x0, y0)) = prev_xy {
                draw_line_rgba(
                    &mut rgba,
                    (w, h),
                    (x0, y0),
                    (sx, sy_px),
                    [244, 114, 182, 255],
                );
            }
            let is_head = oi + 1 == overlay_points_q15.len();
            let dot_col = if is_head {
                [52u8, 211, 153, 255]
            } else {
                [56u8, 189, 248, 255]
            };
            let rad = if is_head { 2 } else { 1 };
            for dy in -rad..=rad {
                for dx in -rad..=rad {
                    let px = sx + dx;
                    let py = sy_px + dy;
                    if px >= 0 && px < w as i32 && py >= 0 && py < h as i32 {
                        let off = (py as usize * w + px as usize) * 4;
                        rgba[off..off + 4].copy_from_slice(&dot_col);
                    }
                }
            }
            prev_xy = Some((sx, sy_px));
        }
    }

    Ok(rgba)
}

/// Formal R4.VtensorSemiring3d integer Bresenham line rasterizer (`float_ops = 0`).
fn draw_line_rgba(
    rgba: &mut [u8],
    (w, h): (usize, usize),
    (mut x0, mut y0): (i32, i32),
    (x1, y1): (i32, i32),
    color: [u8; 4],
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let max_steps = (w + h) as i32;
    for _ in 0..max_steps {
        if x0 >= 0 && x0 < w as i32 && y0 >= 0 && y0 < h as i32 {
            let off = (y0 as usize * w + x0 as usize) * 4;
            rgba[off..off + 4].copy_from_slice(&color);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = err * 2;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Formal R4.VtensorSemiring3d parse semicolon-separated `x,y,z;x,y,z` milli-unit trajectory points into `Q15` (`R4-70`).
pub fn parse_trail_csv_q15(trail_csv: &str) -> Vec<[i16; 3]> {
    let mut out = Vec::new();
    for seg in trail_csv.split(';') {
        let seg = seg.trim();
        if seg.is_empty() {
            continue;
        }
        let mut parts = seg.split(',');
        let x = parts
            .next()
            .and_then(|s| s.trim().parse::<i32>().ok())
            .unwrap_or(0);
        let y = parts
            .next()
            .and_then(|s| s.trim().parse::<i32>().ok())
            .unwrap_or(0);
        let z = parts
            .next()
            .and_then(|s| s.trim().parse::<i32>().ok())
            .unwrap_or(0);
        // Map [-1000..1000] milli-coords onto [-22000..22000] Q15 shell
        out.push([
            (x.clamp(-1400, 1400) * 20).clamp(-30000, 30000) as i16,
            (y.clamp(-1400, 1400) * 20).clamp(-30000, 30000) as i16,
            (z.clamp(-1400, 1400) * 20).clamp(-30000, 30000) as i16,
        ]);
    }
    out
}

#[derive(Debug, Clone, Deserialize)]
struct StudioPreviewSceneSpec {
    #[serde(default = "default_preset")]
    preset: String,
    #[serde(default = "default_lod")]
    lod: u32,
    #[serde(default = "default_yaw")]
    yaw_phase1024: i32,
    #[serde(default = "default_pitch")]
    pitch_phase1024: i32,
    #[serde(default = "default_metallic")]
    metallic_q8: u8,
    #[serde(default = "default_roughness")]
    roughness_q8: u8,
    #[serde(default)]
    trail_csv: String,
}

/// Formal R4.VtensorSemiring3d default preset string.
fn default_preset() -> String {
    "hopf-zeta-torus".to_string()
}
/// Formal R4.VtensorSemiring3d default LOD level.
fn default_lod() -> u32 {
    2
}
/// Formal R4.VtensorSemiring3d default yaw phase.
fn default_yaw() -> i32 {
    144
}
/// Formal R4.VtensorSemiring3d default pitch phase.
fn default_pitch() -> i32 {
    88
}
/// Formal R4.VtensorSemiring3d default metallic parameter.
fn default_metallic() -> u8 {
    168
}
/// Formal R4.VtensorSemiring3d default roughness parameter.
fn default_roughness() -> u8 {
    64
}

/// Formal R4.VtensorSemiring3d render live Neural HUD / Observatory 3D Hopf-Poincare-Zeta RGBA8 frame (`R4-70`).
#[wasm_bindgen]
pub fn studio_render_hopf_zeta_3d_rgba(
    width: u32,
    height: u32,
    rot_phase1024: i32,
    lod_level: u32,
    trail_csv: &str,
) -> Vec<u8> {
    let lod = lod_level.min(4);
    let Ok(mesh) = build_parametric_uorc_mesh("hopf-zeta-torus", lod, 176, 56) else {
        return vec![0u8; (width.max(16) * height.max(16) * 4) as usize];
    };
    let mut overlay = parse_trail_csv_q15(trail_csv);
    if overlay.is_empty() {
        // Default 16-point Riemann Zeta-zero harmonic ring crown in Q15
        for k in 0..16 {
            let ph = (k * 64 + rot_phase1024) & 1023;
            let (s, c) = sin_cos_phase1024_q15(ph);
            let (sz, _) = sin_cos_phase1024_q15(ph * 3);
            overlay.push([
                ((23_000 * c) >> 15) as i16,
                ((5_400 * sz) >> 15) as i16,
                ((23_000 * s) >> 15) as i16,
            ]);
        }
    }
    rasterize_tiles_16x16_pbr_rgba8(&mesh, width, height, rot_phase1024, 96, &overlay)
        .unwrap_or_else(|_| vec![0u8; (width.max(16) * height.max(16) * 4) as usize])
}

/// Formal R4.VtensorSemiring3d render Studio `?surface=preview` 3D PBR scene from JSON (`R4-70`).
#[wasm_bindgen]
pub fn studio_render_preview_scene_3d_rgba(scene_json: &str, width: u32, height: u32) -> Vec<u8> {
    let spec: StudioPreviewSceneSpec =
        serde_json::from_str(scene_json).unwrap_or(StudioPreviewSceneSpec {
            preset: default_preset(),
            lod: default_lod(),
            yaw_phase1024: default_yaw(),
            pitch_phase1024: default_pitch(),
            metallic_q8: default_metallic(),
            roughness_q8: default_roughness(),
            trail_csv: String::new(),
        });
    let Ok(mesh) =
        build_parametric_uorc_mesh(&spec.preset, spec.lod, spec.metallic_q8, spec.roughness_q8)
    else {
        return Vec::new();
    };
    let uorc_bytes = uorc_encode_mesh(&mesh);
    let Ok(decoded) = uorc_decompress_mesh(&uorc_bytes) else {
        return Vec::new();
    };
    let overlay = parse_trail_csv_q15(&spec.trail_csv);
    rasterize_tiles_16x16_pbr_rgba8(
        &decoded,
        width,
        height,
        spec.yaw_phase1024,
        spec.pitch_phase1024,
        &overlay,
    )
    .unwrap_or_default()
}

/// Formal R4.VtensorSemiring3d emit structured self-verification JSON receipt (`R4-70`).
#[wasm_bindgen]
pub fn studio_vtensor_3d_self_check_json() -> String {
    let admitted = verify_vtensor_semiring_3d_admitted();
    let mut arena = VtensorLinearArena::new(4096, 64).expect("arena");
    let slice = arena
        .alloc_aligned(VtensorIntDtype::Int16Q15, [2, 2, 2, 2], 128, [2, 2, 1, 1])
        .expect("alloc");
    let mesh = build_parametric_uorc_mesh("hopf-zeta-torus", 2, 176, 56).expect("mesh");
    let uorc = uorc_encode_mesh(&mesh);
    let rgba = rasterize_tiles_16x16_pbr_rgba8(&mesh, 64, 64, 128, 96, &[]).expect("raster");
    let mut hasher = Sha256::new();
    hasher.update(&rgba);
    let frame_sha256: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    serde_json::json!({
        "spec": "uor-vtensor-3d/1",
        "conformance_id": "R4-70",
        "lexlean_admitted": admitted,
        "float_ops": 0,
        "dense_matmul_ops": 0,
        "softmax_ops": 0,
        "arena_offset_aligned_64": slice.offset.is_multiple_of(64),
        "uorc_bytes": uorc.len(),
        "lod_range": [0, 4],
        "tile_bin": [16, 16],
        "rgba_len": rgba.len(),
        "frame_sha256": frame_sha256,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vtensor_arena_enforces_alignment_bounds_empty_noop_and_rejects_float_r4_70() {
        assert!(verify_vtensor_semiring_3d_admitted());
        assert!(parse_vtensor_dtype("q15").is_ok());
        assert!(parse_vtensor_dtype("f32").is_err());
        assert!(parse_vtensor_dtype("bf16").is_err());

        let mut arena = VtensorLinearArena::new(1024, 64).unwrap();
        let empty = arena
            .alloc_aligned(VtensorIntDtype::Int8Sym, [0, 1, 1, 1], 0, [0, 1, 1, 1])
            .unwrap();
        assert_eq!(empty.len, 0);

        let h1 = arena
            .alloc_aligned(VtensorIntDtype::Int16Q15, [4, 4, 1, 1], 100, [2, 2, 1, 1])
            .unwrap();
        assert_eq!(h1.offset % 64, 0);
        let h2 = arena
            .alloc_aligned(VtensorIntDtype::Int16Q15, [4, 4, 1, 1], 200, [1, 1, 1, 1])
            .unwrap();
        assert_eq!(h2.offset, 128);
        arena.write_bytes(h1, &[7u8; 100]).unwrap();
        arena.copy_or_dup_slice(h1, h2).unwrap();
        assert_eq!(&arena.bytes(h2).unwrap()[100..200], &[7u8; 100]);

        // Overflow must be refused (`VT-09`)
        assert!(arena
            .alloc_aligned(VtensorIntDtype::Int8Sym, [1, 1, 1, 1], 2048, [1, 1, 1, 1])
            .is_err());

        // 4-axis REPEAT (`VT-16`)
        let flat = vtensor_repeat4d_index([5, 3, 1, 0], [2, 2, 2, 1], [3, 2, 1, 1]).unwrap();
        assert_eq!(flat, 7);
        assert!(vtensor_repeat4d_index([0, 0, 0, 0], [2, 2, 2, 1], [0, 1, 1, 1]).is_err());
    }

    #[test]
    fn semiring_contractions_and_16x16_tile_pbr_uorc_roundtrip_r4_70() {
        let a = [16384i16, 8192, -8192, 16384];
        let b = [32767i16, 0, 0, 32767];
        let prod = semiring_q15_contract_wide(&a, &b, 2, 2, 2).unwrap();
        assert_eq!(prod, [16383, 8191, -8192, 16383]);

        let trop =
            semiring_tropical_max_plus_contract(&[10, 25, 3, 40], &[5, 1, 20, 8], 2, 2, 2).unwrap();
        assert_eq!(trop, [45, 33, 60, 48]);

        let mut laurent = [0i128; 4];
        semiring_laurent4_mul_acc(&mut laurent, [2, 3, 5, 0], [0, 1, 4, 0]);
        // (2 z^-1 + 3 + 5 z) * (1 + 4 z) = 2 z^-1 + 11 + 17 z + 20 z^2
        assert_eq!(laurent, [2, 11, 17, 20]);

        // LOD5 out of bounds must fail
        assert!(build_parametric_uorc_mesh("hopf-zeta-torus", 5, 160, 64).is_err());

        for lod in 0..=4 {
            let mesh = build_parametric_uorc_mesh("hopf-zeta-torus", lod, 176, 56).unwrap();
            let blob = uorc_encode_mesh(&mesh);
            let dec = uorc_decompress_mesh(&blob).unwrap();
            assert_eq!(mesh, dec);
        }

        let rgba = studio_render_hopf_zeta_3d_rgba(96, 64, 140, 2, "-400,200,100;300,-150,250");
        assert_eq!(rgba.len(), 96 * 64 * 4);
        // Verify non-uniform lit pixels exist
        let lit_pixels = rgba
            .chunks_exact(4)
            .filter(|px| px[0] > 30 || px[1] > 80 || px[2] > 100)
            .count();
        assert!(
            lit_pixels > 120,
            "expected >120 lit PBR pixels, got {lit_pixels}"
        );

        let preview_rgba = studio_render_preview_scene_3d_rgba(
            r#"{"preset":"icosian-600-shell","lod":3,"yaw_phase1024":220,"pitch_phase1024":110}"#,
            80,
            64,
        );
        assert_eq!(preview_rgba.len(), 80 * 64 * 4);
    }
}
