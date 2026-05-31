use router::Port;
use vello::kurbo::Point;

use crate::{constants::EDGE_FOCUS_SCALER, core::Entity};
pub type EdgeId = usize;

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub id: EdgeId,
    pub source: (Entity, Port),
    pub target: (Entity, Port),
    pub start_position: Point,
    pub end_position: Point,
}
impl Edge {
    const FOCUS_SCALER: f64 = EDGE_FOCUS_SCALER;
    pub fn focus(&self) -> f64 {
        Self::FOCUS_SCALER / self.distance()
    }
    pub fn mid_point(&self) -> Point {
        self.start_position.midpoint(self.end_position)
    }
    pub fn distance(&self) -> f64 {
        self.start_position.distance(self.end_position)
    }
}
