#![expect(missing_docs, reason = "integration test names describe behavior")]

use bevy_cel::{CEL_NPR_DOMAIN, CelNprAuthoring, TextureFilter};
use bevy_gltf_metadata::{MetadataSource, decode_metadata};

fn bits<const N: usize>(values: [f32; N]) -> [u32; N] {
    values.map(f32::to_bits)
}

#[test]
fn npr_domain_decodes_to_exact_runtime_settings_when_authored()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let input = br#"{"schema_version":1,"nodes":{"Hero":{"domains":{"bevy_cel.npr":{"material":{"ramp":{"row":2,"row_count":4,"band_count":5,"smoothing":0.125,"scale":1.5,"offset":-0.25,"image":"ramp.png","filter":"nearest","mipmaps":false},"ilm":{"image":"ilm.png","material_id_source":"vertex_then_texture","filter":"nearest","mipmaps":false},"face":{"left_image":"face_l.png","right_image":"face_r.png","bias":0.1,"rotation":0.2,"softness":0.05,"shadow_tint":[0.2,0.3,0.4],"external_shadow_influence":0.6},"hair":{"enabled":true,"tint":[0.8,0.7,0.6],"shift":0.2,"power":32.0,"threshold":0.4,"softness":0.1,"intensity":2.0,"material_mask":0.9},"specular":{"enabled":true,"tint":[0.7,0.8,0.9],"intensity":1.5,"power":24.0,"threshold":0.5,"softness":0.08},"rim":{"enabled":true,"tint":[0.4,0.5,0.6],"intensity":1.2,"power":8.0,"threshold":0.3,"softness":0.04,"material_mask":0.7,"special_fallback":0.6},"eye_emission":{"eye_enabled":true,"shadow_floor":0.25,"ambient_floor":0.1,"eye_class":0.8,"highlight_preservation":0.9,"emission_enabled":true,"emission_tint":[1.0,0.5,0.25],"emission_intensity":12.0,"emission_image":"emit.png","material_fallback":0.3,"outline_suppression":0.75,"filter":"linear","mipmaps":true},"outline":{"enabled":true,"color":[0.01,0.02,0.03],"width":4.0,"face_exclusion":0.2,"eye_exclusion":0.7,"alpha_mode":"opaque"}},"object":{"face":{"armature":"Rig","head_bone":"Head","forward_bone":"FaceForward","right_bone":"FaceRight"},"post":{"hdr":true,"bloom":true,"tonemapping":true,"vignette":true,"lens_distortion":true,"chromatic_aberration":true,"prepass_edges":false,"bloom_intensity":0.3,"vignette_strength":0.4,"vignette_radius":0.9,"vignette_softness":0.2,"lens_distortion_amount":0.04,"chromatic_aberration_amount":0.005}}}}}}}"#;

    // When
    let envelope = decode_metadata(input, MetadataSource::Sidecar("fixture.cel.json".into()))?;
    let authored = CelNprAuthoring::from_node(&envelope.nodes["Hero"])?;

    // Then
    assert_eq!(CEL_NPR_DOMAIN, "bevy_cel.npr");
    assert_eq!(authored.materials_version, 0);
    assert!(authored.materials().is_empty());
    assert_eq!(authored.default_material().ramp.to_settings().ramp_row(), 2);
    assert_eq!(authored.material.ilm.filter, TextureFilter::Nearest);
    assert_eq!(authored.material.eye_emission.filter, TextureFilter::Linear);
    assert_eq!(
        authored
            .material_for_slot(0)
            .map(|value| value.ramp.to_settings().ramp_row()),
        Some(2)
    );
    assert_eq!(authored.material.ramp.to_settings().ramp_row(), 2);
    assert_eq!(authored.material.ramp.to_settings().band_count(), 5);
    assert_eq!(
        authored.material.ilm.to_settings().material_id_source(),
        bevy_cel::IlmMaterialIdSource::VertexThenTexture
    );
    assert_eq!(
        authored.material.hair.to_settings().intensity().to_bits(),
        2.0_f32.to_bits()
    );
    assert_eq!(
        bits(authored.material.hair.to_settings().tint()),
        bits([0.8, 0.7, 0.6])
    );
    assert_eq!(
        authored.material.specular.to_settings().power().to_bits(),
        24.0_f32.to_bits()
    );
    assert_eq!(
        authored.material.rim.to_settings().softness().to_bits(),
        0.04_f32.to_bits()
    );
    let lighting = authored.material.to_lighting_settings();
    assert!(lighting.hair_enabled());
    assert!(lighting.specular_enabled());
    assert!(lighting.rim_enabled());
    assert_eq!(bits(lighting.material_masks()), bits([0.9, 0.7, 0.6]));
    let face = authored.material.face.to_settings();
    assert_eq!(
        bits([
            face.bias(),
            face.rotation(),
            face.softness(),
            face.external_shadow_influence()
        ]),
        bits([0.1, 0.2, 0.05, 0.6])
    );
    assert_eq!(bits(face.shadow_tint()), bits([0.2, 0.3, 0.4]));
    let eye = authored.material.eye_emission.to_settings();
    let floors = eye.eye_floor();
    let class = eye.eye_class();
    let emission = eye.emission();
    let masks = eye.masks();
    assert_eq!(
        bits([floors.0, floors.1, class.0, class.1]),
        bits([0.25, 0.1, 0.8, 0.9])
    );
    assert_eq!(bits(emission.0), bits([1.0, 0.5, 0.25]));
    assert_eq!(
        bits([emission.1, masks.0, masks.1]),
        bits([12.0, 0.3, 0.75])
    );
    assert_eq!(
        authored.material.outline.to_settings().width().to_bits(),
        4.0_f32.to_bits()
    );
    let post = authored.object.post.to_profile();
    assert!(post.hdr_enabled());
    assert!(post.bloom_enabled());
    assert!(post.tonemapping_enabled());
    assert!(post.vignette_enabled());
    assert!(post.lens_distortion_enabled());
    assert!(post.chromatic_aberration_enabled());
    assert!(!post.prepass_edges_enabled());
    assert_eq!(
        post.settings().bloom_intensity().to_bits(),
        0.3_f32.to_bits()
    );
    assert_eq!(bits(post.settings().vignette()), bits([0.4, 0.9, 0.2]));
    assert_eq!(
        bits([
            post.settings().lens_distortion(),
            post.settings().chromatic_aberration()
        ]),
        bits([0.04, 0.005])
    );
    Ok(())
}
