use std::ops::RangeInclusive;

use vello::peniko::Color;

pub const LUT_ZOOM_THRESHOLD: f64 = 0.4;
pub const NODE_ZOOM_THRESHOLD_UNDER_MOVING: f64 = 2.0;
pub const EDGE_ZOOM_THRESHOLD_UNDER_MOVING: f64 = 5.0;
pub const NODE_ZOOM_THRESHOLD: f64 = 25.0;
pub const EDGE_ZOOM_THRESHOLD: f64 = 25.0;

pub const TILE_BOUNDING_BOX_WIDTH: f64 = 110.0;
pub const TILE_BOUNDING_BOX_HEIGHT: f64 = 100.0;
pub const TILE_INNER_LINE_WIDTH: f64 = 0.75;
pub const TILE_OUTER_LINE_WIDTH: f64 = 0.1;
pub const TILE_PADDING: f64 = 40.0;
pub const TILE_WIDTH: f64 = TILE_BOUNDING_BOX_WIDTH + TILE_PADDING;
pub const TILE_HEIGHT: f64 = TILE_BOUNDING_BOX_HEIGHT + TILE_PADDING;
pub const TILE_INNER_RANGE_X: RangeInclusive<f64> = (TILE_PADDING / 2.0)..=(TILE_WIDTH - TILE_PADDING / 2.0);
pub const TILE_INNER_RANGE_Y: RangeInclusive<f64> = (TILE_PADDING / 2.0)..=(TILE_HEIGHT - TILE_PADDING / 2.0);
pub const TILE_OUTER_SELECT_THRESHOLD: f64 = TILE_OUTER_LINE_WIDTH * 2.0;
pub const TILE_INNER_SELECT_THRESHOLD: f64 = TILE_INNER_LINE_WIDTH * 2.0;
pub const TILE_FOCUS_SCALE: f64 = 4.0;

pub const LUT_WIDTH: f64 = 20.0;
pub const LUT_HEIGHT: f64 = 15.0;
pub const LUT_LINE_WIDTH: f64 = 0.5;
pub const LUT_MARGIN: f64 = 10.0;
pub const LUT_SPACING: f64 = 9.0;
pub const LUT_SELECT_THRESHOLD: f64 = LUT_LINE_WIDTH * 2.0;
pub const LUT_FOCUS_SCALE: f64 = 20.0;

pub const WIRE_LINE_WIDTH: f64 = 0.005;
pub const WIRE_NODE_RADIUS: f64 = 0.1;
pub const EDGE_FOCUS_SCALER: f64 = 1000.0;

pub const SELECTED_WIRE_LINE_WIDTH: f64 = WIRE_LINE_WIDTH * 4.0;

pub const NODE_FOCUS_SCALE: f64 = 90.0;

pub const LUTS_PER_ROW: usize = ((TILE_BOUNDING_BOX_WIDTH - (2.0 * LUT_MARGIN)) / (LUT_WIDTH + LUT_SPACING))
    .floor()
    .max(1.0) as usize;

pub const MIN_ZOOM: f64 = 0.05;
pub const MAX_ZOOM: f64 = 250.0;

pub const COLOR_EDGE_START: Color = Color::GREEN;
pub const COLOR_EDGE_END: Color = Color::BLUE;
