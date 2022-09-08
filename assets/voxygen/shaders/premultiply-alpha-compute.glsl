#version 420 core

// TODO: should we modify this based on the current device?
// TODO: would it be better to have 2D workgroup for writing to a local area in the target image? 
layout(local_size_x = 256) in;

// TODO: writing all images into a single buffer?
layout(set = 0, binding = 0) readonly buffer InputImage {
    uint input_pixels[];
};

layout (std140, set = 0, binding = 1)
uniform u_locals {
    // Size of the input image.
    uvec2 image_size;
    // Offset to place the transformed input image at in the target
    // image.
    uvec2 target_offset;
};

layout(rgba8, set = 0, binding = 2) uniform writeonly image2D target_image;

void main() {
    uint global_id = gl_GlobalInvocationId.x;
    uvec2 src_pixel_pos = uvec2(global_id % image_size.x, global_id / image_size.x);
    // Otherwise this is is an out of bounds compute instance.
    if (src_pixel_pos < image_size.y) {
        uint pixel = input_pixels[global_id]; 
        vec4 nonlinear = vec4((pixel >> 16) & 0xFFu, (pixel >> 8) & 0xFFu, (pixel >> 8) & 0xFFu, pixel & 0xFFu);
        vec4 linear;
        vec4 premultiplied_linear;
        vec4 premultiplied_nonlinear;
        // No free srgb with image store operations https://www.khronos.org/opengl/wiki/Image_Load_Store#Format_compatibility
        imageStore(target_image, src_pixel_pos + target_offset, premultiplied_nonlinear);
    }
}
