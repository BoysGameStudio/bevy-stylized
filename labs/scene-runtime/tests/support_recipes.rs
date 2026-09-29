#![allow(dead_code)]
//! 编译证据测试共享的配方构造（JSON 实例数据；参数 schema 归目录，
//! 这些只是测试输入，不是第二份参数定义）。
//!
//! 05 §10 退役后：全部配方指向 courtyard（唯一注册场景），相机取
//! `scenes::metadata::COURTYARD_CAMERA_PRESETS` 的机位坐标。

use scenario_model::recipe::SceneRecipe;

pub const PANORAMA_CAMERA: &str = r#"{
    "position": [0.0, 3.4, 16.0],
    "target": [0.0, 1.3, -2.0],
    "fov_degrees": 45.0
}"#;

/// center-trio 机位（三对照组特写）。
pub const TRIO_CAMERA: &str = r#"{
    "position": [3.5, 4.0, 4.0],
    "target": [0.0, 1.25, -0.2],
    "fov_degrees": 45.0
}"#;

/// flags 机位（旗帜特写）。
pub const FLAGS_CAMERA: &str = r#"{
    "position": [0.0, 2.4, -0.4],
    "target": [-2.75, 2.0, -4.76],
    "fov_degrees": 45.0
}"#;

pub const SETTLE_OBSERVE: &str = r#"[
    {"id": "settle", "tick": "0", "depends_on": [], "writes": [],
     "kind": {"type": "fixed_step", "ticks": "22", "step": {"numerator": "1", "denominator": "60"}}},
    {"id": "shot", "tick": "0", "depends_on": ["settle"], "writes": [],
     "kind": {"type": "observe", "observation": "capture", "view": null}}
]"#;

pub fn recipe_json(scene: &str, camera: &str, msaa: &str, actions: &str) -> String {
    format!(
        r#"{{
        "schema": {{"major": 1, "minor": 0}},
        "scene": {{"id": "{scene}", "preset": null}},
        "seed": "42",
        "rng": {{"algorithm": "chacha", "version": "1"}},
        "resolution": {{"width": 512, "height": 512}},
        "anti_aliasing": {{"type": "msaa", "samples": {msaa}}},
        "tier": null,
        "required_effects": [],
        "assets": [],
        "camera": {camera},
        "parameters": {{}},
        "actions": {{"nodes": {actions}}}
    }}"#
    )
}

/// courtyard 配方：声明 reference-scene 资产依赖（catalog 要求）。
pub fn courtyard_recipe_json(camera: &str, msaa: &str, actions: &str) -> String {
    recipe_json("courtyard", camera, msaa, actions).replace(
        r#""assets": []"#,
        r#""assets": [{"id": "reference-scene", "source": "fixtures/synthetic/reference-scene", "sha256": null}]"#,
    )
}

pub fn courtyard_recipe() -> SceneRecipe {
    SceneRecipe::from_json_slice(
        courtyard_recipe_json(PANORAMA_CAMERA, "4", SETTLE_OBSERVE).as_bytes(),
    )
    .expect("recipe")
}

pub fn face_threshold_json(threshold: f64) -> String {
    let actions = format!(
        r#"[
        {{"id": "settle", "tick": "0", "depends_on": [], "writes": [],
          "kind": {{"type": "fixed_step", "ticks": "1", "step": {{"numerator": "1", "denominator": "60"}}}}}},
        {{"id": "apply", "tick": "1", "depends_on": ["settle"],
          "writes": [{{"scope": "entity", "id": "trio.face.1"}}],
          "kind": {{"type": "apply_params",
                   "target": {{"scope": "entity", "id": "trio.face.1"}},
                   "params": {{"face_threshold": {threshold}}}}}}},
        {{"id": "shot", "tick": "1", "depends_on": ["apply"], "writes": [],
          "kind": {{"type": "observe", "observation": "capture", "view": null}}}}
    ]"#
    );
    courtyard_recipe_json(TRIO_CAMERA, "4", &actions)
}

pub fn face_threshold_recipe(threshold: f64) -> SceneRecipe {
    SceneRecipe::from_json_slice(face_threshold_json(threshold).as_bytes()).expect("recipe")
}

pub fn strip_deform_recipe(rotation: f64) -> SceneRecipe {
    let actions = format!(
        r#"[
        {{"id": "s1", "tick": "0", "depends_on": [], "writes": [],
          "kind": {{"type": "fixed_step", "ticks": "1", "step": {{"numerator": "1", "denominator": "60"}}}}}},
        {{"id": "bend", "tick": "1", "depends_on": ["s1"],
          "writes": [{{"scope": "entity", "id": "flags.0"}}],
          "kind": {{"type": "deform",
                   "target": {{"scope": "entity", "id": "flags.0"}},
                   "channel": "rotation_z", "value": {rotation}}}}},
        {{"id": "s2", "tick": "1", "depends_on": ["bend"], "writes": [],
          "kind": {{"type": "fixed_step", "ticks": "1", "step": {{"numerator": "1", "denominator": "60"}}}}}},
        {{"id": "shot", "tick": "2", "depends_on": ["s2"], "writes": [],
          "kind": {{"type": "observe", "observation": "capture", "view": null}}}}
    ]"#
    );
    SceneRecipe::from_json_slice(courtyard_recipe_json(FLAGS_CAMERA, "4", &actions).as_bytes())
        .expect("recipe")
}

/// 经配方 `parameters` 声明阈值（catalog schema 路径）的 courtyard 配方。
pub fn face_parameters_recipe(threshold: f64) -> SceneRecipe {
    let actions = r#"[
        {"id": "settle", "tick": "0", "depends_on": [], "writes": [],
         "kind": {"type": "fixed_step", "ticks": "1", "step": {"numerator": "1", "denominator": "60"}}},
        {"id": "shot", "tick": "1", "depends_on": ["settle"], "writes": [],
         "kind": {"type": "observe", "observation": "capture", "view": null}}
    ]"#;
    SceneRecipe::from_json_slice(
        courtyard_recipe_json(TRIO_CAMERA, "4", actions)
            .replace(
                r#""parameters": {}"#,
                &format!(r#""parameters": {{"trio.face.1.face_threshold": {threshold}}}"#),
            )
            .as_bytes(),
    )
    .expect("recipe")
}
