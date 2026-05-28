use router::TileId;
use vello::kurbo::Point;

use crate::constants::*;
#[derive(Debug, Clone, Copy)]
pub struct Lut {
    pub position: Point,
    pub bel_index: char,
    pub tile: TileId,
}

pub fn get_lut_offset(bel_index: char) -> (f64, f64) {
    let lowercase_char = bel_index.to_ascii_lowercase();
    let i = (lowercase_char as usize).saturating_sub('a' as usize);
    let row = i / LUTS_PER_ROW;
    let col = i % LUTS_PER_ROW;

    let lx = LUT_MARGIN + (col as f64 * (LUT_WIDTH + LUT_SPACING));
    let ly = LUT_MARGIN + (row as f64 * (LUT_HEIGHT + LUT_SPACING));
    (lx, ly)
}
