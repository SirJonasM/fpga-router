//! Module `node`
//!
//! This module defines the building blocks of the FPGA fabric graph:
//! nodes, their types, and associated costs for routing algorithms.

/// Edge in the graph with a destination node and cost
#[derive(Debug, Clone)]
pub struct Edge {
    /// Destination node index
    pub node_id: usize,
    /// Cost to traverse this edge
    pub cost: f32,
}

/// A node in the FPGA graph with its type and metadata.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct Node {
    /// Unique identifier of the node
    pub id: String,
    /// X coordinate on the FPGA fabric
    pub x: u8,
    /// Y coordinate on the FPGA fabric
    pub y: u8,
}

impl Node{
    pub fn id(&self) -> String{
        format!("{}/X{}Y{}", self.id, self.x, self.y)
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
    /// Parse a `Node` from a block string and ID
    ///
    /// # Arguments
    /// * `block` - String containing the coordinates (e.g., "X1Y2")
    /// * `id` - Node identifier (e.g., "LA_I0" or "LB_O")
    ///
    /// # Returns
    /// A `Node` 
    pub fn parse_from_pips_line(line: &str) -> Result<(Self,Self), String> {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() != 6 {
            return Err(format!("Invalid line: {}", line));
        }
        let (x1, y1) = match from_str_coords(parts[0]) {
            Ok(res) => res,
            Err(err) => panic!("Error parsing: {}", err),
        };
        let (x2, y2) = match from_str_coords(parts[2]) {
            Ok(res) => res,
            Err(err) => panic!("Error parsing: {}", err),
        };

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
        }))
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

