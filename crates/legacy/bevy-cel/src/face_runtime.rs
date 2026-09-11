use std::{error::Error, fmt};

use bevy::{
    asset::Assets,
    ecs::{component::Component, entity::Entity},
    prelude::{Handle, Reflect},
};

use crate::{CharacterMaterial, FaceBasis};

/// Failure while allocating a character-unique face material.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaceSdfInstanceError {
    /// The requested source handle has no material asset.
    MissingSourceMaterial,
}

impl fmt::Display for FaceSdfInstanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSourceMaterial => {
                formatter.write_str("face source material is unavailable")
            }
        }
    }
}

impl Error for FaceSdfInstanceError {}

/// Head entity and local face basis used to derive one world-space basis.
#[derive(Clone, Copy, Debug, Reflect)]
pub struct FaceSdfRig {
    head: Entity,
    local_basis: FaceBasis,
}

impl FaceSdfRig {
    /// Creates a head-bone binding from a validated local basis.
    #[must_use]
    pub const fn new(head: Entity, local_basis: FaceBasis) -> Self {
        Self { head, local_basis }
    }
}

/// Runtime owner of a material cloned uniquely for one character face.
#[derive(Clone, Debug, Component, Reflect)]
pub struct FaceSdfMaterialInstance {
    head: Entity,
    local_basis: FaceBasis,
    material: Handle<CharacterMaterial>,
}

impl FaceSdfMaterialInstance {
    /// Clones the source material and resets its world basis before first propagation.
    ///
    /// # Errors
    /// Returns [`FaceSdfInstanceError::MissingSourceMaterial`] when the source is absent.
    pub fn try_new(
        rig: FaceSdfRig,
        source: &Handle<CharacterMaterial>,
        materials: &mut Assets<CharacterMaterial>,
    ) -> Result<Self, FaceSdfInstanceError> {
        let mut material = materials
            .get(source)
            .cloned()
            .ok_or(FaceSdfInstanceError::MissingSourceMaterial)?;
        let settings = *material.extension.face_sdf_settings();
        material
            .extension
            .set_face_sdf_settings(settings.with_basis(None));
        Ok(Self {
            head: rig.head,
            local_basis: rig.local_basis,
            material: materials.add(material),
        })
    }

    #[must_use]
    /// Returns the configured head-bone entity.
    pub const fn head(&self) -> Entity {
        self.head
    }

    #[must_use]
    /// Returns the validated head-local basis.
    pub const fn local_basis(&self) -> FaceBasis {
        self.local_basis
    }

    #[must_use]
    /// Returns the character-unique material handle that must be assigned to the mesh.
    pub const fn material(&self) -> &Handle<CharacterMaterial> {
        &self.material
    }
}
