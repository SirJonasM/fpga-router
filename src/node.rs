//! Module `node`
//!
//! This module defines the building blocks of the FPGA fabric graph:
//! nodes, their types, and associated costs for routing algorithms.

use serde::{Deserialize, Serialize};

use crate::error::ParseError;

/// Edge in the graph with a destination node and cost
#[derive(Debug, Clone)]
pub struct Edge {
    /// Destination node index
    pub node_id: usize,
    /// Cost to traverse this edge
    pub cost: f32,
}

/// A node in the FPGA graph with its type and metadata.
#[derive(Hash, Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    /// Unique identifier of the node
    pub id: String,
    /// X coordinate on the FPGA fabric
    pub x: u8,
    /// Y coordinate on the FPGA fabric
    pub y: u8,
}

/// A node in the FPGA graph with its type and metadata.
impl Node {
    pub fn id(&self) -> String {
        format!("X{}Y{}.{}", self.x, self.y, self.id)
    }
}

/// Structure representing costs associated with routing through a node
#[derive(Clone, Debug)]
pub struct Costs {
    /// Historic cost accumulated over routing iterations
    pub historic_cost: f32,
    /// Maximum capacity of the node
    pub capacity: f32,
    /// Current usage count
    pub usage: u32,
}

impl Node {
    /// Parse two `Node` from a pips line
    ///
    /// # Returns
    /// A `(Node, Node)`
    pub fn parse_from_pips_line(line: &str) -> Result<(Self, Self), ParseError> {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() != 6 {
            return Err(ParseError::InvalidLine {parts_found: parts.len(), line: line.to_string()});
        }
        let (x1, y1) = from_str_coords(parts[0])?;
        let (x2, y2) = from_str_coords(parts[2])?;

        Ok((
            Self {
                x: x1,
                y: y1,
                id: parts[1].to_string(),
            },
            Self {
                x: x2,
                y: y2,
                id: parts[3].to_string(),
            },
        ))
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
        token: s.to_string(),
        source: e,
    })?;
    let y = y_part[1..].parse::<u8>().map_err(|e| ParseError::InvalidCoordinate {
        component: "X",
        token: s.to_string(),
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
    ///
    /// Returns `true` if the node is congested (usage > capacity)
    pub fn update(&mut self, historic_factor: f32) -> bool {
        #[allow(clippy::cast_precision_loss)]
        let usage = self.usage as f32;
        let over_use = usage - self.capacity;

        if over_use > 0.0 {
            self.historic_cost += historic_factor * over_use;
        }
        self.usage = 0;
        over_use > 0.0
    }

    /// Calculate total cost for this node
    pub fn calc_costs(&self, base_cost: f32) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        let casted_usage = self.usage as f32;
        (base_cost + self.historic_cost) * (1.0 + casted_usage)
    }

    /// Create a new `Costs` object
    pub(crate) fn new() -> Self {
        Self::default()
    }
}
