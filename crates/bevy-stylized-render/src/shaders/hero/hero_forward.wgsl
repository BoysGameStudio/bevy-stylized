#define_import_path bevy_stylized_render::hero::hero_forward

// Hero@Portable 前向（It-5，provisional pending ADR-0001/0005/0006）。
//
// = Character（cel 基础 + face SDF + hair 高光 + rim + emission）
//   + authored ramp（atlas 行 = MeshTag profile_index）
//   + packed normal（xy 存储 z 重建）
//   + metal matcap（view-space normal 查找 × ILM.B metal mask）
//   + spec ramp（半角高光坐标采样 × 强度）
//
// Portable 合法：全部 fixed binding + filterable float 纹理；无 storage /
// compute / bindless。fallback 两层：材质级（HERO_FLAG_PORTABLE_FALLBACK）
// 与 per-instance LOD（MeshTag flags bits 24..31，V2-b）——两者都关闭
// matcap / spec ramp / packed normal，face SDF 可降为硬 step。
//
// 结构名在本模块内唯一（naga-oil 跨模块同名 struct 冲突规避）。

#import bevy_stylized_render::common::npr_common::{
    deform_vertex, analytic_ramp, face_light_dots, face_threshold_from_dots,
    environment_fog_apply, environment_shadow_tint,
}
#import bevy_pbr::{
    forward_io::{Vertex, VertexOutput, FragmentOutput},
    mesh_functions,
    mesh_view_bindings::{view, lights},
    view_transformations::position_world_to_clip,
}

struct HeroCelParams {
    base_color_factor: vec4<f32>,
    shading: vec4<f32>,
    // x: rim 幂次, y: rim 强度, z: 预留, w: alpha mask cutoff
    rim: vec4<f32>,
    rim_color: vec4<f32>,
}

struct HeroCharacterParams {
    cel: HeroCelParams,
    face_forward_softness: vec4<f32>,
    face_misc: vec4<f32>,
    hair: vec4<f32>,
    emission: vec4<f32>,
    palette_soft_lit: vec4<f32>,
    palette_soft_shadow: vec4<f32>,
    palette_sharp_lit: vec4<f32>,
    palette_sharp_shadow: vec4<f32>,
    face_up: vec4<f32>,
}

struct GenshinLightingParams {
    controls: vec4<f32>,
    lightmap_constant: vec4<f32>,
    specular_color: vec4<f32>,
    specular_rows: array<vec4<f32>,5>,
    shadow_rows: array<vec4<f32>,5>,
    highlight_curve: vec4<f32>,
    highlight_primary: vec4<f32>,
    highlight_secondary: vec4<f32>,
}
struct HeroParams {
    character: HeroCharacterParams,
    // x: matcap 强度, y: spec ramp 强度, z: shadow position（ramp 公式
    // ShadowPosition）, w: ramp width（ShadowRampWidth）
    hero_a: vec4<f32>,
    // rgb: matcap 染色, w: 位标志（bitcast u32）
    hero_b: vec4<f32>,
    genshin: GenshinLightingParams,
}

const CEL_FLAG_ALPHA_MASK: u32 = 8u;
const CHAR_FLAG_ILM_TEXTURE: u32 = 1u;
const CHAR_FLAG_FACE_SDF_TEXTURE: u32 = 2u;
const CHAR_FLAG_HAIR_SPECULAR: u32 = 4u;
const CHAR_FLAG_EMISSION: u32 = 8u;
const HERO_FLAG_RAMP_TEXTURE: u32 = 1u;
const HERO_FLAG_PACKED_NORMAL: u32 = 2u;
const HERO_FLAG_MATCAP: u32 = 4u;
const HERO_FLAG_SPEC_RAMP: u32 = 8u;
const HERO_FLAG_PORTABLE_FALLBACK: u32 = 16u;
// Genshin-profile ramp 寻址（已核实公式）启用位；否则走 MeshTag 行寻址。
const HERO_FLAG_GENSHIN_RAMP: u32 = 32u;

// MeshTag 位段（与 instance.rs 一致；provisional pending ADR-0004）。
const TAG_FACE_THRESHOLD_SHIFT: u32 = 8u;
const TAG_FACE_SIDE_SHIFT: u32 = 16u;
// Per-instance flags（bits 24..31，与 lod.rs 的 INSTANCE_FLAG_* 一致，V2-b）。
const TAG_FLAGS_SHIFT: u32 = 24u;
const INSTANCE_FLAG_DROP_HERO_EXTRAS: u32 = 1u;  // matcap / spec ramp / packed normal
const INSTANCE_FLAG_FACE_HARD_STEP: u32 = 2u;
const INSTANCE_FLAG_HAIR_SPECULAR_OFF: u32 = 4u;

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> params: HeroParams;
@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var base_color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var base_color_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var ilm_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4)
var ilm_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5)
var face_sdf_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6)
var face_sdf_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(7)
var ramp_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(8)
var ramp_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(9)
var packed_normal_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(10)
var packed_normal_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(11)
var matcap_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(12)
var matcap_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(13)
var spec_ramp_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(14)
var spec_ramp_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(15)
var surface_control_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(16)
var surface_control_sampler: sampler;
const HERO_FLAG_GENSHIN_FACE: u32 = 64u;
const HERO_FLAG_PUPIL_HIGHLIGHT: u32 = 128u;

fn hero_view_direction(position: vec3<f32>) -> vec3<f32> {
    if view.clip_from_view[3].w == 1.0 {
        // Camera-local +Z points towards the viewer. Projection depth rows
        // include reverse-Z signs and are not a camera direction basis.
        return normalize(view.world_from_view[2].xyz);
    }
    return normalize(view.world_position-position);
}

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    let deformed = deform_vertex(vertex_in);
    var out: VertexOutput;
#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = normalize(
        (deformed.world_from_local * vec4<f32>(deformed.local_normal, 0.0)).xyz
    );
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        deformed.local_normal,
        deformed.instance_index,
    );
#endif
#endif
#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(
        deformed.world_from_local,
        vec4<f32>(deformed.local_position, 1.0),
    );
    out.position = position_world_to_clip(out.world_position.xyz);
#endif
#ifdef VERTEX_UVS_A
    out.uv = vertex_in.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex_in.uv_b;
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        deformed.world_from_local,
        vertex_in.tangent,
        deformed.instance_index,
    );
#endif
#ifdef VERTEX_COLORS
    out.color = vertex_in.color;

#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = deformed.instance_index;
#endif
    return out;
}

// Cubic B-spline reconstruction of level-zero authored control data.
// Positive weights let each adjacent pair share one bilinear lookup.
fn cubic_weights(t: vec2<f32>) -> array<vec2<f32>,4> {
    let t2=t*t;
    let t3=t2*t;
    return array<vec2<f32>,4>(
        (vec2<f32>(1.0)-t)*(vec2<f32>(1.0)-t)*(vec2<f32>(1.0)-t)/6.0,
        (3.0*t3-6.0*t2+vec2<f32>(4.0))/6.0,
        (-3.0*t3+3.0*t2+3.0*t+vec2<f32>(1.0))/6.0,
        t3/6.0,
    );
}
fn sample_cubic(tex: texture_2d<f32>, smp: sampler, uv: vec2<f32>) -> vec4<f32> {
    let dimensions=vec2<f32>(textureDimensions(tex));
    let pixel=uv*dimensions-0.5;
    let weights=cubic_weights(fract(pixel));
    var result=vec4<f32>(0.0);
    for(var y=0u;y<2u;y+=1u) {
        for(var x=0u;x<2u;x+=1u) {
            let weight=vec2<f32>(weights[2u*x].x+weights[2u*x+1u].x,weights[2u*y].y+weights[2u*y+1u].y);
            let offset=vec2<f32>(weights[2u*x+1u].x,weights[2u*y+1u].y)/weight;
            let coordinate=(floor(pixel)+vec2<f32>(f32(2u*x),f32(2u*y))-0.5+offset)/dimensions;
            result+=textureSampleLevel(tex,smp,coordinate,0.0)*weight.x*weight.y;
        }
    }
    return result;
}

@fragment
fn fragment(vertex: VertexOutput, @builtin(front_facing) front_facing: bool) -> FragmentOutput {
    var in=vertex;
#ifdef VERTEX_UVS_A
#ifdef VERTEX_UVS_B
    if !front_facing && params.genshin.controls.z>0.5 {in.uv=in.uv_b;}
#endif
#endif

    let cel_flags = bitcast<u32>(params.character.cel.shading.w);
    let char_flags = bitcast<u32>(params.character.face_misc.y);
    let hero_flags = bitcast<u32>(params.hero_b.w);

    // Per-instance LOD（V2-b）：MeshTag flags 逐实例削减，材质资产不动。
    var tag: u32 = 0u;
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    tag = mesh_functions::get_tag(in.instance_index);
#endif
    let instance_flags = (tag >> TAG_FLAGS_SHIFT) & 0xFFu;
    let drop_hero_extras = (instance_flags & INSTANCE_FLAG_DROP_HERO_EXTRAS) != 0u;
    let instance_face_hard_step = (instance_flags & INSTANCE_FLAG_FACE_HARD_STEP) != 0u;
    let hair_specular_off = (instance_flags & INSTANCE_FLAG_HAIR_SPECULAR_OFF) != 0u;

    var base = params.character.cel.base_color_factor;
#ifdef VERTEX_UVS_A
    if (hero_flags & 256u)!=0u {base*=sample_cubic(base_color_texture,base_color_sampler,in.uv);}
    else {base *= textureSample(base_color_texture, base_color_sampler, in.uv);}
#endif
    if (cel_flags & CEL_FLAG_ALPHA_MASK) != 0u {
        if base.a < params.character.cel.rim.w {
            discard;
        }
    }

#ifdef STYLIZED_BASE_COLOR
    var base_output: FragmentOutput;
    base_output.color = vec4<f32>(base.rgb,1.0);
    return base_output;
#endif

    // View 层 key light。
    var light_direction = vec3<f32>(0.0, 1.0, 0.0);
    var light_color = vec3<f32>(1.0);
    if lights.n_directional_lights > 0u {
        light_direction = lights.directional_lights[0].direction_to_light;
        light_color = lights.directional_lights[0].color.rgb;
    }

    // Packed normal（xy 存储，z 重建；通用解码，未锁定上游的非常规
    // 重建式——profile 特化属 profile crate）。
    var normal = normalize(in.world_normal);
#ifdef VERTEX_UVS_A
    if (hero_flags & HERO_FLAG_PACKED_NORMAL) != 0u && !drop_hero_extras {
        let packed = textureSample(packed_normal_texture, packed_normal_sampler, in.uv).rg;
        let tangent_xy = packed * 2.0 - vec2<f32>(1.0);
        let tangent_z = sqrt(max(1.0 - dot(tangent_xy, tangent_xy), 0.0));
        // 无切线基时的保守近似：把解码法线视作物体空间扰动叠加到几何法线。
        let perturb = vec3<f32>(tangent_xy.x, tangent_xy.y, tangent_z);
        normal = normalize(normal + (perturb - vec3<f32>(0.0, 0.0, 1.0)) * 0.5);
    }
#endif

#ifdef STYLIZED_NDOTH
    var half_output: FragmentOutput;
    let h=normalize(light_direction+hero_view_direction(in.world_position.xyz));
    half_output.color=vec4<f32>(vec3<f32>(max(dot(normal,h),0.0)),1.0);
    return half_output;
#endif

    let half_lambert = clamp(dot(normal, light_direction) * 0.5 + 0.5, 0.0, 1.0);

    // ILM decode once（docs/reference/07 §3.2 已核实布局）：
    // R=metal mask(>0.9) / G=AO / B=高光抑制(1.03−B) / A=材质分带 ID。
    let genshin_lighting = params.genshin.controls.z > 0.5;
    var packed = vec4<f32>(0.0,1.0,0.0,0.0);
    if genshin_lighting {packed=params.genshin.lightmap_constant;}
#ifdef VERTEX_UVS_A
    if (char_flags & CHAR_FLAG_ILM_TEXTURE) != 0u {packed=textureSample(ilm_texture,ilm_sampler,in.uv);if genshin_lighting {packed=sample_cubic(ilm_texture,ilm_sampler,in.uv);}}
#endif
    var metal_mask = select(0.0,1.0,packed.r>0.9);
    var ao=1.0;
    if (char_flags & CHAR_FLAG_ILM_TEXTURE) != 0u || genshin_lighting {
        ao=mix(0.5,1.0,packed.g);
        if (hero_flags & HERO_FLAG_GENSHIN_RAMP) != 0u || genshin_lighting {ao=packed.g;}
    }
    if genshin_lighting {ao=mix(1.0,packed.g,params.genshin.controls.x);}
#ifdef VERTEX_COLORS
    if genshin_lighting {ao*=mix(1.0,pow(max(in.color.r,0.0),0.4545),params.genshin.controls.w);}
#endif
    let spec_suppress=1.03-packed.b;
    var band=1.0+f32(packed.a>0.2)+f32(packed.a>0.4)+f32(packed.a>0.6)+f32(packed.a>0.8);
    if genshin_lighting || (hero_flags & HERO_FLAG_GENSHIN_RAMP)!=0u {
        let material_order=array<f32,5>(1.0,4.0,3.0,5.0,2.0);
        band=material_order[u32(band)-1u];
    }
    if genshin_lighting && params.genshin.specular_rows[u32(band)-1u].z<0.5 {band=1.0;}

    // LitFactor：half-lambert 经 AO 门控（docs/reference/07 §3.1 已核实）。
    var lit_factor = half_lambert * smoothstep(0.01, 0.4, ao);

    // 阴影色：authored ramp 优先。
    var shadow_color = params.character.palette_soft_shadow.rgb;
    if (hero_flags & HERO_FLAG_RAMP_TEXTURE) != 0u {
        if (hero_flags & HERO_FLAG_GENSHIN_RAMP) != 0u {
            // 已核实 ramp 寻址（docs/reference/07 §3.1）：
            // X = 1 − (ShadowPosition − LitFactor_AO) / (ShadowPosition
            //     × (vcolG × 2) × RampWidth)
            // Y accounts for top-down PNG rows; day uses the upper half.
            // 我方 day_night 约定 1=昼（face_misc.w，材质静态）。
            var vcol_g = 1.0;
#ifdef VERTEX_COLORS
            vcol_g=pow(max(in.color.g,0.0),0.4545);
#endif
            let shadow_position = params.hero_a.z;
            let ramp_width = params.hero_a.w;
            // Blender Math Divide returns zero for a zero denominator.
            // Zero vertex-width controls are authored data, not missing values.
            var position_term=0.0;
            if shadow_position!=0.0 {position_term=(shadow_position-lit_factor)/shadow_position;}
            var width_term=0.0;
            if vcol_g*2.0*ramp_width!=0.0 {width_term=position_term/(vcol_g*2.0*ramp_width);}
            let ramp_x=1.0-width_term;
            let day_row = 0.05 + 0.1 * (band - 1.0);
            let day_night = clamp(params.character.face_misc.w, 0.0, 1.0);
            // PNG rows are top-down, opposite the authored Blender UV axis.
            let ramp_v = day_row + 0.5 * (1.0-day_night);
            let ramp_size=vec2<i32>(textureDimensions(ramp_texture));
            let ramp_pixel=clamp(vec2<i32>(vec2<f32>(clamp(ramp_x,0.0,1.0),ramp_v)*vec2<f32>(ramp_size)),vec2<i32>(0),ramp_size-vec2<i32>(1));
            shadow_color=textureLoad(ramp_texture,ramp_pixel,0).rgb;
            // The authored ramp first modulates diffuse, then its coordinate
            // drives the separate soft/sharp palette factors. It is not a
            // substitute palette shadow color.
            let force_lit = select(0.0,1.0,ao > 0.95);
            let ramp_gate = clamp(select(0.0,1.0,lit_factor < shadow_position)-force_lit,0.0,1.0);
            base = vec4<f32>(base.rgb * mix(vec3<f32>(1.0),shadow_color,ramp_gate),base.a);
            lit_factor = clamp(max(ramp_x,force_lit),0.0,1.0);
            shadow_color = params.character.palette_soft_shadow.rgb;
        } else {
            // 通用 ramp 路径（保留）：MeshTag 行寻址。
            var row = 0.0;
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
            row = f32(mesh_functions::get_tag(in.instance_index) & 0xFFu);
#endif
            let ramp_v = (row + 0.5) / max(params.character.cel.rim.z, 1.0);
            shadow_color = textureSample(
                ramp_texture, ramp_sampler,
                vec2<f32>(half_lambert, ramp_v),
            ).rgb;
        }
    }

    if genshin_lighting && (hero_flags & HERO_FLAG_RAMP_TEXTURE)==0u {
        var mixed=(ao+half_lambert)*0.5;
        if ao>0.95 {mixed=1.0;}
        if ao<0.05 {mixed=0.0;}
        let transition=params.genshin.shadow_rows[u32(band)-1u];
        var shadow=0.0;
        if mixed<params.hero_a.z {shadow=min(pow((params.hero_a.z-mixed)/max(transition.x,0.0001)+0.01,max(transition.y,0.0001)),1.0);}
        lit_factor=1.0-shadow;
    }
    var sharp_factor = select(0.0, 1.0, lit_factor > 0.5);
    if genshin_lighting || ((hero_flags & HERO_FLAG_GENSHIN_RAMP) != 0u && (hero_flags & HERO_FLAG_RAMP_TEXTURE) != 0u) {
        sharp_factor = select(0.0,1.0,lit_factor > 0.999);
    }

    // Face SDF（与 character 一致；共享材质 + MeshTag 实例数据）。
#ifdef VERTEX_UVS_A
    if (char_flags & CHAR_FLAG_FACE_SDF_TEXTURE) != 0u {
        let face_forward_world = normalize(
            (mesh_functions::get_world_from_local(in.instance_index)
                * vec4<f32>(params.character.face_forward_softness.xyz, 0.0)).xyz
        );
        let face_up_world = normalize(
            (mesh_functions::get_world_from_local(in.instance_index)
                * vec4<f32>(params.character.face_up.xyz, 0.0)).xyz
        );
        var face_light = light_direction;
        if (hero_flags & HERO_FLAG_GENSHIN_FACE) != 0u {
            face_light -= face_up_world * dot(face_light,face_up_world);
        }
        let dots = face_light_dots(face_light, face_forward_world, face_up_world);
        var threshold = face_threshold_from_dots(dots.x);
        if (hero_flags & HERO_FLAG_GENSHIN_FACE) != 0u {
            threshold = pow(min((1.0-dots.x)*0.5,0.999), pow(1.5-params.hero_a.z,3.0));
        }
        let tag_threshold = (tag >> TAG_FACE_THRESHOLD_SHIFT) & 0xFFu;
        if tag_threshold != 0u {
            threshold = f32(tag_threshold - 1u) / 254.0;
        }
        var flip = dots.y < 0.0;
        if (hero_flags & HERO_FLAG_GENSHIN_FACE)!=0u {flip=dots.y>=0.0;}
        let force_flip = ((tag >> TAG_FACE_SIDE_SHIFT) & 1u) != 0u
            || params.character.face_misc.z > 0.5;
        if force_flip {
            flip = !flip;
        }
        var uv_x = in.uv.x;
        if flip {
            uv_x = 1.0 - uv_x;
        }
        var sdf = textureSample(face_sdf_texture, face_sdf_sampler, vec2<f32>(uv_x, in.uv.y)).r;
        if (hero_flags & HERO_FLAG_GENSHIN_FACE)!=0u {sdf=sample_cubic(face_sdf_texture,face_sdf_sampler,vec2<f32>(uv_x,in.uv.y)).r;}
        let softness = params.character.face_forward_softness.w;
        if (hero_flags & HERO_FLAG_GENSHIN_FACE) != 0u {
            let control=sample_cubic(surface_control_texture,surface_control_sampler,in.uv);
            let fixed_band=(control.a-select(0.0,1.0,control.a>0.55))*5.0;
            let fixed_shadow=clamp((1.0-threshold-0.71)/0.09,0.0,1.0)*fixed_band;
            sharp_factor=clamp(smoothstep(threshold-softness,threshold+softness,sdf)*control.g+fixed_shadow,0.0,1.0);
            lit_factor=clamp(smoothstep(threshold-softness-0.05,threshold+softness+0.05,sdf+0.05)*control.g+fixed_shadow,0.0,1.0);
            if params.genshin.controls.w>0.5 {
                let order=array<u32,5>(0u,3u,2u,4u,1u);
                var color_band=order[u32(control.a>0.2)+u32(control.a>0.4)+u32(control.a>0.6)+u32(control.a>0.8)];
                if color_band>2u {color_band=0u;}
                let tint=pow(mix(params.genshin.shadow_rows[color_band].rgb,params.genshin.specular_rows[color_band].rgb,params.character.face_misc.w),vec3<f32>(2.2));
                base=vec4<f32>(base.rgb*mix(tint,vec3<f32>(1.0),sharp_factor),base.a);
            }
        } else {

        if (hero_flags & HERO_FLAG_PORTABLE_FALLBACK) != 0u || instance_face_hard_step {
            lit_factor = step(threshold, sdf);
        } else {
            lit_factor = smoothstep(threshold - softness, threshold + softness, sdf);
        }
            sharp_factor = select(0.0,1.0,lit_factor>0.5);
        }
    }
#endif

#ifdef STYLIZED_LIGHTING_FACTORS
    var factor_output: FragmentOutput;
    factor_output.color = vec4<f32>(lit_factor, sharp_factor, 1.0, 1.0);
    return factor_output;
#endif

    // 双档光照（docs/reference/07 §3.7 已核实：sharp/soft 串联相乘 +
    // ambient，全 MULTIPLY；ramp 阴影色替代 softShadow）。
    // 阴影色经环境阴影染色（V2-d；强度 0 时逐位恒等）。
    let soft_mix = mix(environment_shadow_tint(shadow_color), params.character.palette_soft_lit.rgb, lit_factor);
    let sharp_mix = mix(environment_shadow_tint(params.character.palette_sharp_shadow.rgb), params.character.palette_sharp_lit.rgb, sharp_factor);
    var shaded_base=base.rgb;
    if genshin_lighting {
        let row=params.genshin.specular_rows[u32(band)-1u];
        let half_vector=normalize(light_direction+hero_view_direction(in.world_position.xyz));
        let ndoth=max(dot(normal,half_vector),0.001);
        let nonmetal=params.genshin.specular_color.rgb*row.y*packed.r*select(0.0,1.0,pow(ndoth,max(row.x,0.001))>1.03-packed.b);
        // The qualified source recipe disables metallics for both the
        // matcap and specular branches. Do not infer metal from ILM.R alone.
        shaded_base+=nonmetal;
    }
#ifdef STYLIZED_PRE_PALETTE
    var palette_input: FragmentOutput;
    palette_input.color=vec4<f32>(shaded_base,1.0);
    return palette_input;
#endif
    let ambient = lights.ambient_color.rgb;
    var color = shaded_base * soft_mix * sharp_mix * max(ambient, vec3<f32>(0.05));

    // Hair 高光（cheap Blinn × ILM.B 高光抑制）。
    if (char_flags & CHAR_FLAG_HAIR_SPECULAR) != 0u && !hair_specular_off {
        let view_direction = hero_view_direction(in.world_position.xyz);
        let half_vector = normalize(light_direction + view_direction);
        let specular = pow(clamp(dot(normal, half_vector), 0.0, 1.0), params.character.hair.x);
        color += specular * params.character.hair.y * spec_suppress * light_color;
    }

    // Spec ramp（半角高光 × ILM.B 抑制；material fallback 或 per-instance
    // LOD（drop_hero_extras）下关闭）。
    if (hero_flags & HERO_FLAG_SPEC_RAMP) != 0u && !drop_hero_extras {
        let view_direction = hero_view_direction(in.world_position.xyz);
        let half_vector = normalize(light_direction + view_direction);
        let spec_coord = clamp(dot(normal, half_vector), 0.0, 1.0);
        let spec = textureSample(spec_ramp_texture, spec_ramp_sampler, vec2<f32>(spec_coord, 0.5)).rgb;
        color += spec * params.hero_a.y * spec_suppress * light_color;
    }

    // Metal matcap（view-space normal 查找 × ILM.R metal mask（>0.9）；
    // material fallback 或 per-instance LOD 关闭）。
    if (hero_flags & HERO_FLAG_MATCAP) != 0u && !drop_hero_extras {
        let view_normal = normalize(
            (view.view_from_world * vec4<f32>(normal, 0.0)).xyz
        );
        let matcap_uv = view_normal.xy * 0.5 + vec2<f32>(0.5);
        let matcap = textureSample(matcap_texture, matcap_sampler, matcap_uv).rgb;
        color = mix(color, color * params.hero_b.rgb + matcap * params.hero_a.x, metal_mask);
    }

    // Rim。
    if (cel_flags & 4u) != 0u {
        let view_direction = hero_view_direction(in.world_position.xyz);
        let fresnel = pow(clamp(1.0 - dot(normal, view_direction), 0.0, 1.0), params.character.cel.rim.x);
        color += fresnel * params.character.cel.rim.y * params.character.cel.rim_color.rgb;
    }

    // Emission（mask = base color alpha；上游 alpha switch 语义，
    // provisional pending ADR-0023）。
    if (char_flags & CHAR_FLAG_EMISSION) != 0u {
        color += params.character.emission.rgb * params.character.emission.w * base.a;
    }

    var output_alpha=1.0;
#ifdef VERTEX_UVS_A
    if (hero_flags & HERO_FLAG_PUPIL_HIGHLIGHT)!=0u {
        let red=textureSample(surface_control_texture,surface_control_sampler,in.uv).r;
        let a=clamp(red/max(params.genshin.highlight_curve.y,0.0001),0.0,1.0);
        let first=clamp(red/max(params.genshin.highlight_primary.w,0.0001),0.0,1.0);
        let second=clamp((red-params.genshin.highlight_secondary.w)/max(params.genshin.highlight_curve.x-params.genshin.highlight_secondary.w,0.0001),0.0,1.0);
        let emission=params.genshin.highlight_primary.rgb*first+params.genshin.highlight_secondary.rgb*second;
        output_alpha=1.0-a+a*a;
        color=(color*(1.0-a)+emission*a*a)/output_alpha;
    }
#endif

    // 环境 NPR（V2-d）：距离雾（无 DistanceFog 组件时逐位恒等）。
    color = environment_fog_apply(color, in.world_position.xyz, view.world_position);

    color *= view.exposure;

    var out: FragmentOutput;
    out.color = vec4<f32>(color, output_alpha);
    return out;
}
