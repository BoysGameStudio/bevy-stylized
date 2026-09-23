//! 场景内容构建器（`build` feature）：几何/纹理/蒙皮原语与内容组合。
//!
//! 契约（场景↔宿主边界）：构建器组装内容并返回 [`CameraRecipe`]，
//! 不创建相机、捕获 target 或窗口；输出绑定归宿主。
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
    prelude::{App, Entity, Mesh3d, Transform, World},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_stylized::prelude::*;
use bevy_stylized::stylized_render::post::{StylizedDisplayTransform, StylizedPostProcess};

use crate::metadata::LOD_LAYOUT;
use crate::recipe::{CameraAttachment, CameraRecipe};

/// 可寻址场景节点：稳定逻辑 ID 的注册组件（docs/showcase/01 line 92/224
/// 的稳定逻辑 ID 合同）。构建器在为动作寻址的节点上挂载本组件；
/// ID 由构建代码确定（重建按同一确定性规则分配），运行时 Entity 不进入
/// 配方，动作一律以逻辑 ID 寻址。注册合同：ID 在场景内唯一、稳定、
/// 不带宿主/平台语义。
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneTarget(pub &'static str);

fn sphere(world: &mut World, radius: f32) -> bevy::prelude::Handle<Mesh> {
    world
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(radius))
}

fn cel_material(world: &mut World) -> bevy::prelude::Handle<StylizedCelMaterial> {
    world
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial::default())
}

/// 生成 ramp 测试纹理（rows × 64，左半行色右半白）。
pub fn ramp_image(rows: &[[u8; 4]]) -> Image {
    let mut data = Vec::new();
    for row in rows {
        for col in 0..64 {
            let px = if col >= 32 {
                [245, 245, 245, 255]
            } else {
                *row
            };
            data.extend_from_slice(&px);
        }
    }
    let mut image = Image::new_fill(
        Extent3d {
            width: 64,
            height: rows.len() as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.data = Some(data);
    image
}

// ---------------------------------------------------------------------------
// 共享原语：无相机的条带与实例组合
// ---------------------------------------------------------------------------

/// 程序化双骨骼蒙皮条带（无相机）。
///
/// 返回 root 关节实体：调用方各自决定姿态/驱动（showcase 摆一次定姿态，
/// regression 挂标记组件并以确定性帧计数驱动），条带几何/蒙皮只此一份。
pub fn build_strip(app: &mut App) -> Entity {
    build_strip_world(app.world_mut())
}

/// [`build_strip`] 的 `&mut World` 形式（同一实现；App 形式仅为适配）。
pub fn build_strip_world(world: &mut World) -> Entity {
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
            let j = u16::from(v > 0.5);
            joints.push([j, 0, 0, 0]);
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
    let mesh = world.resource_mut::<Assets<Mesh>>().add(mesh);

    // 双骨骼：root + tip（动作寻址的稳定逻辑 ID）。
    let root = world.spawn(Transform::from_xyz(0.0, -1.2, 0.0)).id();
    let tip = world.spawn(Transform::from_xyz(0.0, 0.0, 0.0)).id();
    world.entity_mut(root).insert(SceneTarget("root"));
    world.entity_mut(tip).insert(SceneTarget("tip"));
    world.entity_mut(root).add_child(tip);
    let inverse_bindposes = world
        .resource_mut::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
        .add(bevy::mesh::skinning::SkinnedMeshInverseBindposes::from(
            vec![
                Mat4::from_translation(Vec3::new(0.0, 1.2, 0.0)),
                Mat4::IDENTITY,
            ],
        ));
    let material = cel_material(world);
    world.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        SkinnedMesh {
            inverse_bindposes,
            joints: vec![root, tip],
        },
        Transform::IDENTITY,
    ));
    root
}

/// `hero_1`：1 个 Hero 材质球（全特性开）。
pub fn build_hero_1(app: &mut App) {
    build_hero_1_world(app.world_mut())
}

/// [`build_hero_1`] 的 `&mut World` 形式（同一实现；App 形式仅为适配）。
pub fn build_hero_1_world(world: &mut World) {
    let mesh = world.resource_mut::<Assets<Mesh>>().add(Sphere::new(1.2));
    let ramp = world
        .resource_mut::<Assets<Image>>()
        .add(ramp_image(&[[120, 120, 200, 255]]));
    let material = world
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(StylizedHeroMaterial {
            params: StylizedHeroParams::default().with_ramp_rows(1.0),
            ramp_texture: Some(ramp),
            ..StylizedHeroMaterial::default()
        });
    world.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 1.2, 0.0),
    ));
}

/// portable_N：N 个 Cel 实例（同 mesh+material，profile_index 差异）。
pub fn build_portable_instances(app: &mut App, n: u32) {
    build_portable_instances_world(app.world_mut(), n)
}

/// [`build_portable_instances`] 的 `&mut World` 形式（同一实现；App 形式仅为适配）。
pub fn build_portable_instances_world(world: &mut World, n: u32) {
    let mesh = world.resource_mut::<Assets<Mesh>>().add(Sphere::new(0.3));
    let ramp = world.resource_mut::<Assets<Image>>().add(ramp_image(&[
        [80, 80, 220, 255],
        [220, 80, 80, 255],
        [80, 200, 80, 255],
        [220, 200, 80, 255],
    ]));
    let material = world
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial {
            params: StylizedCelParams::default().with_ramp_rows(4.0),
            ramp_texture: Some(ramp),
            ..StylizedCelMaterial::default()
        });
    let cols = (n as f32).sqrt().ceil() as u32;
    for i in 0..n {
        let x = (i % cols) as f32 - cols as f32 * 0.5;
        // Diameter is 0.6: leave a gap so instanced draw ordering cannot
        // choose different surfaces at equal-depth sphere intersections.
        let y = (i / cols) as f32 * 0.65;
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            StylizedInstanceData {
                profile_index: i % 4,
                ..StylizedInstanceData::default()
            },
            Transform::from_xyz(x * 0.85, y, 0.0),
        ));
    }
}

// ---------------------------------------------------------------------------
// 00 overview：三个 class 并排
// ---------------------------------------------------------------------------
pub fn overview(world: &mut World) -> CameraRecipe {
    let mesh = sphere(world, 0.9);
    let cel = cel_material(world);
    let character = world
        .resource_mut::<Assets<StylizedCharacterMaterial>>()
        .add(StylizedCharacterMaterial {
            params: StylizedCharacterParams::default()
                .with_hair_specular(24.0, 0.4)
                .with_emission(LinearRgba::new(1.0, 0.7, 0.3, 1.0), 0.6),
            ..StylizedCharacterMaterial::default()
        });
    let ramp = world
        .resource_mut::<Assets<Image>>()
        .add(ramp_image(&[[120, 120, 200, 255]]));
    let hero = world
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(StylizedHeroMaterial {
            params: StylizedHeroParams::default().with_ramp_rows(1.0),
            ramp_texture: Some(ramp),
            ..StylizedHeroMaterial::default()
        });
    world.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(cel),
        Transform::from_xyz(-2.4, 0.0, 0.0),
    ));
    world.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(character),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    world.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(hero),
        Transform::from_xyz(2.4, 0.0, 0.0),
    ));
    CameraRecipe::new(Vec3::new(0.0, 0.5, 5.5), Vec3::ZERO)
}

// ---------------------------------------------------------------------------
// 01 ramp：解析 ramp vs 纹理 ramp（行差异）
// ---------------------------------------------------------------------------
pub fn ramp(world: &mut World) -> CameraRecipe {
    let mesh = sphere(world, 0.9);
    let analytic = cel_material(world);
    let ramp = world.resource_mut::<Assets<Image>>().add(ramp_image(&[
        [80, 80, 220, 255],
        [220, 80, 80, 255],
        [80, 200, 80, 255],
    ]));
    let textured = world
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial {
            params: StylizedCelParams::default().with_ramp_rows(3.0),
            ramp_texture: Some(ramp),
            ..StylizedCelMaterial::default()
        });
    // 左：解析 ramp；右三：纹理 ramp 行 0/1/2。
    world.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(analytic),
        Transform::from_xyz(-3.3, 0.0, 0.0),
    ));
    for (i, x) in [-1.1, 1.1, 3.3].into_iter().enumerate() {
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(textured.clone()),
            StylizedInstanceData {
                profile_index: i as u32,
                ..StylizedInstanceData::default()
            },
            Transform::from_xyz(x, 0.0, 0.0),
        ));
    }
    CameraRecipe::new(Vec3::new(0.0, 0.5, 8.0), Vec3::ZERO)
}

// ---------------------------------------------------------------------------
// 02 lighting：key light 方向 + palette + day/night（同帧三段经实例排布；
// view 层昼间设定——昼夜极端见 view 参数测试与 golden 的 day/night 覆盖）
// ---------------------------------------------------------------------------
pub fn lighting(world: &mut World) -> CameraRecipe {
    let mesh = sphere(world, 0.9);
    let material = cel_material(world);
    // 三个球分别：默认昼 / 夜（低 ambient 由 view 驱动不适用 per-instance，
    // 此处展示 key light 方向响应梯度）——横向排布展示 NdotL 变化。
    for x in [-2.4, 0.0, 2.4] {
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(x, 0.0, 0.0),
        ));
    }
    // palette[0] 暖色环境 + key light 侧向 → 明暗梯度清晰。
    let mut settings = world.resource_mut::<StylizedViewSettings>();
    settings.0.key_light_direction = Vec3::new(0.9, 0.2, 0.2).normalize();
    settings.0.palette = vec![LinearRgba::new(0.9, 0.75, 0.6, 1.0)];
    CameraRecipe::new(Vec3::new(0.0, 0.5, 6.5), Vec3::ZERO)
}

// ---------------------------------------------------------------------------
// 03 face：face SDF 逐实例阈值扫描
// ---------------------------------------------------------------------------
pub fn face(world: &mut World) -> CameraRecipe {
    let mesh = sphere(world, 0.9);
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
    let sdf = world.resource_mut::<Assets<Image>>().add(sdf);

    let material = world
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
        let target = match i {
            0 => "instances.0",
            1 => "instances.1",
            _ => "instances.2",
        };
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            StylizedInstanceData {
                face_threshold: threshold,
                ..StylizedInstanceData::default()
            },
            SceneTarget(target),
            Transform::from_xyz(x, 0.0, 0.0),
        ));
    }
    CameraRecipe::new(Vec3::new(0.0, 0.3, 6.5), Vec3::ZERO)
}

// ---------------------------------------------------------------------------
// 04 hair：hair 高光 on/off（ILM.G mask）
// ---------------------------------------------------------------------------
pub fn hair(world: &mut World) -> CameraRecipe {
    let mesh = sphere(world, 0.9);
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
    let ilm = world.resource_mut::<Assets<Image>>().add(ilm);

    let off = world
        .resource_mut::<Assets<StylizedCharacterMaterial>>()
        .add(StylizedCharacterMaterial {
            params: StylizedCharacterParams::default(),
            ilm_texture: Some(ilm.clone()),
            ..StylizedCharacterMaterial::default()
        });
    let on = world
        .resource_mut::<Assets<StylizedCharacterMaterial>>()
        .add(StylizedCharacterMaterial {
            params: StylizedCharacterParams::default().with_hair_specular(32.0, 0.9),
            ilm_texture: Some(ilm),
            ..StylizedCharacterMaterial::default()
        });
    world.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(off),
        Transform::from_xyz(-1.4, 0.0, 0.0),
    ));
    world.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(on),
        Transform::from_xyz(1.4, 0.0, 0.0),
    ));
    CameraRecipe::new(Vec3::new(0.0, 0.3, 5.5), Vec3::ZERO)
}

// ---------------------------------------------------------------------------
// 05 metal：matcap on/off（hero）
// ---------------------------------------------------------------------------
pub fn metal(world: &mut World) -> CameraRecipe {
    let mesh = sphere(world, 0.9);
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
    let matcap = world.resource_mut::<Assets<Image>>().add(matcap);
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
    let ilm = world.resource_mut::<Assets<Image>>().add(ilm);

    let off = world
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(StylizedHeroMaterial {
            ilm_texture: Some(ilm.clone()),
            ..StylizedHeroMaterial::default()
        });
    let on = world
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(StylizedHeroMaterial {
            params: StylizedHeroParams::default()
                .with_matcap(0.9, LinearRgba::new(1.0, 0.9, 0.8, 1.0)),
            ilm_texture: Some(ilm),
            matcap_texture: Some(matcap),
            ..StylizedHeroMaterial::default()
        });
    world.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(off),
        Transform::from_xyz(-1.4, 0.0, 0.0),
    ));
    world.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(on),
        Transform::from_xyz(1.4, 0.0, 0.0),
    ));
    CameraRecipe::new(Vec3::new(0.0, 0.3, 5.5), Vec3::ZERO)
}

// ---------------------------------------------------------------------------
// 06 outline：宽度 0/2/8 + 逐实例缩放（缩放开轮廓材质宽度仍全局；
// 逐实例缩放为 ADR-0004 预留位，当前展示宽度梯度）
// ---------------------------------------------------------------------------
pub fn outline(world: &mut World) -> CameraRecipe {
    let mesh = sphere(world, 0.8);
    let material = cel_material(world);
    for (i, width) in [0.0, 2.0, 8.0].into_iter().enumerate() {
        let outline =
            world
                .resource_mut::<Assets<StylizedOutlineMaterial>>()
                .add(StylizedOutlineMaterial {
                    params: StylizedOutlineParams::default()
                        .with_color(LinearRgba::new(0.02, 0.02, 0.04, 1.0))
                        .with_width(width),
                    class_texture: None,
                });
        let mut entity = world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz((i as f32 - 1.0) * 2.2, 0.0, 0.0),
        ));
        if width > 0.0 {
            entity.insert(StylizedOutline::new(outline));
        }
    }
    CameraRecipe::new(Vec3::new(0.0, 0.4, 6.0), Vec3::ZERO)
}

// ---------------------------------------------------------------------------
// 07 animation：程序化双骨骼蒙皮条带摆动
// ---------------------------------------------------------------------------
pub fn animation(world: &mut World) -> CameraRecipe {
    let root = build_strip_world(world);
    // 摆一下再捕获：旋转 root 让条带弯曲。
    world
        .entity_mut(root)
        .get_mut::<Transform>()
        .expect("transform")
        .rotation = Quat::from_rotation_z(0.5);
    CameraRecipe::new(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO)
}

// ---------------------------------------------------------------------------
// 08 post：vignette + chromatic + display transform hook（单 pass）
// ---------------------------------------------------------------------------
pub fn post(world: &mut World) -> CameraRecipe {
    let mesh = sphere(world, 0.9);
    let material = cel_material(world);
    for x in [-1.3, 1.3] {
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(x, 0.0, 0.0),
        ));
    }
    // 相机组件挂后处理参数（View 层开关默认开）；附件由宿主插入相机。
    CameraRecipe::with_attachment(
        Vec3::new(0.0, 0.3, 5.5),
        Vec3::ZERO,
        CameraAttachment::PostProcess(StylizedPostProcess {
            vignette_strength: 0.5,
            chromatic_aberration: 0.004,
            display_transform: StylizedDisplayTransform::SimpleCompress,
            ..StylizedPostProcess::default()
        }),
    )
}

// ---------------------------------------------------------------------------
// 09 stress：1000 实例
// ---------------------------------------------------------------------------
pub fn stress(world: &mut World) -> CameraRecipe {
    build_portable_instances_world(world, 1000);
    CameraRecipe::new(Vec3::new(0.0, 6.0, 34.0), Vec3::ZERO)
}

// ---------------------------------------------------------------------------
// 10 reference：fixture reference_scene.glb（production 路径）
// ---------------------------------------------------------------------------
pub fn reference(world: &mut World) -> CameraRecipe {
    // 与三层对比同场景同机位（常数见 labs/reference）。
    let scene = world
        .resource::<bevy::asset::AssetServer>()
        .load("reference_scene.glb#Scene0");
    world.spawn(bevy::world_serialization::WorldAssetRoot(scene));
    CameraRecipe::new(Vec3::new(0.0, 1.1, 4.2), Vec3::new(0.0, 0.9, 0.0))
}

// ---------------------------------------------------------------------------
// 11 performance：benchmark 场景捕获 + 运行 JSON baseline 测量
// ---------------------------------------------------------------------------
pub fn performance(world: &mut World) -> CameraRecipe {
    // 捕获面：hero_1 + portable_100 同帧。
    build_hero_1_world(world);
    build_portable_instances_world(world, 100);
    CameraRecipe::new(Vec3::new(0.0, 4.0, 20.0), Vec3::ZERO)
}

// ---------------------------------------------------------------------------
// outline-advanced（V2-a）：hybrid = hull 剪影 + screen-space 内部边缘
// ---------------------------------------------------------------------------
pub fn outline_advanced(world: &mut World) -> CameraRecipe {
    let material = cel_material(world);
    let outline =
        world
            .resource_mut::<Assets<StylizedOutlineMaterial>>()
            .add(StylizedOutlineMaterial {
                params: StylizedOutlineParams::default()
                    .with_color(LinearRgba::new(0.02, 0.02, 0.04, 1.0))
                    .with_width(2.5),
                ..Default::default()
            });
    let big_box =
        world
            .resource_mut::<Assets<Mesh>>()
            .add(bevy::math::primitives::Cuboid::from_size(Vec3::new(
                2.0, 0.8, 1.6,
            )));
    let small_box =
        world
            .resource_mut::<Assets<Mesh>>()
            .add(bevy::math::primitives::Cuboid::from_size(Vec3::new(
                0.9, 0.9, 0.9,
            )));
    let ball = world.resource_mut::<Assets<Mesh>>().add(Sphere::new(0.55));
    for (mesh, translation) in [
        (big_box, Vec3::new(0.0, -0.4, 0.0)),
        (small_box.clone(), Vec3::new(0.0, 0.45, 0.0)),
        (small_box, Vec3::new(0.35, 0.45, 0.2)),
        (ball, Vec3::new(0.7, 0.55, 0.1)),
    ] {
        world.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            StylizedOutline::new(outline.clone()),
            Transform::from_translation(translation),
        ));
    }
    // per-camera opt-in：screen-space 内部边缘（Standard+）；附件由宿主插入相机。
    CameraRecipe::with_attachment(
        Vec3::new(2.2, 1.6, 3.4),
        Vec3::new(0.0, 0.3, 0.0),
        CameraAttachment::ScreenOutline,
    )
}

// ---------------------------------------------------------------------------
// lod（V2-b）：同一 hero 角色随距离梯度降级（near→very far；永不降 class）
// ---------------------------------------------------------------------------
pub fn lod(world: &mut World) -> CameraRecipe {
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
    let matcap = world.resource_mut::<Assets<Image>>().add(matcap);
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
    let ilm = world.resource_mut::<Assets<Image>>().add(ilm);

    // 共享 hero 材质：matcap + hair 高光全开（near 全特性；远处经
    // per-instance flags 削减，材质资产不变）。
    let mut hero_params =
        StylizedHeroParams::default().with_matcap(0.9, LinearRgba::new(1.0, 0.9, 0.8, 1.0));
    hero_params.character = hero_params.character.with_hair_specular(28.0, 0.8);
    let material = world
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
        let mesh = world
            .resource_mut::<Assets<Mesh>>()
            .add(Sphere::new(radius));
        // 几何 LOD：far 起换低模（band 2/3 声明了映射才换 Mesh3d）。
        let low = world
            .resource_mut::<Assets<Mesh>>()
            .add(Sphere::new(radius).mesh().uv(8, 6));
        let mut lod_meshes = StylizedLodMeshes::new(mesh.clone());
        lod_meshes.band_meshes[2] = Some(low.clone());
        lod_meshes.band_meshes[3] = Some(low);
        world.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            StylizedInstanceData::default(),
            lod_meshes,
            Transform::from_translation(position),
        ));
    }
    CameraRecipe::new(crate::metadata::LOD_CAMERA_EYE, Vec3::new(0.0, 0.0, -10.0))
}

// ---------------------------------------------------------------------------
// environment（V2-d）：同一场景 × 三套环境调色板（昼雾 / 黄昏 / 夜，
// 视图 preset 见 crate::views）
// ---------------------------------------------------------------------------
/// 共用场景：双列球体向远处递退 + 地面，雾的距离梯度可见。
pub fn environment(world: &mut World) -> CameraRecipe {
    // 着色基色（雾/阴影染色在有色表面上才可读）。
    let sphere_params = StylizedCelParams {
        base_color_factor: bevy::math::Vec4::new(0.85, 0.62, 0.55, 1.0),
        ..StylizedCelParams::default()
    };
    let material = world
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial {
            params: sphere_params,
            ..StylizedCelMaterial::default()
        });
    let ground_params = StylizedCelParams {
        base_color_factor: bevy::math::Vec4::new(0.55, 0.6, 0.5, 1.0),
        ..StylizedCelParams::default()
    };
    let ground_material =
        world
            .resource_mut::<Assets<StylizedCelMaterial>>()
            .add(StylizedCelMaterial {
                params: ground_params,
                ..StylizedCelMaterial::default()
            });
    let mesh = sphere(world, 0.9);
    for (i, z) in [-2.0, -6.0, -12.0, -20.0, -30.0, -45.0]
        .into_iter()
        .enumerate()
    {
        let x = if i % 2 == 0 { -1.5 } else { 1.5 };
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(x, 0.9, z),
        ));
    }
    // 地面（大 cuboid）：雾在平面上的横向梯度。
    let ground =
        world
            .resource_mut::<Assets<Mesh>>()
            .add(bevy::math::primitives::Cuboid::from_size(Vec3::new(
                60.0, 0.1, 120.0,
            )));
    world.spawn((
        Mesh3d(ground),
        MeshMaterial3d(ground_material),
        Transform::from_xyz(0.0, -0.05, -40.0),
    ));
    CameraRecipe::new(Vec3::new(0.0, 2.5, 4.0), Vec3::new(0.0, 0.5, -15.0))
}

// ---------------------------------------------------------------------------
// effects：效果网格光照基——per-material key light 覆盖（docs/reference/03
// 效果网格光照基 decode 节）。同一 view 光下三个球体：左 = 无覆盖（随 view
// key light）；中 = 锁定的上游解码方向（+Z，正对默认机位）；右 = 方位角
// 翻转（-Z，正面落影）。覆盖的存在与作用域直接可读。
// ---------------------------------------------------------------------------
pub fn effects(world: &mut World) -> CameraRecipe {
    let mesh = sphere(world, 0.85);
    let base = StylizedHeroParams::default().with_ramp_rows(1.0);
    let mut locked = base;
    locked.character = locked.character.with_effect_light_dir(Vec3::Z);
    let mut flipped = base;
    flipped.character = flipped.character.with_effect_light_dir(Vec3::NEG_Z);
    let mut materials = Vec::new();
    for params in [base, locked, flipped] {
        materials.push(world.resource_mut::<Assets<StylizedHeroMaterial>>().add(
            StylizedHeroMaterial {
                params,
                ..StylizedHeroMaterial::default()
            },
        ));
    }
    for (material, x) in materials.into_iter().zip([-1.9, 0.0, 1.9]) {
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz(x, 1.0, 0.0),
        ));
    }
    CameraRecipe::new(Vec3::new(0.0, 1.0, 6.2), Vec3::new(0.0, 1.0, 0.0))
}
