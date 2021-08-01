pub fn animation_timer(pulse: f32) -> f32 {
    (pulse * 4.0/* speed factor */).cos() * 0.5 + 0.8 //y=0.5cos(4x)+0.8
}
