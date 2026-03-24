use std::{
    collections::{HashMap, HashSet},
    fs, path::Path,
};

use serde::{Deserialize, Serialize};

use crate::{FabricError, FabricGraph, FabricResult, slack::SlackReport, graph::node::NodeId};

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
    pub intermediate_nodes: Option<HashMap<NodeId, Vec<NodeId>>>,
    pub priority: Option<NodeId>,
    pub criticallity: f32,
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
            .map(|a| graph.nodes[*a].id())
            .collect::<HashSet<String>>();
        let paths = self
            .paths
            .iter()
            .map(|(sink, path)| {
                (
                    graph.nodes[*sink].id(),
                    path.iter().map(|c| graph.nodes[*c].id()).collect::<Vec<String>>(),
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
        NetListExternal { plan: ex }
    }
}
impl NetInternal {
    #[must_use]
    pub fn to_external(&self, graph: &FabricGraph) -> NetExternal {
        let signal = graph.get_node(self.signal).id();
        let sinks = self.sinks.iter().map(|a| graph.get_node(*a).id()).collect();
        let result = self.result.as_ref().map(|r| r.to_external(graph));

        NetExternal {
            sinks,
            signal,
            result,
            criticallity: Some(self.criticallity),
        }
    }

    /// Transforms a `NetExternal` to a `Self` by mapping the name ids to internal used ids
    /// # Errors
    /// Fails if mapping is not possible
    pub fn from_external(external: &NetExternal, graph: &FabricGraph) -> FabricResult<Self> {
        let mut signal: Option<NodeId> = None;
        let mut sinks_cloned = external.sinks.iter().cloned().collect::<HashSet<String>>();
        let mut sinks: Vec<NodeId> = vec![];

        for node in graph.nodes.iter() {
            let id = node.id();
            let node_id = graph.get_node_id(&id).unwrap();
            if id == external.signal {
                signal = Some(*node_id);
            }
            if sinks_cloned.remove(&id) {
                sinks.push(*node_id);
            }
        }
        match (signal, sinks_cloned.is_empty()) {
            (Some(signal), true) => Ok(Self {
                sinks,
                signal,
                result: None,
                intermediate_nodes: None,
                priority: None,
                criticallity: external.criticallity.unwrap_or(0.0),
            }),
            (Some(_), false) => Err(format!("Sinks: {sinks_cloned:?} do not exist.").into()),
            (None, true) => Err("Signal does not exist in graph".into()),
            (None, false) => Err(format!("Signal does not exist and sinks: {sinks_cloned:?}").into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetListExternal {
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

    pub(crate) fn add_slack(&mut self, slack_report: &SlackReport) {
        for net in &mut self.plan {
            net.criticallity = slack_report.calculate_criticality(&net.signal);
        }
    }
}
