use crate::constants::*;
use router::NodeId;
use router::{TileId, TileManager};
use std::collections::{HashMap, HashSet};
use vello::{
    Scene,
    kurbo::{Affine, Rect},
    peniko::{Color, Fill},
};

use crate::layout::LayoutBuilder;

pub fn build_fabric_scene(
    tile_manager: &TileManager,
    spatial_grid: &SpatialFabricGrid,
    visible_range: &VisibleTileRange,
    scale: f64,
    is_moving: bool,
    selected_node: Option<NodeId>,
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
        draw_visible_nodes(visible_range, spatial_grid, selected_node, &mut scene);
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
    selected_node: Option<NodeId>,
    scene: &mut vello::Scene,
) {
    for x in visible_range.min_x..=visible_range.max_x {
        for y in visible_range.min_y..=visible_range.max_y {
            if x < 0 || y < 0 {
                continue;
            }
            let tile_id = TileId(x as u8, y as u8);
            if let Some(bucket) = spatial_grid.buckets.get(&tile_id) {
                bucket.node_data.iter().for_each(|(id, position)| {
                    let color = if Some(*id) == selected_node {
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

                    let line = vello::kurbo::Line::new(edge.start_pos, edge.end_pos);

                    let gradient = vello::peniko::Gradient::new_linear(edge.start_pos, edge.end_pos)
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

#[derive(Default)]
pub struct SpatialFabricGrid {
    // Keys are the structural Tile locations (e.g., TileId(x, y))
    pub buckets: HashMap<TileId, TileBucket>,
}
#[derive(Debug)]
pub struct TileBucket {
    // Left top edge of TILE
    pub tile_data: vello::kurbo::Point,
    // Left top edge of LUT
    pub lut_data: Vec<(char, vello::kurbo::Point)>,
    pub node_data: Vec<(NodeId, vello::kurbo::Point)>,
    pub edge_data: Vec<SpatialEdgeData>,
}

#[derive(Copy, Clone, Debug)]
pub struct SpatialEdgeData {
    pub source_node: NodeId,
    pub target_node: NodeId,
    pub start_pos: vello::kurbo::Point,
    pub end_pos: vello::kurbo::Point,
}

impl SpatialFabricGrid {
    pub fn build_from_graph(graph: &router::FabricGraph, tile_manager: &router::TileManager) -> Self {
        let mut grid = Self::default();
        for tile in tile_manager.0.values() {
            let pos = LayoutBuilder::new().tile(&tile.id).build();

            let bucket = grid.buckets.entry(tile.id).or_insert_with(|| TileBucket {
                tile_data: pos,
                lut_data: Vec::new(),
                node_data: Vec::new(),
                edge_data: Vec::new(),
            });
            let mut lut_list = vec![];
            for lut in &tile.luts {
                let pos = LayoutBuilder::new().tile(&tile.id).tile_inner().lut(lut.bel_index).build();
                lut_list.push((lut.bel_index, pos))
            }
            bucket.lut_data = lut_list;
        }

        for node in graph.nodes.iter() {
            if let Some(pos) = crate::layout::get_node_pos(node) {
                let bucket = grid.buckets.get_mut(&node.tile).expect("Error in pips and bel definition.");
                let node_id = graph.get_node_id(&node.id()).unwrap();
                bucket.node_data.push((*node_id, pos));
            }
        }
        for (start_id, edge) in graph.edges() {
            let start_node = graph.get_node(start_id);
            let end_node = graph.get_node(edge.node_id);
            if let Some(pos1) = crate::layout::get_node_pos(start_node)
                && let Some(pos2) = crate::layout::get_node_pos(end_node)
            {
                let edge_data = SpatialEdgeData {
                    source_node: start_id,
                    target_node: edge.node_id,
                    start_pos: pos1,
                    end_pos: pos2,
                };
                let crossed_tiles = get_tiles_intersected_by_line(pos1, pos2);

                for tile_id in crossed_tiles {
                    let bucket = grid.buckets.get_mut(&tile_id).expect("Error in pips and bel definition.");
                    bucket.edge_data.push(edge_data);
                }
            }
        }

        for arr in grid.buckets.values_mut() {
            arr.node_data.sort_unstable_by(|a, b| a.1.x.total_cmp(&b.1.x));
        }

        grid
    }
}
/// Calculates all TileIds that a line segment crosses between pos1 and pos2
pub fn get_tiles_intersected_by_line(pos1: vello::kurbo::Point, pos2: vello::kurbo::Point) -> Vec<TileId> {
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

pub struct VisibleTileRange {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

pub fn calculate_visible_tiles(world_min: vello::kurbo::Point, world_max: vello::kurbo::Point) -> VisibleTileRange {
    let tile_step_x = TILE_WIDTH;
    let tile_step_y = TILE_HEIGHT;

    VisibleTileRange {
        min_x: (world_min.x / tile_step_x).floor() as i32,
        max_x: (world_max.x / tile_step_x).ceil() as i32,
        min_y: (world_min.y / tile_step_y).floor() as i32,
        max_y: (world_max.y / tile_step_y).ceil() as i32,
    }
}

pub fn render_placeholder_vello(scene: &mut Scene, viewport: &egui::Rect) {
    let offset_x = viewport.min.x as f64;
    let offset_y = viewport.min.y as f64;

    let rect = Rect::new(offset_x + 500.0, offset_y + 500.0, offset_x + 750.0, offset_y + 750.0);

    scene.fill(Fill::NonZero, Affine::IDENTITY, Color::rgb8(255, 255, 255), None, &rect);

    let circle = vello::kurbo::Circle::new((offset_x + 450.0, offset_y + 120.0), 70.0);

    scene.fill(Fill::NonZero, Affine::IDENTITY, Color::rgb8(38, 139, 210), None, &circle);

    let mut triangle = vello::kurbo::BezPath::new();

    triangle.move_to((offset_x + 200.0, offset_y + 260.0));
    triangle.line_to((offset_x + 450.0, offset_y + 500.0));
    triangle.line_to((offset_x + 50.0, offset_y + 500.0));
    triangle.close_path();

    scene.fill(Fill::NonZero, Affine::IDENTITY, Color::rgb8(133, 153, 0), None, &triangle);
}
