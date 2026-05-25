use router::{Node, NodeType, TileId};
pub use builder::LayoutBuilder;

use crate::constants::*;
mod builder;


pub fn get_tile_pos(tile: &TileId) -> (f64, f64) {
    (
        tile.0 as f64 * (TILE_WIDTH + TILE_PADDING),
        tile.1 as f64 * (TILE_WIDTH + TILE_PADDING),
    )
}
pub fn get_lut_offset(bel_index: char) -> (f64, f64) {
    let lowercase_char = bel_index.to_ascii_lowercase();
    let i = (lowercase_char as usize).saturating_sub('a' as usize);
    let row = i / LUTS_PER_ROW;
    let col = i % LUTS_PER_ROW;

    let lx = LUT_MARGIN + (col as f64 * (LUT_WIDTH + LUT_SPACING));
    let ly = LUT_MARGIN + (row as f64 * (LUT_HEIGHT + LUT_SPACING));
    (lx, ly)
}
pub fn get_node_pos(node: &Node) -> Option<vello::kurbo::Point> {
    let position_builder = LayoutBuilder::new().tile(&node.tile);
    let point = match &node.typ {
        NodeType::North(direction) => position_builder.cable_north(direction).build(),
        NodeType::South(direction) => position_builder.cable_south(direction).build(),
        NodeType::East(direction) => position_builder.cable_east(direction).build(),
        NodeType::West(direction) => position_builder.cable_west(direction).build(),
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

