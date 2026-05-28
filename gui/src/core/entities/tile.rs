use router::TileId;
use vello::kurbo::Point;

use crate::constants::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tile {
    pub id: TileId,
    pub position_outer: Point,
    pub position_inner: Point,
}
impl Tile {
    pub fn mid_point(&self) -> Point {
        let x = self.position_outer.x + TILE_WIDTH * 0.5;
        let y = self.position_outer.y + TILE_HEIGHT * 0.5;
        Point::new(x, y)
    }
}

pub fn get_tile_pos(tile: &TileId) -> (f64, f64) {
    (tile.0 as f64 * (TILE_WIDTH), tile.1 as f64 * (TILE_HEIGHT))
}
