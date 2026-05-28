use vello::kurbo::Point;

use crate::constants::*;

pub struct VisibleTileRange {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

impl VisibleTileRange {
    pub fn new(world_min: Point, world_max: Point) -> Self {
        let tile_step_x = TILE_WIDTH;
        let tile_step_y = TILE_HEIGHT;

        VisibleTileRange {
            min_x: (world_min.x / tile_step_x).floor() as i32,
            max_x: (world_max.x / tile_step_x).ceil() as i32,
            min_y: (world_min.y / tile_step_y).floor() as i32,
            max_y: (world_max.y / tile_step_y).ceil() as i32,
        }
    }
}
