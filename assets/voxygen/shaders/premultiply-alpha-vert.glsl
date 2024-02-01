#version 440 core

layout(push_constant) uniform Params {
    // Size of the source image.
    uint source_size_xy;
    // Offset to place the image at in the target texture.
    //
    // Origin is the top-left.
    uint target_offset_xy;
    // Size of the target texture.
    uint target_size_xy;
};

layout(location = 0) out vec2 source_coords;

vec2 unpack(uint xy) {
    return vec2(xy & 0xFFFF, (xy >> 16) & 0xFFFF);
}

void main() {
    vec2 source_size = unpack(source_size_xy);
    vec2 target_offset = unpack(target_offset_xy);
    vec2 target_size = unpack(target_size_xy);

    // Generate rectangle (counter clockwise triangles)
    //
    // 0 0 1 1 1 0
    float x_select = float(((uint(gl_VertexIndex) + 1u) / 3u) % 2u);
    // 1 0 0 0 1 1
    float y_select = float(((uint(gl_VertexIndex) + 5u) / 3u) % 2u);

    source_coords = vec2(
        // left -> right (on screen)
        mix(0.0, 1.0, x_select),
        // bottom -> top (on screen)
        mix(1.0, 0.0, y_select)
    ) * source_size;

    vec2 target_coords_normalized = (target_offset + source_coords) / target_size;

    // Flip y and transform [0.0, 1.0] -> [-1.0, 1.0] to get NDC coordinates.
    vec2 v_pos = ((target_coords_normalized * 2.0) - vec2(1.0)) * vec2(1.0, -1.0); 

    gl_Position = vec4(v_pos, 0.0, 1.0);
}
