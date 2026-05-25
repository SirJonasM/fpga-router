pub const LUT_ZOOM_THRESHOLD: f64 = 0.4;
pub const NODE_ZOOM_THRESHOLD: f64 = 1.8; 
pub const EDGE_ZOOM_THRESHOLD: f64 = 2.5; 

pub const TILE_WIDTH: f64 = 110.0;
pub const TILE_HEIGHT: f64 = 100.0;
pub const TILE_PADDING: f64 = 20.0;
pub const TILE_LINE_WIDTH: f64 = 0.75;

pub const LUT_WIDTH: f64 = 20.0;
pub const LUT_HEIGHT: f64 = 15.0;
pub const LUT_MARGIN: f64 = 10.0;
pub const LUT_SPACING: f64 = 9.0;
pub const LUT_LINE_WIDTH: f64 = 0.5;

pub const WIRE_LINE_WIDTH: f64 = 0.005;

pub const LUTS_PER_ROW: usize = ((TILE_WIDTH - (2.0 * LUT_MARGIN)) / (LUT_WIDTH + LUT_SPACING))
    .floor()
    .max(1.0) as usize;
