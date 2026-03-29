use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use crate::{FabricError, FabricResult, fabric::node::TileId};

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    High,
    Low,
}

#[derive(Debug)]
pub enum LutState {
    Free,
    Used,
    Borrowed(State),
}
#[derive(Debug, Eq, PartialEq)]
pub enum LutInputState {
    Free,
    Used,
}

#[derive(Debug)]
pub struct Lut {
    bel_index: char,
    state: LutState,
    output_pin: String,
    input_pin: [(String, LutInputState); 4],
}

#[derive(Debug)]
pub struct Tile {
    id: TileId,
    luts: Vec<Lut>,
}

#[derive(Debug)]
pub struct TileManager(pub HashMap<TileId, Tile>);

impl TileManager {
    /// Reads from the bel.txt file and creates a `TileManager`
    /// # Errors
    /// Io errors
    pub fn from_file<P: AsRef<Path>>(path: &P) -> FabricResult<Self> {
        let file = File::open(path).map_err(|source| FabricError::Io {
            path: path.as_ref().to_path_buf(),
            source,
        })?;
        let reader = BufReader::new(file);
        let mut tiles: HashMap<TileId, Tile> = HashMap::new();

        for (line_number, line) in reader.lines().enumerate() {
            let line = line.map_err(|source| FabricError::Io {
                path: path.as_ref().to_path_buf(),
                source,
            })?;
            // Skip comments and empty lines
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split(',').collect();

            // Basic validation for the FABULOUS_LC rows
            if parts.len() < 13 || parts[4] != "FABULOUS_LC" {
                continue;
            }

            // Parse Coordinates: Expecting "X1Y1" format in parts[0]
            // Or use parts[1] and parts[2] if they are raw integers

            let tile_id = TileId::from_str_coords(parts[0]).map_err(|e| FabricError::ParseError { line_number, source: e })?;

            // Construct the LUT
            let lut = Lut {
                // parts[3] is "A", "B", etc.
                bel_index: parts[3].chars().next().unwrap_or('?'),
                state: LutState::Free,
                // parts[12] is the output pin (e.g., "LA_O")
                output_pin: parts[12].to_string(),
                // parts[5..9] are I0, I1, I2, I3
                input_pin: [
                    (parts[5].to_string(), LutInputState::Free),
                    (parts[6].to_string(), LutInputState::Free),
                    (parts[7].to_string(), LutInputState::Free),
                    (parts[8].to_string(), LutInputState::Free),
                ],
            };

            // Insert into the tile manager
            tiles
                .entry(tile_id)
                .or_insert_with(|| Tile {
                    id: tile_id,
                    luts: Vec::new(),
                })
                .luts
                .push(lut);
        }

        Ok(Self(tiles))
    }
    /// Internal helper to find a LUT by index within a specific tile
    fn find_lut_mut(&mut self, tile_id: TileId, bel_index: char) -> Option<&mut Lut> {
        self.0
            .get_mut(&tile_id)
            .and_then(|tile| tile.luts.iter_mut().find(|lut| lut.bel_index == bel_index))
    }

    /// Marks a LUT as 'Used' (called during placement parsing)
    pub fn mark_lut_used(&mut self, tile: TileId, bel_index: char) -> Option<String> {
        if let Some(lut) = self.find_lut_mut(tile, bel_index) {
            // Safety check: only borrow if it's actually free
            if matches!(lut.state, LutState::Free) {
                lut.state = LutState::Used;
                return Some(lut.output_pin.clone());
            }
        }
        None
    }

    /// Sets the lut input to used
    /// # Returns
    /// true: the lut input state was changed
    /// false: the lut input was already in the requested state
    /// # Errors
    /// `LutDoesNotExist`: if there is no Lut in the requested tile
    /// `LutInputDoesNotExist`: if the lut does not contain the requested input.
    pub fn mark_lut_input_used(&mut self, tile: TileId, bel_index: char, input: &str) -> FabricResult<bool> {
        let lut = self
            .find_lut_mut(tile, bel_index)
            .ok_or(FabricError::LutDoesNotExist { tile, bel_index })?;
        let input = lut
            .input_pin
            .iter_mut()
            .find(|a| a.0 == input)
            .ok_or_else(|| FabricError::LutInputDoesNotExist {
                tile,
                bel_index,
                input: input.to_string(),
            })?;
        if input.1 == LutInputState::Free {
            input.1 = LutInputState::Used;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    /// Sets the lut input to free
    /// # Returns
    /// true: the lut input state was changed
    /// false: the lut input was already in the requested state
    /// # Errors
    /// `LutDoesNotExist`: if there is no Lut in the requested tile
    /// `LutInputDoesNotExist`: if the lut does not contain the requested input.
    pub fn free_lut_input(&mut self, tile: TileId, bel_index: char, input: &str) -> FabricResult<bool> {
        let lut = self
            .find_lut_mut(tile, bel_index)
            .ok_or(FabricError::LutDoesNotExist { tile, bel_index })?;
        let input = lut
            .input_pin
            .iter_mut()
            .find(|a| a.0 == input)
            .ok_or_else(|| FabricError::LutInputDoesNotExist {
                tile,
                bel_index,
                input: input.to_string(),
            })?;
        if input.1 == LutInputState::Used {
            input.1 = LutInputState::Free;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Searched the tile for a lut that can be set to either all 0's or 1's to produce a *VCC* or
    /// *GND* signal
    pub fn request_constant(&mut self, start_tile: TileId, state: State) -> Option<(TileId, String)> {
        let search_order = [
            start_tile,
            TileId(start_tile.0 + 1, start_tile.1), // East
            TileId(start_tile.0, start_tile.1 + 1), // North
        ];

        for &tid in &search_order {
            if let Some(tile) = self.0.get_mut(&tid) {
                let existing = tile
                    .luts
                    .iter()
                    .find(|l| matches!(&l.state, LutState::Borrowed(s) if s == &state));

                if let Some(lut) = existing {
                    return Some((tile.id, lut.output_pin.clone()));
                }

                let free_lut_index = tile.luts.iter().position(|l| matches!(l.state, LutState::Free));

                if let Some(idx) = free_lut_index {
                    let lut = &mut tile.luts[idx];
                    lut.state = LutState::Borrowed(state);
                    return Some((tile.id, lut.output_pin.clone()));
                }
            }
        }
        None
    }
    /// Iterates through all tiles and generates FASM configuration strings
    /// for LUTs that were borrowed as constant drivers.
    #[must_use]
    pub fn generate_constant_fasm(&self) -> Vec<String> {
        let mut fasm_lines = Vec::new();

        for (tile_id, tile) in &self.0 {
            for lut in &tile.luts {
                if let LutState::Borrowed(state) = &lut.state {
                    // Example FASM Format: Tile_X1Y1.LC_A.INIT[15:0] = 16'h0000
                    let init_val = match state {
                        State::Low => "16'b0000000000000000",
                        State::High => "16'h1111111111111111",
                    };

                    // We use the bel_index (e.g., 'A', 'B') to specify which LUT in the tile
                    let line = format!("X{}Y{}.{}.INIT[15:0] = {}", tile_id.0, tile_id.1, lut.bel_index, init_val);

                    fasm_lines.push(line);
                }
            }
        }
        fasm_lines
    }

    /// Returns the free lut inputs of a specified lut and sets them as used.
    pub(crate) fn get_free_lut_inputs(&mut self, tile: TileId, bel_index: char) -> FabricResult<Vec<String>> {
        let lut = self
            .find_lut_mut(tile, bel_index)
            .ok_or(FabricError::LutDoesNotExist { tile, bel_index })?;
        let result = lut
            .input_pin
            .iter_mut()
            .filter_map(|(a, b)| {
                if b == &LutInputState::Used {
                    return None;
                }
                *b = LutInputState::Used;
                Some(a.clone())
            })
            .collect();
        Ok(result)
    }
}
