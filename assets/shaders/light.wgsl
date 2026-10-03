// Darkness with a flickering, warm circle of candle light. The circle is
// squashed vertically to lie flat on the isometric floor.

#import bevy_sprite::{
    mesh2d_vertex_output::VertexOutput,
    mesh2d_view_bindings::globals,
}

struct Light {
    // xy: center of the light in world coordinates, z: radius in pixels,
    // w: darkness outside of the light (0..1).
    params: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> light: Light;

fn flicker(t: f32) -> f32 {
    // Several sine waves with unrelated frequencies look like an unsteady flame.
    return 0.5 * sin(t * 7.3) + 0.3 * sin(t * 13.1 + 1.7) + 0.2 * sin(t * 23.9 + 4.1);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let t = globals.time;
    let center = light.params.xy;
    let radius = light.params.z * (1.0 + 0.05 * flicker(t));
    let darkness = light.params.w;

    let offset = mesh.world_position.xy - center;
    let distance = length(vec2(offset.x, offset.y * 2.0)) / max(radius, 1.0);

    // Bright core, soft falloff towards the edge of the light.
    let lit = 1.0 - smoothstep(0.3, 1.0, distance);
    let brightness = lit * (0.93 + 0.07 * flicker(t * 1.3 + 2.0));
    let shadow = darkness * (1.0 - brightness);
    // A slight warm tint of the candle light, strongest close to the flame.
    let tint = darkness * 0.14 * lit * lit;
    let alpha = shadow + tint;
    let color = vec3(1.0, 0.6, 0.25) * tint / max(alpha, 0.001);
    return vec4(color, alpha);
}
