use vek::*;

pub struct Plot {
    kind: PlotKind,
    center_tpos: Vec2<i32>,
    units: Vec2<Vec2<i8>>,
}

pub enum PlotKind {
    Path,
    House { height: i32 },
}
