use router::TileId;
use vello::kurbo::Point;

use crate::constants::*;
pub type LutId = usize;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lut {
    pub id: LutId,
}
impl Lut {
    pub fn mid_point(&self) -> Point {
        let x = self.position.x + LUT_WIDTH * 0.5;
        let y = self.position.y + LUT_HEIGHT * 0.5;
        Point::new(x, y)
    }
}
