#[derive(Clone, Copy)]
struct OptionalAttribute {
    attribute: bevy::mesh::MeshVertexAttribute,
    shader_location: u32,
    shader_def: &'static str,
}

fn append_skin_attributes(
    descriptor: &mut RenderPipelineDescriptor,
    layout: &MeshVertexBufferLayoutRef,
) -> Result<(), SpecializedMeshPipelineError> {
    if !layout.0.contains(ATTRIBUTE_CEL_JOINTS_1)
        && !layout.0.contains(ATTRIBUTE_CEL_WEIGHTS_1) {
        return Ok(());
    }
    // Reject half of a skin set; never render with silently missing influences.
    layout.0.get_layout(&[
        ATTRIBUTE_CEL_JOINTS_1.at_shader_location(8),
        ATTRIBUTE_CEL_WEIGHTS_1.at_shader_location(9),
    ])?;
    append_optional_attribute(descriptor, layout,
        OptionalAttribute::new(ATTRIBUTE_CEL_JOINTS_1, 8, "CEL_SKIN_EXTENDED"));
    append_optional_attribute(descriptor, layout,
        OptionalAttribute::new(ATTRIBUTE_CEL_WEIGHTS_1, 9, "CEL_SKIN_WEIGHTS_EXTENDED"));
    Ok(())
}

impl OptionalAttribute {
    const fn new(
        attribute: bevy::mesh::MeshVertexAttribute,
        shader_location: u32,
        shader_def: &'static str,
    ) -> Self {
        Self {
            attribute,
            shader_location,
            shader_def,
        }
    }
}

fn append_optional_attribute(
    descriptor: &mut RenderPipelineDescriptor,
    layout: &MeshVertexBufferLayoutRef,
    optional: OptionalAttribute,
) {
    if !layout.0.contains(optional.attribute) {
        return;
    }
    let attribute_id = optional.attribute.into();
    if let Some(attribute_index) = layout
        .0
        .attribute_ids()
        .iter()
        .position(|id| *id == attribute_id)
        && let Some(vertex_buffer) = descriptor.vertex.buffers.first_mut()
    {
        let mut layout_attribute = layout.0.layout().attributes[attribute_index];
        layout_attribute.shader_location = optional.shader_location;
        vertex_buffer.attributes.push(layout_attribute);
    }
    descriptor
        .vertex
        .shader_defs
        .push(optional.shader_def.into());
    if let Some(fragment) = descriptor.fragment.as_mut() {
        fragment.shader_defs.push(optional.shader_def.into());
    }
}
