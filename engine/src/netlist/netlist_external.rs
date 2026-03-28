use std::{collections::{HashMap, HashSet}, fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::{FabricError, FabricResult};


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
    pub sinks: Vec<String>,
    /// Source signal node
    pub signal: String,
    /// Optional routing result after computation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<NetResultExternal>,
    #[serde(skip)]
    pub criticallity: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetResultExternal {
    /// Paths from source to each sink
    pub paths: HashMap<String, Vec<String>>,
    /// All nodes used in the routing
    pub nodes: HashSet<String>,
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
}
