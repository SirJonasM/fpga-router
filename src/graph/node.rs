//! Module `node`
//!
//! This module defines the building blocks of the FPGA fabric graph:
//! nodes, their types, and associated costs for routing algorithms.

use std::fmt::Display;

use crate::{FabricError, FabricResult, error::ParseError};

type NodeIdType = u16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub(super) NodeIdType);

impl NodeId {
    pub(super) fn new(id: usize) -> FabricResult<Self> {
        NodeIdType::try_from(id)
            .map(Self)
            .map_err(|_e| FabricError::NodeIdValueSpaceTooSmall)
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T> std::ops::Index<NodeId> for Vec<T> {
    type Output = T;
    fn index(&self, index: NodeId) -> &Self::Output {
        #[allow(clippy::cast_possible_truncation)]
        let index = index.0 as usize;
        &self[index]
    }
}

impl<T> std::ops::IndexMut<NodeId> for Vec<T> {
    fn index_mut(&mut self, index: NodeId) -> &mut Self::Output {
        #[allow(clippy::cast_possible_truncation)]
        let index = index.0 as usize;
        &mut self[index]
    }
}

/// Programmable Connectio between nodes
#[derive(Debug, Clone)]
pub struct Edge {
    /// Destination node index
    pub node_id: NodeId,
    /// Cost to traverse this edge
    pub cost: f32,
}

/// A node in the FPGA graph
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct Node {
    /// Unique identifier of the node
    pub id: String,
    /// X coordinate on the FPGA fabric
    pub x: u8,
    /// Y coordinate on the FPGA fabric
    pub y: u8,
}

/// Structure representing costs associated with routing through a node
#[derive(Clone, Debug)]
pub struct Costs {
    /// Historic cost accumulated over routing iterations
    pub historic_cost: f32,
    /// Maximum capacity of the node
    pub capacity: f32,
    /// Current usage count
    pub usage: u16,
}

impl Node {
    pub const fn new(id: String, x: u8, y: u8) -> Self {
        Self { id, x, y }
    }
    pub fn parse(id: &str, coords: &str) -> Result<Self, ParseError> {
        let (x, y) = from_str_coords(coords)?;
        Ok(Self::new(id.to_string(), x, y))
    }
    pub fn id(&self) -> String {
        format!("X{}Y{}.{}", self.x, self.y, self.id)
    }
}

/// Parse coordinates from a string of the form "X<num>Y<num>"
fn from_str_coords(s: &str) -> std::result::Result<(u8, u8), ParseError> {
    if !s.starts_with('X') {
        return Err(ParseError::MissingPrefix {
            prefix: 'X',
            token: s.to_string(),
        });
    }
    let (x_part, y_part) = s.split_once('Y').ok_or_else(|| ParseError::MissingPrefix {
        prefix: 'Y',
        token: s.to_string(),
    })?;
    let x = x_part[1..].parse::<u8>().map_err(|e| ParseError::InvalidCoordinate {
        component: "X",
        token: x_part[1..].to_string(),
        source: e,
    })?;
    let y = y_part.parse::<u8>().map_err(|e| ParseError::InvalidCoordinate {
        component: "Y",
        token: y_part.to_string(),
        source: e,
    })?;
    Ok((x, y))
}

impl Default for Costs {
    fn default() -> Self {
        Self {
            historic_cost: 0.0,
            capacity: 1.0,
            usage: 0,
        }
    }
}

impl Costs {
    /// Update the cost of the node based on usage and historic factor
    /// clears the `usage`
    ///
    /// Returns `true` if the node is congested (`usage` > `capacity`)
    pub fn update(&mut self, historic_factor: f32) -> bool {
        #[allow(clippy::cast_precision_loss)]
        let usage = f32::from(self.usage);
        let over_use = usage - self.capacity;

        if over_use > 0.0 {
            self.historic_cost += historic_factor * over_use;
        }
        self.usage = 0;
        over_use > 0.0
    }

    /// Calculate total cost for this node
    pub fn calc_costs(&self, base_cost: f32, criticallity: f32) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        let casted_usage = f32::from(self.usage);
        let congestion_cost = (1.0 + self.historic_cost) * (1.0 + casted_usage);

        criticallity.mul_add(base_cost, (1.0 - criticallity) * congestion_cost)
    }

    /// Create a new `Costs` object
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    const TOLERANCE: f32 = 0.001;
    #[test]
    fn test_update_costs_uncongested() {
        let mut costs = Costs {
            usage: 1,
            historic_cost: 2.0,
            ..Default::default()
        };
        let c = costs.update(1.0);
        assert!(!c);
    }

    #[test]
    fn test_update_costs_congested() {
        let mut costs = Costs {
            usage: 2,
            historic_cost: 0.0,
            ..Default::default()
        };
        let updated_costs = Costs {
            usage: 0,
            historic_cost: 1.0,
            ..Default::default()
        };
        let c = costs.update(1.0);
        assert!(c);
        assert!((updated_costs.historic_cost - costs.historic_cost).abs() < TOLERANCE);
        assert_eq!(updated_costs.usage, costs.usage);
        assert!((updated_costs.capacity - costs.capacity).abs() < TOLERANCE);
    }

    #[test]
    fn test_calculate_costs() {
        let costs = Costs {
            usage: 1,
            historic_cost: 2.0,
            ..Default::default()
        };
        let c = costs.calc_costs(1.0, 0.0);
        assert!((c - 6.0).abs() < TOLERANCE);
    }

    #[test]
    fn test_parse_node() {
        let node_id = "Test";
        let node_cords = "X1Y2";
        let node_expected = Node {
            id: "Test".to_string(),
            x: 1,
            y: 2,
        };
        let node = Node::parse(node_id, node_cords).unwrap();
        assert_eq!(node, node_expected);
    }
    #[test]
    fn test_from_str_coords_fail_x() {
        let node_id = "Test";
        let node_cords = "XpY2";
        let error_message = "Failed to parse 'X' coordinate: p".to_string();
        let result = Node::parse(node_id, node_cords).unwrap_err().to_string();
        assert_eq!(error_message, result);
    }
    #[test]
    fn test_from_str_coords_fail_y() {
        let node_id = "Test";
        let node_cords = "X1Yp";
        let error_message = "Failed to parse 'Y' coordinate: p".to_string();
        let result = Node::parse(node_id, node_cords).unwrap_err().to_string();
        assert_eq!(error_message, result);
    }
}
