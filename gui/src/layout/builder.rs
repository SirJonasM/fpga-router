use router::Direction;

use super::*;

#[derive(Copy, Clone)]
pub struct Empty;
#[derive(Copy, Clone)]
pub struct AtTile;
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
    pub fn tile(self, tile_id: &TileId) -> LayoutBuilder<AtTile> {
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

// Methods available ONLY after you have specified a tile
impl LayoutBuilder<AtTile> {
    /// Moves to a LUT. Transitions the builder to the `AtLut` state.
    pub fn lut(mut self, lut_id: char) -> LayoutBuilder<AtLut> {
        let (lx, ly) = get_lut_offset(lut_id);
        self.x += lx;
        self.y += ly;

        LayoutBuilder {
            x: self.x,
            y: self.y,
            _marker: std::marker::PhantomData,
        }
    }
    pub fn cable_north(mut self, direction: &Direction) -> Self {
        let x_offset = -2.0 - direction.id as f64;
        let y_offset = 10.0 + direction.length as f64;
        self.x += x_offset;
        self.y += y_offset;
        self
    }

    pub fn cable_south(mut self, direction: &Direction) -> Self {
        let x_offset = 2.0 + direction.id as f64 + TILE_WIDTH;
        let y_offset = TILE_HEIGHT - 10.0 - direction.length as f64;
        self.x += x_offset;
        self.y += y_offset;
        self
    }

    pub fn cable_east(mut self, direction: &Direction) -> Self {
        let x_offset = 10.0 + direction.length as f64;
        let y_offset = -2.0 - direction.id as f64;
        self.x += x_offset;
        self.y += y_offset;
        self
    }

    pub fn cable_west(mut self, direction: &Direction) -> Self {
        let x_offset = TILE_WIDTH - 10.0 - direction.length as f64;
        let y_offset = 2.0 + direction.id as f64 + TILE_HEIGHT;
        self.x += x_offset;
        self.y += y_offset;
        self
    }
    pub fn carry_in(mut self) -> Self {
        self.x += TILE_WIDTH / 2.0;
        self.y += TILE_HEIGHT - 1.0;
        self
    }
    pub fn carry_out(mut self) -> Self {
        self.x += TILE_WIDTH / 2.0;
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

