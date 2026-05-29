use router::TileId;
use vello::kurbo::Point;

use crate::constants::*;

#[derive(PartialEq, Clone, Copy)]
pub struct VisibleTileRange {
    pub min_x: u8,
    pub max_x: u8,
    pub min_y: u8,
    pub max_y: u8,
}

impl VisibleTileRange {
    pub fn new(world_min: Point, world_max: Point) -> Self {
        let tile_step_x = TILE_WIDTH;
        let tile_step_y = TILE_HEIGHT;

        VisibleTileRange {
            min_x: (world_min.x / tile_step_x).floor() as u8,
            max_x: (world_max.x / tile_step_x).ceil() as u8,
            min_y: (world_min.y / tile_step_y).floor() as u8,
            max_y: (world_max.y / tile_step_y).ceil() as u8,
        }
    }
    pub fn is_visibile(&self, tile_id: TileId) -> bool {
        (self.min_x..=self.max_x).contains(&tile_id.0) && (self.min_y..=self.max_y).contains(&tile_id.1)
    }
}

pub struct VisibleTileRangeIter {
    range: VisibleTileRange,
    current_x: u8,
    current_y: u8,
}

impl IntoIterator for VisibleTileRange {
    type Item = TileId;
    type IntoIter = VisibleTileRangeIter;

    fn into_iter(self) -> Self::IntoIter {
        VisibleTileRangeIter {
            range: self,
            current_x: self.min_x,
            current_y: self.min_y,
        }
    }
}

impl Iterator for VisibleTileRangeIter {
    type Item = TileId;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_y > self.range.max_y || self.range.min_x > self.range.max_x {
            return None;
        }

        let tile_id = TileId(self.current_x, self.current_y);

        self.current_x += 1;
        if self.current_x > self.range.max_x {
            self.current_x = self.range.min_x;
            self.current_y += 1;
        }

        Some(tile_id)
    }
}
