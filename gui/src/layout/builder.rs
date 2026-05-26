use router::{CablePoint, Direction};

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

            Compass::East => {
                (cx - (local_y - cy), cy + (local_x - cx))
            }

            Compass::South => {
                (cx - (local_x - cx), cy - (local_y - cy))
            }

            Compass::West => {
                (cx + (local_y - cy), cy - (local_x - cx))
            }
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
pub enum TargetLocation {
    /// The coordinates are within the tile's outer padding/cabling region.
    Outer(TileId),
    /// The coordinates land cleanly inside a specific LUT within the logic block.
    Inner(TileId, char),
    /// The coordinates land in the inner block, but are touching empty routing space between LUTs.
    InnerEmpty(TileId),
    /// The coordinates land completely outside the layout grid.
    None,
}

pub fn find_location_at_world_pos(x: f64, y: f64) -> TargetLocation {
    // 1. Grid tracking: Tiles are placed edge-to-edge using TILE_WIDTH/HEIGHT
    if x < 0.0 || y < 0.0 {
        return TargetLocation::None;
    }

    let tile_x = (x / TILE_WIDTH).floor() as u8;
    let tile_y = (y / TILE_HEIGHT).floor() as u8;
    let tile_id = TileId(tile_x, tile_y);

    // 2. Localize coordinates relative to this specific tile's top-left origin
    let (tile_origin_x, tile_origin_y) = get_tile_pos(&tile_id);
    let local_x = x - tile_origin_x;
    let local_y = y - tile_origin_y;

    // Safety check against the hard tile boundaries
    if local_x > TILE_WIDTH || local_y > TILE_HEIGHT {
        return TargetLocation::None;
    }

    // 3. Define the Inner Box threshold boundaries (matching your tile_inner() layout code)
    let inner_start_x = TILE_PADDING / 2.0;
    let inner_start_y = TILE_PADDING / 2.0;
    let inner_end_x = inner_start_x + TILE_BOUNDING_BOX_WIDTH;
    let inner_end_y = inner_start_y + TILE_BOUNDING_BOX_HEIGHT;

    let is_inner = local_x >= inner_start_x && local_x <= inner_end_x && local_y >= inner_start_y && local_y <= inner_end_y;

    if !is_inner {
        return TargetLocation::Outer(tile_id);
    }

    let logic_x = local_x - inner_start_x;
    let logic_y = local_y - inner_start_y;

    for lut_index in 0..8 {
        let lut_id = match lut_index {
            0 => 'A',
            1 => 'B',
            2 => 'C',
            3 => 'D',
            4 => 'E',
            5 => 'F',
            6 => 'G',
            7 => 'H',
            _ => break,
        };

        let row = lut_index / LUTS_PER_ROW;
        let col = lut_index % LUTS_PER_ROW;

        let lut_offset_x = LUT_MARGIN + (col as f64 * (LUT_WIDTH + LUT_SPACING));
        let lut_offset_y = LUT_MARGIN + (row as f64 * (LUT_HEIGHT + LUT_SPACING));

        let inside_lut = logic_x >= lut_offset_x
            && logic_x <= (lut_offset_x + LUT_WIDTH)
            && logic_y >= lut_offset_y
            && logic_y <= (lut_offset_y + LUT_HEIGHT);

        if inside_lut {
            return TargetLocation::Inner(tile_id, lut_id);
        }
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
