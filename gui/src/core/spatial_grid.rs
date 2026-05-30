use crate::{
    constants::*,
    core::{
        Edge, TargetLocation,
        entities::{EdgeId, Lut, LutId, NodeMetadata, Tile, get_node_pos},
        layout::LayoutBuilder,
    },
};
use router::{NodeId, TileId};
use std::collections::HashMap;

#[derive(Default)]
pub struct SpatialFabricGrid {
    pub buckets: HashMap<TileId, TileBucket>,
    pub node_index: Vec<Option<NodeMetadata>>,
    pub edge_index: HashMap<EdgeId, Edge>,
    pub lut_index: HashMap<LutId, Lut>,
    pub tile_index: HashMap<TileId, Tile>,
}
#[derive(Debug)]
pub struct TileBucket {
    pub tile_data: TileId,
    pub lut_data: Vec<LutId>,
    pub node_data: Vec<NodeId>,
    pub edge_data: Vec<EdgeId>,
}

impl SpatialFabricGrid {
    pub fn build_from_graph(graph: &router::FabricGraph, tile_manager: &router::TileManager) -> Self {
        let mut grid = Self::default();
        let mut lut_index = HashMap::new();
        let mut edge_index = HashMap::new();
        let mut tile_index = HashMap::new();

        let max_node_id = graph
            .nodes
            .iter()
            .filter_map(|node| graph.get_node_id(&node.id()))
            .map(|id| id.raw() as usize)
            .max()
            .unwrap_or(0);

        grid.node_index = vec![None; max_node_id + 1];

        for tile in tile_manager.0.values() {
            let position_outer = LayoutBuilder::new().tile(&tile.id).build();
            let position_inner = LayoutBuilder::new().tile(&tile.id).tile_inner().build();

            let tile_data = Tile {
                id: tile.id,
                position_outer,
                position_inner,
            };
            tile_index.insert(tile.id, tile_data);

            let bucket = grid.buckets.entry(tile.id).or_insert_with(|| TileBucket {
                tile_data: tile.id,
                lut_data: Vec::new(),
                node_data: Vec::new(),
                edge_data: Vec::new(),
            });

            let mut lut_list = vec![];
            for lut in &tile.luts {
                let pos = LayoutBuilder::new().tile(&tile.id).tile_inner().lut(lut.bel_index).build();
                let id = lut_index.len();
                let lut_struct = Lut {
                    id,
                    bel_index: lut.bel_index,
                    position: pos,
                    tile: tile.id,
                };
                lut_index.insert(id, lut_struct);
                lut_list.push(id);
            }
            bucket.lut_data = lut_list;
        }

        grid.lut_index = lut_index;
        grid.tile_index = tile_index;

        for node in graph.nodes.iter() {
            let label = node.id.to_string();
            let id = *graph.get_node_id(&node.id()).unwrap();
            if let Some(pos) = get_node_pos(node) {
                let Some(bucket) = grid.buckets.get_mut(&node.tile) else {
                    panic!("Error in pips and bel definition. Tile: {:?}", node.tile);
                };

                let node_data = NodeMetadata {
                    id,
                    position: pos,
                    label,
                    tile_id: node.tile,
                    outgoing_edges: Vec::new(),
                    incoming_edges: Vec::new(),
                };

                grid.node_index[id] = Some(node_data);
                bucket.node_data.push(id);
            } else {
                let pre = graph
                    .get_previous(id)
                    .iter()
                    .map(|x| graph.get_node(*x).id())
                    .collect::<Vec<String>>();
                if pre.len() == 1 {
                    let prev_id = graph.get_node_id(&pre[0]).unwrap();
                    let pre = graph
                        .get_previous(*prev_id)
                        .iter()
                        .map(|x| graph.get_node(*x).id())
                        .collect::<Vec<String>>();
                    println!("[{}]", pre.join(", "));
                }
                println!("[{}]", pre.join(", "));
                println!("{node}");
                let next = graph
                    .get_next(id)
                    .iter()
                    .map(|x| graph.get_node(*x).id())
                    .collect::<Vec<String>>();
                println!("[{}]", next.join(", "));

                if next.len() == 1 {
                    let next_id = graph.get_node_id(&next[0]).unwrap();
                    let next = graph
                        .get_next(*next_id)
                        .iter()
                        .map(|x| graph.get_node(*x).id())
                        .collect::<Vec<String>>();
                    println!("[{}]", next.join(", "));
                }

                println!();
            }
        }

        for (start_id, end_id) in graph.edges() {
            let start_node = graph.get_node(start_id);
            let end_node = graph.get_node(end_id.node_id);

            if let Some(pos1) = get_node_pos(start_node)
                && let Some(pos2) = get_node_pos(end_node)
            {
                let id = edge_index.len();
                let edge_data = Edge {
                    id,
                    source_node: start_id,
                    target_node: end_id.node_id,
                    start_position: pos1,
                    end_position: pos2,
                };

                let crossed_tiles = get_tiles_intersected_by_line(pos1, pos2);
                for tile_id in crossed_tiles {
                    if let Some(bucket) = grid.buckets.get_mut(&tile_id) {
                        bucket.edge_data.push(id);
                    }
                }

                if let Some(source_meta) = grid.node_index[start_id].as_mut() {
                    source_meta.outgoing_edges.push(id);
                }

                if let Some(target_meta) = grid.node_index[end_id.node_id].as_mut() {
                    target_meta.incoming_edges.push(id);
                }

                edge_index.insert(id, edge_data);
            }
        }
        grid.edge_index = edge_index;

        let nodes_ref = &grid.node_index;
        for bucket in grid.buckets.values_mut() {
            bucket.node_data.sort_unstable_by(|a, b| {
                let pos_a = nodes_ref[*a].as_ref().map(|n| n.position.x).unwrap_or(0.0);
                let pos_b = nodes_ref[*b].as_ref().map(|n| n.position.x).unwrap_or(0.0);
                pos_a.total_cmp(&pos_b)
            });
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
        let Some(tile) = self.tile_index.get(&tile_id) else {
            return TargetLocation::None;
        };
        let tile_pos = tile.position_outer;
        let tile_x_offset = x - tile_pos.x;
        let tile_y_offset = y - tile_pos.y;
        let is_in_inner_box = TILE_INNER_RANGE_X.contains(&tile_x_offset) && TILE_INNER_RANGE_Y.contains(&tile_y_offset);
        if !is_in_inner_box {
            return TargetLocation::Outer(tile_id);
        }
        if let Some(bel) = bucket.lut_data.iter().find_map(|lut_id| {
            let lut = self.lut_index.get(lut_id)?;

            let lut_x_offset = x - lut.position.x;
            let lut_y_offset = y - lut.position.y;
            if (0.0..=LUT_WIDTH).contains(&lut_x_offset) & (0.0..=LUT_HEIGHT).contains(&lut_y_offset) {
                Some(lut.bel_index)
            } else {
                None
            }
        }) {
            return TargetLocation::Inner(tile_id, bel);
        }
        TargetLocation::InnerEmpty(tile_id)
    }
}

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
impl SpatialFabricGrid {
    #[inline(always)]
    pub fn get_edge(&self, id: EdgeId) -> Option<&Edge> {
        self.edge_index.get(&id)
    }

    #[inline(always)]
    pub fn get_node(&self, id: NodeId) -> Option<&NodeMetadata> {
        self.node_index[id].as_ref()
    }

    #[inline(always)]
    pub fn get_lut(&self, id: LutId) -> Option<&Lut> {
        self.lut_index.get(&id)
    }

    #[inline(always)]
    pub fn get_tile(&self, id: TileId) -> Option<&Tile> {
        self.tile_index.get(&id)
    }

    pub fn get_node_edges(&self, id: NodeId) -> Option<(&[EdgeId], &[EdgeId])> {
        let meta = self.node_index[id].as_ref()?;
        Some((meta.incoming_edges.as_slice(), meta.outgoing_edges.as_slice()))
    }

    pub fn get_outgoing_edges(&self, id: NodeId) -> impl Iterator<Item = &Edge> {
        self.get_node(id)
            .map(|meta| meta.outgoing_edges.iter().filter_map(|e_id| self.get_edge(*e_id)))
            .into_iter()
            .flatten()
    }

    pub fn get_incoming_edges(&self, id: NodeId) -> impl Iterator<Item = &Edge> {
        self.get_node(id)
            .map(|meta| meta.incoming_edges.iter().filter_map(|e_id| self.get_edge(*e_id)))
            .into_iter()
            .flatten()
    }
    /// Find a NodeId inside a specific tile that matches a string label
    pub fn find_node_by_label(&self, tile_id: TileId, label: &str) -> Option<&NodeMetadata> {
        let bucket = self.buckets.get(&tile_id)?;

        bucket
            .node_data
            .iter()
            .copied()
            .find(|&node_id| {
                if let Some(node_meta) = self.node_index[node_id].as_ref() {
                    node_meta.label == label
                } else {
                    false
                }
            })
            .and_then(|node_id| self.node_index[node_id].as_ref())
    }

    /// Find an EdgeId by specifying its source node's tile + label and its target node's tile + label
    pub fn find_edge_by_connections(
        &self,
        source_tile: TileId,
        source_label: &str,
        target_tile: TileId,
        target_label: &str,
    ) -> Option<&Edge> {
        let source_node_id = self.find_node_by_label(source_tile, source_label)?;

        let source_meta = self.node_index[source_node_id.id].as_ref()?;

        for &edge_id in &source_meta.outgoing_edges {
            if let Some(edge) = self.edge_index.get(&edge_id) {
                // Check if the edge's target node matches our target destination criteria
                if let Some(target_meta) = self.node_index[edge.target_node].as_ref()
                    && target_meta.tile_id == target_tile
                    && target_meta.label == target_label
                {
                    return Some(edge);
                }
            }
        }
        None
    }

    /// Find a LutId by its tile location and its specific BEL (Basic Element Layer) index
    pub fn find_lut_by_bel(&self, tile_id: TileId, bel_index: char) -> Option<&Lut> {
        let bucket = self.buckets.get(&tile_id)?;

        // Scan the LUTs present inside this single bucket
        bucket
            .lut_data
            .iter()
            .copied()
            .find(|&lut_id| {
                if let Some(lut) = self.lut_index.get(&lut_id) {
                    lut.bel_index == bel_index
                } else {
                    false
                }
            })
            .and_then(|lut_id| self.get_lut(lut_id))
    }

    /// Checks if a structural Tile coordinates exists inside the grid, returning its verified TileId token
    pub fn find_tile_existence(&self, tile_id: TileId) -> Option<&Tile> {
        self.tile_index.get(&tile_id)
    }
}
