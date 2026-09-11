use crate::ValidationCode;

impl ValidationCode {
    pub(super) const fn message(self) -> &'static str {
        match self {
            Self::CliInputMissing => "no input asset was supplied",
            Self::CliFormatUnsupported => "the requested output format is unsupported",
            Self::CliArgumentUnknown => "the command line contains an unknown option",
            Self::InputExtensionUnsupported => "the input suffix is not .glb, .gltf, or .cel.json",
            Self::InputUnreadable => "the input file could not be read",
            Self::GlbHeaderTruncated => "the GLB header is truncated",
            Self::GlbMagicInvalid => "the GLB magic bytes are invalid",
            Self::GlbVersionUnsupported => "the GLB container version is unsupported",
            Self::GlbLengthInvalid => "the GLB declared length does not match its file length",
            Self::GlbChunkHeaderTruncated => "a GLB chunk header is truncated",
            Self::GlbChunkLengthInvalid => "a GLB chunk extends beyond the container",
            Self::GlbChunkAlignmentInvalid => "a GLB chunk length is not four-byte aligned",
            Self::GlbJsonMissing => "the GLB has no JSON chunk",
            Self::GlbJsonDuplicate => "the GLB has more than one JSON chunk",
            Self::GlbBinaryDuplicate => "the GLB has more than one binary chunk",
            Self::JsonSyntaxInvalid => "the JSON document cannot be parsed",
            Self::JsonShapeInvalid => "the JSON document has an invalid shape",
            Self::GltfVersionUnsupported => "the glTF asset version must be 2.0",
            Self::FileReferenceMissing => "a referenced file does not exist",
            Self::RemoteReferenceUnverified => "a remote URI cannot be verified locally",
            Self::FileReferenceMalformed => "a file reference has invalid syntax",
            Self::BinaryChunkMissing => "an embedded buffer has no GLB binary chunk",
            Self::CelAttributeUnknown => "the CEL vertex semantic is not registered",
            Self::CelAttributeAccessorMalformed => "the CEL semantic has no accessor index",
            Self::CelAttributeAccessorUnknown => "the CEL semantic references an unknown accessor",
            Self::CelAttributeFormatInvalid => "the CEL accessor differs from the canonical format",
            Self::BoneReferenceMalformed => "the bone reference syntax is invalid",
            Self::BoneTargetUnknown => "the bone target does not exist",
            Self::PrefabReferenceMalformed => "the prefab reference syntax is invalid",
            Self::PrefabTargetUnknown => "the prefab target does not exist",
            Self::ColliderReferenceMalformed => "the collider reference syntax is invalid",
            Self::ColliderTargetUnknown => "the collider target does not exist",
            Self::MetadataJsonInvalid => "the CEL metadata document has an invalid shape",
            Self::MetadataVersionUnsupported => "the CEL metadata schema version is unsupported",
            Self::MetadataNodeUnknown => "the CEL metadata node target does not exist",
            Self::ComponentTypeUnknown => "the reflected component type is not registered",
            Self::ComponentRegistrationInvalid => "the reflected type is not a component",
            Self::ComponentValueInvalid => "the reflected component value has an invalid shape",
        }
    }

    pub(super) const fn remediation(self) -> &'static str {
        match self {
            Self::CliInputMissing | Self::CliArgumentUnknown => "run --help and supply asset paths",
            Self::CliFormatUnsupported => "use --format human or --format json",
            Self::InputExtensionUnsupported => "supply a .glb, .gltf, or .cel.json file",
            Self::InputUnreadable => "verify the path and read permissions",
            Self::GlbHeaderTruncated
            | Self::GlbMagicInvalid
            | Self::GlbVersionUnsupported
            | Self::GlbLengthInvalid
            | Self::GlbChunkHeaderTruncated
            | Self::GlbChunkLengthInvalid
            | Self::GlbChunkAlignmentInvalid
            | Self::GlbJsonMissing
            | Self::GlbJsonDuplicate
            | Self::GlbBinaryDuplicate => "re-export the asset as a valid glTF 2.0 binary",
            Self::JsonSyntaxInvalid | Self::JsonShapeInvalid => "repair the glTF JSON shape",
            Self::GltfVersionUnsupported => "export the asset as glTF 2.0",
            Self::FileReferenceMissing | Self::PrefabTargetUnknown => {
                "publish or correct the relative file path"
            }
            Self::RemoteReferenceUnverified => "verify the URI in the publishing environment",
            Self::FileReferenceMalformed | Self::PrefabReferenceMalformed => {
                "use a relative path or data URI"
            }
            Self::BinaryChunkMissing => "embed a BIN chunk or provide a URI",
            Self::CelAttributeUnknown => "use a semantic from CEL_VERTEX_ATTRIBUTES",
            Self::CelAttributeAccessorMalformed | Self::CelAttributeAccessorUnknown => {
                "reference an existing accessor index"
            }
            Self::CelAttributeFormatInvalid => "use the registry component type and arity",
            Self::BoneReferenceMalformed | Self::ColliderReferenceMalformed => {
                "use schema version 1 reference syntax"
            }
            Self::BoneTargetUnknown | Self::ColliderTargetUnknown | Self::MetadataNodeUnknown => {
                "use an exact glTF node name"
            }
            Self::MetadataJsonInvalid => "repair the CEL schema at the reported path",
            Self::MetadataVersionUnsupported => "export CEL metadata with schema_version 1",
            Self::ComponentTypeUnknown => "register the full reflected type path",
            Self::ComponentRegistrationInvalid => "register ReflectComponent for the type",
            Self::ComponentValueInvalid => "match the registered component schema",
        }
    }
}
