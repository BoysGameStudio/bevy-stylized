//! Bounded host-input ABI for opt-in material transport.
//!
//! These packing primitives do not install a renderer system or change existing
//! materials. Binding, extraction, shader consumption and view association need
//! their own qualification before a host can use this transport.
use bevy::{
    math::{UVec4, Vec4},
    render::render_resource::{
        ShaderType,
        encase::{UniformBuffer, internal::WriteInto},
    },
};

/// Version of this GPU record layout; independent of the existing MeshTag ABI.
pub const HOST_INPUT_ABI_VERSION: u32 = 1;
/// Number of distinct active rendering-input records, not a claimed actor limit.
pub const HOST_INSTANCE_CAPACITY: usize = 128;
/// Number of separately identified view records.
pub const HOST_VIEW_CAPACITY: usize = 16;
/// Conservative maximum size of one Portable uniform binding.
pub const PORTABLE_UNIFORM_BYTES: usize = 16_384;

/// Six aligned lanes. Colors are linear; opacity is tint.w.
#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct HostInstanceRecord {
    pub tint: Vec4,
    pub emission: Vec4,
    pub rim: Vec4,
    /// World-space forward and a 0/1 enable flag.
    pub face_forward: Vec4,
    /// World-space up and outline width multiplier.
    pub face_up: Vec4,
    /// Profile, view, palette and quality flags. No implicit truncation.
    pub indices: UVec4,
}

impl Default for HostInstanceRecord {
    fn default() -> Self {
        Self {
            tint: Vec4::ONE,
            emission: Vec4::ZERO,
            rim: Vec4::ZERO,
            face_forward: Vec4::new(0.0, 0.0, 1.0, 0.0),
            face_up: Vec4::new(0.0, 1.0, 0.0, 1.0),
            indices: UVec4::ZERO,
        }
    }
}

/// Ten aligned view lanes. This is input data, not camera ownership or a light.
#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct HostViewRecord {
    pub direction: Vec4,
    pub key_color: Vec4,
    pub ambient: Vec4,
    pub soft_lit: Vec4,
    pub soft_shadow: Vec4,
    pub sharp_lit: Vec4,
    pub sharp_shadow: Vec4,
    /// Ramp-page blend, exposure multiplier, output mode and reserved.
    pub controls: Vec4,
    pub background: Vec4,
    pub fog: Vec4,
}

impl Default for HostViewRecord {
    fn default() -> Self {
        Self {
            direction: Vec4::new(0.0, 1.0, 0.0, 0.0),
            key_color: Vec4::ONE,
            ambient: Vec4::ZERO,
            soft_lit: Vec4::ONE,
            soft_shadow: Vec4::ONE,
            sharp_lit: Vec4::ONE,
            sharp_shadow: Vec4::ONE,
            controls: Vec4::new(0.0, 1.0, 0.0, 0.0),
            background: Vec4::ZERO,
            fog: Vec4::ZERO,
        }
    }
}

#[derive(ShaderType)]
struct HostTables {
    instances: [HostInstanceRecord; HOST_INSTANCE_CAPACITY],
    views: [HostViewRecord; HOST_VIEW_CAPACITY],
}

/// A rejected table must not replace the last valid renderer inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostInputError {
    Capacity,
    MissingView,
    NonFinite,
    InvalidOpacity,
    InvalidFaceBasis,
    InvalidView,
    UniformTooLarge,
    Encoding(String),
}

fn checked_tables(
    instances: &[HostInstanceRecord],
    views: &[HostViewRecord],
) -> Result<HostTables, HostInputError> {
    if instances.len() > HOST_INSTANCE_CAPACITY || views.len() > HOST_VIEW_CAPACITY {
        return Err(HostInputError::Capacity);
    }
    for view in views {
        if [
            view.direction,
            view.key_color,
            view.ambient,
            view.soft_lit,
            view.soft_shadow,
            view.sharp_lit,
            view.sharp_shadow,
            view.controls,
            view.background,
            view.fog,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            return Err(HostInputError::NonFinite);
        }
        if view.direction.truncate().length_squared() < 1e-12
            || !(0.0..=1.0).contains(&view.controls.x)
            || view.controls.y <= 0.0
        {
            return Err(HostInputError::InvalidView);
        }
    }
    for instance in instances {
        if [
            instance.tint,
            instance.emission,
            instance.rim,
            instance.face_forward,
            instance.face_up,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            return Err(HostInputError::NonFinite);
        }
        if instance.indices.y as usize >= views.len() {
            return Err(HostInputError::MissingView);
        }
        if !(0.0..=1.0).contains(&instance.tint.w) {
            return Err(HostInputError::InvalidOpacity);
        }
        if !matches!(instance.face_forward.w, 0.0 | 1.0)
            || !(0.0..=2.0).contains(&instance.face_up.w)
            || (instance.face_forward.w == 1.0
                && instance
                    .face_forward
                    .truncate()
                    .cross(instance.face_up.truncate())
                    .length_squared()
                    < 1e-12)
        {
            return Err(HostInputError::InvalidFaceBasis);
        }
    }
    let mut tables = HostTables {
        instances: [HostInstanceRecord::default(); HOST_INSTANCE_CAPACITY],
        views: [HostViewRecord::default(); HOST_VIEW_CAPACITY],
    };
    tables.instances[..instances.len()].copy_from_slice(instances);
    tables.views[..views.len()].copy_from_slice(views);
    Ok(tables)
}

/// Pack a static material prefix and bounded frame tables into its existing
/// uniform binding. This avoids adding two bindings to constrained pipelines.
/// The shader must declare exactly the matching prefix followed by HostTables.
/// This function never writes GPU state or modifies a material asset.
pub fn pack_host_material_uniform<T: ShaderType + WriteInto>(
    material: &T,
    instances: &[HostInstanceRecord],
    views: &[HostViewRecord],
) -> Result<Vec<u8>, HostInputError> {
    let tables = checked_tables(instances, views)?;
    let mut material_buffer = UniformBuffer::new(Vec::<u8>::new());
    material_buffer
        .write(material)
        .map_err(|e| HostInputError::Encoding(e.to_string()))?;
    let mut bytes = material_buffer.into_inner();
    bytes.resize(bytes.len().div_ceil(16) * 16, 0);
    let mut table_buffer = UniformBuffer::new(Vec::<u8>::new());
    table_buffer
        .write(&tables)
        .map_err(|e| HostInputError::Encoding(e.to_string()))?;
    bytes.extend(table_buffer.into_inner());
    if bytes.len() > PORTABLE_UNIFORM_BYTES {
        return Err(HostInputError::UniformTooLarge);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        character::StylizedCharacterParams, hero::StylizedHeroParams, material::StylizedCelParams,
        outline::StylizedOutlineParams,
    };

    #[test]
    fn current_material_prefixes_fit_without_additional_uniform_bindings() {
        fn fits<T: ShaderType + WriteInto + Default>() {
            let bytes = pack_host_material_uniform(
                &T::default(),
                &[HostInstanceRecord::default()],
                &[HostViewRecord::default()],
            )
            .unwrap();
            assert!(bytes.len() <= PORTABLE_UNIFORM_BYTES);
            assert_eq!(bytes.len() % 16, 0);
        }
        assert_eq!(HostInstanceRecord::min_size().get(), 96);
        assert_eq!(HostViewRecord::min_size().get(), 160);
        assert_eq!(HostTables::min_size().get(), 14_848);
        fits::<StylizedCelParams>();
        fits::<StylizedCharacterParams>();
        fits::<StylizedHeroParams>();
        fits::<StylizedOutlineParams>();
    }

    #[test]
    fn second_view_and_hdr_tint_survive_the_record_boundary() {
        let record = HostInstanceRecord {
            tint: Vec4::new(2.0, 0.5, 0.25, 0.7),
            indices: UVec4::new(7, 1, 9, 0x80),
            ..Default::default()
        };
        let bytes =
            pack_host_material_uniform(&Vec4::ZERO, &[record], &[HostViewRecord::default(); 2])
                .unwrap();
        assert_eq!(f32::from_le_bytes(bytes[16..20].try_into().unwrap()), 2.0);
        assert_eq!(u32::from_le_bytes(bytes[100..104].try_into().unwrap()), 1);
        assert_eq!(
            u32::from_le_bytes(bytes[108..112].try_into().unwrap()),
            0x80
        );
    }

    #[test]
    fn invalid_or_overflowing_input_is_rejected_without_mutation() {
        let original = HostInstanceRecord::default();
        let views = [HostViewRecord::default()];
        assert_eq!(
            checked_tables(&[original; 129], &views).err(),
            Some(HostInputError::Capacity)
        );
        assert_eq!(
            checked_tables(&[original], &[]).err(),
            Some(HostInputError::MissingView)
        );
        let invalid = HostInstanceRecord {
            tint: Vec4::splat(f32::NAN),
            ..original
        };
        assert_eq!(
            checked_tables(&[invalid], &views).err(),
            Some(HostInputError::NonFinite)
        );
        let invalid = HostInstanceRecord {
            face_forward: Vec4::new(0.0, 1.0, 0.0, 1.0),
            ..original
        };
        assert_eq!(
            checked_tables(&[invalid], &views).err(),
            Some(HostInputError::InvalidFaceBasis)
        );
        assert_eq!(original, HostInstanceRecord::default());
    }
}
