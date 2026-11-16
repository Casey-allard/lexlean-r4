Feature: In-Browser Storage Tier, MicroVM CoW Persistence, Model Serving & SpiralCore Zero-NIC Vsock Relay (R4-49)

  @R4-49 @build
  Scenario: In-browser Foundry storage tier, MicroVM CoW persistence, SmolLM2 model serving, and SpiralCore zero-NIC vsock relay integration
    Given the in-browser storage, persistence, and networking definitions in crates/wasm, public/app.js, and relay/
    When OPFS DAG blob storage, IndexedDB commit-boundary adapter, Web Locks serialization, MicroVM CoW sector persistence, and SpiralCore vsock framing are evaluated
    Then atomic .partial rename with SHA-256 validation, transaction.oncomplete resolution, uor-r4-storage-lock coordination, MVM_COW1 64 KB sector persistence, SmolLM2 model serving, and 16-byte SPVS vsock framing (CID 3 <-> CID 2, port 5257) with seamless relay failover are admitted
    And corrupted SHA-256 blobs, uncommitted IDB transactions, race-prone concurrent writes, unpersisted dirty sectors, and non-SPVS or invalid CID vsock frames are refused
