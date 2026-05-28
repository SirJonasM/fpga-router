use crate::constants::*;
use crate::core::Entity;
use crate::core::LayoutBuilder;
use crate::core::SpatialFabricGrid;
use crate::core::VisibleTileRange;
use router::NodeId;
use router::{TileId, TileManager};
use std::collections::HashSet;
use vello::Scene;

pub fn fabric_scene(
    tile_manager: &TileManager,
    spatial_grid: &SpatialFabricGrid,
    visible_range: &VisibleTileRange,
    scale: f64,
    is_moving: bool,
    selected_entity: &Option<Entity>,
    selected_edge: Option<(NodeId, NodeId)>,
) -> vello::Scene {
    let mut scene = vello::Scene::new();
    draw_visible_tiles(visible_range, tile_manager, &mut scene);
    if scale > LUT_ZOOM_THRESHOLD {
        draw_visible_luts(visible_range, tile_manager, &mut scene);
    }
    if (!is_moving && scale > EDGE_ZOOM_THRESHOLD_UNDER_MOVING) || scale > EDGE_ZOOM_THRESHOLD {
        draw_visible_edges(visible_range, spatial_grid, selected_edge, &mut scene);
    }
    if (!is_moving && scale > NODE_ZOOM_THRESHOLD_UNDER_MOVING) || scale > NODE_ZOOM_THRESHOLD {
        draw_visible_nodes(visible_range, spatial_grid, selected_entity, &mut scene);
    }

    scene
}
fn draw_visible_tiles(visible_range: &VisibleTileRange, tile_manager: &TileManager, scene: &mut Scene) {
    for x in visible_range.min_x..=visible_range.max_x {
        for y in visible_range.min_y..=visible_range.max_y {
            if x < 0 || y < 0 {
                continue;
            }
            let tile_id = TileId(x as u8, y as u8);
            if !tile_manager.0.contains_key(&tile_id) {
                continue;
            };
            let cord = LayoutBuilder::new().tile(&tile_id);
            let rect = vello::kurbo::Rect::new(cord.x, cord.y, cord.x + TILE_WIDTH, cord.y + TILE_HEIGHT);
            scene.stroke(
                &vello::kurbo::Stroke::new(TILE_OUTER_LINE_WIDTH),
                vello::kurbo::Affine::IDENTITY,
                vello::peniko::Color::rgb8(25, 25, 25),
                None,
                &rect,
            );
            let cord = cord.tile_inner();
            let rect = vello::kurbo::Rect::new(
                cord.x,
                cord.y,
                cord.x + TILE_BOUNDING_BOX_WIDTH,
                cord.y + TILE_BOUNDING_BOX_HEIGHT,
            );
            scene.stroke(
                &vello::kurbo::Stroke::new(TILE_INNER_LINE_WIDTH),
                vello::kurbo::Affine::IDENTITY,
                vello::peniko::Color::rgb8(25, 25, 25),
                None,
                &rect,
            );
        }
    }
}
fn draw_visible_nodes(
    visible_range: &VisibleTileRange,
    spatial_grid: &SpatialFabricGrid,
    selected_entity: &Option<Entity>,
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
    selected_edge: Option<(NodeId, NodeId)>,
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

                    let gradient = vello::peniko::Gradient::new_linear(edge.start_position, edge.end_position)
                        .with_stops([(0.0, COLOR_EDGE_START), (1.0, COLOR_EDGE_END)].as_slice());
                    if Some((edge.source_node, edge.target_node)) == selected_edge {
                        scene.stroke(
                            &vello::kurbo::Stroke::new(SELECTED_WIRE_LINE_WIDTH),
                            vello::kurbo::Affine::IDENTITY,
                            vello::peniko::Color::WHITE,
                            None,
                            &line,
                        );
                    } else {
                        scene.stroke(
                            &vello::kurbo::Stroke::new(WIRE_LINE_WIDTH),
                            vello::kurbo::Affine::IDENTITY,
                            &vello::peniko::Brush::Gradient(gradient),
                            None,
                            &line,
                        );
                    };
                }
            }
        }
    }
}
fn draw_visible_luts(visible_range: &VisibleTileRange, tile_manager: &TileManager, scene: &mut Scene) {
    for x in visible_range.min_x..=visible_range.max_x {
        for y in visible_range.min_y..=visible_range.max_y {
            if x < 0 || y < 0 {
                continue;
            }
            let tile_id = TileId(x as u8, y as u8);
            let Some(tile) = tile_manager.0.get(&tile_id) else {
                continue;
            };
            let cord = LayoutBuilder::new().tile(&tile_id).tile_inner();

            tile.luts.iter().for_each(|lut| {
                let lut = cord.lut(lut.bel_index);
                let lut_rect = vello::kurbo::Rect::new(lut.x, lut.y, lut.x + LUT_WIDTH, lut.y + LUT_HEIGHT);

                scene.stroke(
                    &vello::kurbo::Stroke::new(LUT_LINE_WIDTH),
                    vello::kurbo::Affine::IDENTITY,
                    vello::peniko::Color::rgb8(100, 100, 110),
                    None,
                    &lut_rect,
                );
            })
        }
    }
}
