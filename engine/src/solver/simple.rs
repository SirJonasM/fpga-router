use std::collections::{HashMap, HashSet};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    Fabric, FabricError, FabricResult, RouteNet,
    graph::node::NodeId,
    netlist::{NetInternal, NetResultInternal},
};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SimpleSolver;

impl RouteNet for SimpleSolver {
    fn pre_process(&self, _graph: &mut Fabric, _route_plan: &mut [NetInternal]) -> FabricResult<()> {
        Ok(())
    }
    fn solve(&self, fabric: &Fabric, net: &mut NetInternal) -> FabricResult<()> {
        let signal = net.signal;
        let paths: HashMap<NodeId, Vec<NodeId>> = net
            .sinks
            .par_iter()
            .map(|sink| {
                let crit = if let Some(slack_report) = &fabric.slack_report {
                    if let Some(crit) = slack_report.criticalities.get(&(signal, *sink)) {
                        *crit
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };
                let (path, _cost) =
                    fabric
                        .graph
                        .dijkstra(signal, *sink, crit)
                        .ok_or_else(|| FabricError::PathfindingFailed {
                            start: signal.as_node(&fabric.graph),
                            sink: sink.as_node(&fabric.graph),
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
