use std::collections::{HashMap, HashSet};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{FabricError, FabricGraph, FabricResult, NetInternal, RouteNet, graph::node::NodeId, netlist::NetResultInternal};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SimpleSolver;

impl RouteNet for SimpleSolver {
    fn pre_process(&self, _graph: &mut FabricGraph, _route_plan: &mut [NetInternal]) -> FabricResult<()> {
        Ok(())
    }
    fn solve(&self, graph: &FabricGraph, net: &mut NetInternal) -> FabricResult<()> {
        let signal = net.signal;
        let criticallity = net.criticallity;
        let paths: HashMap<NodeId, Vec<NodeId>> = net
            .sinks
            .par_iter()
            .map(|sink| {
                let (path, _cost) = graph
                    .dijkstra(signal, *sink, criticallity)
                    .ok_or(FabricError::PathfindingFailed {
                        start: signal,
                        sink: *sink,
                    })?;
                Ok((*sink, path))
            })
            .collect::<Result<HashMap<NodeId, Vec<NodeId>>, FabricError>>()?;

        let nodes = paths.values().flatten().copied().collect::<HashSet<NodeId>>();

        net.result = Some(NetResultInternal { paths, nodes });
        Ok(())
    }

    fn identifier(&self) -> &'static str {
        "Simple Solver"
    }
}
