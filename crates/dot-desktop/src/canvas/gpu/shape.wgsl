struct Shaded {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vertex_main(
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
) -> Shaded {
    var shaded: Shaded;
    shaded.position = vec4<f32>(position, 0.0, 1.0);
    shaded.color = color;
    return shaded;
}

@fragment
fn fragment_main(shaded: Shaded) -> @location(0) vec4<f32> {
    return shaded.color;
}
