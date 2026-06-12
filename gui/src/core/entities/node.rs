use std::{collections::HashMap, fmt::Display};

use router::{LutPort, MuxPort, NodeId, NodeType, Port, TileId, TilePort};
use vello::kurbo::Point;

use crate::{
    constants::*,
    core::{entities::EdgeId, layout::LayoutBuilder},
};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MuxPorts {
    pub begin: Option<(NodeId, Point)>,
    pub mid: Option<(NodeId, Point)>,
    pub begin_b: Option<(NodeId, Point)>,
    pub end: Option<(NodeId, Point)>,
}
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LutPorts {
    pub inputs: HashMap<u8, (NodeId, Point)>,
    pub output: Option<(NodeId, Point)>,
    pub set_reset: Option<(NodeId, Point)>,
    pub enable: Option<(NodeId, Point)>,
    pub carry_in: Option<(NodeId, Point)>,
    pub carry_out: Option<(NodeId, Point)>,
}
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TilePorts {
    pub carry_in: Option<(NodeId, Point)>,
    pub carry_out: Option<(NodeId, Point)>,
    pub ground: Option<(NodeId, Point)>,
    pub vcc: Option<(NodeId, Point)>,
    pub lut: Option<(NodeId, Point)>,
}

pub type MuxId = (TileId, String);
pub type LutId = usize;
pub enum GraphNode {
    Mux(MuxMetadata),
    Lut(LutMetadata),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Metadata {
    Tile(TileMetadata),
    Lut(LutMetadata),
    Mux(MuxMetadata),
}
impl Metadata {
    pub fn get_port(&self, port: &Port) -> Option<(NodeId, Point)> {
        match (self, port) {
            (Self::Tile(tile_metadata), Port::Tile(tile_port)) => tile_metadata.get_port(tile_port),
            (Self::Lut(lut_metadata), Port::Lut(lut_port)) => lut_metadata.get_port(lut_port),
            (Self::Mux(mux_metadata), Port::Mux(mux_port)) => mux_metadata.get_port(mux_port),
            _ => None,
        }
    }
}
impl TileMetadata {
    pub fn get_port(&self, tile_port: &TilePort) -> Option<(NodeId, Point)> {
        match tile_port {
            router::TilePort::CarryIn(_) => self.ports.carry_in,
            router::TilePort::CarryOut(_) => self.ports.carry_in,
            router::TilePort::Ground(_) => self.ports.carry_in,
            router::TilePort::VCC(_) => self.ports.carry_in,
            router::TilePort::Lut(_) => self.ports.carry_in,
        }
    }
}
impl MuxMetadata {
    pub fn get_port(&self, mux_port: &MuxPort) -> Option<(NodeId, Point)> {
        match mux_port {
            router::MuxPort::Begin => self.ports.begin,
            router::MuxPort::BeginB => self.ports.begin_b,
            router::MuxPort::Mid => self.ports.mid,
            router::MuxPort::End => self.ports.end,
        }
    }
}
impl LutMetadata {
    pub fn get_port(&self, lut_port: &LutPort) -> Option<(NodeId, Point)> {
        match lut_port {
            router::LutPort::Input(id) => self.ports.inputs.get(&id).copied(),
            router::LutPort::Output => self.ports.output,
            router::LutPort::CarryIn => self.ports.carry_in,
            router::LutPort::CarryOut => self.ports.carry_out,
            router::LutPort::Enable => self.ports.enable,
            router::LutPort::SetReset => self.ports.set_reset,
        }
    }
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

#[derive(Clone, Debug, PartialEq)]
pub struct LutMetadata {
    pub tile_id: TileId,
    pub id: LutId,
    pub position: Point,
    pub ports: LutPorts,
    pub bel_index: char,
}
impl Default for LutMetadata {
    fn default() -> Self {
        Self {
            tile_id: TileId(0, 0),
            id: Default::default(),
            position: Default::default(),
            ports: Default::default(),
            bel_index: Default::default(),
        }
    }
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
