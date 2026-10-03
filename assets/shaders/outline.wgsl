// Draws a pulsing glow around the opaque pixels of a sprite. The outline mesh is
// placed behind the sprite and is larger than it by the outline width on every side.

#import bevy_sprite::{
    mesh2d_vertex_output::VertexOutput,
    mesh2d_view_bindings::globals,
}

struct Outline {
    color: vec4<f32>,
    // Region of the texture shown by the sprite (min.xy, max.xy) in UV coordinates.
    uv_rect: vec4<f32>,
    // x: outline width in pixels, y: 1 if the sprite is flipped horizontally,
    // zw: sprite size in pixels.
    params: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> outline: Outline;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var sprite_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var sprite_sampler: sampler;

// Coverage of the sprite at the given position in sprite pixels. Translucent
// pixels count as covered, so translucent sprites (e.g. light) get a full outline.
fn alpha_at(position: vec2<f32>) -> f32 {
    let size = outline.params.zw;
    if position.x < 0.0 || position.y < 0.0 || position.x > size.x || position.y > size.y {
        return 0.0;
    }
    var uv = position / size;
    if outline.params.y > 0.5 {
        uv.x = 1.0 - uv.x;
    }
    let alpha = textureSampleLevel(
        sprite_texture,
        sprite_sampler,
        mix(outline.uv_rect.xy, outline.uv_rect.zw, uv),
        0.0,
    ).a;
    return smoothstep(0.02, 0.15, alpha);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let width = outline.params.x;
    let position = mesh.uv * (outline.params.zw + 2.0 * width) - width;
    if alpha_at(position) > 0.5 {
        // The sprite itself covers this pixel.
        return vec4(0.0);
    }
    var glow = 0.0;
    for (var i = 0; i < 16; i++) {
        let angle = f32(i) * 6.2831853 / 16.0;
        let direction = vec2(cos(angle), sin(angle));
        glow = max(glow, alpha_at(position + direction * width * 0.4));
        glow = max(glow, 0.8 * alpha_at(position + direction * width * 0.7));
        glow = max(glow, 0.4 * alpha_at(position + direction * width));
    }
    let pulse = 0.6 + 0.4 * sin(globals.time * 5.0);
    return vec4(outline.color.rgb, outline.color.a * glow * pulse);
}
