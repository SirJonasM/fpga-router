use router::{Compass, TileId, Wire, WirePoint};

use crate::{
    constants::*,
    core::entities::{get_lut_offset, get_tile_pos},
};

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

impl LayoutBuilder<AtTileInner> {
    pub fn lut(self, lut_id: char) -> LayoutBuilder<AtLut> {
        let (lx, ly) = get_lut_offset(lut_id);

        LayoutBuilder {
            x: self.x + lx,
            y: self.y + ly,
            _marker: std::marker::PhantomData,
        }
    }
    pub fn wire_oriented(mut self, wire: &Wire) -> Self {
        let (local_x, local_y) = if wire.jump {
            (
                0.0 + match wire.wire_point {
                    WirePoint::Begin => 1.0,
                    WirePoint::BeginB => -4.0,
                    WirePoint::Mid => 0.0,
                    WirePoint::End => -1.0,
                },
                10.0 + wire.id as f64,
            )
        } else if wire.double {
            (
                0.0 + match wire.wire_point {
                    WirePoint::Begin => -1.0,
                    WirePoint::BeginB => -4.0,
                    WirePoint::Mid => 0.0,
                    WirePoint::End => 1.0,
                },
                TILE_BOUNDING_BOX_HEIGHT - 10.0 - wire.id as f64,
            )
        } else {
            let xx = wire_offset(&wire.wire_point);
            (
                -2.0 - wire.id as f64 - wire.length as f64 * WIRE_NODE_RADIUS * 2.0,
                10.0 + xx + wire.length as f64,
            )
        };

        let cx = TILE_BOUNDING_BOX_WIDTH / 2.0;
        let cy = TILE_BOUNDING_BOX_HEIGHT / 2.0;

        let (rotated_x, rotated_y) = match wire.direction {
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

impl<State> LayoutBuilder<State> {
    pub fn build(self) -> vello::kurbo::Point {
        vello::kurbo::Point::new(self.x, self.y)
    }
}

const fn wire_offset(x: &WirePoint) -> f64 {
    match x {
        router::WirePoint::Begin => 0.0,
        router::WirePoint::BeginB => 20.0,
        router::WirePoint::Mid => 40.0,
        router::WirePoint::End => 60.0,
    }
}
