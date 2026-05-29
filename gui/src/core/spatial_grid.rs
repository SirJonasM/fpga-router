use crate::{
    constants::*,
    core::{
        Edge, TargetLocation,
        entities::{Lut, Node, Tile, get_node_pos},
        layout::LayoutBuilder,
    },
};
use router::{NodeType, TileId};
use std::collections::HashMap;

#[derive(Default)]
pub struct SpatialFabricGrid {
    // Keys are the structural Tile locations (e.g., TileId(x, y))
    pub buckets: HashMap<TileId, TileBucket>,
}
#[derive(Debug)]
pub struct TileBucket {
    // Left top edge of TILE
    pub tile_data: Tile,
    // Left top edge of LUT
    pub lut_data: Vec<Lut>,
    pub node_data: Vec<Node>,
    pub edge_data: Vec<Edge>,
}

impl SpatialFabricGrid {
    pub fn build_from_graph(graph: &router::FabricGraph, tile_manager: &router::TileManager) -> Self {
        let mut grid = Self::default();
        for tile in tile_manager.0.values() {
            let position_outer = LayoutBuilder::new().tile(&tile.id).build();
            let position_inner = LayoutBuilder::new().tile(&tile.id).tile_inner().build();

            let bucket = grid.buckets.entry(tile.id).or_insert_with(|| TileBucket {
                tile_data: Tile {
                    position_outer,
                    position_inner,
                    id: tile.id,
                },
                lut_data: Vec::new(),
                node_data: Vec::new(),
                edge_data: Vec::new(),
            });
            let mut lut_list = vec![];
            for lut in &tile.luts {
                let pos = LayoutBuilder::new().tile(&tile.id).tile_inner().lut(lut.bel_index).build();
                lut_list.push(Lut {
                    bel_index: lut.bel_index,
                    position: pos,
                    tile: tile.id,
                })
            }
            bucket.lut_data = lut_list;
        }

        let mut total = 0;
        let mut other = 0;
        for node in graph.nodes.iter() {
            if node.typ == NodeType::Other {
                other += 1;
            }
            total += 1;
            if let Some(pos) = get_node_pos(node) {
                let bucket = grid
                    .buckets
                    .get_mut(&node.tile)
                    .unwrap_or_else(|| panic!("Error in pips and bel definition. Tile: {:?}", node.tile));
                let node_id = graph.get_node_id(&node.id()).unwrap();

                bucket.node_data.push(Node {
                    id: *node_id,
                    position: pos,
                });
            }
        }
        println!("{other}|{total} -> {}%", (1.0 - other as f64 / total as f64) * 100.0);
        for (start_id, edge) in graph.edges() {
            let start_node = graph.get_node(start_id);
            let end_node = graph.get_node(edge.node_id);
            if let Some(pos1) = get_node_pos(start_node)
                && let Some(pos2) = get_node_pos(end_node)
            {
                let edge_data = Edge {
                    source_node: start_id,
                    target_node: edge.node_id,
                    start_position: pos1,
                    end_position: pos2,
                };
                let crossed_tiles = get_tiles_intersected_by_line(pos1, pos2);

                for tile_id in crossed_tiles {
                    let Some(bucket) = grid.buckets.get_mut(&tile_id) else {
                        continue;
                    };
                    bucket.edge_data.push(edge_data);
                }
            }
        }

        for arr in grid.buckets.values_mut() {
            arr.node_data.sort_unstable_by(|a, b| a.position.x.total_cmp(&b.position.x));
        }

        grid
    }
    pub fn find_location_at_world_pos(&self, x: f64, y: f64) -> TargetLocation {
        if x < 0.0 || y < 0.0 {
            return TargetLocation::None;
        }

        let tile_x = (x / TILE_WIDTH).floor() as u8;
        let tile_y = (y / TILE_HEIGHT).floor() as u8;
        let tile_id = TileId(tile_x, tile_y);
        let Some(bucket) = self.buckets.get(&tile_id) else {
            return TargetLocation::None;
        };
        let tile_pos = bucket.tile_data.position_outer;
        let tile_x_offset = x - tile_pos.x;
        let tile_y_offset = y - tile_pos.y;
        let is_in_inner_box = TILE_INNER_RANGE_X.contains(&tile_x_offset) && TILE_INNER_RANGE_Y.contains(&tile_y_offset);
        if !is_in_inner_box {
            return TargetLocation::Outer(tile_id);
        }
        if let Some(bel) = bucket.lut_data.iter().find_map(|Lut { bel_index, position, .. }| {
            let lut_x_offset = x - position.x;
            let lut_y_offset = y - position.y;
            if (0.0..=LUT_WIDTH).contains(&lut_x_offset) & (0.0..=LUT_HEIGHT).contains(&lut_y_offset) {
                Some(bel_index)
            } else {
                None
            }
        }) {
            return TargetLocation::Inner(tile_id, *bel);
        }
        TargetLocation::InnerEmpty(tile_id)
    }
}

/// Calculates all TileIds that a line segment crosses between pos1 and pos2
/// It does not guarantee that all tiles exist. Pure mathematical calculation
fn get_tiles_intersected_by_line(pos1: vello::kurbo::Point, pos2: vello::kurbo::Point) -> Vec<TileId> {
    let mut tiles = Vec::new();

    let t1_x = (pos1.x / TILE_WIDTH).floor() as i32;
    let t1_y = (pos1.y / TILE_HEIGHT).floor() as i32;
    let t2_x = (pos2.x / TILE_WIDTH).floor() as i32;
    let t2_y = (pos2.y / TILE_HEIGHT).floor() as i32;

    let min_x = t1_x.min(t2_x).max(0) as u8;
    let max_x = t1_x.max(t2_x).max(0) as u8;
    let min_y = t1_y.min(t2_y).max(0) as u8;
    let max_y = t1_y.max(t2_y).max(0) as u8;

    for x in min_x..=max_x {
        for y in min_y..=max_y {
            let tile_id = TileId(x, y);

            tiles.push(tile_id);
        }
    }

    tiles
}
