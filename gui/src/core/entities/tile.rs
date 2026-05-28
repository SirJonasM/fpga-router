use router::TileId;
use vello::kurbo::Point;

use crate::constants::*;

#[derive(Debug, Clone, Copy)]
pub struct Tile {
    pub id: TileId,
    pub position: Point,
}
pub fn get_tile_pos(tile: &TileId) -> (f64, f64) {
    (tile.0 as f64 * (TILE_WIDTH), tile.1 as f64 * (TILE_HEIGHT))
}
