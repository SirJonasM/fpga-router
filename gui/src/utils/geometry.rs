use crate::app::ViewTransform;

/// Calculates the shortest distance squared from point `p` to line segment `a_to_b`.
pub fn distance_to_segment(point: vello::kurbo::Point, segment_a: vello::kurbo::Point, segment_b: vello::kurbo::Point) -> f64 {
    let ab = vello::kurbo::Vec2::new(segment_b.x - segment_a.x, segment_b.y - segment_a.y);
    let ap = vello::kurbo::Vec2::new(point.x - segment_a.x, point.y - segment_a.y);

    let ab_len_sq = ab.dot(ab);
    if ab_len_sq == 0.0 {
        return ap.dot(ap);
    }

    let t = (ap.dot(ab) / ab_len_sq).clamp(0.0, 1.0);

    let closest_point = vello::kurbo::Point::new(segment_a.x + t * ab.x, segment_a.y + t * ab.y);

    let dx = point.x - closest_point.x;
    let dy = point.y - closest_point.y;
    dx * dx + dy * dy
}

pub fn is_on_rect_border(
    target_point: vello::kurbo::Point,
    rect_point: vello::kurbo::Point,
    width: f64,
    height: f64,
    threshold: f64,
) -> bool {
    let min_x = rect_point.x;
    let max_x = rect_point.x + width;
    let min_y = rect_point.y;
    let max_y = rect_point.y + height;

    if target_point.x < (min_x - threshold)
        || target_point.x > (max_x + threshold)
        || target_point.y < (min_y - threshold)
        || target_point.y > (max_y + threshold)
    {
        return false;
    }

    let near_left = (target_point.x - min_x).abs() <= threshold;
    let near_right = (target_point.x - max_x).abs() <= threshold;
    let near_top = (target_point.y - min_y).abs() <= threshold;
    let near_bottom = (target_point.y - max_y).abs() <= threshold;

    near_left || near_right || near_top || near_bottom
}
/// Translates a screen-space pixel position (e.g., from egui mouse inputs)
/// into the underlying world-space coordinates of your FPGA fabric.
pub fn screen_to_world(
    screen_pos: egui::Pos2,
    viewport: &egui::Rect,
    view_transform: &ViewTransform,
    pixels_per_point: f64,
) -> vello::kurbo::Point {
    let sx = screen_pos.x as f64 * pixels_per_point;
    let sy = screen_pos.y as f64 * pixels_per_point;

    let vx = viewport.min.x as f64 * pixels_per_point;
    let vy = viewport.min.y as f64 * pixels_per_point;

    vello::kurbo::Point::new(
        (sx - vx - view_transform.pan.x) / view_transform.scale,
        (sy - vy - view_transform.pan.y) / view_transform.scale,
    )
}
