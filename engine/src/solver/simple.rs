use std::collections::{HashMap, HashSet};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    Fabric, FabricError, FabricResult, RouteNet,
    fabric::node::{Node, NodeId, NodeType},
    netlist::{NetInternal, NetResultInternal},
};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SimpleSolver;

impl RouteNet for SimpleSolver {
    fn pre_process(&self, _graph: &mut Fabric, _route_plan: &mut [NetInternal]) -> FabricResult<()> {
        Ok(())
    }
    fn solve(&self, fabric: &mut Fabric, net: &mut NetInternal) -> FabricResult<()> {
        let signal = net.signal;
        let sinks = net
            .sinks
            .iter()
            .filter_map(|sink| {
                let sink_node = fabric.graph.get_node(*sink);
                if let NodeType::LutInput(bel_index) = sink_node.typ {
                    return Some((sink, sink_node, bel_index));
                }
                None
            })
            .map(|(sink, sink_node, bel_index)| {
                let free_lut_inputs = fabric.tile_manager.get_free_lut_inputs(sink_node.tile, bel_index)?;
                let mut sinks_free = free_lut_inputs
                    .iter()
                    .map(|sink_id_str| format!("{}.{}", sink_node.tile, sink_id_str))
                    .map(|a| {
                        fabric
                            .graph
                            .get_node_id(&a)
                            .copied()
                            .ok_or(FabricError::InvalidStringNodeId(a))
                    })
                    .collect::<FabricResult<HashSet<NodeId>>>()?;
                sinks_free.insert(*sink);

                Ok((*sink, sinks_free))
            })
            .collect::<FabricResult<HashMap<NodeId, HashSet<NodeId>>>>()?;

        let paths: HashMap<NodeId, Vec<NodeId>> = sinks
            .par_iter()
            .map(|(current_sink, sinks)| {
                let criticality = fabric
                    .slack_report
                    .as_ref()
                    .map_or(0.0, |a| *a.criticalities.get(&(signal, *current_sink)).unwrap_or(&0.0));
                let (node_found, path, _cost) = fabric.graph.dijkstra_find_one(signal, sinks, criticality).ok_or_else(|| {
                    FabricError::FindOnePathfindingFailed {
                        start: signal.as_node(&fabric.graph),
                        sink: sinks
                            .iter()
                            .map(|sink| fabric.graph.get_node(*sink).clone())
                            .collect::<HashSet<Node>>(),
                    }
                })?;
                Ok((node_found, path))
            })
            .collect::<FabricResult<HashMap<NodeId, Vec<NodeId>>>>()?;

        let nodes = paths.values().flatten().copied().collect::<HashSet<NodeId>>();
        net.sinks = paths.keys().copied().collect::<Vec<NodeId>>();
        //Now we need to unmark the unused inputs of the luts.
        sinks
            .into_values()
            .flatten()
            .filter(|a| !paths.contains_key(a))
            .for_each(|a| {
                let node = fabric.graph.get_node(a);
                if let NodeType::LutInput(bel_index) = &node.typ {
                    fabric.tile_manager.free_lut_input(node.tile, *bel_index, &node.id).unwrap();
                }
            });
        net.result = Some(NetResultInternal { paths, nodes });
        Ok(())
    }

    fn identifier(&self) -> &'static str {
        "Simple Solver"
    }
}
