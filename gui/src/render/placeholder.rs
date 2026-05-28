use vello::{
    Scene,
    kurbo::{Affine, Rect},
    peniko::{Color, Fill},
};

pub fn render_placeholder_vello(scene: &mut Scene, viewport: &egui::Rect) {
    let offset_x = viewport.min.x as f64;
    let offset_y = viewport.min.y as f64;

    let rect = Rect::new(offset_x + 500.0, offset_y + 500.0, offset_x + 750.0, offset_y + 750.0);

    scene.fill(Fill::NonZero, Affine::IDENTITY, Color::rgb8(255, 255, 255), None, &rect);

    let circle = vello::kurbo::Circle::new((offset_x + 450.0, offset_y + 120.0), 70.0);

    scene.fill(Fill::NonZero, Affine::IDENTITY, Color::rgb8(38, 139, 210), None, &circle);

    let mut triangle = vello::kurbo::BezPath::new();

    triangle.move_to((offset_x + 200.0, offset_y + 260.0));
    triangle.line_to((offset_x + 450.0, offset_y + 500.0));
    triangle.line_to((offset_x + 50.0, offset_y + 500.0));
    triangle.close_path();

    scene.fill(Fill::NonZero, Affine::IDENTITY, Color::rgb8(133, 153, 0), None, &triangle);
}
