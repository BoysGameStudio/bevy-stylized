#![allow(dead_code)]
//! 编译证据测试共享的配方构造（JSON 实例数据；参数 schema 归目录，
//! 这些只是测试输入，不是第二份参数定义）。

use scenario_model::recipe::SceneRecipe;

pub const OUTLINE_CAMERA: &str = r#"{
    "position": [0.0, 0.4, 6.0],
    "target": [0.0, 0.0, 0.0],
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

pub fn outline_recipe() -> SceneRecipe {
    SceneRecipe::from_json_slice(
        recipe_json("outline", OUTLINE_CAMERA, "4", SETTLE_OBSERVE).as_bytes(),
    )
    .expect("recipe")
}

pub fn face_threshold_json(threshold: f64) -> String {
    let actions = format!(
        r#"[
        {{"id": "settle", "tick": "0", "depends_on": [], "writes": [],
          "kind": {{"type": "fixed_step", "ticks": "1", "step": {{"numerator": "1", "denominator": "60"}}}}}},
        {{"id": "apply", "tick": "1", "depends_on": ["settle"],
          "writes": [{{"scope": "entity", "id": "instances.1"}}],
          "kind": {{"type": "apply_params",
                   "target": {{"scope": "entity", "id": "instances.1"}},
                   "params": {{"face_threshold": {threshold}}}}}}},
        {{"id": "shot", "tick": "1", "depends_on": ["apply"], "writes": [],
          "kind": {{"type": "observe", "observation": "capture", "view": null}}}}
    ]"#
    );
    recipe_json(
        "face",
        r#"{"position": [0.0, 0.3, 6.5], "target": [0.0, 0.0, 0.0], "fov_degrees": 45.0}"#,
        "4",
        &actions,
    )
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
          "writes": [{{"scope": "entity", "id": "root"}}],
          "kind": {{"type": "deform",
                   "target": {{"scope": "entity", "id": "root"}},
                   "channel": "rotation_z", "value": {rotation}}}}},
        {{"id": "s2", "tick": "1", "depends_on": ["bend"], "writes": [],
          "kind": {{"type": "fixed_step", "ticks": "1", "step": {{"numerator": "1", "denominator": "60"}}}}}},
        {{"id": "shot", "tick": "2", "depends_on": ["s2"], "writes": [],
          "kind": {{"type": "observe", "observation": "capture", "view": null}}}}
    ]"#
    );
    SceneRecipe::from_json_slice(
        recipe_json(
            "animation",
            r#"{"position": [0.0, 0.0, 5.0], "target": [0.0, 0.0, 0.0], "fov_degrees": 45.0}"#,
            "4",
            &actions,
        )
        .as_bytes(),
    )
    .expect("recipe")
}

/// 经配方 `parameters` 声明阈值（catalog schema 路径）的 face 配方。
pub fn face_parameters_recipe(threshold: f64) -> SceneRecipe {
    let actions = r#"[
        {"id": "settle", "tick": "0", "depends_on": [], "writes": [],
         "kind": {"type": "fixed_step", "ticks": "1", "step": {"numerator": "1", "denominator": "60"}}},
        {"id": "shot", "tick": "1", "depends_on": ["settle"], "writes": [],
         "kind": {"type": "observe", "observation": "capture", "view": null}}
    ]"#;
    SceneRecipe::from_json_slice(
        recipe_json(
            "face",
            r#"{"position": [0.0, 0.3, 6.5], "target": [0.0, 0.0, 0.0], "fov_degrees": 45.0}"#,
            "4",
            actions,
        )
        .replace(
            "\"parameters\": {}",
            &format!("\"parameters\": {{\"instances.1.face_threshold\": {threshold}}}"),
        )
        .as_bytes(),
    )
    .expect("recipe")
}
