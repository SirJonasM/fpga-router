//! Module `node`
//!
//! This module defines the building blocks of the FPGA fabric graph:
//! nodes, their types, and associated costs for routing algorithms.

use std::hash::Hash;
use regex::Regex;
/// Edge in the graph with a destination node and cost
#[derive(Debug, Clone)]
pub struct Edge {
    /// Destination node index
    pub node_id: usize,
    /// Cost to traverse this edge
    pub cost: f32,
}

/// A node in the FPGA graph with its type and metadata.
#[derive(Clone, Debug)]
pub struct GraphNode {
    /// The underlying node containing ID and coordinates
    pub node: Node,
    /// The type of node (input, output, or default)
    pub typ: NodeType,
}

/// Basic representation of a node with an ID and coordinates.
#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub struct Node {
    /// Unique identifier of the node
    pub id: String,
    /// X coordinate on the FPGA fabric
    pub x: u8,
    /// Y coordinate on the FPGA fabric
    pub y: u8,
}

/// Enum representing the type of node
#[derive(Clone, Debug)]
pub enum NodeType {
    /// Input of a LUT
    LutInput(LutInput),
    /// Output of a LUT
    LutOutput(LutOutput),
    /// Default node (non-LUT)
    Default(DefaultNode),
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

/// Placeholder struct for default nodes
#[derive(Clone, Debug)]
pub struct DefaultNode;

/// Represents a LUT output node
#[derive(Clone, Debug)]
pub struct LutOutput;

/// Represents a LUT input node
#[derive(Clone, Debug)]
pub struct LutInput {
    /// The letter of the LUT (e.g., 'A', 'B')
    pub letter: char,
    /// Input pin number
    pub _input_pin: u8,
    /// Other LUT inputs sharing the same coordinates
    pub others: Vec<usize>,
}

impl LutInput {
    /// Returns all other connected LUT input indices
    pub(crate) fn _get_others(&self) -> Vec<usize> {
        self.others.clone()
    }
}

impl GraphNode {
    /// Parse a `GraphNode` from a block string and ID
    ///
    /// # Arguments
    /// * `block` - String containing the coordinates (e.g., "X1Y2")
    /// * `id` - Node identifier (e.g., "LA_I0" or "LB_O")
    ///
    /// # Returns
    /// A `GraphNode` with appropriate `NodeType`
    pub fn parse(block: &str, id: &str) -> Self {
        let (x, y) = match from_str_coords(block) {
            Ok(res) => res,
            Err(err) => panic!("Error parsing: {}", err),
        };
        let lut_input_reg: Regex = Regex::new(r"^L[ABCDEFGH]_I[0-3]$").unwrap();
        let lut_output_reg: Regex = Regex::new(r"^L[ABCDEFGH]_O$").unwrap();

        let typ = if matches!(id, l if lut_input_reg.is_match(l)) {
            let left = id.split('_').next().expect("Could not split the '_' in an Lut Input.");
            let letter = left.chars().nth(1).expect("No second part in Lut Input");
            let input_pin = id
                .split('_')
                .next_back()
                .expect("No second part.")
                .trim_start_matches('I')
                .parse::<u8>()
                .expect("No pin on LutInput.");
            NodeType::LutInput(LutInput {
                letter,
                _input_pin: input_pin,
                others: Vec::new(),
            })
        } else if matches!(id, l if lut_output_reg.is_match(l)) {
            NodeType::LutOutput(LutOutput {})
        } else {
            NodeType::Default(DefaultNode {})
        };
        Self {
            node: Node {
                x,
                y,
                id: id.to_string(),
            },
            typ,
        }
    }
}

/// Parse coordinates from a string of the form "X<num>Y<num>"
fn from_str_coords(s: &str) -> std::result::Result<(u8, u8), String> {
    if !s.starts_with('X') {
        return Err(format!("Invalid BlockID, missing 'X': {}", s));
    }
    let Some((x_part, y_part)) = s.split_once('Y') else {
        return Err(format!("Invalid BlockID, missing 'Y': {}", s));
    };
    let x = x_part[1..].parse::<u8>().map_err(|_| format!("Invalid X number in BlockID: {}", s))?;
    let y = y_part.parse::<u8>().map_err(|_| format!("Invalid Y number in BlockID: {}", s))?;
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
        (base_cost + self.historic_cost) * (1.0 + self.usage as f32)
    }

    /// Create a new `Costs` object
    pub(crate) fn new() -> Self {
        Self {
            historic_cost: 0.0,
            capacity: 1.0,
            usage: 0,
        }
    }
}

