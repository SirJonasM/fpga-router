use std::fmt::Display;

use router::{NodeId, NodeType, TileId};
use vello::kurbo::Point;

use crate::{
    constants::*,
    core::{entities::EdgeId, layout::LayoutBuilder},
};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MuxPorts {
    pub begin: Option<NodeId>,
    pub mid: Option<NodeId>,
    pub begin_b: Option<NodeId>,
    pub end: Option<NodeId>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LutPorts {
    pub inputs: Vec<NodeId>,
    pub output: Option<NodeId>,
    pub set_reset: Option<NodeId>,
    pub enable: Option<NodeId>,
    pub carry_in: Option<NodeId>,
    pub carry_out: Option<NodeId>,
}
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TilePorts {
    pub carry_in: Option<NodeId>,
    pub carry_out: Option<NodeId>,
    pub ground: Option<NodeId>,
    pub vcc: Option<NodeId>,
    pub lut: Option<NodeId>,
}

pub type MuxId = (TileId, String);
pub type LutId = usize;
pub enum GraphNode {
    Mux(MuxMetadata),
    Lut(LutMetadata),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TileMetadata {
    pub id: TileId,
    pub position_outer: Point,
    pub position_inner: Point,
    pub ports: TilePorts,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MuxMetadata {
    pub id: MuxId,
    pub tile_id: TileId,
    pub label: String,
    pub position: Point,
    pub ports: MuxPorts,
    pub outgoing: Vec<EdgeId>,
    pub incoming: Vec<EdgeId>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct LutMetadata {
    pub tile_id: TileId,
    pub id: LutId,
    pub position: Point,
    pub ports: LutPorts,
    pub bel_index: char,
}

impl Display for MuxMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.tile_id, self.label)
    }
}

pub fn get_node_pos(node: &router::Node) -> Option<vello::kurbo::Point> {
    let position_builder = LayoutBuilder::new().tile(&node.tile).tile_inner();
    let point = match &node.typ {
        NodeType::MuxPort(direction) => position_builder.mux_oriented(direction).build(),
        NodeType::CarryIn(_id) => position_builder.carry_in().build(),
        NodeType::CarryOut(_id) => position_builder.carry_out().build(),
        NodeType::VCC(_id) => position_builder.vdd().build(),
        NodeType::Ground(_id) => position_builder.ground().build(),
        NodeType::LutInput(lut, pin) => position_builder.lut(*lut).input(*pin).build(),
        NodeType::LutOutput(lut) => position_builder.lut(*lut).output().build(),
        NodeType::LutCarryOut(lut) => position_builder.lut(*lut).carry_out().build(),
        NodeType::LutCarryIn(lut) => position_builder.lut(*lut).carry_in().build(),
        NodeType::LutSetReset(lut) => position_builder.lut(*lut).set_reset().build(),
        NodeType::LutEnable(lut) => position_builder.lut(*lut).enable().build(),
        _ => return None,
    };
    Some(point)
}
pub fn get_lut_offset(bel_index: char) -> Point {
    let lowercase_char = bel_index.to_ascii_lowercase();
    let i = (lowercase_char as usize).saturating_sub('a' as usize);
    let row = i % LUTS_PER_COLUMN;
    let col = i / LUTS_PER_COLUMN;

    if col == 0 {
        Point::new(LUT_MARGIN, LUT_MARGIN + (row as f64 * (LUT_HEIGHT + LUT_SPACING_HEIGHT)))
    } else {
        Point::new(
            TILE_BOUNDING_BOX_WIDTH - LUT_WIDTH - LUT_MARGIN,
            LUT_MARGIN + (row as f64 * (LUT_HEIGHT + LUT_SPACING_HEIGHT)),
        )
    }
}
