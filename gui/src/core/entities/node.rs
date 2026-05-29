use router::{NodeId, NodeType};

use crate::core::layout::LayoutBuilder;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Node {
    pub id: NodeId,
    pub position: vello::kurbo::Point,
}

pub fn get_node_pos(node: &router::Node) -> Option<vello::kurbo::Point> {
    let position_builder = LayoutBuilder::new().tile(&node.tile).tile_inner();
    let point = match &node.typ {
        NodeType::Wire(direction) => position_builder.wire_oriented(direction).build(),
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
