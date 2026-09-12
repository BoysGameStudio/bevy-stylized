//! 12 个 showcase 用例的场景构建器（docs/showcase/01 关卡表）。
//!
//! 全部使用自制/程序化几何与纹理（fixtures 纪律：showcase 不充当
//! fixture；资产最小稳定）。

use bevy::{
    asset::{Assets, RenderAssetUsages},
    color::LinearRgba,
    image::Image,
    math::{Mat4, Quat, Vec3, primitives::Sphere},
    mesh::{Mesh, Meshable, skinning::SkinnedMesh},
    pbr::MeshMaterial3d,
    prelude::{App, Camera3d, Mesh3d, Transform, With},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_stylized::prelude::*;
use bevy_stylized::stylized_render::post::{StylizedDisplayTransform, StylizedPostProcess};

use crate::{case_camera, ramp_image};

fn sphere(app: &mut App, radius: f32) -> bevy::prelude::Handle<Mesh> {
    app.world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(radius))
}

fn cel_material(app: &mut App) -> bevy::prelude::Handle<StylizedCelMaterial> {
    app.world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial::default())
}

// ---------------------------------------------------------------------------
// 00 overview：三个 class 并排
// ---------------------------------------------------------------------------
pub fn overview(app: &mut App) {
    let mesh = sphere(app, 0.9);
    let cel = cel_material(app);
    let character = app
        .world_mut()
        .resource_mut::<Assets<StylizedCharacterMaterial>>()
        .add(StylizedCharacterMaterial {
            params: StylizedCharacterParams::default()
                .with_hair_specular(24.0, 0.4)
                .with_emission(LinearRgba::new(1.0, 0.7, 0.3, 1.0), 0.6),
            ..StylizedCharacterMaterial::default()
        });
    let ramp = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(ramp_image(&[[120, 120, 200, 255]]));
    let hero = app
        .world_mut()
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(StylizedHeroMaterial {
            params: StylizedHeroParams::default().with_ramp_rows(1.0),
            ramp_texture: Some(ramp),
            ..StylizedHeroMaterial::default()
        });
    app.world_mut().spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(cel),
        Transform::from_xyz(-2.4, 0.0, 0.0),
    ));
    app.world_mut().spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(character),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    app.world_mut().spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(hero),
        Transform::from_xyz(2.4, 0.0, 0.0),
    ));
    case_camera(app, Vec3::new(0.0, 0.5, 5.5), Vec3::ZERO);
}

// ---------------------------------------------------------------------------
// 01 ramp：解析 ramp vs 纹理 ramp（行差异）
// ---------------------------------------------------------------------------
pub fn ramp(app: &mut App) {
    let mesh = sphere(app, 0.9);
    let analytic = cel_material(app);
    let ramp = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(ramp_image(&[
            [80, 80, 220, 255],
            [220, 80, 80, 255],
            [80, 200, 80, 255],
        ]));
    let textured = app
        .world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial {
            params: StylizedCelParams::default().with_ramp_rows(3.0),
            ramp_texture: Some(ramp),
            ..StylizedCelMaterial::default()
        });
    // 左：解析 ramp；右三：纹理 ramp 行 0/1/2。
    app.world_mut().spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(analytic),
        Transform::from_xyz(-3.3, 0.0, 0.0),
    ));
    for (i, x) in [-1.1, 1.1, 3.3].into_iter().enumerate() {
        app.world_mut().spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(textured.clone()),
            StylizedInstanceData {
                profile_index: i as u32,
                ..StylizedInstanceData::default()
            },
            Transform::from_xyz(x, 0.0, 0.0),
        ));
    }
    case_camera(app, Vec3::new(0.0, 0.5, 8.0), Vec3::ZERO);
}

// ---------------------------------------------------------------------------
// 02 lighting：key light 方向 + palette + day/night（同帧三段经实例排布；
// view 层昼间设定——昼夜极端见 view 参数测试与 golden 的 day/night 覆盖）
// ---------------------------------------------------------------------------
pub fn lighting(app: &mut App) {
    let mesh = sphere(app, 0.9);
    let material = cel_material(app);
    // 三个球分别：默认昼 / 夜（低 ambient 由 view 驱动不适用 per-instance，
    // 此处展示 key light 方向响应梯度）——横向排布展示 NdotL 变化。
    for x in [-2.4, 0.0, 2.4] {
        app.world_mut().spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(x, 0.0, 0.0),
        ));
    }
    // palette[0] 暖色环境 + key light 侧向 → 明暗梯度清晰。
    let mut settings = app.world_mut().resource_mut::<StylizedViewSettings>();
    settings.0.key_light_direction = Vec3::new(0.9, 0.2, 0.2).normalize();
    settings.0.palette = vec![LinearRgba::new(0.9, 0.75, 0.6, 1.0)];
    case_camera(app, Vec3::new(0.0, 0.5, 6.5), Vec3::ZERO);
}

// ---------------------------------------------------------------------------
// 03 face：face SDF 逐实例阈值扫描
// ---------------------------------------------------------------------------
pub fn face(app: &mut App) {
    let mesh = sphere(app, 0.9);
    // 程序化 face SDF：左亮右暗线性渐变。
    let (w, h) = (128usize, 128usize);
    let mut data = Vec::with_capacity(w * h * 4);
    for _y in 0..h {
        for x in 0..w {
            let v = (255.0 * (1.0 - x as f32 / (w - 1) as f32)) as u8;
            data.extend_from_slice(&[v, v, v, 255]);
        }
    }
    let mut sdf = Image::new_fill(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    sdf.data = Some(data);
    let sdf = app.world_mut().resource_mut::<Assets<Image>>().add(sdf);

    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedCharacterMaterial>>()
        .add(StylizedCharacterMaterial {
            params: StylizedCharacterParams::default()
                .with_face_forward(Vec3::Z, 0.05)
                .with_face_threshold_bias(0.5),
            face_sdf_texture: Some(sdf),
            ..StylizedCharacterMaterial::default()
        });
    for (i, x) in [-2.4, 0.0, 2.4].into_iter().enumerate() {
        let threshold = [Some(0.2), None, Some(0.8)][i];
        app.world_mut().spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            StylizedInstanceData {
                face_threshold: threshold,
                ..StylizedInstanceData::default()
            },
            Transform::from_xyz(x, 0.0, 0.0),
        ));
    }
    case_camera(app, Vec3::new(0.0, 0.3, 6.5), Vec3::ZERO);
}

// ---------------------------------------------------------------------------
// 04 hair：hair 高光 on/off（ILM.G mask）
// ---------------------------------------------------------------------------
pub fn hair(app: &mut App) {
    let mesh = sphere(app, 0.9);
    // ILM（上游布局）：R=metal(0)，G=AO(255)，B=高光抑制(0)，A=band(0)。
    let (w, h) = (16usize, 16usize);
    let mut data = Vec::with_capacity(w * h * 4);
    for _ in 0..w * h {
        data.extend_from_slice(&[0, 255, 0, 0]);
    }
    let mut ilm = Image::new_fill(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    ilm.data = Some(data);
    let ilm = app.world_mut().resource_mut::<Assets<Image>>().add(ilm);

    let off = app
        .world_mut()
        .resource_mut::<Assets<StylizedCharacterMaterial>>()
        .add(StylizedCharacterMaterial {
            params: StylizedCharacterParams::default(),
            ilm_texture: Some(ilm.clone()),
            ..StylizedCharacterMaterial::default()
        });
    let on = app
        .world_mut()
        .resource_mut::<Assets<StylizedCharacterMaterial>>()
        .add(StylizedCharacterMaterial {
            params: StylizedCharacterParams::default().with_hair_specular(32.0, 0.9),
            ilm_texture: Some(ilm),
            ..StylizedCharacterMaterial::default()
        });
    app.world_mut().spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(off),
        Transform::from_xyz(-1.4, 0.0, 0.0),
    ));
    app.world_mut().spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(on),
        Transform::from_xyz(1.4, 0.0, 0.0),
    ));
    case_camera(app, Vec3::new(0.0, 0.3, 5.5), Vec3::ZERO);
}

// ---------------------------------------------------------------------------
// 05 metal：matcap on/off（hero）
// ---------------------------------------------------------------------------
pub fn metal(app: &mut App) {
    let mesh = sphere(app, 0.9);
    // 程序化 matcap：径向金属渐变。
    let (w, h) = (128usize, 128usize);
    let mut data = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 / w as f32 - 0.5;
            let dy = y as f32 / h as f32 - 0.5;
            let r = (dx * dx + dy * dy).sqrt();
            let v = ((1.0 - (r * 2.0).clamp(0.0, 1.0)) * 255.0) as u8;
            data.extend_from_slice(&[v, v / 2, 255 - v, 255]);
        }
    }
    let mut matcap = Image::new_fill(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    matcap.data = Some(data);
    let matcap = app.world_mut().resource_mut::<Assets<Image>>().add(matcap);
    // ILM（上游布局）：R=metal mask 全 1，G=AO 全 1。
    let mut ilm_data = Vec::new();
    for _ in 0..16 * 16 {
        ilm_data.extend_from_slice(&[255, 255, 0, 0]);
    }
    let mut ilm = Image::new_fill(
        Extent3d {
            width: 16,
            height: 16,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    ilm.data = Some(ilm_data);
    let ilm = app.world_mut().resource_mut::<Assets<Image>>().add(ilm);

    let off = app
        .world_mut()
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(StylizedHeroMaterial {
            ilm_texture: Some(ilm.clone()),
            ..StylizedHeroMaterial::default()
        });
    let on = app
        .world_mut()
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(StylizedHeroMaterial {
            params: StylizedHeroParams::default()
                .with_matcap(0.9, LinearRgba::new(1.0, 0.9, 0.8, 1.0)),
            ilm_texture: Some(ilm),
            matcap_texture: Some(matcap),
            ..StylizedHeroMaterial::default()
        });
    app.world_mut().spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(off),
        Transform::from_xyz(-1.4, 0.0, 0.0),
    ));
    app.world_mut().spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(on),
        Transform::from_xyz(1.4, 0.0, 0.0),
    ));
    case_camera(app, Vec3::new(0.0, 0.3, 5.5), Vec3::ZERO);
}

// ---------------------------------------------------------------------------
// 06 outline：宽度 0/2/8 + 逐实例缩放（缩放开轮廓材质宽度仍全局；
// 逐实例缩放为 ADR-0004 预留位，当前展示宽度梯度）
// ---------------------------------------------------------------------------
pub fn outline(app: &mut App) {
    let mesh = sphere(app, 0.8);
    let material = cel_material(app);
    for (i, width) in [0.0, 2.0, 8.0].into_iter().enumerate() {
        let outline = app
            .world_mut()
            .resource_mut::<Assets<StylizedOutlineMaterial>>()
            .add(StylizedOutlineMaterial {
                params: StylizedOutlineParams::default()
                    .with_color(LinearRgba::new(0.02, 0.02, 0.04, 1.0))
                    .with_width(width),
            });
        let mut entity = app.world_mut().spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz((i as f32 - 1.0) * 2.2, 0.0, 0.0),
        ));
        if width > 0.0 {
            entity.insert(StylizedOutline::new(outline));
        }
    }
    case_camera(app, Vec3::new(0.0, 0.4, 6.0), Vec3::ZERO);
}

// ---------------------------------------------------------------------------
// 07 animation：程序化双骨骼蒙皮条带摆动
// ---------------------------------------------------------------------------
pub fn animation(app: &mut App) {
    // 竖直条带：两段关节权重。
    let (cols, rows) = (2usize, 16usize);
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut joints: Vec<[u16; 4]> = Vec::new();
    let mut weights = Vec::new();
    for r in 0..=rows {
        let v = r as f32 / rows as f32;
        for c in 0..=cols {
            let u = c as f32 / cols as f32;
            positions.push([(u - 0.5) * 0.8, v * 2.4 - 1.2, 0.0]);
            normals.push([0.0, 0.0, 1.0]);
            uvs.push([u, v]);
            let j = if v > 0.5 { 1 } else { 0 };
            joints.push([j as u16, 0, 0, 0]);
            weights.push([1.0, 0.0, 0.0, 0.0]);
        }
    }
    let mut indices = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let i = (r * (cols + 1) + c) as u32;
            let w = cols as u32 + 1;
            indices.extend_from_slice(&[i, i + 1, i + w, i + 1, i + w + 1, i + w]);
        }
    }
    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_JOINT_INDEX,
        bevy::mesh::VertexAttributeValues::Uint16x4(joints),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_JOINT_WEIGHT,
        bevy::mesh::VertexAttributeValues::Float32x4(weights),
    );
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));
    let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);

    // 双骨骼：root + tip。
    let root = app
        .world_mut()
        .spawn(Transform::from_xyz(0.0, -1.2, 0.0))
        .id();
    let tip = app
        .world_mut()
        .spawn(Transform::from_xyz(0.0, 0.0, 0.0))
        .id();
    app.world_mut().entity_mut(root).add_child(tip);
    let inverse_bindposes = app
        .world_mut()
        .resource_mut::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
        .add(bevy::mesh::skinning::SkinnedMeshInverseBindposes::from(
            vec![
                Mat4::from_translation(Vec3::new(0.0, 1.2, 0.0)),
                Mat4::IDENTITY,
            ],
        ));
    let material = cel_material(app);
    app.world_mut().spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        SkinnedMesh {
            inverse_bindposes,
            joints: vec![root, tip],
        },
        Transform::IDENTITY,
    ));
    // 摆一下再捕获：旋转 root 让条带弯曲。
    app.world_mut()
        .entity_mut(root)
        .get_mut::<Transform>()
        .expect("transform")
        .rotation = Quat::from_rotation_z(0.5);
    case_camera(app, Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO);
}

// ---------------------------------------------------------------------------
// 08 post：vignette + chromatic + display transform hook（单 pass）
// ---------------------------------------------------------------------------
pub fn post(app: &mut App) {
    let mesh = sphere(app, 0.9);
    let material = cel_material(app);
    for x in [-1.3, 1.3] {
        app.world_mut().spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(x, 0.0, 0.0),
        ));
    }
    case_camera(app, Vec3::new(0.0, 0.3, 5.5), Vec3::ZERO);
    // 相机组件挂后处理参数（View 层开关默认开）。
    let mut cameras = app
        .world_mut()
        .query_filtered::<bevy::prelude::Entity, With<Camera3d>>();
    let camera = cameras.single(app.world()).expect("camera");
    app.world_mut()
        .entity_mut(camera)
        .insert(StylizedPostProcess {
            vignette_strength: 0.5,
            chromatic_aberration: 0.004,
            display_transform: StylizedDisplayTransform::SimpleCompress,
            ..StylizedPostProcess::default()
        });
}

// ---------------------------------------------------------------------------
// 09 stress：1000 实例（复用 benchmarks 的场景构建器，不复制）
// ---------------------------------------------------------------------------
pub fn stress(app: &mut App) {
    benchmarks::build_portable_instances(app, 1000);
    case_camera(app, Vec3::new(0.0, 6.0, 34.0), Vec3::ZERO);
}

// ---------------------------------------------------------------------------
// 10 reference：fixture reference_scene.glb（production 路径）
// ---------------------------------------------------------------------------
pub fn reference(app: &mut App) {
    // 与三层对比同场景同机位（常数见 crates/dev/reference-comparison）。
    let scene = app
        .world()
        .resource::<bevy::asset::AssetServer>()
        .load("reference_scene.glb#Scene0");
    app.world_mut()
        .spawn(bevy::world_serialization::WorldAssetRoot(scene));
    case_camera(app, Vec3::new(0.0, 1.1, 4.2), Vec3::new(0.0, 0.9, 0.0));
}

// ---------------------------------------------------------------------------
// 11 performance：benchmark 场景捕获 + 运行 JSON baseline 测量
// ---------------------------------------------------------------------------
pub fn performance(app: &mut App) {
    // 捕获面：hero_1 + portable_1000 同帧（benchmark 场景构建器复用）。
    benchmarks::build_hero_1(app);
    benchmarks::build_portable_instances(app, 100);
    case_camera(app, Vec3::new(0.0, 4.0, 20.0), Vec3::ZERO);
}

// ---------------------------------------------------------------------------
// outline-advanced（V2-a）：hybrid = hull 剪影 + screen-space 内部边缘
// ---------------------------------------------------------------------------
pub fn outline_advanced(app: &mut App) {
    let material = cel_material(app);
    let outline = app
        .world_mut()
        .resource_mut::<Assets<StylizedOutlineMaterial>>()
        .add(StylizedOutlineMaterial {
            params: StylizedOutlineParams::default()
                .with_color(LinearRgba::new(0.02, 0.02, 0.04, 1.0))
                .with_width(2.5),
        });
    let big_box = app.world_mut().resource_mut::<Assets<Mesh>>().add(
        bevy::math::primitives::Cuboid::from_size(Vec3::new(2.0, 0.8, 1.6)),
    );
    let small_box = app.world_mut().resource_mut::<Assets<Mesh>>().add(
        bevy::math::primitives::Cuboid::from_size(Vec3::new(0.9, 0.9, 0.9)),
    );
    let ball = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(0.55));
    for (mesh, translation) in [
        (big_box, Vec3::new(0.0, -0.4, 0.0)),
        (small_box.clone(), Vec3::new(0.0, 0.45, 0.0)),
        (small_box, Vec3::new(0.35, 0.45, 0.2)),
        (ball, Vec3::new(0.7, 0.55, 0.1)),
    ] {
        app.world_mut().spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            StylizedOutline::new(outline.clone()),
            Transform::from_translation(translation),
        ));
    }
    case_camera(app, Vec3::new(2.2, 1.6, 3.4), Vec3::new(0.0, 0.3, 0.0));
    // per-camera opt-in：screen-space 内部边缘（Standard+）。
    let mut cameras = app
        .world_mut()
        .query_filtered::<bevy::prelude::Entity, With<Camera3d>>();
    let camera = cameras.single(app.world()).expect("camera");
    app.world_mut()
        .entity_mut(camera)
        .insert(bevy_stylized::stylized_render::screen_outline::StylizedScreenOutline::default());
}

// ---------------------------------------------------------------------------
// lod（V2-b）：同一 hero 角色随距离梯度降级（near→very far；永不降 class）
// ---------------------------------------------------------------------------
/// LOD 用例的相机/布局常数（单测断言分带用）。
pub const LOD_CAMERA_EYE: Vec3 = Vec3::ZERO;
/// （角度度, 距离, 半径）× 4：near/mid/far/very_far。
pub const LOD_LAYOUT: [(f32, f32, f32); 4] = [
    (-16.0, 6.0, 0.62),
    (-4.0, 17.0, 1.7),
    (6.0, 32.0, 3.2),
    (16.0, 55.0, 5.7),
];

pub fn lod(app: &mut App) {
    // 程序化 matcap（径向金属渐变）+ ILM（R=metal mask 全 1，G=AO 全 1）。
    let (w, h) = (64usize, 64usize);
    let mut data = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 / w as f32 - 0.5;
            let dy = y as f32 / h as f32 - 0.5;
            let r = (dx * dx + dy * dy).sqrt();
            let v = ((1.0 - (r * 2.0).clamp(0.0, 1.0)) * 255.0) as u8;
            data.extend_from_slice(&[v, v / 2, 255 - v, 255]);
        }
    }
    let mut matcap = Image::new_fill(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    matcap.data = Some(data);
    let matcap = app.world_mut().resource_mut::<Assets<Image>>().add(matcap);
    let mut ilm_data = Vec::new();
    for _ in 0..16 * 16 {
        ilm_data.extend_from_slice(&[255, 255, 0, 0]);
    }
    let mut ilm = Image::new_fill(
        Extent3d {
            width: 16,
            height: 16,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    ilm.data = Some(ilm_data);
    let ilm = app.world_mut().resource_mut::<Assets<Image>>().add(ilm);

    // 共享 hero 材质：matcap + hair 高光全开（near 全特性；远处经
    // per-instance flags 削减，材质资产不变）。
    let mut hero_params =
        StylizedHeroParams::default().with_matcap(0.9, LinearRgba::new(1.0, 0.9, 0.8, 1.0));
    hero_params.character = hero_params.character.with_hair_specular(28.0, 0.8);
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(StylizedHeroMaterial {
            params: hero_params,
            ilm_texture: Some(ilm),
            matcap_texture: Some(matcap),
            ..StylizedHeroMaterial::default()
        });

    for (angle_deg, dist, radius) in LOD_LAYOUT {
        let x = dist * angle_deg.to_radians().tan();
        let position = Vec3::new(x, 0.0, -dist);
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Sphere::new(radius));
        // 几何 LOD：far 起换低模（band 2/3 声明了映射才换 Mesh3d）。
        let low = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Sphere::new(radius).mesh().uv(8, 6));
        let mut lod_meshes = StylizedLodMeshes::new(mesh.clone());
        lod_meshes.band_meshes[2] = Some(low.clone());
        lod_meshes.band_meshes[3] = Some(low);
        app.world_mut().spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            StylizedInstanceData::default(),
            lod_meshes,
            Transform::from_translation(position),
        ));
    }
    case_camera(app, LOD_CAMERA_EYE, Vec3::new(0.0, 0.0, -10.0));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 布局确定性：四球落在 near/mid/far/very_far 四带（相机原点）。
    #[test]
    fn lod_layout_bands_are_deterministic() {
        let thresholds = bevy_stylized::stylized_core::StylizedLodThresholds::default();
        let expected = [0, 1, 2, 3];
        for (i, (angle_deg, dist, _)) in LOD_LAYOUT.iter().enumerate() {
            let x = dist * angle_deg.to_radians().tan();
            let actual = (Vec3::new(x, 0.0, -dist) - LOD_CAMERA_EYE).length();
            let band = bevy_stylized::stylized_core::resolve_lod_band(actual, &thresholds);
            assert_eq!(band, expected[i], "layout[{i}] distance {actual}");
            // 离带边界留 ≥15% 余量（防止布局微调时静默跨带）。
            let bounds = [
                0.0,
                thresholds.mid_start,
                thresholds.far_start,
                thresholds.very_far_start,
                f32::INFINITY,
            ];
            let margin = (actual - bounds[band as usize]).min(bounds[band as usize + 1] - actual);
            assert!(margin > 1.0, "layout[{i}] too close to band edge: {actual}");
        }
    }
}
