use std::{
    collections::{HashMap, HashSet},
    fs,
};

use serde::{Deserialize, Serialize};

use crate::{node::NodeId, FabricError, FabricGraph, FabricResult};

pub struct NetListInternal {
    pub plan: Vec<NetInternal>,
}

/// Routing request from a source to multiple sinks
#[derive(Debug, Clone)]
pub struct NetInternal {
    /// Source signal node
    pub signal: NodeId,
    /// Destination node indices
    pub sinks: Vec<NodeId>,
    /// Optional routing result after computation
    pub result: Option<NetResultInternal>,
    pub steiner_tree: Option<HashMap<NodeId, Vec<NodeId>>>,
    pub priority: Option<NodeId>,
}

/// Routing result for a routing request
#[derive(Debug, Clone)]
pub struct NetResultInternal {
    /// Paths from source to each sink
    pub paths: HashMap<NodeId, Vec<NodeId>>,
    /// All nodes used in the routing
    pub nodes: HashSet<NodeId>,
}

impl NetResultInternal {
    pub fn to_external(&self, graph: &FabricGraph) -> NetResultExternal {
        let nodes = self
            .nodes
            .iter()
            .map(|a| graph.nodes[*a as usize].id())
            .collect::<HashSet<String>>();
        let paths = self
            .paths
            .iter()
            .map(|(sink, path)| {
                (
                    graph.nodes[*sink as usize].id(),
                    path.iter().map(|c| graph.nodes[*c as usize].id()).collect::<Vec<String>>(),
                )
            })
            .collect::<HashMap<String, Vec<String>>>();

        NetResultExternal { paths, nodes }
    }
}

impl NetListInternal {
    /// Transforms a `NetListExternal` to `Self` by mapping the id names to the internal used ids.
    /// # Errors
    /// Fails when a Mapping of a Net is not possible for example when the graph does not contain
    /// the provided id name
    pub fn from_external(graph: &FabricGraph, external: &NetListExternal) -> FabricResult<Self> {
        let route_plan = external
            .plan
            .iter()
            .map(|externel_routing| {
                NetInternal::from_external(externel_routing, graph).map_err(|e| FabricError::MappingExternelNet {
                    signal: externel_routing.signal.clone(),
                    reason: e.to_string(),
                })
            })
            .collect::<Result<Vec<NetInternal>, FabricError>>()?;
        Ok(Self { plan: route_plan })
    }
    #[must_use]
    pub fn to_external(&self, graph: &FabricGraph) -> NetListExternal {
        let ex = self.plan.iter().map(|x| x.to_external(graph)).collect::<Vec<_>>();
        NetListExternal { graph: Some(graph.filename.clone()), plan: ex }
    }
}
impl NetInternal {
    #[must_use]
    pub fn to_external(&self, graph: &FabricGraph) -> NetExternal {
        let signal = graph.get_node(self.signal).id();
        let sinks = self.sinks.iter().map(|a| graph.get_node(*a).id()).collect();
        let result = self.result.as_ref().map(|r| r.to_external(graph));

        NetExternal { sinks, signal, result }
    }

    /// Transforms a `NetExternal` to a `Self` by mapping the name ids to internal used ids
    /// # Errors
    /// Fails if mapping is not possible
    pub fn from_external(external: &NetExternal, graph: &FabricGraph) -> FabricResult<Self> {
        let mut signal: Option<NodeId> = None;
        let mut sinks_cloned = external.sinks.iter().cloned().collect::<HashSet<String>>();
        let mut sinks: Vec<NodeId> = vec![];

        for (i, node) in graph.nodes.iter().enumerate() {
            let id = node.id();
            if id == external.signal {
                #[allow(clippy::cast_possible_truncation)]
                let x = i as NodeId;
                signal = Some(x);
            }
            if sinks_cloned.remove(&id) {
                #[allow(clippy::cast_possible_truncation)]
                let x = i as NodeId;
                sinks.push(x);
            }
        }
        match (signal, sinks_cloned.is_empty()) {
            (Some(signal), true) => Ok(Self {
                sinks,
                signal,
                result: None,
                steiner_tree: None,
                priority: None,
            }),
            (Some(_), false) => Err(format!("Sinks: {sinks_cloned:?} do not exist.").into()),
            (None, true) => Err("Signal does not exist in graph".into()),
            (None, false) => Err(format!("Signal does not exist and sinks: {sinks_cloned:?}").into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetListExternal {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<String>,
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
    pub fn from_file(file: &str) -> FabricResult<Self> {
        let data: String = fs::read_to_string(file).map_err(|e| FabricError::Io {
            path: file.to_string(),
            source: e,
        })?;
        Ok(Self {
            graph: None,
            plan: serde_json::de::from_str(&data)?,
        })
    }
}
