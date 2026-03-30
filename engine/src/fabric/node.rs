//! Module `node`
//!
//! This module defines the building blocks of the FPGA fabric graph:
//! nodes, their types, and associated costs for routing algorithms.

use std::fmt::Display;
use std::cmp::Ordering;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use super::error::ParseError;
use crate::FabricGraph;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub(super) NodeIdType);

impl NodeId {
    pub(super) fn new(id: usize) -> Self {
        let x = NodeIdType::try_from(id)
            .expect("The id space is too small to create this NodeId. Try building the engine with a internal NodeId");
        Self(x)
    }
    pub(crate) fn as_node(self, graph: &FabricGraph) -> Node {
        graph.get_node(self).clone()
    }
    pub(crate) fn name(self, graph: &FabricGraph) -> String {
        graph.get_node(self).id()
    }
}

impl<T> std::ops::Index<NodeId> for Vec<T> {
    type Output = T;
    fn index(&self, index: NodeId) -> &Self::Output {
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::unnecessary_cast)]
        let index = index.0 as usize;
        &self[index]
    }
}

impl<T> std::ops::IndexMut<NodeId> for Vec<T> {
    fn index_mut(&mut self, index: NodeId) -> &mut Self::Output {
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::unnecessary_cast)]
        let index = index.0 as usize;
        &mut self[index]
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct TileId(pub u8, pub u8);
type NodeIdType = u32;

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
    pub tile: TileId,
    pub typ: NodeType,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // 1. Compare X coordinate (tile.0)
        self.tile.0.cmp(&other.tile.0)
            // 2. If X is equal, compare Y coordinate (tile.1)
            .then_with(|| self.tile.1.cmp(&other.tile.1))
            // 3. If both coordinates are equal, compare by id
            .then_with(|| self.id.cmp(&other.id))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Serialize for Node {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Leverage the Display trait as you mentioned
        // Format: X{tile.0}Y{tile.1}.{id}
        serializer.serialize_str(&format!("{}.{}", self.tile, self.id))
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // 1. Get the string from the deserializer
        let s = String::deserialize(deserializer)?;

        // 2. Split into "XnYn" and "NodeId" parts via the '.'
        let (tile_part, id_part) = s.split_once('.').ok_or_else(|| {
            de::Error::custom(format!("Expected '.' separator in node string: {s}"))
        })?;

        // 3. Parse TileId using your helper
        let tile = TileId::from_str_coords(tile_part).map_err(de::Error::custom)?;

        // 4. Parse NodeType using your From<&str> implementation
        // Note: From<&str> is used here; if you need to handle errors, 
        // consider TryFrom instead.
        let typ = NodeType::from(id_part);

        Ok(Self {
            id: id_part.to_string(),
            tile,
            typ,
        })
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.tile, self.id)
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum NodeType {
    LutInput(char),
    LutOutput(char),
    Other,
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

impl From<&str> for NodeType {
    fn from(value: &str) -> Self {

        let mut typ = Self::Other;
        let mut bel_index = '0';
        for (index, char) in value.char_indices(){
            match (index, char){
                (0, 'L') | (2, '_') | (3, 'I') => {}
                (1, a) => bel_index = a,
                (3, 'O') => typ = Self::LutOutput(bel_index),
                (4, a) if a.is_ascii_digit() => typ = Self::LutInput(bel_index),
                (0 | 2 | 3 | _, _) => return Self::Other,
            } 
        }
        typ
    }
}

impl Node {
    pub const fn new(id: String, tile: TileId, typ: NodeType) -> Self {
        Self { id, tile, typ }
    }
    pub fn parse(id: &str, coords: &str) -> Result<Self, ParseError> {
        let tile = TileId::from_str_coords(coords)?;
        let typ = id.into();
        Ok(Self::new(id.to_string(), tile, typ))
    }
    pub fn id(&self) -> String {
        format!("{}.{}", self.tile, self.id)
    }
}

impl Display for TileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "X{}Y{}", self.0, self.1)
    }
}

impl TileId {
    /// Parse coordinates from a string of the form "X<num>Y<num>"
    pub fn from_str_coords(s: &str) -> std::result::Result<Self, ParseError> {
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
        Ok(Self(x, y))
    }
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
            tile: TileId(1, 2),
            typ: NodeType::Other,
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
