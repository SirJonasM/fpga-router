use std::fmt::Display;

use router::{CablePoint, Direction};

use crate::render::SpatialFabricGrid;

use super::*;

#[derive(Copy, Clone)]
pub struct Empty;
#[derive(Copy, Clone)]
pub struct AtTileOuter;
#[derive(Copy, Clone)]
pub struct AtTileInner;
#[derive(Copy, Clone)]
pub struct AtLut;

#[derive(Copy, Clone)]
pub struct LayoutBuilder<State> {
    pub x: f64,
    pub y: f64,
    _marker: std::marker::PhantomData<State>,
}

// Methods available at the very beginning
impl LayoutBuilder<Empty> {
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            _marker: std::marker::PhantomData,
        }
    }

    /// Moves to a tile. Transitions the builder to the `AtTile` state.
    pub fn tile(self, tile_id: &TileId) -> LayoutBuilder<AtTileOuter> {
        let (tx, ty) = get_tile_pos(tile_id);
        LayoutBuilder {
            x: tx,
            y: ty,
            _marker: std::marker::PhantomData,
        }
    }
}

impl Default for LayoutBuilder<Empty> {
    fn default() -> Self {
        Self::new()
    }
}
impl LayoutBuilder<AtTileOuter> {
    /// Moves to a tile. Transitions the builder to the `AtTile` state.
    pub fn tile_inner(self) -> LayoutBuilder<AtTileInner> {
        LayoutBuilder {
            x: self.x + TILE_PADDING / 2.0,
            y: self.y + TILE_PADDING / 2.0,
            _marker: std::marker::PhantomData,
        }
    }
    pub fn tile_middle_point(mut self) -> Self {
        self.x += TILE_WIDTH / 2.0;
        self.y += TILE_HEIGHT / 2.0;
        self
    }
}
pub enum Compass {
    North,
    South,
    East,
    West,
}

impl LayoutBuilder<AtTileInner> {
    pub fn lut(self, lut_id: char) -> LayoutBuilder<AtLut> {
        let (lx, ly) = get_lut_offset(lut_id);

        LayoutBuilder {
            x: self.x + lx,
            y: self.y + ly,
            _marker: std::marker::PhantomData,
        }
    }
    pub fn cable_oriented(mut self, direction: &Direction, orientation: Compass) -> Self {
        let xx = cable_offset(&direction.cable_point);

        let local_x = -2.0 - direction.id as f64 - direction.length as f64 * WIRE_NODE_RADIUS * 2.0;
        let local_y = 10.0 + xx + direction.length as f64;

        let cx = TILE_BOUNDING_BOX_WIDTH / 2.0;
        let cy = TILE_BOUNDING_BOX_HEIGHT / 2.0;

        let (rotated_x, rotated_y) = match orientation {
            Compass::North => (local_x, local_y),
            Compass::East => (cx - (local_y - cy), cy + (local_x - cx)),
            Compass::South => (cx - (local_x - cx), cy - (local_y - cy)),
            Compass::West => (cx + (local_y - cy), cy - (local_x - cx)),
        };

        // 4. Apply the rotated local coordinates back to your absolute position
        self.x += rotated_x;
        self.y += rotated_y;
        self
    }

    pub fn carry_in(mut self) -> Self {
        self.x += TILE_BOUNDING_BOX_WIDTH / 2.0;
        self.y += TILE_BOUNDING_BOX_HEIGHT - 1.0;
        self
    }
    pub fn carry_out(mut self) -> Self {
        self.x += TILE_BOUNDING_BOX_WIDTH / 2.0;
        self.y += 1.0;
        self
    }

    pub fn vdd(mut self) -> Self {
        self.x += 1.0;
        self.y += 1.0;
        self
    }

    pub fn ground(mut self) -> Self {
        self.x += 2.0;
        self.y += 1.0;
        self
    }
}

// Methods available ONLY after you are inside a LUT context
impl LayoutBuilder<AtLut> {
    pub fn lut_middle_point(mut self) -> Self {
        self.x += LUT_WIDTH / 2.0;
        self.y += LUT_HEIGHT / 2.0;
        self
    }
    pub fn input(mut self, pin: u8) -> Self {
        self.y += (LUT_HEIGHT / 2.0) + (pin as f64 - 2.0) * 2.0;
        self
    }

    pub fn output(mut self) -> Self {
        self.x += LUT_WIDTH;
        self.y += LUT_HEIGHT / 2.0;
        self
    }

    pub fn carry_out(mut self) -> Self {
        self.x += LUT_WIDTH / 2.0;
        self
    }

    pub fn carry_in(mut self) -> Self {
        self.x += LUT_WIDTH / 2.0;
        self.y += LUT_HEIGHT;
        self
    }

    pub fn set_reset(mut self) -> Self {
        self.x += 3.2 * LUT_WIDTH / 4.0;
        self.y += LUT_HEIGHT;
        self
    }
    pub fn enable(mut self) -> Self {
        self.x += 3.0 * LUT_WIDTH / 4.0;
        self.y += LUT_HEIGHT;
        self
    }
}

// Global methods available in ANY state except Empty (if you want)
// Or you can restrict `.build()` so it can only be called from specific states!
impl<State> LayoutBuilder<State> {
    pub fn build(self) -> vello::kurbo::Point {
        vello::kurbo::Point::new(self.x, self.y)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub location: TargetLocation,
    pub world_position: vello::kurbo::Point,
    pub mouse_position: egui::Pos2,
}
#[derive(Debug, Clone, PartialEq)]
pub enum TargetLocation {
    /// The coordinates are within the tile's outer padding/cabling region.
    Outer(TileId),
    Inner(TileId, char),
    /// The coordinates land in the inner block, but are touching empty routing space between LUTs.
    InnerEmpty(TileId),
    /// The coordinates land completely outside the layout grid.
    None,
}
impl Display for TargetLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetLocation::Outer(TileId(x, y)) => writeln!(f, "Tile({x} {y}): Outer",),
            TargetLocation::Inner(TileId(x, y), bel) => writeln!(f, "Tile({x} {y}): LUT {}", bel.to_ascii_uppercase()),
            TargetLocation::InnerEmpty(TileId(x, y)) => writeln!(f, "Tile({x} {y}): Inner"),
            TargetLocation::None => writeln!(f, "Empty space."),
        }
    }
}

pub fn find_location_at_world_pos(x: f64, y: f64, spatial_grid: &SpatialFabricGrid) -> TargetLocation {
    if x < 0.0 || y < 0.0 {
        return TargetLocation::None;
    }

    let tile_x = (x / TILE_WIDTH).floor() as u8;
    let tile_y = (y / TILE_HEIGHT).floor() as u8;
    let tile_id = TileId(tile_x, tile_y);
    let Some(bucket) = spatial_grid.buckets.get(&tile_id) else {
        return TargetLocation::None;
    };
    let tile_pos = bucket.tile_data;
    let tile_x_offset = x - tile_pos.x;
    let tile_y_offset = y - tile_pos.y;
    let is_in_inner_box = TILE_INNER_RANGE_X.contains(&tile_x_offset) && TILE_INNER_RANGE_Y.contains(&tile_y_offset);
    if !is_in_inner_box {
        return TargetLocation::Outer(tile_id);
    }
    if let Some(bel) = bucket.lut_data.iter().find_map(|(bel, pos)| {
        let lut_x_offset = x - pos.x;
        let lut_y_offset = y - pos.y;
        if (0.0..=LUT_WIDTH).contains(&lut_x_offset) & (0.0..=LUT_HEIGHT).contains(&lut_y_offset) {
            Some(bel)
        } else {
            None
        }
    }) {
        return TargetLocation::Inner(tile_id, *bel);
    }
    TargetLocation::InnerEmpty(tile_id)
}
fn cable_offset(x: &CablePoint) -> f64 {
    match x {
        router::CablePoint::Begin => 0.0,
        router::CablePoint::BeginB => 20.0,
        router::CablePoint::Mid => 40.0,
        router::CablePoint::End => 60.0,
    }
}
