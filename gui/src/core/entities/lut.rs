use router::TileId;
use vello::kurbo::Point;

use crate::constants::*;
pub type LutId = usize;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lut {
    pub id: LutId,
    pub position: Point,
    pub bel_index: char,
    pub tile: TileId,
}
impl Lut {
    pub fn mid_point(&self) -> Point {
        let x = self.position.x + LUT_WIDTH * 0.5;
        let y = self.position.y + LUT_HEIGHT * 0.5;
        Point::new(x, y)
    }
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
