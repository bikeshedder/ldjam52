// A dark vignette around the candle light. Inside the light the scene is shown
// unchanged, towards the edge it fades to darkness. The light is squashed
// vertically to lie flat on the isometric floor and flickers like a flame.

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
    let radius = light.params.z * (1.0 + 0.04 * flicker(globals.time));
    let offset = mesh.world_position.xy - light.params.xy;
    let distance = length(vec2(offset.x, offset.y * 2.0)) / max(radius, 1.0);
    // Fully lit in the center, smoothly darker towards the edge.
    let shadow = smoothstep(0.25, 1.0, distance);
    return vec4(0.0, 0.0, 0.0, light.params.w * shadow);
}
