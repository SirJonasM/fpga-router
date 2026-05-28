use crate::{
    constants::*,
    core::{Edge, Entity, Lut, Node, Position, SpatialFabricGrid, Tile},
    utils::{distance_to_segment, is_on_rect_border},
};

pub fn find_tile_at_pos(position: &Position, spatial_grid: &SpatialFabricGrid) -> Option<Tile> {
    let target_tile_id = match position.location {
        crate::core::TargetLocation::Outer(tile_id) => tile_id,
        crate::core::TargetLocation::Inner(tile_id, _) => tile_id,
        crate::core::TargetLocation::InnerEmpty(tile_id) => tile_id,
        crate::core::TargetLocation::None => return None,
    };
    let tile = spatial_grid.buckets.get(&target_tile_id).map(|bucket| bucket.tile_data)?;
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
    if is_on_border {
        println!("clicked on TILe");
        Some(tile)
    } else {
        None
    }
}
pub fn find_lut_at_pos(position: &Position, spatial_grid: &SpatialFabricGrid) -> Option<Lut> {
    let crate::core::TargetLocation::Inner(tile, bel) = position.location else {
        return None;
    };
    let lut = spatial_grid
        .buckets
        .get(&tile)
        .and_then(|bucket| bucket.lut_data.iter().find(|a| a.bel_index == bel))?;

    if is_on_rect_border(
        position.world_position,
        lut.position,
        LUT_WIDTH,
        LUT_HEIGHT,
        LUT_SELECT_THRESHOLD,
    ) {
        println!("clicked on LUT");
        Some(*lut)
    } else {
        None
    }
}
pub fn find_edge_at_pos(position: &Position, spatial_grid: &SpatialFabricGrid) -> Option<Edge> {
    let target_tile_id = match position.location {
        crate::core::TargetLocation::Outer(tile_id) => tile_id,
        crate::core::TargetLocation::Inner(tile_id, _) => tile_id,
        crate::core::TargetLocation::InnerEmpty(tile_id) => tile_id,
        crate::core::TargetLocation::None => return None,
    };
    let bucket = spatial_grid.buckets.get(&target_tile_id)?;

    const CLICK_TOLERANCE: f64 = WIRE_LINE_WIDTH * 10.0;
    const TOLERANCE_SQ: f64 = CLICK_TOLERANCE * CLICK_TOLERANCE;

    let mut closest_edge = None;
    let mut min_distance_sq = TOLERANCE_SQ;

    for edge in &bucket.edge_data {
        let dist_sq = distance_to_segment(position.world_position, edge.start_position, edge.end_position);

        if dist_sq < min_distance_sq {
            min_distance_sq = dist_sq;
            closest_edge = Some(*edge);
        }
    }

    closest_edge
}

pub fn find_node_at_pos(position: &Position, spatial_grid: &SpatialFabricGrid) -> Option<Node> {
    let world_x = position.world_position.x;
    let world_y = position.world_position.y;

    let target_tile_id = match position.location {
        crate::core::TargetLocation::Outer(tile_id) => tile_id,
        crate::core::TargetLocation::Inner(tile_id, _) => tile_id,
        crate::core::TargetLocation::InnerEmpty(tile_id) => tile_id,
        crate::core::TargetLocation::None => return None,
    };

    let bucket = spatial_grid.buckets.get(&target_tile_id)?;

    const RADIUS: f64 = WIRE_NODE_RADIUS * 1.2;
    const RADIUS_SQ: f64 = (WIRE_NODE_RADIUS * 1.2) * (WIRE_NODE_RADIUS * 1.2);
    let min_x = world_x - RADIUS;
    let max_x = world_x + RADIUS;

    let start_idx = match bucket
        .node_data
        .binary_search_by(|Node { position, .. }| position.x.partial_cmp(&min_x).unwrap())
    {
        Ok(idx) => idx,
        Err(idx) => idx,
    };
    for node in &bucket.node_data[start_idx..] {
        let pos = node.position;
        if node.position.x > max_x {
            break;
        }
        let dx = world_x - pos.x;
        let dy = world_y - pos.y;
        let distance = dx * dx + dy * dy;

        if distance <= RADIUS_SQ {
            return Some(*node);
        }
    }

    None
}
pub fn find_entities_at_position(position: &Position, spatial_grid: &SpatialFabricGrid) -> Option<Entity> {
    find_node_at_pos(position, spatial_grid).map(Entity::Node).or_else(|| {
        find_edge_at_pos(position, spatial_grid).map(Entity::Edge).or_else(|| {
            find_lut_at_pos(position, spatial_grid)
                .map(Entity::Lut)
                .or_else(|| find_tile_at_pos(position, spatial_grid).map(Entity::Tile))
        })
    })
}
