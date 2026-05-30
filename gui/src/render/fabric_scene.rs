use crate::constants::*;
use crate::core::Entity;
use crate::core::NodeMetadata;
use crate::core::SpatialFabricGrid;
use crate::core::VisibleTileRange;
use vello::Scene;
use vello::peniko::Color;

pub fn fabric_highlight_scene(spatial_grid: &SpatialFabricGrid, selection: Entity) -> Scene {
    let mut scene = vello::Scene::new();

    match selection {
        Entity::Tile(tile) => {
            if let Some(tile) = spatial_grid.get_tile(tile) {
                let highlighting = TileHighlightingConfig {
                    outer_line_color: Color::DARK_RED,
                    outer_line_width: TILE_OUTER_LINE_WIDTH,
                    inner_line_color: Color::RED,
                    inner_line_width: TILE_INNER_LINE_WIDTH,
                };
                draw_tile_base(tile, &mut scene, &highlighting)
            }
        }
        Entity::Lut(lut) => {
            if let Some(lut) = spatial_grid.get_lut(lut) {
                let highlighting = LutHighlightingConfig {
                    line_color: Color::RED,
                    line_width: LUT_LINE_WIDTH,
                };
                draw_lut_base(lut, &mut scene, &highlighting)
            }
        }
        Entity::Node(node) => {
            if let Some(node) = spatial_grid.get_node(node) {
                let highlighting = NodeHighlightingConfig {
                    fill_color: Color::RED,
                    radius: WIRE_NODE_RADIUS * 1.2,
                };
                draw_node_base(node, &mut scene, &highlighting);
                for edge in spatial_grid.get_outgoing_edges(node.id) {
                    let highlighting = EdgeHighlightingConfig {
                        line_highlighting: EdgeHighlighting::Solid(INCOMING_EDGE_COLOR),
                        line_width: WIRE_LINE_WIDTH * 2.0,
                    };
                    draw_edge_base(edge, &mut scene, &highlighting);
                }
                for edge in spatial_grid.get_incoming_edges(node.id) {
                    let highlighting = EdgeHighlightingConfig {
                        line_highlighting: EdgeHighlighting::Solid(OUTGOING_EDGE_COLOR),
                        line_width: WIRE_LINE_WIDTH * 2.0,
                    };
                    draw_edge_base(edge, &mut scene, &highlighting);
                }
            }
        }
        Entity::Edge(edge) => {
            if let Some(edge) = spatial_grid.get_edge(edge)
                && let Some(source_node) = spatial_grid.get_node(edge.source_node)
                && let Some(target_node) = spatial_grid.get_node(edge.target_node)
            {
                let highlighting = EdgeHighlightingConfig {
                    line_highlighting: EdgeHighlighting::Solid(Color::WHITE),
                    line_width: WIRE_LINE_WIDTH * 2.0,
                };
                draw_edge_base(edge, &mut scene, &highlighting);
                let mut highlighting = NodeHighlightingConfig {
                    fill_color: Color::YELLOW,
                    radius: WIRE_NODE_RADIUS * 1.2,
                };
                draw_node_base(source_node, &mut scene, &highlighting);
                highlighting.fill_color = Color::RED;
                draw_node_base(target_node, &mut scene, &highlighting);
            }
        }
    }
    scene
}
pub fn fabric_base_scene(
    spatial_grid: &SpatialFabricGrid,
    visible_range: VisibleTileRange,
    scale: f64,
    is_moving: bool,
) -> vello::Scene {
    let mut scene = vello::Scene::new();

    let draw_luts = scale > LUT_ZOOM_THRESHOLD;
    let draw_edges = (!is_moving && scale > EDGE_ZOOM_THRESHOLD_UNDER_MOVING) || scale > EDGE_ZOOM_THRESHOLD;
    let draw_nodes = (!is_moving && scale > NODE_ZOOM_THRESHOLD_UNDER_MOVING) || scale > NODE_ZOOM_THRESHOLD;

    let tile_highlighting = TileHighlightingConfig::default();
    let lut_highlighting = LutHighlightingConfig::default();
    let edge_highlighting = EdgeHighlightingConfig::default();
    let node_highlighting = NodeHighlightingConfig::default();
    visible_range
        .into_iter()
        .filter_map(|tile_id| spatial_grid.buckets.get(&tile_id))
        .for_each(|bucket| {
            if let Some(tile) = spatial_grid.get_tile(bucket.tile_data) {
                draw_tile_base(tile, &mut scene, &tile_highlighting);
            }

            if draw_luts {
                for lut in &bucket.lut_data {
                    if let Some(lut) = spatial_grid.get_lut(*lut) {
                        draw_lut_base(lut, &mut scene, &lut_highlighting);
                    }
                }
            }

            if draw_edges {
                for edge in &bucket.edge_data {
                    if let Some(edge) = spatial_grid.get_edge(*edge) {
                        draw_edge_base(edge, &mut scene, &edge_highlighting);
                    }
                }
            }

            if draw_nodes {
                for node in &bucket.node_data {
                    if let Some(node) = spatial_grid.get_node(*node) {
                        draw_node_base(node, &mut scene, &node_highlighting);
                    }
                }
            }
        });
    scene
}

struct TileHighlightingConfig {
    pub outer_line_color: Color,
    pub outer_line_width: f64,
    pub inner_line_color: Color,
    pub inner_line_width: f64,
}

impl Default for TileHighlightingConfig {
    fn default() -> Self {
        Self {
            outer_line_color: Color::rgb8(25, 25, 25),
            outer_line_width: TILE_OUTER_LINE_WIDTH,
            inner_line_color: Color::rgb8(25, 25, 25),
            inner_line_width: TILE_INNER_LINE_WIDTH,
        }
    }
}
#[inline(always)]
fn draw_tile_base(tile: &crate::core::Tile, scene: &mut Scene, highlighting: &TileHighlightingConfig) {
    let rect_outer = vello::kurbo::Rect::new(
        tile.position_outer.x,
        tile.position_outer.y,
        tile.position_outer.x + TILE_WIDTH,
        tile.position_outer.y + TILE_HEIGHT,
    );
    scene.stroke(
        &vello::kurbo::Stroke::new(highlighting.outer_line_width),
        vello::kurbo::Affine::IDENTITY,
        highlighting.outer_line_color,
        None,
        &rect_outer,
    );

    let rect_inner = vello::kurbo::Rect::new(
        tile.position_inner.x,
        tile.position_inner.y,
        tile.position_inner.x + TILE_BOUNDING_BOX_WIDTH,
        tile.position_inner.y + TILE_BOUNDING_BOX_HEIGHT,
    );
    scene.stroke(
        &vello::kurbo::Stroke::new(highlighting.inner_line_width),
        vello::kurbo::Affine::IDENTITY,
        highlighting.inner_line_color,
        None,
        &rect_inner,
    );
}

#[inline(always)]
fn draw_edge_base(edge: &crate::core::Edge, scene: &mut Scene, highlighting: &EdgeHighlightingConfig) {
    let line = vello::kurbo::Line::new(edge.start_position, edge.end_position);
    match highlighting.line_highlighting {
        EdgeHighlighting::Gradient { start_color, end_color } => {
            let brush = vello::peniko::Gradient::new_linear(edge.start_position, edge.end_position)
                .with_stops([(0.0, start_color), (1.0, end_color)].as_slice());
            scene.stroke(
                &vello::kurbo::Stroke::new(highlighting.line_width),
                vello::kurbo::Affine::IDENTITY,
                &brush,
                None,
                &line,
            );
        }
        EdgeHighlighting::Solid(color) => {
            scene.stroke(
                &vello::kurbo::Stroke::new(highlighting.line_width),
                vello::kurbo::Affine::IDENTITY,
                color,
                None,
                &line,
            );
        }
    }
}

#[inline(always)]
fn draw_node_base(node: &NodeMetadata, scene: &mut Scene, highlighting: &NodeHighlightingConfig) {
    let circle = vello::kurbo::Circle::new(node.position, highlighting.radius);
    scene.fill(
        vello::peniko::Fill::NonZero,
        vello::kurbo::Affine::IDENTITY,
        highlighting.fill_color,
        None,
        &circle,
    );
}

#[inline(always)]
fn draw_lut_base(lut: &crate::core::Lut, scene: &mut Scene, highlighting: &LutHighlightingConfig) {
    let lut_rect = vello::kurbo::Rect::new(
        lut.position.x,
        lut.position.y,
        lut.position.x + LUT_WIDTH,
        lut.position.y + LUT_HEIGHT,
    );
    scene.stroke(
        &vello::kurbo::Stroke::new(highlighting.line_width),
        vello::kurbo::Affine::IDENTITY,
        highlighting.line_color,
        None,
        &lut_rect,
    );
}

enum EdgeHighlighting {
    Gradient { start_color: Color, end_color: Color },
    Solid(Color),
}
pub struct EdgeHighlightingConfig {
    line_highlighting: EdgeHighlighting,
    line_width: f64,
}

impl Default for EdgeHighlightingConfig {
    fn default() -> Self {
        Self {
            line_highlighting: EdgeHighlighting::Gradient {
                start_color: COLOR_EDGE_START,
                end_color: COLOR_EDGE_END,
            },
            line_width: WIRE_LINE_WIDTH,
        }
    }
}

pub struct NodeHighlightingConfig {
    pub fill_color: Color,
    pub radius: f64,
}

impl Default for NodeHighlightingConfig {
    fn default() -> Self {
        Self {
            fill_color: Color::rgb8(38, 139, 210),
            radius: WIRE_NODE_RADIUS,
        }
    }
}

pub struct LutHighlightingConfig {
    pub line_color: Color,
    pub line_width: f64,
}

impl Default for LutHighlightingConfig {
    fn default() -> Self {
        Self {
            line_color: Color::rgb8(100, 100, 110),
            line_width: LUT_LINE_WIDTH,
        }
    }
}
