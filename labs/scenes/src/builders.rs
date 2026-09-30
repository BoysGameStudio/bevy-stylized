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
    ecs::reflect::ReflectResource,
    image::Image,
    math::{Mat4, Quat, Vec3, primitives::Cylinder, primitives::Sphere},
    mesh::{Mesh, Meshable, skinning::SkinnedMesh},
    pbr::MeshMaterial3d,
    prelude::{App, Children, Entity, Mesh3d, Transform, World},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_stylized::prelude::*;
use bevy_stylized::stylized_render::post::{StylizedDisplayTransform, StylizedPostProcess};

use crate::metadata::{COURTYARD_LOD_BALLS, COURTYARD_TRIO_BALLS};
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

/// 程序化 matcap 纹理（径向金属渐变；metal/lod/courtyard 共用）。
pub fn matcap_image(size: usize) -> Image {
    let mut data = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 / size as f32 - 0.5;
            let dy = y as f32 / size as f32 - 0.5;
            let r = (dx * dx + dy * dy).sqrt();
            let v = ((1.0 - (r * 2.0).clamp(0.0, 1.0)) * 255.0) as u8;
            data.extend_from_slice(&[v, v / 2, 255 - v, 255]);
        }
    }
    let mut image = Image::new_fill(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    image.data = Some(data);
    image
}

/// courtyard 专用银灰 matcap（金属银灰：中心亮银 (0.90,0.92,0.95) →
/// 边缘深灰蓝 (0.25,0.28,0.34)）。默认 [`matcap_image`] 的紫蓝配色服务
/// metal/lod 旧场景的 golden 基线，不动；courtyard 的 Hero+matcap 球
/// （LOD 轴、metal 排）统一用本变体降噪。
pub fn matcap_image_silver(size: usize) -> Image {
    let center = [0.90f32, 0.92, 0.95];
    let edge = [0.25f32, 0.28, 0.34];
    let mut data = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 / size as f32 - 0.5;
            let dy = y as f32 / size as f32 - 0.5;
            let r = (dx * dx + dy * dy).sqrt();
            let t = 1.0 - (r * 2.0).clamp(0.0, 1.0);
            let px = [0, 1, 2].map(|c| {
                ((edge[c] + (center[c] - edge[c]) * t).clamp(0.0, 1.0) * 255.0).round() as u8
            });
            data.extend_from_slice(&[px[0], px[1], px[2], 255]);
        }
    }
    let mut image = Image::new_fill(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    image.data = Some(data);
    image
}

/// 程序化 ILM 纹理（上游布局：R=metal mask，G=AO 全 1；16×16 纯色）。
/// metal/lod/courtyard metal 排用 `metal_mask = 255`（全金属）；
/// courtyard hair 排用 0（对应 hair 场景的 ILM）。
pub fn ilm_image(metal_mask: u8) -> Image {
    let mut data = Vec::with_capacity(16 * 16 * 4);
    for _ in 0..16 * 16 {
        data.extend_from_slice(&[metal_mask, 255, 0, 0]);
    }
    let mut image = Image::new_fill(
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
    let material = cel_material(world);
    build_strip_world_with_material(world, material)
}

/// [`build_strip_world`] 的材质可指定变体（courtyard 旗帜配色用；
/// 几何/蒙皮同一份实现）。
pub fn build_strip_world_with_material(
    world: &mut World,
    material: bevy::prelude::Handle<StylizedCelMaterial>,
) -> Entity {
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
// courtyard（docs/showcase/05）：庭院展台合成场景——渲染特性按分区组织，
// v2 分区激活（聚焦分区展品生成，画框常驻；overview = 同屏全显浏览模式）；
// 参数面（§8）经 CourtyardSettings 资源进入构建（缺席 = §0 默认状态
// trio/day/hull/100/false/rest/None）。
// ---------------------------------------------------------------------------

/// courtyard 时刻参数（§8：view preset 选择）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, bevy::prelude::Reflect)]
pub enum CourtyardTimeOfDay {
    /// 昼（§0 默认）。
    #[default]
    Day,
    /// 黄昏。
    Dusk,
    /// 夜。
    Night,
}

/// courtyard 描边模式（§8：P2/P3 内容 + 相机附件；hybrid 需 Standard）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, bevy::prelude::Reflect)]
pub enum CourtyardOutlineMode {
    /// 无描边。
    Off,
    /// hull 描边（§0 默认）。
    #[default]
    Hull,
    /// hull + screen-space 内部边缘。
    Hybrid,
}

/// courtyard 旗帜姿态（§8：posed = root 旋转 0.5 rad 的确定性定姿态）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, bevy::prelude::Reflect)]
pub enum CourtyardFlagsPose {
    /// 静止（§0 默认）。
    #[default]
    Rest,
    /// 定姿态（root 旋转 0.5 rad）。
    Posed,
}

/// courtyard 展台内容（v3 展亭模式，2026-09-30 owner 决策）：每个档位在
/// 中央平台生成对应展品；画框（场地壳/平台）常驻。v2 的 `Overview`
/// 同屏浏览模式随 v3 退役（同位必穿模；浏览全部内容归导览模式 G）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, bevy::prelude::Reflect)]
pub enum CourtyardZone {
    /// 中央三对照组（§0 默认内容）。
    #[default]
    Trio,
    /// P1 ramp。
    P1,
    /// P2 描边 hull。
    P2,
    /// P3 描边 hybrid。
    P3,
    /// P4 光照对照。
    P4,
    /// P5 旗帜。
    P5,
    /// LOD 轴。
    Lod,
    /// 角色台（GLB，资产门控）。
    Stage,
    /// 实例灯阵。
    Lamps,
    /// rim 边缘光对照（V4）。
    Rim,
    /// 阴影形状对照（V4）。
    Shadow,
    /// Character 四色 palette 对照（V4）。
    Palette,
    /// Hero 金属三色 + spec ramp（V4）。
    Metal,
    /// alpha 镂空卡片（V4）。
    Cutout,
}

impl CourtyardZone {
    /// zone 参数值（§8；catalog 枚举选项与捕获规格钉值共用）。
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            CourtyardZone::Trio => "trio",
            CourtyardZone::P1 => "p1",
            CourtyardZone::P2 => "p2",
            CourtyardZone::P3 => "p3",
            CourtyardZone::P4 => "p4",
            CourtyardZone::P5 => "p5",
            CourtyardZone::Lod => "lod",
            CourtyardZone::Stage => "stage",
            CourtyardZone::Lamps => "lamps",
            CourtyardZone::Rim => "rim",
            CourtyardZone::Shadow => "shadow",
            CourtyardZone::Palette => "palette",
            CourtyardZone::Metal => "metal",
            CourtyardZone::Cutout => "cutout",
        }
    }

    /// 从参数值解析（捕获规格/控制器入口；catalog 已校验取值）。
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "trio" => CourtyardZone::Trio,
            "p1" => CourtyardZone::P1,
            "p2" => CourtyardZone::P2,
            "p3" => CourtyardZone::P3,
            "p4" => CourtyardZone::P4,
            "p5" => CourtyardZone::P5,
            "lod" => CourtyardZone::Lod,
            "stage" => CourtyardZone::Stage,
            "lamps" => CourtyardZone::Lamps,
            "rim" => CourtyardZone::Rim,
            "shadow" => CourtyardZone::Shadow,
            "palette" => CourtyardZone::Palette,
            "metal" => CourtyardZone::Metal,
            "cutout" => CourtyardZone::Cutout,
            _ => return None,
        })
    }
}

/// courtyard 角色台展品（v2 增补，2026-09-29 owner 决策）：`Fixture` =
/// 合成 reference-scene GLB（golden 判定/证据测试内容，可提交基线）；
/// `Odette` = 真 Genshin 角色（compiled static-v3，经 `odette://` 资产源 +
/// `.npr.json` sidecar adoption；第三方美术只进交互展示，不进 golden——
/// ADR-0017 来源政策，golden-local 已退役）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, bevy::prelude::Reflect)]
pub enum CourtyardStageCharacter {
    /// 合成 fixture（默认；全部捕获规格/证据测试使用）。
    #[default]
    Fixture,
    /// 真 Genshin 角色（genshin-odette compiled static-v3；交互展示档）。
    Odette,
}

impl CourtyardStageCharacter {
    /// stage_character 参数值（catalog 枚举选项共用）。
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            CourtyardStageCharacter::Fixture => "fixture",
            CourtyardStageCharacter::Odette => "odette",
        }
    }

    /// 从参数值解析（控制器入口；catalog 已校验取值）。
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "fixture" => CourtyardStageCharacter::Fixture,
            "odette" => CourtyardStageCharacter::Odette,
            _ => return None,
        })
    }
}

/// courtyard 场景参数（§8 参数面的构建期通道；§0 默认状态 = `Default`：
/// zone=trio / day / hull / crowd=100 / post off / flags rest /
/// face_threshold None / stage_character=fixture）。
///
/// builder 经 `world.get_resource::<CourtyardSettings>()` 读取，缺席用默认。
#[derive(bevy::prelude::Resource, bevy::prelude::Reflect, Clone, Copy, Debug, PartialEq)]
#[reflect(Resource)]
pub struct CourtyardSettings {
    /// 分区激活（v2：聚焦分区展品生成 + 机位联动；overview = 全展品）。
    pub zone: CourtyardZone,
    /// 时刻（day/dusk/night）。
    pub time_of_day: CourtyardTimeOfDay,
    /// 描边模式（off/hull/hybrid）。
    pub outline_mode: CourtyardOutlineMode,
    /// 灯阵密度（0/100/1000）。
    pub crowd: u32,
    /// 后处理开关。
    pub post: bool,
    /// 旗帜姿态（rest/posed）。
    pub flags: CourtyardFlagsPose,
    /// 中央前排 face 组 SDF 阈值覆盖（None = 三阈值梯度 0.2/默认/0.8）。
    pub face_threshold: Option<f32>,
    /// 角色台展品（fixture/odette；默认 fixture——golden 与证据测试内容）。
    pub stage_character: CourtyardStageCharacter,
}

impl Default for CourtyardSettings {
    fn default() -> Self {
        Self {
            zone: CourtyardZone::Trio,
            time_of_day: CourtyardTimeOfDay::Day,
            outline_mode: CourtyardOutlineMode::Hull,
            crowd: 100,
            post: false,
            flags: CourtyardFlagsPose::Rest,
            face_threshold: None,
            stage_character: CourtyardStageCharacter::Fixture,
        }
    }
}

/// 庭院展台：分区合成（§3 布局；§4 分区详设；v2 分区激活——只有聚焦
/// 分区的展品生成，场地壳/展台画框常驻；overview = 全展品同屏）。
pub fn courtyard(world: &mut World) -> CameraRecipe {
    let settings = world
        .get_resource::<CourtyardSettings>()
        .copied()
        .unwrap_or_default();
    // 实体桶登记簿（W3 动态装卸）：每代重建——画框桶随场地壳常驻，
    // 展品桶随内容档位生成。
    world.insert_resource(CourtyardBuckets::default());
    // time_of_day → 庭院专用视图 preset（雾范围按庭院景深重调，见
    // views::courtyard_*；builder 在 driver 应用 binding.view 之后运行，
    // 写 StylizedViewSettings 即生效，与 builders::lighting 同一通道）。
    {
        let mut view = world.resource_mut::<StylizedViewSettings>();
        match settings.time_of_day {
            CourtyardTimeOfDay::Day => crate::views::courtyard_day(&mut view.0),
            CourtyardTimeOfDay::Dusk => crate::views::courtyard_dusk(&mut view.0),
            CourtyardTimeOfDay::Night => crate::views::courtyard_night(&mut view.0),
        }
    }
    // 画框桶（W3：跨代常驻，任何参数切换不装卸）。
    build_bucket(world, "frame", courtyard_shell);
    for zone in EXHIBIT_ZONES {
        if settings.zone == zone {
            build_courtyard_zone(world, zone, &settings);
        }
    }
    // CameraRecipe 只有单附件槽（多附件替换 API 是 §11.1 的后续阶段）；
    // §9 捕获矩阵中 hybrid 与 post 不组合，hybrid 优先。
    let attachment = courtyard_attachment(&settings);
    // 机位随内容档位（§7 一一对应）。
    let preset_id = crate::metadata::courtyard_zone_camera_preset(settings.zone.id())
        .expect("zone camera preset resolves");
    let preset = crate::metadata::COURTYARD_CAMERA_PRESETS
        .iter()
        .find(|preset| preset.id == preset_id)
        .expect("zone camera preset in table");
    CameraRecipe::with_attachment(preset.eye, preset.look, attachment)
}

/// courtyard 展品分区（装卸循环序；overview 不是展品桶——它激活全部）。
const EXHIBIT_ZONES: [CourtyardZone; 14] = [
    CourtyardZone::Trio,
    CourtyardZone::P1,
    CourtyardZone::P2,
    CourtyardZone::P3,
    CourtyardZone::P4,
    CourtyardZone::P5,
    CourtyardZone::Lod,
    CourtyardZone::Stage,
    CourtyardZone::Lamps,
    CourtyardZone::Rim,
    CourtyardZone::Shadow,
    CourtyardZone::Palette,
    CourtyardZone::Metal,
    CourtyardZone::Cutout,
];

/// 构建一个分区展品桶（构建与增量更新共用，单一源）。
fn build_courtyard_zone(world: &mut World, zone: CourtyardZone, settings: &CourtyardSettings) {
    build_bucket(world, zone.id(), |world| match zone {
        CourtyardZone::Trio => courtyard_trio(world, settings.face_threshold),
        CourtyardZone::P1 => courtyard_p1_ramp(world),
        CourtyardZone::P2 => courtyard_p2_outline(world),
        CourtyardZone::P3 => courtyard_p3_hybrid(world),
        CourtyardZone::P4 => courtyard_p4_effects(world),
        CourtyardZone::P5 => courtyard_p5_flags(world, settings.flags),
        CourtyardZone::Lod => courtyard_lod_axis(world),
        CourtyardZone::Stage => courtyard_reference_stage(world, settings.stage_character),
        CourtyardZone::Lamps => courtyard_lamps(world, settings.crowd),
        CourtyardZone::Rim => courtyard_rim(world),
        CourtyardZone::Shadow => courtyard_shadow(world),
        CourtyardZone::Palette => courtyard_palette(world),
        CourtyardZone::Metal => courtyard_metal(world),
        CourtyardZone::Cutout => courtyard_cutout(world),
    });
}

/// courtyard 增量更新的结果（W3）。
pub struct CourtyardUpdate {
    /// 新相机配方（机位联动 §7 + 附件 W1，与全量构建同一来源）。
    pub recipe: CameraRecipe,
    /// 待退役实体（旧分区/重装桶；despawn 时序归调用方——先装后卸，
    /// 补间落定后才退役旧区，见 05 §12 W1–W3 时序规则）。
    pub retire: Vec<bevy::prelude::Entity>,
}

/// courtyard 增量更新（W3：分区展品动态装卸）。按 prev → next 的 diff
/// 装载/退役展品桶；画框与观测相机常驻不动。只读写世界内容，
/// 代次/账本/相机应用归 driver（`CaseDriver::note_content_update`）。
///
/// 参数敏感性（W0 矩阵）：zone = 卸旧区装新区；stage_character = 重装
/// stage 桶（在场时）；crowd = 重装 lamps 桶（在场时）；face_threshold/
/// flags/post/outline_mode/time_of_day 不进本函数（live 路径，05 §11.2）。
pub fn courtyard_update(
    world: &mut World,
    prev: CourtyardSettings,
    next: CourtyardSettings,
) -> CourtyardUpdate {
    let mut retire = Vec::new();
    for zone in EXHIBIT_ZONES {
        let id = zone.id();
        let want = next.zone == zone;
        // 内容参数重装触发（展品在场才有意义；不在场 = 设置已记录，
        // 下次激活时按新值生成）。
        let reload = match zone {
            CourtyardZone::Stage => prev.stage_character != next.stage_character,
            CourtyardZone::Lamps => prev.crowd != next.crowd,
            _ => false,
        };
        let present = world
            .get_resource::<CourtyardBuckets>()
            .is_some_and(|buckets| buckets.exhibits.contains_key(id));
        if present && (!want || reload) {
            if let Some(mut buckets) = world.get_resource_mut::<CourtyardBuckets>() {
                retire.extend(buckets.unload(id));
            }
        }
        if want && (!present || reload) {
            build_courtyard_zone(world, zone, &next);
        }
    }
    // 相机配方与全量构建同一来源（机位联动 + 附件选择）。
    let preset_id = crate::metadata::courtyard_zone_camera_preset(next.zone.id())
        .expect("zone camera preset resolves");
    let preset = crate::metadata::COURTYARD_CAMERA_PRESETS
        .iter()
        .find(|preset| preset.id == preset_id)
        .expect("zone camera preset in table");
    CourtyardUpdate {
        recipe: CameraRecipe::with_attachment(preset.eye, preset.look, courtyard_attachment(&next)),
        retire,
    }
}

/// courtyard 相机附件选择（§11.1：附件由参数驱动、与机位正交；hybrid 与
/// post 不组合，hybrid 优先）。builder 与控制器 live 附件切换（05 §11.2
/// post/outline_mode 行）共用单一源。
#[must_use]
pub fn courtyard_attachment(settings: &CourtyardSettings) -> CameraAttachment {
    match (settings.outline_mode, settings.post) {
        (CourtyardOutlineMode::Hybrid, _) => CameraAttachment::ScreenOutline,
        (_, true) => CameraAttachment::PostProcess(StylizedPostProcess {
            vignette_strength: 0.5,
            chromatic_aberration: 0.004,
            display_transform: StylizedDisplayTransform::SimpleCompress,
            ..StylizedPostProcess::default()
        }),
        (_, false) => CameraAttachment::None,
    }
}

/// courtyard 实体桶（W3 分区动态装卸的登记簿）：画框桶跨代常驻，展品桶
/// 随分区装卸。builder 构建（[`courtyard`]）与增量更新（[`courtyard_update`]）
/// 共用；只登记根实体（子实体随递归 despawn 退役，与账本同一口径）。
#[derive(bevy::prelude::Resource, Clone, Debug, Default, PartialEq)]
pub struct CourtyardBuckets {
    /// 画框（地面/铺装/墙/平台）——任何参数切换都不动（v3：展台环退役）。
    pub frame: Vec<bevy::prelude::Entity>,
    /// 分区展品（key = 分区 id；只有激活分区的桶在场）。
    pub exhibits: std::collections::HashMap<&'static str, Vec<bevy::prelude::Entity>>,
}

impl CourtyardBuckets {
    /// 登记实体到桶（`"frame"` = 画框，其余 = 分区 id）。
    pub fn register(&mut self, bucket: &'static str, entity: bevy::prelude::Entity) {
        if bucket == "frame" {
            self.frame.push(entity);
        } else {
            self.exhibits.entry(bucket).or_default().push(entity);
        }
    }

    /// 卸下一个分区桶（从登记簿移除并返回实体；despawn 时序归调用方——
    /// 05 §12 W1–W3 时序规则：先装后卸，补间落定后才退役旧区）。
    pub fn unload(&mut self, zone: &str) -> Vec<bevy::prelude::Entity> {
        self.exhibits.remove(zone).unwrap_or_default()
    }
}

/// 在桶内构建（W3：分区动态装卸登记——差值登记法：构建前后实体快照差 =
/// 本桶内容，登记与生成代码零耦合、无漏登风险）。子实体一并入桶，退役
/// 时按根过滤（[`retire_entities`]，与 SceneLedger::retire 同一口径）。
fn build_bucket(world: &mut World, bucket: &'static str, build: impl FnOnce(&mut World)) {
    let before: std::collections::HashSet<bevy::prelude::Entity> =
        world.iter_entities().map(|entity| entity.id()).collect();
    build(world);
    let spawned: Vec<bevy::prelude::Entity> = world
        .iter_entities()
        .map(|entity| entity.id())
        .filter(|entity| !before.contains(entity))
        .collect();
    if let Some(mut buckets) = world.get_resource_mut::<CourtyardBuckets>() {
        for entity in spawned {
            buckets.register(bucket, entity);
        }
    }
}

/// 退役一批实体（W3 增量更新：调用方按补间时序执行——先装后卸，补间
/// 落定后才退役旧区；只 despawn 桶内根实体，子实体随递归 despawn
/// 退役——与 SceneLedger::retire 同一口径；已不在场的实体跳过）。
pub fn retire_entities(world: &mut World, entities: &[bevy::prelude::Entity]) {
    let set: std::collections::HashSet<bevy::prelude::Entity> =
        entities.iter().copied().collect();
    for &entity in entities {
        let is_root = world
            .get_entity(entity)
            .ok()
            .and_then(|entity_ref| {
                entity_ref
                    .get::<bevy::prelude::ChildOf>()
                    .map(|parent| parent.parent())
            })
            .is_none_or(|parent| !set.contains(&parent));
        if !is_root {
            continue;
        }
        if let Ok(entity_mut) = world.get_entity_mut(entity) {
            entity_mut.despawn();
        }
    }
}

/// 低饱和 Cel 材质（§2 色彩纪律：固有色低饱和，饱和度只出现在展品上）。
fn courtyard_cel_material(
    world: &mut World,
    base_color: LinearRgba,
) -> bevy::prelude::Handle<StylizedCelMaterial> {
    world
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial {
            params: StylizedCelParams {
                base_color_factor: bevy::math::Vec4::new(
                    base_color.red,
                    base_color.green,
                    base_color.blue,
                    1.0,
                ),
                ..StylizedCelParams::default()
            },
            ..StylizedCelMaterial::default()
        })
}

/// 场地壳（§3）：大地面 / 广场铺装 / 环形矮墙（四段弧墙，南留门）/
/// 中央平台 / 展台环 6 座（同一形制，是画框不是展品）。
///
/// 固有色（美术定案）：深灰绿地面收底、暖白石材铺装、暖灰矮墙、暖白
/// 展台——饱和度只出现在被演示的材质上（§2 色彩纪律）。
fn courtyard_shell(world: &mut World) {
    let ground_material = courtyard_cel_material(world, LinearRgba::new(0.32, 0.34, 0.33, 1.0));
    let paving_material = courtyard_cel_material(world, LinearRgba::new(0.62, 0.60, 0.56, 1.0));
    let wall_material = courtyard_cel_material(world, LinearRgba::new(0.48, 0.46, 0.43, 1.0));
    let stand_material = courtyard_cel_material(world, LinearRgba::new(0.70, 0.68, 0.64, 1.0));

    // 大地面：半径 60 圆盘，深灰。
    let ground = world
        .resource_mut::<Assets<Mesh>>()
        .add(Cylinder::new(60.0, 0.1));
    world.spawn((
        Mesh3d(ground),
        MeshMaterial3d(ground_material),
        Transform::from_xyz(0.0, -0.05, 0.0),
    ));
    // 广场铺装：半径 12 圆盘，抬高 0.02，浅灰（描边可读底）。
    let paving = world
        .resource_mut::<Assets<Mesh>>()
        .add(Cylinder::new(12.0, 0.04));
    world.spawn((
        Mesh3d(paving),
        MeshMaterial3d(paving_material),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    // 环形矮墙 r13（高 1.2、厚 0.4）：四段 80° 弧墙，南门开口 40°
    // （方位角 0 = 南 +Z）；每段 5 根旋转 box 弦近似（全部弦等长，共享
    // 一个 mesh）。
    let (wall_radius, wall_height, wall_thickness) = (13.0f32, 1.2f32, 0.4f32);
    let chord_span = 16.0f32; // 80°/段 ÷ 5 弦
    let chord_length = 2.0 * wall_radius * (chord_span * 0.5).to_radians().sin() * 1.06;
    let chord_mesh =
        world
            .resource_mut::<Assets<Mesh>>()
            .add(bevy::math::primitives::Cuboid::from_size(Vec3::new(
                chord_length,
                wall_height,
                wall_thickness,
            )));
    for segment in 0..4 {
        let start = 20.0 + segment as f32 * 80.0;
        for chord in 0..5 {
            let azimuth = (start + (chord as f32 + 0.5) * chord_span).to_radians();
            world.spawn((
                Mesh3d(chord_mesh.clone()),
                MeshMaterial3d(wall_material.clone()),
                Transform::from_xyz(
                    wall_radius * azimuth.sin(),
                    wall_height * 0.5,
                    wall_radius * azimuth.cos(),
                )
                .with_rotation(Quat::from_rotation_y(azimuth)),
            ));
        }
    }
    // 中央平台：半径 2.2、高 0.3 圆台。v3 展亭模式：唯一舞台（展台环
    // 退役——空展台在单舞台模式里读作误导）。
    let platform = world.resource_mut::<Assets<Mesh>>().add(Cylinder::new(
        crate::metadata::COURTYARD_PLATFORM_RADIUS,
        0.3,
    ));
    world.spawn((
        Mesh3d(platform),
        MeshMaterial3d(stand_material.clone()),
        Transform::from_xyz(0.0, 0.15, 0.0),
    ));
}

/// 中央平台：三对照组 7 球（§4；face 前排 / hair 中排 / metal 后排，
/// 全部面朝南 +Z，坐标与半径来自 metadata::COURTYARD_TRIO_BALLS）。
fn courtyard_trio(world: &mut World, face_threshold: Option<f32>) {
    // 基座：顶面 y=1.0（平台顶 0.3 + 柱高 0.7；球心 y=1.32/1.40 按 §4 锁定）。
    let pedestal_mesh = world
        .resource_mut::<Assets<Mesh>>()
        .add(Cylinder::new(0.12, 0.7));
    let pedestal_material = courtyard_cel_material(world, LinearRgba::new(0.70, 0.68, 0.64, 1.0));
    for (center, _radius) in COURTYARD_TRIO_BALLS {
        world.spawn((
            Mesh3d(pedestal_mesh.clone()),
            MeshMaterial3d(pedestal_material.clone()),
            Transform::from_xyz(center.x, 0.65, center.z),
        ));
        // 三对照组球 = trio 展品本体（06 §5）。
    }

    // face 前排：Character + face SDF（builders::face 的 128×128 线性渐变）。
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
    // 对照组底色（美术定案）：face 肤色底（阈值梯度语义不变）。
    let mut face_params = StylizedCharacterParams::default()
        .with_face_forward(Vec3::Z, 0.05)
        .with_face_threshold_bias(0.5);
    face_params.cel.base_color_factor = bevy::math::Vec4::new(0.85, 0.72, 0.65, 1.0);
    let face_material = world
        .resource_mut::<Assets<StylizedCharacterMaterial>>()
        .add(StylizedCharacterMaterial {
            params: face_params,
            face_sdf_texture: Some(sdf),
            ..StylizedCharacterMaterial::default()
        });
    let face_mesh = sphere(world, 0.32);
    let defaults = [Some(0.2), None, Some(0.8)];
    for (i, (center, _)) in COURTYARD_TRIO_BALLS[..3].iter().enumerate() {
        let target = match i {
            0 => "trio.face.0",
            1 => "trio.face.1",
            _ => "trio.face.2",
        };
        world.spawn((
            Mesh3d(face_mesh.clone()),
            MeshMaterial3d(face_material.clone()),
            StylizedInstanceData {
                face_threshold: face_threshold.or(defaults[i]),
                ..StylizedInstanceData::default()
            },
            SceneTarget(target),
            Transform::from_translation(*center),
        ));
    }

    // hair 中排：Character + ILM（hair 布局 R=metal 0），off/on 对照；
    // 深发色底（美术定案；off/on 语义不变）。
    let hair_ilm = world.resource_mut::<Assets<Image>>().add(ilm_image(0));
    let hair_base = bevy::math::Vec4::new(0.16, 0.14, 0.18, 1.0);
    let mut hair_off_params = StylizedCharacterParams::default();
    hair_off_params.cel.base_color_factor = hair_base;
    let hair_off = world
        .resource_mut::<Assets<StylizedCharacterMaterial>>()
        .add(StylizedCharacterMaterial {
            params: hair_off_params,
            ilm_texture: Some(hair_ilm.clone()),
            ..StylizedCharacterMaterial::default()
        });
    let mut hair_on_params = StylizedCharacterParams::default().with_hair_specular(32.0, 0.9);
    hair_on_params.cel.base_color_factor = hair_base;
    let hair_on = world
        .resource_mut::<Assets<StylizedCharacterMaterial>>()
        .add(StylizedCharacterMaterial {
            params: hair_on_params,
            ilm_texture: Some(hair_ilm),
            ..StylizedCharacterMaterial::default()
        });
    let trio_mesh = sphere(world, 0.40);
    for (material, (center, _)) in [hair_off, hair_on]
        .into_iter()
        .zip(COURTYARD_TRIO_BALLS[3..5].iter())
    {
        world.spawn((
            Mesh3d(trio_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(*center),
        ));
    }

    // metal 后排：Hero + 银灰 matcap（courtyard 变体，降噪）+ 金属 ILM，
    // 金属灰底；off/on 对照语义不变。
    let metal_ilm = world.resource_mut::<Assets<Image>>().add(ilm_image(255));
    let matcap = world
        .resource_mut::<Assets<Image>>()
        .add(matcap_image_silver(128));
    let metal_base = bevy::math::Vec4::new(0.60, 0.62, 0.66, 1.0);
    let mut metal_off_params = StylizedHeroParams::default();
    metal_off_params.character.cel.base_color_factor = metal_base;
    let metal_off =
        world
            .resource_mut::<Assets<StylizedHeroMaterial>>()
            .add(StylizedHeroMaterial {
                params: metal_off_params,
                ilm_texture: Some(metal_ilm.clone()),
                ..StylizedHeroMaterial::default()
            });
    let mut metal_on_params =
        StylizedHeroParams::default().with_matcap(0.9, LinearRgba::new(1.0, 1.0, 1.0, 1.0));
    metal_on_params.character.cel.base_color_factor = metal_base;
    let metal_on = world
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(StylizedHeroMaterial {
            params: metal_on_params,
            ilm_texture: Some(metal_ilm),
            matcap_texture: Some(matcap),
            ..StylizedHeroMaterial::default()
        });
    for (material, (center, _)) in [metal_off, metal_on]
        .into_iter()
        .zip(COURTYARD_TRIO_BALLS[5..7].iter())
    {
        world.spawn((
            Mesh3d(trio_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(*center),
        ));
    }
}

/// 展台球基座（trio 形制：r0.12、高 0.7 圆柱，台面 y0.3 起、顶 y1.0）——
/// v3 平台内容的统一展台语言（p1/p2/p4 横排球共用）。
fn spawn_pedestals(world: &mut World, xs: &[f32]) {
    let mesh = world
        .resource_mut::<Assets<Mesh>>()
        .add(Cylinder::new(0.12, 0.7));
    let material = courtyard_cel_material(world, LinearRgba::new(0.70, 0.68, 0.64, 1.0));
    for &x in xs {
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(x, 0.65, 0.0),
        ));
    }
}

/// P1 · ramp 内容（v3 上平台）：4 小球横排，左 1 解析 ramp，右 3 纹理
/// ramp 行 0/1/2（回收 builders::ramp 的对照语义）。
fn courtyard_p1_ramp(world: &mut World) {
    let mesh = sphere(world, 0.28);
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
    // 横排（沿 X，正对 hero 机位），球心高 = 基座顶 1.0 + r0.28。
    spawn_pedestals(world, &[-1.05, -0.35, 0.35, 1.05]);
    world.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(analytic),
        Transform::from_xyz(-1.05, 1.28, 0.0),
    ));
    for (i, dx) in [-0.35, 0.35, 1.05].into_iter().enumerate() {
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(textured.clone()),
            StylizedInstanceData {
                profile_index: i as u32,
                ..StylizedInstanceData::default()
            },
            Transform::from_xyz(dx, 1.28, 0.0),
        ));
    }
}

/// P2 · 描边 hull 内容（v3 上平台）：3 球横排，hull 宽度 0/2/8
/// （回收 builders::outline 的宽度梯度；横排正对 hero 机位读宽度梯度）。
fn courtyard_p2_outline(world: &mut World) {
    let mesh = sphere(world, 0.35);
    let material = cel_material(world);
    spawn_pedestals(world, &[-0.75, 0.0, 0.75]);
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
            Transform::from_xyz((i as f32 - 1.0) * 0.75, 1.35, 0.0),
        ));
        if width > 0.0 {
            entity.insert(StylizedOutline::new(outline));
        }
    }
}

/// P3 · 描边 hybrid 内容（v3 上平台）：方块堆（2 cuboid + 1 小球），hull
/// 剪影始终在；screen-space 内部边缘经相机附件声明（outline_mode=Hybrid
/// 时由 CameraRecipe 携带；Off/Hull 时 hull-only）。
fn courtyard_p3_hybrid(world: &mut World) {
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
                0.6, 0.25, 0.5,
            )));
    let small_box =
        world
            .resource_mut::<Assets<Mesh>>()
            .add(bevy::math::primitives::Cuboid::from_size(Vec3::new(
                0.3, 0.3, 0.3,
            )));
    let ball = world.resource_mut::<Assets<Mesh>>().add(Sphere::new(0.18));
    // 台面（y0.3）起堆：大盒贴面、小盒与球叠上。
    for (mesh, offset) in [
        (big_box, Vec3::new(0.0, 0.425, 0.0)),
        (small_box, Vec3::new(-0.1, 0.7, 0.05)),
        (ball, Vec3::new(0.15, 0.73, 0.1)),
    ] {
        world.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            StylizedOutline::new(outline.clone()),
            Transform::from_translation(offset),
        ));
    }
}

/// P4 · 光照对照内容（v3 上平台）：3 个 Hero 球横排——无覆盖 / key light
/// 锁 +Z / 锁 −Z（`with_effect_light_dir`，回收 builders::effects 的对照
/// 语义）。
fn courtyard_p4_effects(world: &mut World) {
    let mesh = sphere(world, 0.35);
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
    spawn_pedestals(world, &[-1.0, 0.0, 1.0]);
    for (material, dx) in materials.into_iter().zip([-1.0, 0.0, 1.0]) {
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz(dx, 1.35, 0.0),
        ));
    }
}

/// P5 · 旗帜（v3 上平台）：两根旗杆（高 2.6，立于台面）挂双骨骼蒙皮条带
/// 旗面（build_strip_world）；posed 时 root 旋转 0.5 rad（确定性定姿态，
/// 回收 builders::animation 的姿态语义）。
fn courtyard_p5_flags(world: &mut World, pose: CourtyardFlagsPose) {
    let pole_mesh = world
        .resource_mut::<Assets<Mesh>>()
        .add(Cylinder::new(0.05, 2.6));
    // 美术定案：旗杆深灰、旗面砖红（不再是默认灰白卡片）。
    let pole_material = courtyard_cel_material(world, LinearRgba::new(0.25, 0.25, 0.27, 1.0));
    let flag_material = courtyard_cel_material(world, LinearRgba::new(0.75, 0.22, 0.20, 1.0));
    const FLAG_TARGETS: [(&str, &str); 2] =
        [("flags.0", "flags.0.tip"), ("flags.1", "flags.1.tip")];
    // v3 上平台：杆 x ±0.4、立于台面（y0.3 起，杆顶 2.9）；条带自杆顶垂下。
    for (i, pole_x) in [-0.4f32, 0.4].into_iter().enumerate() {
        world.spawn((
            Mesh3d(pole_mesh.clone()),
            MeshMaterial3d(pole_material.clone()),
            Transform::from_xyz(pole_x, 1.6, 0.0),
        ));
        let root = build_strip_world_with_material(world, flag_material.clone());
        // 条带 bind 姿态占 y∈[-1.2,1.2]、root 关节在带底；root 放到
        // (x, 0.5, 0) 使旗面占 y∈[0.5,2.9]（自杆顶垂下），横向偏移
        // +0.45 避免旗杆穿透旗面。
        let (root_target, tip_target) = FLAG_TARGETS[i];
        let tip = world
            .get::<Children>(root)
            .and_then(|children| children.first().copied())
            .expect("strip has a tip joint");
        {
            let mut entity = world.entity_mut(root);
            let mut transform = entity.get_mut::<Transform>().expect("transform");
            transform.translation = Vec3::new(pole_x + 0.45, 0.5, 0.0);
            if pose == CourtyardFlagsPose::Posed {
                transform.rotation = Quat::from_rotation_z(0.5);
            }
        }
        world.entity_mut(root).insert(SceneTarget(root_target));
        world.entity_mut(tip).insert(SceneTarget(tip_target));
        // 蒙皮条带的剔除 Aabb 按 bind 姿态（原点）计算；关节移到展台后
        // 原点的包围盒会被非原点机位视锥剔除（旗面消失）——按旗面的实际
        // 世界范围（含 posed 摆动极值）手工覆盖，并挂 NoAutoAabb 防止
        // calculate_bounds 在网格资产事件后重算覆盖（bevy_camera
        // visibility 的既有合同）。
        let strip_entity = {
            let mut query = world.query::<(Entity, &SkinnedMesh)>();
            query
                .iter(world)
                .find(|(_, skinned)| skinned.joints.first() == Some(&root))
                .expect("strip mesh entity")
                .0
        };
        world.entity_mut(strip_entity).insert((
            bevy::camera::primitives::Aabb::from_min_max(
                Vec3::new(pole_x - 1.2, 0.2, -0.2),
                Vec3::new(pole_x + 0.95, 3.1, 0.2),
            ),
            bevy::camera::visibility::NoAutoAabb,
        ));
    }
}

/// P6 · LOD 轴（§6）：四球递退，near→very_far 各落一带；共享 hero 材质
/// （银灰 matcap + hair 高光全开；银灰版替代默认紫蓝以降低全景噪音），
/// far 带起经 StylizedLodMeshes.band_meshes[2/3] 换低模（同 builders::lod
/// 的几何 LOD 模式）。
fn courtyard_lod_axis(world: &mut World) {
    let matcap = world
        .resource_mut::<Assets<Image>>()
        .add(matcap_image_silver(64));
    let ilm = world.resource_mut::<Assets<Image>>().add(ilm_image(255));
    let mut hero_params =
        StylizedHeroParams::default().with_matcap(0.9, LinearRgba::new(1.0, 1.0, 1.0, 1.0));
    hero_params.character = hero_params.character.with_hair_specular(28.0, 0.8);
    let material = world
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(StylizedHeroMaterial {
            params: hero_params,
            ilm_texture: Some(ilm),
            matcap_texture: Some(matcap),
            ..StylizedHeroMaterial::default()
        });
    for (center, radius) in COURTYARD_LOD_BALLS {
        let mesh = world
            .resource_mut::<Assets<Mesh>>()
            .add(Sphere::new(radius));
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
            Transform::from_translation(center),
        ));
    }
}

/// 角色台（v3 上平台：台基座立于台面 (0,0.55,0)，角色站基座顶 y0.8）：
/// fixture = 合成 reference-scene GLB（golden/证据内容）；odette = 真
/// Genshin 角色（compiled static-v3，经 `odette://` 资产源 + sidecar 资产
/// adoption，交互展示档）。
/// 资产缺席 = 该 lane BLOCKED，不做空台降级（S4 决策）。
///
/// fixture 角色经 `StylizedMaterialSet` 挂 NPR 材质（Character class，底色取
/// GLB 内联 baseColorFactor）：fixture 的 `_NPR_*` 语义与 §4「角色特性在角色
/// 本体上的真实 NPR 效果展示」由此落实；同时消除 vanilla StandardMaterial
/// 路径（其 PBR 在 Dawn/Metal 下渲染纯黑——host 限定上游差异，见
/// docs/platforms）。odette 档材质集由 `StylizedSidecarSource` 从
/// `odette://character.npr.json` 资产生成（同一 adoption 链路，宿主只须装配
/// `StylizedAssetIntegrationPlugin` 并注册 `odette` 资产源）。
fn courtyard_reference_stage(world: &mut World, character: CourtyardStageCharacter) {
    let stage_material = courtyard_cel_material(world, LinearRgba::new(0.70, 0.68, 0.64, 1.0));
    let stage = world
        .resource_mut::<Assets<Mesh>>()
        .add(Cylinder::new(1.5, 0.5));
    world.spawn((
        Mesh3d(stage),
        MeshMaterial3d(stage_material),
        Transform::from_xyz(0.0, 0.55, 0.0),
    ));
    let server = world.resource::<bevy::asset::AssetServer>().clone();
    match character {
        CourtyardStageCharacter::Odette => {
            let scene = server.load("odette://character.glb#Scene0");
            let sidecar = server.load("odette://character.npr.json");
            // 面部 SDF 光照基订正：static-v3 是静态烘焙导出——Face 网格节点
            // 无旋转（恒等变换，顶点已是最终朝向），sidecar 的 face 基向量
            // 是 Blender 原作者空间（forward (0,−1,0)），直接代入会把脸判定
            // 成背光（黑脸；六视角验收靠 per-case host face_forward 校准补偿，
            // 庭院无该通道）。角色台上面部朝 +Z（南，机位侧），覆盖为
            // 世界朝向 + 正面来光——per-material 覆盖只动该材质，不改全局
            // view（golden 光照口径不变）。材质名是 odette compiled 资产
            // 合同的一部分。
            let face_basis = std::collections::BTreeMap::from([
                ("face_forward_x".to_string(), 0.0),
                ("face_forward_y".to_string(), 0.0),
                ("face_forward_z".to_string(), 1.0),
                ("face_up_x".to_string(), 0.0),
                ("face_up_y".to_string(), 1.0),
                ("face_up_z".to_string(), 0.0),
                ("effect_light_dir_0".to_string(), 0.0),
                ("effect_light_dir_1".to_string(), 0.0),
                ("effect_light_dir_2".to_string(), 1.0),
            ]);
            world.spawn((
                bevy::world_serialization::WorldAssetRoot(scene),
                bevy_stylized::StylizedSidecarSource {
                    sidecar,
                    texture_prefix: "odette://".to_string(),
                    param_overrides: std::collections::BTreeMap::from([(
                        "runtime_HoYoverse___Odette_Face".to_string(),
                        face_basis,
                    )]),
                },
                Transform::from_xyz(0.0, 0.8, 0.0),
            ));
            return;
        }
        CourtyardStageCharacter::Fixture => {}
    }
    let scene = server.load("reference_scene.glb#Scene0");
    let descriptor = |name: &str, base: [f32; 3]| StylizedMaterialDescriptor {
        name: name.to_string(),
        surface_class: StylizedSurfaceClass::Character,
        static_flags: bevy_stylized::stylized_core::StylizedMaterialFlags {
            double_sided: true,
            ..Default::default()
        },
        static_params: [
            ("base_color_r".to_string(), base[0]),
            ("base_color_g".to_string(), base[1]),
            ("base_color_b".to_string(), base[2]),
            ("base_color_a".to_string(), 1.0),
        ]
        .into_iter()
        .collect(),
        ..StylizedMaterialDescriptor::default()
    };
    world.spawn((
        bevy::world_serialization::WorldAssetRoot(scene),
        bevy_stylized::StylizedMaterialSet {
            materials: [
                descriptor("FixtureMatBody", [0.72, 0.42, 0.30]),
                descriptor("FixtureMatHead", [0.86, 0.66, 0.56]),
                descriptor("FixtureMatHair", [0.32, 0.20, 0.11]),
                descriptor("FixtureMatGround", [0.18, 0.20, 0.24]),
            ]
            .into_iter()
            .map(|descriptor| (descriptor.name.clone(), descriptor))
            .collect(),
            texture_prefix: String::new(),
        },
        Transform::from_xyz(0.0, 0.8, 0.0),
    ));
}

/// 实例灯阵（北半环 r8.5–11 四圈网格，纵深 ≤38m）：灯柱 + 发光灯球。
/// 灯球用 Character 材质 + with_emission（同色——emission 是 per-material
/// uniform，Cel 无 emission 字段；§4 已验证约束）；密度 = crowd 参数。
/// 美术定案：灯柱深灰；灯球受光面经 base_color 染暖黄（Character 无
/// ramp 通道——§4 注记的 profile_index 选行只存在于 Cel/Hero，此处以
/// 底色替代），emission 不动。
fn courtyard_lamps(world: &mut World, crowd: u32) {
    if crowd == 0 {
        return;
    }
    let mut lamp_params =
        StylizedCharacterParams::default().with_emission(LinearRgba::new(1.0, 0.7, 0.3, 1.0), 0.8);
    lamp_params.cel.base_color_factor = bevy::math::Vec4::new(0.95, 0.80, 0.50, 1.0);
    let lamp_material = world
        .resource_mut::<Assets<StylizedCharacterMaterial>>()
        .add(StylizedCharacterMaterial {
            params: lamp_params,
            ..StylizedCharacterMaterial::default()
        });
    let ball_mesh = sphere(world, 0.12);
    let pole_mesh = world
        .resource_mut::<Assets<Mesh>>()
        .add(Cylinder::new(0.04, 1.6));
    let pole_material = courtyard_cel_material(world, LinearRgba::new(0.3, 0.3, 0.32, 1.0));
    // 北半环（方位角 90°..270°，0=南 +Z）：四圈 r∈[8.5,11]，灯数按圈周长
    // 比例分配（末圈吸收舍入余量；总量恰为 crowd，确定性排布）。
    let rings = [8.8125f32, 9.4375, 10.0625, 10.6875];
    let weight_sum: f32 = rings.iter().sum();
    let mut remaining = crowd;
    let mut registered = false;
    for (ring, radius) in rings.iter().enumerate() {
        let count = if ring + 1 == rings.len() {
            remaining
        } else {
            let count = (crowd as f32 * radius / weight_sum).round() as u32;
            let count = count.min(remaining);
            remaining -= count;
            count
        };
        for j in 0..count {
            let azimuth = (90.0 + (j as f32 + 0.5) * 180.0 / count as f32).to_radians();
            let x = radius * azimuth.sin();
            let z = radius * azimuth.cos();
            world.spawn((
                Mesh3d(pole_mesh.clone()),
                MeshMaterial3d(pole_material.clone()),
                Transform::from_xyz(x, 0.8, z),
            ));
            let mut ball = world.spawn((
                Mesh3d(ball_mesh.clone()),
                MeshMaterial3d(lamp_material.clone()),
                StylizedInstanceData::default(),
                Transform::from_xyz(x, 1.72, z),
            ));
            if !registered {
                ball.insert(SceneTarget("lamps"));
                registered = true;
            }
        }
    }
}

/// rim · 边缘光对照（V4）：3 球横排——rim off / 白 rim 强 / 暖色 rim 弱。
/// 底色压暗以读边缘光。
fn courtyard_rim(world: &mut World) {
    let mesh = sphere(world, 0.35);
    spawn_pedestals(world, &[-0.75, 0.0, 0.75]);
    let variants = [
        (0.0, LinearRgba::WHITE),                      // off（强度 0 = 关）
        (1.2, LinearRgba::WHITE),                      // 白 rim 强
        (0.6, LinearRgba::new(1.0, 0.75, 0.45, 1.0)),  // 暖色 rim 弱
    ];
    for (i, (intensity, color)) in variants.into_iter().enumerate() {
        let mut params = StylizedCelParams::default().with_rim(2.5, intensity, color);
        params.base_color_factor = bevy::math::Vec4::new(0.35, 0.38, 0.45, 1.0);
        let material = world
            .resource_mut::<Assets<StylizedCelMaterial>>()
            .add(StylizedCelMaterial {
                params,
                ..StylizedCelMaterial::default()
            });
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz((i as f32 - 1.0) * 0.75, 1.35, 0.0),
        ));
    }
}

/// shadow · 阴影形状对照（V4）：3 球横排——默认（0.5/0.1/0.3）/ 硬高阈
/// （0.7/0.02/0.15）/ 软低下限（0.35/0.3/0.45）。
fn courtyard_shadow(world: &mut World) {
    let mesh = sphere(world, 0.35);
    spawn_pedestals(world, &[-0.75, 0.0, 0.75]);
    let variants = [
        (0.5, 0.1, 0.3),
        (0.7, 0.02, 0.15),
        (0.35, 0.3, 0.45),
    ];
    for (i, (threshold, softness, floor)) in variants.into_iter().enumerate() {
        let mut params = StylizedCelParams::default().with_shadow(threshold, softness, floor);
        params.base_color_factor = bevy::math::Vec4::new(0.55, 0.6, 0.68, 1.0);
        let material = world
            .resource_mut::<Assets<StylizedCelMaterial>>()
            .add(StylizedCelMaterial {
                params,
                ..StylizedCelMaterial::default()
            });
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz((i as f32 - 1.0) * 0.75, 1.35, 0.0),
        ));
    }
}

/// palette · Character 四色调色板对照（V4）：3 球横排——默认灰阴影 /
/// 暖阴影（夕照感）/ 冷阴影（蓝紫）。
fn courtyard_palette(world: &mut World) {
    let mesh = sphere(world, 0.35);
    spawn_pedestals(world, &[-0.75, 0.0, 0.75]);
    let white = LinearRgba::WHITE;
    let variants = [
        // 默认（灰阴影）。
        (white, LinearRgba::new(0.55, 0.55, 0.62, 1.0), white, LinearRgba::new(0.7, 0.7, 0.75, 1.0)),
        // 暖阴影。
        (white, LinearRgba::new(0.72, 0.52, 0.42, 1.0), white, LinearRgba::new(0.85, 0.62, 0.5, 1.0)),
        // 冷阴影（蓝紫）。
        (white, LinearRgba::new(0.42, 0.48, 0.72, 1.0), white, LinearRgba::new(0.55, 0.6, 0.85, 1.0)),
    ];
    for (i, (soft_lit, soft_shadow, sharp_lit, sharp_shadow)) in variants.into_iter().enumerate() {
        let mut params = StylizedCharacterParams::default()
            .with_palette(soft_lit, soft_shadow, sharp_lit, sharp_shadow);
        params.cel.base_color_factor = bevy::math::Vec4::new(0.85, 0.72, 0.65, 1.0);
        let material = world
            .resource_mut::<Assets<StylizedCharacterMaterial>>()
            .add(StylizedCharacterMaterial {
                params,
                ..StylizedCharacterMaterial::default()
            });
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz((i as f32 - 1.0) * 0.75, 1.35, 0.0),
        ));
    }
}

/// metal · Hero 金属三色 + spec ramp（V4）：3 球横排——无 matcap 平色 /
/// matcap 银灰 / matcap + 金属三色调 + spec ramp 强（递进可读）。
fn courtyard_metal(world: &mut World) {
    let mesh = sphere(world, 0.35);
    let ilm = world.resource_mut::<Assets<Image>>().add(ilm_image(255));
    let matcap = world
        .resource_mut::<Assets<Image>>()
        .add(matcap_image_silver(128));
    spawn_pedestals(world, &[-0.75, 0.0, 0.75]);
    for (i, (matcap_on, metal_enabled, spec_ramp)) in [
        (false, false, 0.0f32),
        (true, false, 0.0),
        (true, true, 1.2),
    ]
    .into_iter()
    .enumerate()
    {
        let mut params = StylizedHeroParams::default();
        params.character.cel.base_color_factor = bevy::math::Vec4::new(0.60, 0.62, 0.66, 1.0);
        if matcap_on {
            params = params.with_matcap(0.9, LinearRgba::new(1.0, 1.0, 1.0, 1.0));
        }
        if metal_enabled {
            params.metal.controls.x = 1.0;
            // 金属三色调（暗部青灰/亮部暖白/阴影蓝灰）。
            params.metal.dark = bevy::math::Vec4::new(0.35, 0.45, 0.5, 1.0);
            params.metal.light = bevy::math::Vec4::new(1.0, 0.95, 0.85, 1.0);
            params.metal.shadow = bevy::math::Vec4::new(0.4, 0.45, 0.6, 1.0);
        }
        if spec_ramp > 0.0 {
            params = params.with_spec_ramp_intensity(spec_ramp);
        }
        let material = world
            .resource_mut::<Assets<StylizedHeroMaterial>>()
            .add(StylizedHeroMaterial {
                params,
                ilm_texture: Some(ilm.clone()),
                matcap_texture: matcap_on.then(|| matcap.clone()),
                ..StylizedHeroMaterial::default()
            });
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz((i as f32 - 1.0) * 0.75, 1.35, 0.0),
        ));
    }
}

/// cutout · alpha 镂空（V4）：双竖卡（棋盘格 alpha 纹理 + cutoff 0.5 +
/// double_sided）——镂空剪影语义（叶片/网格类内容的通道）。
fn courtyard_cutout(world: &mut World) {
    // 程序化棋盘格 alpha 纹理（8×8 格，交替透明/不透明）。
    let n = 64usize;
    let mut data = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let on = ((x / 8) + (y / 8)) % 2 == 0;
            let offset = (y * n + x) * 4;
            data[offset] = 0x9a;
            data[offset + 1] = 0x68;
            data[offset + 2] = 0x4a;
            data[offset + 3] = if on { 255 } else { 0 };
        }
    }
    let texture = world.resource_mut::<Assets<Image>>().add(Image::new_fill(
        Extent3d {
            width: n as u32,
            height: n as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    let mut params = StylizedCelParams::default().with_alpha_cutoff(0.5);
    params.base_color_factor = bevy::math::Vec4::new(0.75, 0.55, 0.35, 1.0);
    let material = world
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial {
            params,
            base_color_texture: Some(texture),
            double_sided: true,
            ..StylizedCelMaterial::default()
        });
    let card = world
        .resource_mut::<Assets<Mesh>>()
        .add(bevy::math::primitives::Plane3d::new(Vec3::Z, bevy::math::Vec2::splat(0.55)));
    for x in [-0.55f32, 0.55] {
        world.spawn((
            Mesh3d(card.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(x, 1.0, 0.0),
        ));
    }
}

#[cfg(test)]
mod courtyard_tests {
    //! courtyard builder 的纯 CPU 冒烟：默认/参数化构建的分区实体计数、
    //! SceneTarget 唯一性、相机配方与附件选择（不触碰 GPU）。
    use super::*;
    use bevy::asset::AssetApp;

    pub(super) fn test_app(settings: Option<CourtyardSettings>) -> App {
        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins)
            .add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<bevy::world_serialization::WorldAsset>();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<StylizedCelMaterial>>()
            .init_resource::<Assets<StylizedCharacterMaterial>>()
            .init_resource::<Assets<StylizedHeroMaterial>>()
            .init_resource::<Assets<StylizedOutlineMaterial>>()
            .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
            .insert_resource(StylizedViewSettings(StylizedViewDescriptor::default()));
        if let Some(settings) = settings {
            app.insert_resource(settings);
        }
        app
    }

    fn scene_targets(world: &mut World) -> Vec<&'static str> {
        let mut query = world.query::<&SceneTarget>();
        let mut targets: Vec<&'static str> = query.iter(world).map(|target| target.0).collect();
        targets.sort_unstable();
        targets
    }

    fn count<T: bevy::prelude::Component>(world: &mut World) -> usize {
        world.query::<&T>().iter(world).count()
    }

    #[test]
    fn courtyard_builds_focused_zone_with_default_settings() {
        let mut app = test_app(None);
        let world = app.world_mut();
        let mesh_before = count::<Mesh3d>(world);
        let instance_before = count::<StylizedInstanceData>(world);
        let recipe = courtyard(world);
        // v3：默认 zone=trio——机位 = center-trio（内容联动），无相机附件。
        let preset = &crate::metadata::COURTYARD_CAMERA_PRESETS[0];
        assert_eq!(preset.id, "center-trio");
        assert_eq!(recipe.eye, preset.eye);
        assert_eq!(recipe.look, preset.look);
        assert!(matches!(recipe.attachment, CameraAttachment::None));
        // 内容档位（相对构建前的增量）：mesh 37 = 场地壳 23（地面/铺装/
        // 墙 20 弦/平台；v3 展台环退役）+ 三对照组 14（基座 7 + 球 7）；
        // 其余内容不生成。
        assert_eq!(count::<Mesh3d>(world) - mesh_before, 37);
        // 实例数据：face 3（P1/LOD/灯球不在场）。
        assert_eq!(count::<StylizedInstanceData>(world) - instance_before, 3);
        // P2/P3 未激活：无 hull 描边组件、无蒙皮条带。
        assert_eq!(count::<StylizedOutline>(world), 0);
        assert_eq!(count::<SkinnedMesh>(world), 0);
        // SceneTarget：唯一；未激活分区的落点不在场。
        let targets = scene_targets(world);
        let unique: std::collections::BTreeSet<&&str> = targets.iter().collect();
        assert_eq!(
            unique.len(),
            targets.len(),
            "SceneTarget ids must be unique"
        );
        for expected in ["trio.face.0", "trio.face.1", "trio.face.2"] {
            assert!(
                targets.contains(&expected),
                "missing SceneTarget {expected}"
            );
        }
        for absent in ["lamps", "flags.0", "flags.1"] {
            assert!(
                !targets.contains(&absent),
                "inactive zone target {absent} must not spawn"
            );
        }
        // 默认时刻 = day：庭院 preset 已落（雾 end=110），昼夜因子不动。
        let view = world.resource::<StylizedViewSettings>();
        assert_eq!(view.0.environment.fog.end, 110.0);
        assert_eq!(view.0.day_night_factor, 1.0);
        // face 组默认三阈值梯度 0.2/默认/0.8。
        let mut query = world.query::<(&SceneTarget, &StylizedInstanceData)>();
        let thresholds: Vec<(&str, Option<f32>)> = query
            .iter(world)
            .filter(|(target, _)| target.0.starts_with("trio.face."))
            .map(|(target, data)| (target.0, data.face_threshold))
            .collect();
        assert_eq!(thresholds.len(), 3);
        for (target, threshold) in thresholds {
            let expected = match target {
                "trio.face.0" => Some(0.2),
                "trio.face.1" => None,
                _ => Some(0.8),
            };
            assert_eq!(threshold, expected, "{target}");
        }
    }

    /// W3：增量装卸——切内容只动展品桶（先装后卸，退役时序归调用方），
    /// 画框常驻不动；相机配方随内容联动。
    #[test]
    fn courtyard_update_loads_and_unloads_zone_buckets() {
        let mut app = test_app(None);
        let world = app.world_mut();
        courtyard(world);
        let frame_before = world.resource::<CourtyardBuckets>().frame.clone();
        assert_eq!(frame_before.len(), 23, "shell = 画框 23 实体（v3 展台环退役）");
        assert_eq!(
            world.resource::<CourtyardBuckets>().exhibits["trio"].len(),
            14
        );
        // trio → p1：装 p1 桶（4 球 + 4 基座），trio 桶出登记簿进退役列表。
        let update = courtyard_update(
            world,
            CourtyardSettings::default(),
            CourtyardSettings {
                zone: CourtyardZone::P1,
                ..CourtyardSettings::default()
            },
        );
        assert_eq!(update.retire.len(), 14, "trio 桶 14 实体待退役");
        let buckets = world.resource::<CourtyardBuckets>();
        assert_eq!(buckets.exhibits["p1"].len(), 8);
        assert!(!buckets.exhibits.contains_key("trio"));
        assert_eq!(buckets.frame, frame_before, "画框常驻不动");
        // 配方联动：p1 → ramp 机位（§7 一一对应）。
        assert_eq!(update.recipe.eye, Vec3::new(0.0, 1.7, 5.2));
        // 调用方退役后：trio 实体离开世界，p1 与画框在场。
        retire_entities(world, &update.retire);
        let alive = world.iter_entities().count();
        for entity in &update.retire {
            assert!(world.get_entity(*entity).is_err(), "retired entity gone");
        }
        assert!(alive > 0);
    }

    /// W3：装卸/重装语义——内容切换只留目标桶；crowd/stage_character 只在
    /// 桶在场时触发重装（不在场 = 设置已记录，下次激活生效）。
    #[test]
    fn courtyard_update_reload_semantics() {
        let mut app = test_app(None);
        let world = app.world_mut();
        courtyard(world);
        // trio → p2：卸 trio 装 p2。
        let update = courtyard_update(
            world,
            CourtyardSettings::default(),
            CourtyardSettings {
                zone: CourtyardZone::P2,
                ..CourtyardSettings::default()
            },
        );
        retire_entities(world, &update.retire);
        let buckets = world.resource::<CourtyardBuckets>();
        assert_eq!(buckets.exhibits.len(), 1);
        assert!(buckets.exhibits.contains_key("p2"));
        // crowd 变化但 lamps 不在场（zone=p2）：不装卸，只留设置语义。
        let update = courtyard_update(
            world,
            CourtyardSettings {
                zone: CourtyardZone::P2,
                crowd: 100,
                ..CourtyardSettings::default()
            },
            CourtyardSettings {
                zone: CourtyardZone::P2,
                crowd: 0,
                ..CourtyardSettings::default()
            },
        );
        assert!(update.retire.is_empty());
        assert_eq!(world.resource::<CourtyardBuckets>().exhibits.len(), 1);
        // lamps 在场时 crowd 重装：p2 → lamps（crowd=100 装 200 实体），
        // 再 crowd 100→0（桶重装——退役 200，空阵不登记）。
        let update = courtyard_update(
            world,
            CourtyardSettings {
                zone: CourtyardZone::P2,
                crowd: 0,
                ..CourtyardSettings::default()
            },
            CourtyardSettings {
                zone: CourtyardZone::Lamps,
                crowd: 100,
                ..CourtyardSettings::default()
            },
        );
        retire_entities(world, &update.retire);
        assert_eq!(
            world.resource::<CourtyardBuckets>().exhibits["lamps"].len(),
            200
        );
        let update = courtyard_update(
            world,
            CourtyardSettings {
                zone: CourtyardZone::Lamps,
                crowd: 100,
                ..CourtyardSettings::default()
            },
            CourtyardSettings {
                zone: CourtyardZone::Lamps,
                crowd: 0,
                ..CourtyardSettings::default()
            },
        );
        assert_eq!(update.retire.len(), 200, "crowd 重装退役整个 lamps 桶");
        retire_entities(world, &update.retire);
        let buckets = world.resource::<CourtyardBuckets>();
        assert!(
            !buckets.exhibits.contains_key("lamps"),
            "crowd=0 空阵无实体登记（与全量构建同一语义）"
        );
        assert!(!buckets.exhibits.contains_key("p2"));
    }

    #[test]
    fn courtyard_parameters_drive_content_and_attachments() {
        // lamps 内容 + crowd=0 → 空阵（只有场地壳 23），且无 lamps 落点；
        // crowd=100 → 灯阵 200（柱 + 球）+ 100 实例。
        let mut app = test_app(Some(CourtyardSettings {
            zone: CourtyardZone::Lamps,
            crowd: 0,
            ..CourtyardSettings::default()
        }));
        let world = app.world_mut();
        let mesh_before = count::<Mesh3d>(world);
        courtyard(world);
        assert_eq!(count::<Mesh3d>(world) - mesh_before, 23);
        assert!(!scene_targets(world).contains(&"lamps"));
        let mut app = test_app(Some(CourtyardSettings {
            zone: CourtyardZone::Lamps,
            ..CourtyardSettings::default()
        }));
        let world = app.world_mut();
        let mesh_before = count::<Mesh3d>(world);
        courtyard(world);
        assert_eq!(count::<Mesh3d>(world) - mesh_before, 223);
        assert!(scene_targets(world).contains(&"lamps"));

        // night → day_night_factor=0；post → PostProcess 附件；
        // face_threshold 覆盖三实例。
        let mut app = test_app(Some(CourtyardSettings {
            time_of_day: CourtyardTimeOfDay::Night,
            post: true,
            face_threshold: Some(0.5),
            ..CourtyardSettings::default()
        }));
        let recipe = courtyard(app.world_mut());
        assert_eq!(
            app.world_mut()
                .resource::<StylizedViewSettings>()
                .0
                .day_night_factor,
            0.0
        );
        assert!(matches!(
            recipe.attachment,
            CameraAttachment::PostProcess(_)
        ));
        let mut query = app
            .world_mut()
            .query::<(&SceneTarget, &StylizedInstanceData)>();
        for (target, data) in query
            .iter(app.world())
            .filter(|(target, _)| target.0.starts_with("trio.face."))
        {
            assert_eq!(data.face_threshold, Some(0.5), "{}", target.0);
        }

        // hybrid → ScreenOutline 附件（优先于 post）；posed → 旗 root 旋转
        // 0.5 rad（P5 分区激活）。
        let mut app = test_app(Some(CourtyardSettings {
            zone: CourtyardZone::P5,
            outline_mode: CourtyardOutlineMode::Hybrid,
            post: true,
            flags: CourtyardFlagsPose::Posed,
            ..CourtyardSettings::default()
        }));
        let recipe = courtyard(app.world_mut());
        assert!(matches!(recipe.attachment, CameraAttachment::ScreenOutline));
        let mut query = app.world_mut().query::<(&SceneTarget, &Transform)>();
        let flag = query
            .iter(app.world())
            .find(|(target, _)| target.0 == "flags.0")
            .expect("flags.0");
        assert_eq!(flag.1.rotation, Quat::from_rotation_z(0.5));
    }

    /// 蒙皮旗面必须携带覆盖旗面实际位置的剔除 Aabb + NoAutoAabb
    /// （关节移到展台后，bind 姿态的原点包围盒会被非原点机位剔除；
    /// 且 calculate_bounds 会在网格资产事件后重算覆盖——两者缺一不可，
    /// 回归证据：capture 矩阵的 flags 机位曾因此丢旗面）。
    #[test]
    fn flag_strips_carry_overridden_culling_aabb() {
        let mut app = test_app(Some(CourtyardSettings {
            zone: CourtyardZone::P5,
            ..CourtyardSettings::default()
        }));
        courtyard(app.world_mut());
        let mut query = app.world_mut().query::<(
            &SkinnedMesh,
            &bevy::camera::primitives::Aabb,
            &bevy::camera::visibility::NoAutoAabb,
        )>();
        let strips: Vec<_> = query.iter(app.world()).collect();
        assert_eq!(strips.len(), 2, "both flag strips carry overridden aabbs");
        for (_, aabb, _) in strips {
            assert!(
                aabb.center.z == 0.0 && aabb.center.y > 1.0,
                "aabb must cover the flag content on the platform, not the origin: {:?}",
                aabb.center
            );
        }
    }
}
