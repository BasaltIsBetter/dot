struct Shaded {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) offset: vec2<f32>,
    @location(2) ring: vec2<f32>,
}

@vertex
fn vertex_main(
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) offset: vec2<f32>,
    @location(3) ring: vec2<f32>,
) -> Shaded {
    var shaded: Shaded;
    shaded.position = vec4<f32>(position, 0.0, 1.0);
    shaded.color = color;
    shaded.offset = offset;
    shaded.ring = ring;
    return shaded;
}

@fragment
fn fragment_main(shaded: Shaded) -> @location(0) vec4<f32> {
    let radius = shaded.ring.x;
    if radius <= 0.0 {
        return shaded.color;
    }
    let half_thickness = shaded.ring.y;
    let distance = abs(length(shaded.offset) - radius);
    let coverage = clamp(half_thickness + 0.5 - distance, 0.0, 1.0);
    return vec4<f32>(shaded.color.rgb, shaded.color.a * coverage);
}
