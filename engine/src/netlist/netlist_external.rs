use std::{collections::{HashMap, HashSet},  fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::{FabricError, FabricResult, fabric::node::Node};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetListExternal {
    pub hash: Option<String>,
    pub plan: Vec<NetExternal>,
}

/// Represents a entry in the `NetList`
/// each net has a start point (signal) and endpoints (sinks)
/// result contains the paths of the signal to each sink
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetExternal {
    /// Destination node indices
    pub sinks: Vec<Node>,
    /// Source signal node
    pub signal: Node,
    /// Optional routing result after computation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<NetResultExternal>,
    #[serde(skip)]
    pub criticallity: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetResultExternal {
    /// Paths from source to each sink
    pub paths: HashMap<Node, Vec<Node>>,
    /// All nodes used in the routing
    pub nodes: HashSet<Node>,
}

impl NetListExternal {
    /// Creates a `NetListExternal` from a Jsonfile
    ///
    /// # Errors
    /// - Returns `FabricError::Io` in case of failing reading the file
    /// - Returns `FabricError::Json` when the deserialization fails
    pub fn from_file<P: AsRef<Path>>(file: P) -> FabricResult<Self> {
        let path_ref = file.as_ref();
        let data: String = fs::read_to_string(path_ref).map_err(|e| FabricError::Io {
            path: path_ref.to_path_buf(),
            source: e,
        })?;
        let x: Self = serde_json::de::from_str(&data)?;
        Ok(x)
    }

    pub fn swapped_inputs(&self, old: &Self) -> Vec<Swap> {
        let mut x1 = self
            .plan
            .iter()
            .flat_map(|net| net.sinks.iter().map(|a| (net.signal.clone(), a.clone())))
            .collect::<Vec<(Node, Node)>>();
        let mut x2 = old
            .plan
            .iter()
            .flat_map(|net| net.sinks.iter().map(|a| (net.signal.clone(), a.clone())))
            .collect::<Vec<(Node, Node)>>();
        x1.sort();
        x2.sort();
        x1.into_iter()
            .zip(x2)
            .filter_map(|((signal, sink_old), (signal2, sink_new))| if signal == signal2 && sink_new != sink_old {Some(Swap {signal, sink_old, sink_new})} else { None })
            .collect::<Vec<Swap>>()
    }
}

pub struct Swap {
    pub signal: Node,
    pub sink_old: Node,
    pub sink_new: Node,
}
impl std::fmt::Display for Swap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Using padding (e.g., {:<20}) to keep columns aligned
        write!(
            f,
            "Signal: {:<15} | {:<20} -> {:<20}",
            self.signal.id, 
            self.sink_old.id, 
            self.sink_new.id
        )
    }
}
