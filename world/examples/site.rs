use svg_fmt::*;
use veloren_world::site2::test_site;

fn main() {
    let site = test_site();
    let size = site.bounds().size();
    println!("{}", BeginSvg {
        w: size.w as f32,
        h: size.h as f32
    });

    for plot in site.plots() {
        let bounds = plot.find_bounds();
        println!("{}", Rectangle {
            x: bounds.min.x as f32,
            y: bounds.min.y as f32,
            w: bounds.size().w as f32,
            h: bounds.size().h as f32,
            style: Style {
                fill: Fill::Color(Color {
                    r: 50,
                    g: 50,
                    b: 50
                }),
                stroke: Stroke::Color(Color { r: 0, g: 0, b: 0 }, 1.0),
                opacity: 1.0,
                stroke_opacity: 1.0,
            },
            border_radius: 0.0,
        });
    }

    println!("{}", EndSvg);
}
