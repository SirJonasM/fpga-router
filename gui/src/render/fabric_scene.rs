use crate::constants::*;
use crate::core::Entity;
use crate::core::SpatialFabricGrid;
use crate::core::VisibleTileRange;
use router::TileId;
use std::collections::HashSet;
use vello::Scene;

pub fn fabric_scene(
    spatial_grid: &SpatialFabricGrid,
    visible_range: &VisibleTileRange,
    scale: f64,
    is_moving: bool,
    selected_entity: Option<Entity>,
) -> vello::Scene {
    let mut scene = vello::Scene::new();
    draw_visible_tiles(visible_range, spatial_grid, selected_entity, &mut scene);
    if scale > LUT_ZOOM_THRESHOLD {
        draw_visible_luts(visible_range, spatial_grid, selected_entity, &mut scene);
    }
    if (!is_moving && scale > EDGE_ZOOM_THRESHOLD_UNDER_MOVING) || scale > EDGE_ZOOM_THRESHOLD {
        draw_visible_edges(visible_range, spatial_grid, selected_entity, &mut scene);
    }
    if (!is_moving && scale > NODE_ZOOM_THRESHOLD_UNDER_MOVING) || scale > NODE_ZOOM_THRESHOLD {
        draw_visible_nodes(visible_range, spatial_grid, selected_entity, &mut scene);
    }

    scene
}
fn draw_visible_tiles(
    visible_range: &VisibleTileRange,
    spatial_grid: &SpatialFabricGrid,
    selected_entity: Option<Entity>,
    scene: &mut Scene,
) {
    for x in visible_range.min_x..=visible_range.max_x {
        for y in visible_range.min_y..=visible_range.max_y {
            if x < 0 || y < 0 {
                continue;
            }
            let tile_id = TileId(x as u8, y as u8);
            let Some(buckets) = spatial_grid.buckets.get(&tile_id) else {
                continue;
            };
            let tile = buckets.tile_data;
            let (color1, color2) = if let Some(Entity::Tile(selected_entity)) = selected_entity
                && selected_entity.id == tile_id
            {
                (vello::peniko::Color::RED, vello::peniko::Color::DARK_RED)
            } else {
                (vello::peniko::Color::rgb8(25, 25, 25), vello::peniko::Color::rgb8(25, 25, 25))
            };
            let rect = vello::kurbo::Rect::new(
                tile.position_outer.x,
                tile.position_outer.y,
                tile.position_outer.x + TILE_WIDTH,
                tile.position_outer.y + TILE_HEIGHT,
            );
            scene.stroke(
                &vello::kurbo::Stroke::new(TILE_OUTER_LINE_WIDTH),
                vello::kurbo::Affine::IDENTITY,
                color1,
                None,
                &rect,
            );
            let rect = vello::kurbo::Rect::new(
                tile.position_inner.x,
                tile.position_inner.y,
                tile.position_inner.x + TILE_BOUNDING_BOX_WIDTH,
                tile.position_inner.y + TILE_BOUNDING_BOX_HEIGHT,
            );
            scene.stroke(
                &vello::kurbo::Stroke::new(TILE_INNER_LINE_WIDTH),
                vello::kurbo::Affine::IDENTITY,
                color2,
                None,
                &rect,
            );
        }
    }
}
fn draw_visible_nodes(
    visible_range: &VisibleTileRange,
    spatial_grid: &SpatialFabricGrid,
    selected_entity: Option<Entity>,
    scene: &mut vello::Scene,
) {
    for x in visible_range.min_x..=visible_range.max_x {
        for y in visible_range.min_y..=visible_range.max_y {
            if x < 0 || y < 0 {
                continue;
            }
            let tile_id = TileId(x as u8, y as u8);
            if let Some(bucket) = spatial_grid.buckets.get(&tile_id) {
                bucket.node_data.iter().for_each(|crate::core::Node { id, position }| {
                    let color = if let Some(Entity::Node(node)) = selected_entity
                        && node.id == *id
                    {
                        vello::peniko::Color::RED
                    } else {
                        vello::peniko::Color::rgb8(38, 139, 210)
                    };
                    let circle = vello::kurbo::Circle::new(*position, WIRE_NODE_RADIUS);
                    scene.fill(
                        vello::peniko::Fill::NonZero,
                        vello::kurbo::Affine::IDENTITY,
                        color,
                        None,
                        &circle,
                    );
                })
            }
        }
    }
}

fn draw_visible_edges(
    visible_range: &VisibleTileRange,
    spatial_grid: &SpatialFabricGrid,
    selected_entity: Option<Entity>,
    scene: &mut vello::Scene,
) {
    let mut drawn_edges = HashSet::new();
    for x in visible_range.min_x..=visible_range.max_x {
        for y in visible_range.min_y..=visible_range.max_y {
            if x < 0 || y < 0 {
                continue;
            }
            let tile_id = TileId(x as u8, y as u8);

            if let Some(bucket) = spatial_grid.buckets.get(&tile_id) {
                for edge in &bucket.edge_data {
                    let edge_key = (edge.source_node, edge.target_node);
                    if !drawn_edges.insert(edge_key) {
                        continue;
                    }

                    let line = vello::kurbo::Line::new(edge.start_position, edge.end_position);

                    // Determine the brush (color/gradient) for this specific edge
                    let brush = match selected_entity {
                        // Case 1: An Edge is explicitly selected (Keep original logic)
                        Some(Entity::Edge(selected_edge)) if *edge == selected_edge => {
                            vello::peniko::Brush::Solid(vello::peniko::Color::WHITE)
                        }

                        // Case 2: A Node is selected -> Highlight its connected edges
                        Some(Entity::Node(selected_node)) => {
                            if edge.source_node == selected_node.id {
                                // Outgoing edge from the selected node
                                vello::peniko::Brush::Solid(OUTGOING_EDGE_COLOR)
                            } else if edge.target_node == selected_node.id {
                                // Incoming edge to the selected node
                                vello::peniko::Brush::Solid(INCOMING_EDGE_COLOR)
                            } else {
                                // Node is selected, but this edge isn't connected to it
                                let gradient = vello::peniko::Gradient::new_linear(edge.start_position, edge.end_position)
                                    .with_stops([(0.0, COLOR_EDGE_START), (1.0, COLOR_EDGE_END)].as_slice());
                                vello::peniko::Brush::Gradient(gradient)
                            }
                        }

                        // Case 3: Default fallback (No selection, or non-matching edge/node)
                        _ => {
                            let gradient = vello::peniko::Gradient::new_linear(edge.start_position, edge.end_position)
                                .with_stops([(0.0, COLOR_EDGE_START), (1.0, COLOR_EDGE_END)].as_slice());
                            vello::peniko::Brush::Gradient(gradient)
                        }
                    };

                    // Determine line width based on selection status
                    let width = match selected_entity {
                        Some(Entity::Edge(selected_edge)) if *edge == selected_edge => SELECTED_WIRE_LINE_WIDTH,
                        Some(Entity::Node(selected_node))
                            if edge.source_node == selected_node.id || edge.target_node == selected_node.id =>
                        {
                            // Optional: Make highlighted node edges thicker too.
                            // Change to WIRE_LINE_WIDTH if you want them to remain normal thickness.
                            SELECTED_WIRE_LINE_WIDTH
                        }
                        _ => WIRE_LINE_WIDTH,
                    };

                    // Render the stroke with the calculated properties
                    scene.stroke(
                        &vello::kurbo::Stroke::new(width),
                        vello::kurbo::Affine::IDENTITY,
                        &brush,
                        None,
                        &line,
                    );
                }
            }
        }
    }
}
fn draw_visible_luts(
    visible_range: &VisibleTileRange,
    spatial_grid: &SpatialFabricGrid,
    selected_entity: Option<Entity>,
    scene: &mut Scene,
) {
    for x in visible_range.min_x..=visible_range.max_x {
        for y in visible_range.min_y..=visible_range.max_y {
            if x < 0 || y < 0 {
                continue;
            }

            let tile_id = TileId(x as u8, y as u8);
            let Some(buckets) = spatial_grid.buckets.get(&tile_id) else {
                continue;
            };
            buckets.lut_data.iter().for_each(|lut| {
                let lut_rect = vello::kurbo::Rect::new(
                    lut.position.x,
                    lut.position.y,
                    lut.position.x + LUT_WIDTH,
                    lut.position.y + LUT_HEIGHT,
                );
                let color = if let Some(Entity::Lut(selected_lut)) = selected_entity
                    && selected_lut.bel_index == lut.bel_index
                    && selected_lut.tile == lut.tile
                {
                    vello::peniko::Color::RED
                } else {
                    vello::peniko::Color::rgb8(100, 100, 110)
                };
                scene.stroke(
                    &vello::kurbo::Stroke::new(LUT_LINE_WIDTH),
                    vello::kurbo::Affine::IDENTITY,
                    color,
                    None,
                    &lut_rect,
                );
            })
        }
    }
}
