use crate::{
    constants::*,
    core::{
        Edge, TargetLocation,
        entities::{EdgeId, LutId, LutMetadata, MuxId, MuxMetadata, MuxPorts, Tile, TileMetadata, TilePorts},
        layout::LayoutBuilder,
    },
};
use router::{NodeId, TileId};
use std::collections::HashMap;

#[derive(Default)]
pub struct SpatialFabricGrid {
    pub buckets: HashMap<TileId, TileBucket>,

    pub tile_index: HashMap<TileId, TileMetadata>,
    pub lut_index: HashMap<LutId, LutMetadata>,
    pub mux_index: HashMap<MuxId, MuxMetadata>,
    pub edge_index: HashMap<EdgeId, Edge>,
}
#[derive(Debug)]
pub struct TileBucket {
    pub lut_data: Vec<LutId>,
    pub mux_data: Vec<MuxId>,
    pub edge_data: Vec<EdgeId>,
}

impl SpatialFabricGrid {
    pub fn build_from_graph(graph: &router::FabricGraph, tile_manager: &router::TileManager) -> Self {
        let mut grid = Self::default();
        let mut tile_index = HashMap::new();
        let mut lut_index = HashMap::new();
        let mut mux_index: HashMap<(TileId, String), MuxMetadata> = HashMap::new();
        let mut edge_index = HashMap::new();

        for tile in tile_manager.0.values() {
            let position_outer = LayoutBuilder::new().tile(&tile.id).build();
            let position_inner = LayoutBuilder::new().tile(&tile.id).tile_inner().build();

            let tile_data = TileMetadata {
                id: tile.id,
                position_outer,
                position_inner,
                ports: TilePorts::default(),
            };
            grid.buckets.entry(tile.id).or_insert_with(|| TileBucket {
                lut_data: Vec::new(),
                mux_data: Vec::new(),
                edge_data: Vec::new(),
            });
            tile_index.insert(tile.id, tile_data);
        }

        for (node_id, node) in graph.nodes() {
            match &node.typ {
                router::NodeType::Lut { bel, port } => {
                    let id = (node.tile.0 as usize) << 16 | (node.tile.1 as usize) << 8 | (*bel as usize);
                    let ports = lut_index.entry(id).or_insert(LutMetadata::default()).ports;
                    match port {
                        router::LutPort::Input(_) => ports.inputs.push(node_id),
                        router::LutPort::Output => ports.output = Some(node_id),
                        router::LutPort::CarryIn => ports.carry_in = Some(node_id),
                        router::LutPort::CarryOut => ports.carry_out = Some(node_id),
                        router::LutPort::Enable => ports.enable = Some(node_id),
                        router::LutPort::SetReset => ports.set_reset = Some(node_id),
                    }
                }
                router::NodeType::Tile { port } => {
                    let tile_metadata = &mut tile_index.get_mut(&node.tile).unwrap().ports;
                    match port {
                        router::TilePort::CarryIn(_) => tile_metadata.carry_in = Some(node_id),
                        router::TilePort::CarryOut(_) => tile_metadata.carry_out = Some(node_id),
                        router::TilePort::Ground(_) => tile_metadata.ground = Some(node_id),
                        router::TilePort::VCC(_) => tile_metadata.vcc = Some(node_id),
                        router::TilePort::Lut(_) => tile_metadata.lut = Some(node_id),
                    }
                }
                router::NodeType::Mux(mux_node) => {
                    let label = node
                        .id
                        .replace("BEGb", "")
                        .replace("BEG", "")
                        .replace("MID", "")
                        .replace("END", "");
                    let id = &(node.tile, label.clone());
                    let position = LayoutBuilder::new()
                        .tile(&node.tile)
                        .tile_inner()
                        .mux_oriented(mux_node)
                        .build();

                    let ports = &mut mux_index
                        .entry(id.clone())
                        .or_insert_with(|| MuxMetadata {
                            id: id.clone(),
                            tile_id: node.tile,
                            label,
                            position,
                            ports: MuxPorts::default(),
                            outgoing: Vec::new(),
                            incoming: Vec::new(),
                        })
                        .ports;
                    match mux_node.port {
                        router::MuxPort::Begin => ports.end = Some(node_id),
                        router::MuxPort::BeginB => ports.begin_b = Some(node_id),
                        router::MuxPort::Mid => ports.mid = Some(node_id),
                        router::MuxPort::End => ports.end = Some(node_id),
                    };
                }
                router::NodeType::Other => {}
            }
        }

        for (source_node_id, edge) in graph.edges() {
            let source = graph.get_node(source_node_id);
            let target = graph.get_node(edge.node_id);
        }

        for (id, lut) in &lut_index {
            let bucket = grid.buckets.get_mut(&lut.tile_id).unwrap();
            bucket.lut_data.push((*id, lut.position));
        }
        for (id, mux) in &mux_index {
            let bucket = grid.buckets.get_mut(&mux.tile_id).unwrap();
            bucket.mux_data.push((id.clone(), mux.position));
        }

        grid.lut_index = lut_index;
        grid.tile_index = tile_index;
        grid.edge_index = edge_index;
        grid.mux_index = mux_index;
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
        if let Some(bel) = bucket.lut_data.iter().find_map(|(lut_id, position)| {
            let lut_x_offset = x - position.x;
            let lut_y_offset = y - position.y;
            if (0.0..=LUT_WIDTH).contains(&lut_x_offset) & (0.0..=LUT_HEIGHT).contains(&lut_y_offset) {
                self.lut_index.get(lut_id).map(|a| a.bel_index)
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
    pub fn get_node(&self, id: NodeId) -> Option<&MuxMetadata> {
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
            .map(|meta| meta.outgoing.iter().filter_map(|e_id| self.get_edge(*e_id)))
            .into_iter()
            .flatten()
    }

    pub fn get_incoming_edges(&self, id: NodeId) -> impl Iterator<Item = &Edge> {
        self.get_node(id)
            .map(|meta| meta.incoming.iter().filter_map(|e_id| self.get_edge(*e_id)))
            .into_iter()
            .flatten()
    }
    /// Find a NodeId inside a specific tile that matches a string label
    pub fn find_node_by_label(&self, tile_id: TileId, label: &str) -> Option<&MuxMetadata> {
        let bucket = self.buckets.get(&tile_id)?;

        bucket.node_data.iter().copied().find_map(|(node_id, _position)| {
            if let Some(node_meta) = self.node_index[node_id].as_ref()
                && node_meta.label == label
            {
                Some(node_meta)
            } else {
                None
            }
        })
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
        bucket.lut_data.iter().copied().find_map(|lut_id| {
            if let Some(lut) = self.lut_index.get(&lut_id.0)
                && lut.bel_index == bel_index
            {
                Some(lut)
            } else {
                None
            }
        })
    }

    /// Checks if a structural Tile coordinates exists inside the grid, returning its verified TileId token
    pub fn find_tile_existence(&self, tile_id: TileId) -> Option<&Tile> {
        self.tile_index.get(&tile_id)
    }
}
