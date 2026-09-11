//! ADR-0028 证据测试：`_NPR_*` 顶点属性的 GltfPlugin 初始化顺序。
//!
//! - 正确顺序：经 facade helper 配置的 App 能加载携带 `_NPR_MASK` 与
//!   namespaced extras 的合成 glTF，属性完整进入 `Mesh`；
//! - 错误顺序：GltfPlugin 已加入 App 后再配置 → fail-closed，报明确错误，
//!   而不是静默丢弃 attribute。
//!
//! 本测试不需要 GPU。

#![cfg(feature = "gltf")]

use std::{io, path::Path, thread};

use bevy::{
    asset::{AssetMetaCheck, LoadState},
    gltf::GltfAssetLabel,
    mesh::VertexAttributeValues,
    prelude::*,
};
use bevy_stylized::prelude::*;

/// 生成一个最小合成 glTF：单三角形，携带 `_NPR_MASK` attribute 与
/// `bevy_stylized.material/v1` extras；position/mask 数据写入外部 bin。
fn write_synthetic_gltf(dir: &Path) -> io::Result<()> {
    let positions: [[f32; 3]; 3] = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let mask: [f32; 3] = [0.0, 0.5, 1.0];
    let mut bin = Vec::new();
    for position in positions {
        for component in position {
            bin.extend_from_slice(&component.to_le_bytes());
        }
    }
    for value in mask {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    std::fs::write(dir.join("triangle.bin"), &bin)?;

    let gltf = serde_json::json!({
        "asset": { "version": "2.0" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0, "name": "tri" }],
        "meshes": [{
            "primitives": [{
                "attributes": { "POSITION": 0, "_NPR_MASK": 1 },
                "material": 0,
                "mode": 4
            }]
        }],
        "materials": [{
            "name": "hero",
            "extras": {
                "bevy_stylized.material/v1": {
                    "name": "hero",
                    "surface_class": "character",
                    "static_params": { "hair_specular": 0.5 }
                }
            }
        }],
        "buffers": [{ "uri": "triangle.bin", "byteLength": bin.len() }],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
            { "buffer": 0, "byteOffset": 36, "byteLength": 12 }
        ],
        "accessors": [
            {
                "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
                "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 0.0]
            },
            { "bufferView": 1, "componentType": 5126, "count": 3, "type": "SCALAR" }
        ]
    });
    std::fs::write(
        dir.join("triangle.gltf"),
        serde_json::to_string_pretty(&gltf).expect("json serialize"),
    )
}

fn build_loader_app(asset_root: &Path) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::transform::TransformPlugin)
        .add_plugins(AssetPlugin {
            file_path: asset_root.to_string_lossy().into_owned(),
            meta_check: AssetMetaCheck::Never,
            ..default()
        })
        .add_plugins(bevy::world_serialization::WorldSerializationPlugin)
        .add_plugins(bevy::scene::ScenePlugin)
        .add_plugins(bevy::image::ImagePlugin::default())
        .add_plugins(bevy::mesh::MeshPlugin)
        .add_plugins(bevy::animation::AnimationPlugin)
        .init_asset::<StandardMaterial>()
        // 正确顺序（ADR-0028）：先经 facade helper 配置 GltfPlugin，再加入 App。
        .add_plugins(stylized_gltf_plugin())
        .add_plugins(StylizedPlugin::default());
    app.finish();
    app.cleanup();
    app
}

fn load_primitive_mesh(app: &mut App, asset: &'static str) -> Handle<Mesh> {
    let handle = app.world().resource::<AssetServer>().load(
        GltfAssetLabel::Primitive {
            mesh: 0,
            primitive: 0,
        }
        .from_asset(asset),
    );
    for _ in 0..2_000 {
        app.update();
        match app
            .world()
            .resource::<AssetServer>()
            .get_load_state(&handle)
        {
            Some(LoadState::Loaded) => return handle,
            Some(LoadState::Failed(error)) => {
                panic!("mesh asset failed to load: {asset}: {error}");
            }
            Some(LoadState::NotLoaded | LoadState::Loading) | None => thread::yield_now(),
        }
    }
    panic!("mesh asset did not load: {asset}");
}

#[test]
fn facade_configured_app_loads_npr_attribute_and_extras() {
    // Given
    let dir = tempfile::tempdir().expect("tempdir");
    write_synthetic_gltf(dir.path()).expect("write synthetic gltf");
    let mut app = build_loader_app(dir.path());

    // When
    let mesh_handle = load_primitive_mesh(&mut app, "triangle.gltf");

    // Then：`_NPR_MASK` 完整进入 Mesh（Float32，3 个顶点）。
    let meshes = app.world().resource::<Assets<Mesh>>();
    let mesh = meshes.get(&mesh_handle).expect("mesh asset");
    assert!(
        mesh.contains_attribute(bevy_stylized::stylized_gltf::ATTRIBUTE_NPR_MASK),
        "loaded mesh must carry the registered _NPR_MASK attribute"
    );
    match mesh.attribute(bevy_stylized::stylized_gltf::ATTRIBUTE_NPR_MASK) {
        Some(VertexAttributeValues::Float32(values)) => {
            assert_eq!(values.as_slice(), [0.0, 0.5, 1.0]);
        }
        other => panic!("unexpected _NPR_MASK values: {other:?}"),
    }

    // And：namespaced extras 可 typed 导入为 core descriptor（CPU 侧解析）。
    let gltf_json = std::fs::read(dir.path().join("triangle.gltf")).expect("read gltf");
    let document = gltf::Gltf::from_slice_without_validation(&gltf_json)
        .expect("gltf document")
        .document;
    let materials = bevy_stylized::stylized_gltf::extract_materials_from_document(&document)
        .expect("typed extras import");
    assert_eq!(materials.len(), 1);
    let (name, descriptor) = &materials[0];
    assert_eq!(name, "hero");
    assert_eq!(descriptor.surface_class, StylizedSurfaceClass::Character);
    assert_eq!(descriptor.static_params["hair_specular"], 0.5);
}

#[test]
fn configuring_gltf_after_plugin_added_fails_closed() {
    // Given：错误顺序——plain GltfPlugin 已加入 App。
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin {
            meta_check: AssetMetaCheck::Never,
            ..default()
        })
        .add_plugins(bevy::gltf::GltfPlugin::default());

    // When / Then：顺序守卫 fail-closed，错误信息明确指出正确做法。
    let error = check_gltf_plugin_not_added(&app).expect_err("must fail closed");
    assert_eq!(error, StylizedGltfConfigError::GltfPluginAlreadyAdded);
    let message = error.to_string();
    assert!(
        message.contains("BEFORE"),
        "error must explain the ordering requirement, got: {message}"
    );

    // And：正确路径下守卫放行。
    let fresh = App::new();
    assert!(check_gltf_plugin_not_added(&fresh).is_ok());
}
