use std::collections::{HashMap, HashSet};

use super::error::{MapExternalError, MapExternalResult};
use crate::{
    FabricGraph, NetExternal, NetListExternal, NetResultExternal,
    fabric::node::{Node, NodeId},
};

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
}

/// Routing result for a routing request
#[derive(Debug, Clone)]
pub struct NetResultInternal {
    /// Paths from source to each sink
    pub paths: HashMap<NodeId, Vec<NodeId>>,
    /// All nodes used in the routing
    pub nodes: HashSet<NodeId>,
}

impl NetListInternal {
    /// Transforms a `NetListExternal` to `Self` by mapping the id names to the internal used ids.
    /// # Errors
    /// Fails when a Mapping of a Net is not possible for example when the graph does not contain
    /// the provided id name
    pub fn from_external(graph: &FabricGraph, external: &NetListExternal) -> MapExternalResult<Self> {
        let route_plan = external
            .plan
            .iter()
            .map(|externel_routing| {
                NetInternal::from_external(externel_routing, graph).map_err(|e| MapExternalError::Net {
                    signal: externel_routing.signal.clone(),
                    source: Box::new(e),
                })
            })
            .collect::<Result<Vec<NetInternal>, MapExternalError>>()?;
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
    /// Transforms a `NetExternal` to a `Self` by mapping the name ids to internal used ids
    /// # Errors
    /// Fails if mapping is not possible
    pub fn from_external(external: &NetExternal, graph: &FabricGraph) -> MapExternalResult<Self> {
        let map_id = |name: &Node| {
            graph
                .index
                .get(&name.id())
                .copied()
                .ok_or_else(|| MapExternalError::Id(name.id()))
        };
        let signal = map_id(&external.signal).map_err(|_| MapExternalError::Signal)?;
        let sinks = external
            .sinks
            .iter()
            .map(map_id)
            .collect::<Result<Vec<NodeId>, MapExternalError>>()
            .map_err(|e| MapExternalError::Sink(Box::new(e)))?;

        let result = if let Some(result) = &external.result {
            let result = NetResultInternal::from_external(graph, result)?;
            Some(result)
        } else {
            None
        };
        let x = Self {
            signal,
            sinks,
            result,
            intermediate_nodes: Option::default(),
        };

        Ok(x)
    }

    #[must_use]
    pub fn to_external(&self, graph: &FabricGraph) -> NetExternal {
        let signal = graph.get_node(self.signal).clone();
        let sinks = self.sinks.iter().map(|a| graph.get_node(*a).clone()).collect();
        let result = self.result.as_ref().map(|r| r.to_external(graph));

        NetExternal {
            sinks,
            signal,
            result,
        }
    }
}

impl NetResultInternal {
    fn from_external(graph: &FabricGraph, result: &NetResultExternal) -> MapExternalResult<Self> {
        let map_id = |name: &String| {
            graph
                .index
                .get(name)
                .copied()
                .ok_or_else(|| MapExternalError::Id(name.clone()))
        };

        let nodes = result
            .nodes
            .iter()
            .map(|a| map_id(&a.id()))
            .collect::<MapExternalResult<HashSet<NodeId>>>()
            .map_err(|e| MapExternalError::NetResultNodes(Box::new(e)))?;
        let paths = result
            .paths
            .iter()
            .map(|(key, value)| {
                let sink = map_id(&key.id())?;
                let path = value
                    .iter()
                    .map(|a| map_id(&a.id()))
                    .collect::<MapExternalResult<Vec<NodeId>>>()?;
                Ok((sink, path))
            })
            .collect::<MapExternalResult<HashMap<NodeId, Vec<NodeId>>>>()
            .map_err(|e| MapExternalError::NetResultPaths(Box::new(e)))?;
        Ok(Self { paths, nodes })
    }

    #[must_use]
    pub fn to_external(&self, graph: &FabricGraph) -> NetResultExternal {
        let nodes = self
            .nodes
            .iter()
            .map(|a| graph.get_node(*a))
            .cloned()
            .collect::<HashSet<Node>>();
        let paths = self
            .paths
            .iter()
            .map(|(sink, path)| {
                (
                    graph.nodes[*sink].clone(),
                    path.iter().map(|c| graph.get_node(*c)).cloned().collect::<Vec<Node>>(),
                )
            })
            .collect::<HashMap<Node, Vec<Node>>>();

        NetResultExternal { paths, nodes }
    }
}
