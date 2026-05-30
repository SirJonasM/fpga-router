use router::{NodeId, TileId};

use crate::{
    constants::*,
    core::{
        Entity, Position, SpatialFabricGrid,
        entities::{EdgeId, LutId},
    },
    utils::{distance_to_segment, is_on_rect_border},
};
impl SpatialFabricGrid {
    pub fn find_entities_at_position(&self, position: &Position) -> Option<Entity> {
        self.find_node_at_pos(position)
            .map(Entity::Node)
            .or_else(|| self.find_edge_at_pos(position).map(Entity::Edge))
            .or_else(|| self.find_lut_at_pos(position).map(Entity::Lut))
            .or_else(|| self.find_tile_at_pos(position).map(Entity::Tile))
    }

    pub fn find_tile_at_pos(&self, position: &Position) -> Option<TileId> {
        let target_tile_id = match position.location {
            crate::core::TargetLocation::Outer(tile_id)
            | crate::core::TargetLocation::Inner(tile_id, _)
            | crate::core::TargetLocation::InnerEmpty(tile_id) => tile_id,
            crate::core::TargetLocation::None => return None,
        };

        let tile = self.tile_index.get(&target_tile_id)?;

        let is_on_outer_border = is_on_rect_border(
            position.world_position,
            tile.position_inner,
            TILE_BOUNDING_BOX_WIDTH,
            TILE_BOUNDING_BOX_HEIGHT,
            TILE_INNER_SELECT_THRESHOLD,
        );
        let is_on_border = is_on_outer_border
            || is_on_rect_border(
                position.world_position,
                tile.position_outer,
                TILE_WIDTH,
                TILE_HEIGHT,
                TILE_OUTER_SELECT_THRESHOLD,
            );

        if is_on_border { Some(target_tile_id) } else { None }
    }

    pub fn find_lut_at_pos(&self, position: &Position) -> Option<LutId> {
        let crate::core::TargetLocation::Inner(tile_id, bel) = position.location else {
            return None;
        };

        let bucket = self.buckets.get(&tile_id)?;

        // Scan the IDs in the localized bucket
        for &lut_id in &bucket.lut_data {
            if let Some(lut) = self.lut_index.get(&lut_id)
                && lut.bel_index == bel
                && is_on_rect_border(
                    position.world_position,
                    lut.position,
                    LUT_WIDTH,
                    LUT_HEIGHT,
                    LUT_SELECT_THRESHOLD,
                )
            {
                return Some(lut_id);
            }
        }
        None
    }

    pub fn find_edge_at_pos(&self, position: &Position) -> Option<EdgeId> {
        let target_tile_id = match position.location {
            crate::core::TargetLocation::Outer(tile_id)
            | crate::core::TargetLocation::Inner(tile_id, _)
            | crate::core::TargetLocation::InnerEmpty(tile_id) => tile_id,
            crate::core::TargetLocation::None => return None,
        };

        let bucket = self.buckets.get(&target_tile_id)?;

        const CLICK_TOLERANCE: f64 = WIRE_LINE_WIDTH * 10.0;
        const TOLERANCE_SQ: f64 = CLICK_TOLERANCE * CLICK_TOLERANCE;

        let mut closest_edge = None;
        let mut min_distance_sq = TOLERANCE_SQ;

        // Bucket contains EdgeIds, grab actual geometry out of edge_index map
        for &edge_id in &bucket.edge_data {
            if let Some(edge) = self.edge_index.get(&edge_id) {
                let dist_sq = distance_to_segment(position.world_position, edge.start_position, edge.end_position);

                if dist_sq < min_distance_sq {
                    min_distance_sq = dist_sq;
                    closest_edge = Some(edge_id);
                }
            }
        }

        closest_edge
    }

    pub fn find_node_at_pos(&self, position: &Position) -> Option<NodeId> {
        let world_x = position.world_position.x;
        let world_y = position.world_position.y;

        let target_tile_id = match position.location {
            crate::core::TargetLocation::Outer(tile_id)
            | crate::core::TargetLocation::Inner(tile_id, _)
            | crate::core::TargetLocation::InnerEmpty(tile_id) => tile_id,
            crate::core::TargetLocation::None => return None,
        };

        let bucket = self.buckets.get(&target_tile_id)?;

        const RADIUS: f64 = WIRE_NODE_RADIUS * 1.2;
        const RADIUS_SQ: f64 = (WIRE_NODE_RADIUS * 1.2) * (WIRE_NODE_RADIUS * 1.2);
        let min_x = world_x - RADIUS;
        let max_x = world_x + RADIUS;

        // Binary search setup over NodeIds using your custom bracket index reference structure
        let nodes_ref = &self.node_index;
        let start_idx = match bucket.node_data.binary_search_by(|&id| {
            let pos_x = nodes_ref[id].as_ref().map(|n| n.position.x).unwrap_or(0.0);
            pos_x.partial_cmp(&min_x).unwrap()
        }) {
            Ok(idx) | Err(idx) => idx,
        };

        for &node_id in &bucket.node_data[start_idx..] {
            if let Some(node) = nodes_ref[node_id].as_ref() {
                if node.position.x > max_x {
                    break;
                }
                let dx = world_x - node.position.x;
                let dy = world_y - node.position.y;
                let distance = dx * dx + dy * dy;

                if distance <= RADIUS_SQ {
                    return Some(node_id);
                }
            }
        }

        None
    }
}
