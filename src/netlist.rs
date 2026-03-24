use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::{FabricError, FabricGraph, FabricResult, graph::node::NodeId, slack::SlackReport};

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

impl NetInternal {
    fn new(signal: NodeId, sinks: Vec<NodeId>) -> Self {
        Self {
            signal,
            sinks,
            result: None,
            intermediate_nodes: Option::default(),
            priority: Option::default(),
            criticallity: 0.0,
        }
    }
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
        let nodes = self.nodes.iter().map(|a| graph.nodes[*a].id()).collect::<HashSet<String>>();
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
        let plan = self.plan.iter().map(|x| x.to_external(graph)).collect::<Vec<_>>();
        let hash = Some(graph.calculate_structure_hash());
        NetListExternal { hash, plan }
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
        let map_id = |name: &String| {
            graph.index.get(name).copied().ok_or_else(|| FabricError::MappingExternelNet {
                signal: external.signal.clone(),
                reason: "Index did not contain a internal NodeId for that Id.".to_string(),
            })
        };
        let signal = map_id(&external.signal)?;
        let sinks = external
            .sinks
            .iter()
            .map(map_id)
            .collect::<Result<Vec<NodeId>, FabricError>>()?;

        Ok(Self::new(signal, sinks))
    }
}

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

    pub(crate) fn add_slack(&mut self, slack_report: &SlackReport) {
        for net in &mut self.plan {
            net.criticallity = slack_report.calculate_criticality(&net.signal);
        }
    }
}
