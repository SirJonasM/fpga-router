use std::collections::HashMap;

use crate::constants::*;
use router::{FabricGraph, TileId, TileManager};
use vello::{
    kurbo::{Affine, Rect},
    peniko::{Color, Fill},
    Scene,
};

use crate::{
    app::ViewTransform,
    layout::{
        LayoutBuilder, 
    },
};


pub fn build_fabric_scene(
    graph: &FabricGraph,
    tile_manager: &TileManager,
    spatial_grid: &SpatialFabricGrid,
    visible_range: &VisibleTileRange,
    scale: f64,
) -> vello::Scene {
    let mut scene = vello::Scene::new();
    let cord = LayoutBuilder::new();

    for x in visible_range.min_x..=visible_range.max_x {
        for y in visible_range.min_y..=visible_range.max_y {
            if x < 0 || y < 0 {
                continue;
            }

            let tile_id = TileId(x as u8, y as u8);

            if let Some(tile) = tile_manager.0.get(&tile_id) {
                let cord = cord.tile(&tile_id);

                let rect = vello::kurbo::Rect::new(cord.x, cord.y, cord.x + TILE_WIDTH, cord.y + TILE_HEIGHT);
                scene.stroke(
                    &vello::kurbo::Stroke::new(TILE_LINE_WIDTH),
                    vello::kurbo::Affine::IDENTITY,
                    vello::peniko::Color::rgb8(25, 25, 25),
                    None,
                    &rect,
                );

                if scale > LUT_ZOOM_THRESHOLD {
                    for lut in tile.luts.iter() {
                        let lut = cord.lut(lut.bel_index);
                        let lut_rect = vello::kurbo::Rect::new(lut.x, lut.y, lut.x + LUT_WIDTH, lut.y + LUT_HEIGHT);

                        scene.stroke(
                            &vello::kurbo::Stroke::new(LUT_LINE_WIDTH),
                            vello::kurbo::Affine::IDENTITY,
                            vello::peniko::Color::rgb8(100, 100, 110),
                            None,
                            &lut_rect,
                        );
                    }
                }

                if let Some(bucket) = spatial_grid.buckets.get(&tile_id) {
                    if scale > EDGE_ZOOM_THRESHOLD {
                        for &(node_id, position) in &bucket.node_data {
                            let edges = &graph.map[node_id];
                            for edge in edges {
                                let end_node = graph.get_node(edge.node_id);
                                if let Some(position_end) = crate::layout::get_node_pos(end_node) {
                                    let line = vello::kurbo::Line::new(position, position_end);
                                    scene.stroke(
                                        &vello::kurbo::Stroke::new(WIRE_LINE_WIDTH),
                                        vello::kurbo::Affine::IDENTITY,
                                        vello::peniko::Color::rgb8(20, 20, 20),
                                        None,
                                        &line,
                                    );
                                }
                            }
                        }
                    }

                    if scale > NODE_ZOOM_THRESHOLD {
                        for &(_, position) in &bucket.node_data {
                            let circle = vello::kurbo::Circle::new(position, 0.1);
                            scene.fill(
                                vello::peniko::Fill::NonZero,
                                vello::kurbo::Affine::IDENTITY,
                                vello::peniko::Color::rgb8(38, 139, 210),
                                None,
                                &circle,
                            );
                        }
                    }
                }
            }
        }
    }

    scene
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

#[derive(Default)]
pub struct SpatialFabricGrid {
    // Keys are the structural Tile locations (e.g., TileId(x, y))
    pub buckets: HashMap<TileId, TileBucket>,
}

pub struct TileBucket {
    /// Stores tuples of (original_node_id, cached_pixel_position)
    pub node_data: Vec<(usize, vello::kurbo::Point)>,
}

impl SpatialFabricGrid {
    pub fn build_from_graph(graph: &router::FabricGraph) -> Self {
        let mut grid = Self::default();

        for (id, node) in graph.nodes.iter().enumerate() {
            if let Some(pos) = crate::layout::get_node_pos(node) {
                let bucket = grid
                    .buckets
                    .entry(node.tile)
                    .or_insert_with(|| TileBucket { node_data: Vec::new() });
                bucket.node_data.push((id, pos));
            }
        }
        grid
    }
}

pub struct VisibleTileRange {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

pub fn calculate_visible_tiles(viewport: egui::Rect, transform: &ViewTransform) -> VisibleTileRange {
    // 1. Get the screen screen boundary points
    let screen_min_x = viewport.min.x as f64;
    let screen_min_y = viewport.min.y as f64;
    let screen_max_x = viewport.max.x as f64;
    let screen_max_y = viewport.max.y as f64;

    // 2. Reverse the pan and zoom scaling to transform back into World Coordinates
    let world_min_x = (screen_min_x - transform.pan.x) / transform.scale;
    let world_min_y = (screen_min_y - transform.pan.y) / transform.scale;
    let world_max_x = (screen_max_x - transform.pan.x) / transform.scale;
    let world_max_y = (screen_max_y - transform.pan.y) / transform.scale;

    // 3. Determine the tile index boundaries based on tile sizes + layout padding
    let tile_step_x = TILE_WIDTH + TILE_PADDING;
    let tile_step_y = TILE_HEIGHT + TILE_PADDING;

    VisibleTileRange {
        min_x: (world_min_x / tile_step_x).floor() as i32,
        max_x: (world_max_x / tile_step_x).ceil() as i32,
        min_y: (world_min_y / tile_step_y).floor() as i32,
        max_y: (world_max_y / tile_step_y).ceil() as i32,
    }
}
