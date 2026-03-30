use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{Fabric, FabricResult, RouteNet, fabric::node::NodeId, netlist::{NetInternal, NetResultInternal}};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SteinerSolver;

#[derive(Debug, Clone)]
struct SteinerCandidate {
    base_path: Vec<NodeId>,
    mid_points: HashMap<NodeId, NodeId>,
    costs: f32,
}

impl RouteNet for SteinerSolver {
    fn identifier(&self) -> &'static str {
        "Steiner Solver"
    }
    fn pre_process(&self, _graph: &mut Fabric, _route_plan: &mut [NetInternal]) -> FabricResult<()> {
        Ok(())
    }
    fn solve(&self, fabric: &mut Fabric, net: &mut NetInternal) -> FabricResult<()> {
        let dists = net
            .sinks
            .par_iter()
            .map(|sink| (*sink, fabric.graph.dijkstra_all(*sink)))
            .collect::<HashMap<NodeId, Vec<f32>>>();
        let signal = net.signal;
        let base_paths: Vec<(NodeId, NodeId)> = net.sinks.iter().map(|&sink| (signal, sink)).collect();

        // 1. Parallel reduction to find the single best SteinerCandidate
        let best_candidate: Result<SteinerCandidate, String> = base_paths
            .into_par_iter()
            .map(|(start, base_sink)| {
                // --- Computation to find the MINIMUM COST ---
                // Calculate the cost of the base path (Dijkstra is still necessary here)
                let Some((base_path, mut costs)) = fabric.graph.dijkstra(start, base_sink, 0.0) else {
                    let start_name = fabric.graph.get_node(start).id();
                    let base_sink_name = fabric.graph.get_node(base_sink).id();
                    return Err(format!("Could not find a base path start: {start_name}, base sink: {base_sink_name}"));
                };

                // Calculate the cost of connecting all other sinks to this base path
                let mid_points = net
                    .sinks
                    .iter()
                    .map(|sink| {
                        let Some(terminal_distances) = dists.get(sink) else {
                            let sink_name = fabric.graph.get_node(*sink).id();
                            return Err(format!("No precalculated distances for the sink: {sink_name}"));
                        };

                        // Find the connection node (min_node) on the base_path
                        let (min_node, cost_to_base_path) = base_path
                            .iter()
                            .map(|&node| (node, terminal_distances[node]))
                            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Greater))
                            .unwrap();

                        // This cost is the *shortest path cost* from the base path to the sink.
                        costs += cost_to_base_path;
                        Ok((*sink, min_node))
                    })
                    .collect::<Result<HashMap<NodeId, NodeId>, String>>();
                match mid_points {
                    Ok(mid_points) => Ok(SteinerCandidate {
                        base_path,
                        mid_points,
                        costs,
                    }),
                    Err(err) => Err(err),
                }
            })
            // 2. Reduce the candidates to find the one with the minimum cost.
            .reduce(
                || Err("No minmum".to_string()),
                |acc, item| match (acc, item) {
                    (Err(err1), Err(err2)) => Err(format!("err: {err1}\n err: {err2}\n")),
                    (Ok(current_best), Err(_err)) => Ok(current_best),
                    (Err(_err), Ok(item)) => Ok(item),
                    (Ok(current_best), Ok(item)) => {
                        if item.costs < current_best.costs {
                            Ok(item)
                        } else {
                            Ok(current_best)
                        }
                    }
                },
            );

        // 3. Final Calculation: Sequentially calculate the full result for the winner.
        if let Ok(best_candidate) = best_candidate {
            let mut nodes = HashSet::new();
            nodes.extend(&best_candidate.base_path);

            let mut paths = HashMap::new();

            for (sink, mid_point) in &best_candidate.mid_points {
                let Some((mut path_to_mid, _cost)) = fabric.graph.dijkstra(signal, *mid_point, 0.0) else {
                    let sink_name = fabric.graph.get_node(*sink).id();
                    return Err(format!("Could not find a route for sink: {sink_name}").into());
                };
                let Some((path_from_mid, _cost)) = fabric.graph.dijkstra(*mid_point, *sink, 0.0) else {
                    let sink_name = fabric.graph.get_node(*sink).id();
                    return Err(format!("Could not find a route for sink: {sink_name}").into());
                };
                nodes.extend(&path_from_mid);
                path_to_mid.extend(&path_from_mid[1..]);
                paths.insert(*sink, path_to_mid);
            }

            net.result = Some(NetResultInternal { paths, nodes });
            Ok(())
        } else {
            net.result = None; // No sinks found
            Err("Error".into())
        }
    }
}
