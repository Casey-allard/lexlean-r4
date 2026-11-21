//! Conformance test for R4-70: Zero-Float `vTensor` Certified Storage Planner (`VT-09`,
//! `VT-10`, `VT-14`, `VT-15`, `VT-16`) & `verdant-tensor` Semiring `16x16` Tile-Binned 3D PBR WebGUI.

#![allow(missing_docs)]

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_vtensor_semiring_3d::{
    admit_semiring_raster_3d, admit_vtensor_storage_plan, four_axis_repeat_volume,
    verify_vtensor_semiring_3d_admitted, SemiringRaster3dCheck, SemiringRaster3dVerdict,
    VtensorStoragePlanCheck, VtensorStorageVerdict,
};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn check_lex_module(root: &Path, rel: &str, expected_mod: &str, required_decls: &[&str]) {
    let raw =
        std::fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("missing {rel}: {e}"));
    assert!(raw.contains(&format!("\\begin{{lexlean}}{{{expected_mod}}}")));
    let (_, after) = raw
        .split_once("\\semanticdata{")
        .unwrap_or_else(|| panic!("missing \\semanticdata{{ in {rel}"));
    let (json_block, _) = after
        .split_once("\n\\end{semanticmodule}")
        .unwrap_or_else(|| panic!("missing \\end{{semanticmodule}} in {rel}"));
    let json_line = json_block
        .trim()
        .strip_suffix('}')
        .unwrap_or(json_block.trim());
    let parsed: Value = serde_json::from_str(json_line).expect("valid JSON semantic module");
    assert_eq!(parsed["spec"], "lexlean/semantic-module/2");
    for sym in required_decls {
        assert!(raw.contains(sym), "{rel} must declare {sym}");
    }
}

#[test]
fn test_r4_70_vtensor_semiring_3d_r4_70() {
    let root = repo_root();

    assert_eq!(four_axis_repeat_volume(4, 2, 2, 1), 16);
    assert_eq!(four_axis_repeat_volume(0, 2, 2, 1), 0);

    let ok_plan = VtensorStoragePlanCheck {
        is_float_dtype: false,
        is_empty_node: false,
        alignment_mod: 0,
        offset_bytes: 4096,
        transfer_bytes: 16384,
        capacity_bytes: 65536,
        repeat_axis0: 4,
        repeat_axis1: 2,
        repeat_axis2: 2,
        repeat_axis3: 1,
    };
    assert_eq!(
        admit_vtensor_storage_plan(ok_plan.clone()),
        VtensorStorageVerdict::StorageGraphAdmitted
    );

    let mut empty_plan = ok_plan.clone();
    empty_plan.is_empty_node = true;
    empty_plan.repeat_axis0 = 0;
    assert_eq!(
        admit_vtensor_storage_plan(empty_plan),
        VtensorStorageVerdict::EmptyNodeNoOpAdmitted
    );

    let mut float_plan = ok_plan.clone();
    float_plan.is_float_dtype = true;
    assert_eq!(
        admit_vtensor_storage_plan(float_plan),
        VtensorStorageVerdict::RefusedFloatStorageDtype
    );

    let mut unaligned = ok_plan.clone();
    unaligned.alignment_mod = 8;
    assert_eq!(
        admit_vtensor_storage_plan(unaligned),
        VtensorStorageVerdict::RefusedUnalignedBufferPlacement
    );

    let mut overflow = ok_plan.clone();
    overflow.transfer_bytes = 65536;
    assert_eq!(
        admit_vtensor_storage_plan(overflow),
        VtensorStorageVerdict::RefusedTransferRangeOverflow
    );

    let mut bad_rep = ok_plan;
    bad_rep.repeat_axis2 = 0;
    assert_eq!(
        admit_vtensor_storage_plan(bad_rep),
        VtensorStorageVerdict::RefusedInvalidFourAxisRepeatFactor
    );

    let ok_raster = SemiringRaster3dCheck {
        float_ops: 0,
        accumulate_exact_no_intermediate_rounding: true,
        tile_bin_width: 16,
        tile_bin_height: 16,
        lod_level: 4,
        observatory_manifold_bound: true,
        studio_preview_scene_bound: true,
    };
    assert_eq!(
        admit_semiring_raster_3d(ok_raster.clone()),
        SemiringRaster3dVerdict::TilePbrObservatoryAndStudioAdmitted
    );

    let mut f_raster = ok_raster.clone();
    f_raster.float_ops = 1;
    assert_eq!(
        admit_semiring_raster_3d(f_raster),
        SemiringRaster3dVerdict::RefusedFloatRasterOrShaderOp
    );

    let mut bad_tile = ok_raster.clone();
    bad_tile.tile_bin_width = 8;
    assert_eq!(
        admit_semiring_raster_3d(bad_tile),
        SemiringRaster3dVerdict::RefusedInvalidTileBinDimension
    );

    let mut bad_lod = ok_raster;
    bad_lod.lod_level = 5;
    assert_eq!(
        admit_semiring_raster_3d(bad_lod),
        SemiringRaster3dVerdict::RefusedProgressiveLodOutOfBounds
    );

    assert!(verify_vtensor_semiring_3d_admitted());

    check_lex_module(
        &root,
        "src/R4/VtensorSemiring3d.lex.tex",
        "R4.VtensorSemiring3d",
        &[
            "VtensorStoragePlanCheck",
            "VtensorStorageVerdict",
            "SemiringRaster3dCheck",
            "SemiringRaster3dVerdict",
            "fourAxisRepeatVolume",
            "admitVtensorStoragePlan",
            "admitSemiringRaster3d",
            "verifyVtensorSemiring3dAdmitted",
            "breaksFloatDtypeVtensorAdmitted",
            "breaksTransferRangeOverflowAdmitted",
            "breaksFloatRasterOpAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/VtensorSemiring3dControls.lex.tex",
        "VtensorSemiring3dControls",
        &[
            "vtensorSemiring3dAdmitted",
            "breaksFloatDtypeVtensorAdmittedCtrl",
            "breaksTransferRangeOverflowAdmittedCtrl",
            "breaksFloatRasterOpAdmittedCtrl",
        ],
    );

    let rs_mod = std::fs::read_to_string(root.join("crates/wasm/src/vtensor_semiring_3d.rs"))
        .expect("crates/wasm/src/vtensor_semiring_3d.rs must exist");
    assert!(rs_mod.contains("pub fn studio_render_hopf_zeta_3d_rgba"));
    assert!(rs_mod.contains("pub fn studio_render_preview_scene_3d_rgba"));
    assert!(rs_mod.contains("pub fn rasterize_tiles_16x16_pbr_rgba8"));

    let app_js = std::fs::read_to_string(root.join("public/app.js")).expect("public/app.js");
    assert!(
        app_js.contains("studio_render_hopf_zeta_3d_rgba"),
        "public/app.js must wire 3D Hopf-Poincare-Zeta PBR rendering into HUD/Observatory"
    );
    assert!(
        app_js.contains("studio_render_preview_scene_3d_rgba") || app_js.contains("uorVtensor3d"),
        "public/app.js must expose 3D WebGUI rendering to ?surface=preview"
    );
}
