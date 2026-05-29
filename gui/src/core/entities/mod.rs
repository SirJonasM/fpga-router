mod edge;
mod lut;
mod node;
mod tile;

pub use edge::{Edge, EdgeId};
pub use lut::{Lut, LutId, get_lut_offset};
pub use node::{NodeMetadata, get_node_pos};
use router::NodeId;
use router::TileId;
use std::fmt::Display;
pub use tile::Tile;
pub use tile::get_tile_pos;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Entity {
    Tile(TileId),
    Lut(LutId),
    Node(NodeId),
    Edge(EdgeId),
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
    /// The coordinates land in the inner block, and land in a LUT.
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
