#![expect(missing_docs, reason = "integration test names describe behavior")]

use bevy_cel::{CelNprAuthoring, TextureFilter};
use bevy_gltf_metadata::{MetadataSource, decode_metadata};

#[test]
fn two_material_entries_decode_without_value_crossover_and_keep_legacy_default()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let material = |row: u32, intensity: f32| {
        serde_json::json!({
            "ramp":{"row":row,"row_count":2,"band_count":3,"smoothing":0.0,"scale":1.0,"offset":0.0,"image":"portable.cel-resources/ramp.png","filter":"nearest","mipmaps":false},
            "ilm":{"image":null,"material_id_source":"texture_then_vertex","filter":"nearest","mipmaps":false},
            "face":{"left_image":null,"right_image":null,"bias":0.0,"rotation":0.0,"softness":0.0,"shadow_tint":[1.0,1.0,1.0],"external_shadow_influence":0.0,"filter":"linear","mipmaps":false},
            "hair":{"enabled":true,"tint":[1.0,1.0,1.0],"shift":0.0,"power":1.0,"threshold":1.0,"softness":0.0,"intensity":intensity,"material_mask":1.0},
            "specular":{"enabled":false,"tint":[1.0,1.0,1.0],"intensity":0.0,"power":1.0,"threshold":1.0,"softness":0.0},
            "rim":{"enabled":false,"tint":[1.0,1.0,1.0],"intensity":0.0,"power":1.0,"threshold":1.0,"softness":0.0,"material_mask":0.0,"special_fallback":1.0},
            "eye_emission":{"eye_enabled":false,"shadow_floor":0.2,"ambient_floor":0.04,"eye_class":1.0,"highlight_preservation":1.0,"emission_enabled":false,"emission_tint":[1.0,1.0,1.0],"emission_intensity":0.0,"emission_image":null,"material_fallback":0.0,"outline_suppression":1.0,"filter":"nearest","mipmaps":false},
            "outline":{"enabled":false,"color":[0.015,0.02,0.035],"width":2.0,"face_exclusion":0.0,"eye_exclusion":0.0,"alpha_mode":"opaque"}
        })
    };
    let first = material(1, 2.0);
    let second = material(0, 7.0);
    let input = serde_json::to_vec(
        &serde_json::json!({"schema_version":1,"nodes":{"Hero":{"domains":{"bevy_cel.npr":{"material":first,"materials_version":1,"materials":[{"slot":0,"name":"B_Second","settings":first},{"slot":1,"name":"A_First","settings":second}],"object":{"face":{"armature":null,"head_bone":"","forward_bone":"","right_bone":""},"post":{"hdr":true,"bloom":true,"tonemapping":true,"vignette":true,"lens_distortion":true,"chromatic_aberration":true,"prepass_edges":false,"bloom_intensity":0.18,"vignette_strength":0.38,"vignette_radius":0.86,"vignette_softness":0.3,"lens_distortion_amount":0.035,"chromatic_aberration_amount":0.004}}}}}}}),
    )?;

    // When
    let envelope = decode_metadata(&input, MetadataSource::Sidecar("fixture.cel.json".into()))?;
    let authored = CelNprAuthoring::from_node(&envelope.nodes["Hero"])?;

    // Then
    assert_eq!(authored.materials().len(), 2);
    assert_eq!(authored.materials_version, 1);
    assert_eq!(
        authored.materials()[0].settings.ramp.filter,
        TextureFilter::Nearest
    );
    assert_eq!(
        authored.materials()[0].settings.face.filter,
        TextureFilter::Linear
    );
    assert_eq!(authored.materials()[0].slot, 0);
    assert_eq!(authored.materials()[1].slot, 1);
    assert_eq!(
        authored.materials()[0]
            .settings
            .ramp
            .to_settings()
            .ramp_row(),
        1
    );
    assert_eq!(
        authored.materials()[1]
            .settings
            .ramp
            .to_settings()
            .ramp_row(),
        0
    );
    assert_eq!(
        authored
            .default_material()
            .hair
            .to_settings()
            .intensity()
            .to_bits(),
        2.0_f32.to_bits()
    );
    assert_eq!(
        authored
            .material_for_slot(1)
            .map(|value| value.hair.to_settings().intensity().to_bits()),
        Some(7.0_f32.to_bits())
    );
    Ok(())
}
