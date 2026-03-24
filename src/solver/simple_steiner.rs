use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{FabricError, FabricGraph, FabricResult, NetInternal, RouteNet, graph::node::NodeId, netlist::NetResultInternal};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SimpleSteinerSolver;

struct SteinerTreeCandidate {
    pub steiner_nodes: HashMap<NodeId, Vec<NodeId>>,
    pub nodes: HashSet<NodeId>,
    pub costs: f32,
}

impl RouteNet for SimpleSteinerSolver {
    fn pre_process(&self, graph: &mut FabricGraph, route_plan: &mut [NetInternal]) -> FabricResult<()> {
        let mut used_nodes = HashSet::new();
        for route in route_plan.iter_mut() {
            let signal_id = route.signal;
            let steiner_tree = pre_calc_steiner_tree(graph, route).map_err(|e| FabricError::RoutePreProcessing {
                signal: signal_id,
                source: e.into(),
            })?;

            for &node_id in &steiner_tree.values().flatten().copied().collect::<HashSet<NodeId>>() {
                if !used_nodes.insert(node_id) {
                    return Err(FabricError::RoutePreProcessing {
                        signal: signal_id,
                        source: Box::new(FabricError::ResourceConflict { node_id }),
                    });
                }
            }
            route.intermediate_nodes = Some(steiner_tree);
        }
        graph.reset_usage();
        Ok(())
    }
    fn solve(&self, graph: &FabricGraph, net: &mut NetInternal) -> FabricResult<()> {
        let criticallity = net.criticallity;
        if let Some(steiner_tree) = &net.intermediate_nodes {
            let mut paths = HashMap::new();
            let mut nodes = HashSet::new();
            for (terminal, route) in steiner_tree {
                let mut path = Vec::new();
                for steiner_node in route.windows(2) {
                    let (start, end) = (steiner_node[0], steiner_node[1]);
                    let Some((a, _b)) = graph.dijkstra(start, end, criticallity) else {
                        return Err(format!("Could not find path between steinere nodes: {start}, {end}").into());
                    };
                    nodes.extend(&a);
                    path.extend(&a[..a.len() - 1]);
                }
                path.push(*terminal);
                paths.insert(*terminal, path);
            }
            net.result = Some(NetResultInternal { paths, nodes });
            Ok(())
        } else {
            Err("No steiner Tree precalculated.".into())
        }
    }

    fn identifier(&self) -> &'static str {
        "SimpleSteinerSolver"
    }
}

fn pre_calc_steiner_tree(graph: &mut FabricGraph, net: &NetInternal) -> FabricResult<HashMap<NodeId, Vec<NodeId>>> {
    let criticallity = net.criticallity;
    let dists = net
        .sinks
        .par_iter()
        .map(|sink| (*sink, graph.dijkstra_all(*sink, criticallity)))
        .collect::<HashMap<NodeId, Vec<f32>>>();
    let signal = net.signal;
    let base_paths: Vec<(NodeId, NodeId)> = net.sinks.iter().map(|&sink| (signal, sink)).collect();

    // 1. Parallel reduction to find the single best SteinerCandidate
    let best_candidate: Vec<SteinerTreeCandidate> = base_paths
        .into_par_iter()
        .map(|(start, base_sink)| {
            let (base_path, mut costs) = graph
                .dijkstra(start, base_sink, criticallity)
                .ok_or(FabricError::PathfindingFailed { start, sink: base_sink })?;

            let mut nodes = HashSet::new();
            let min_points = net
                .sinks
                .iter()
                .copied()
                .map(|sink| {
                    let terminal_distances = dists
                        .get(&sink)
                        .expect("Dists map was built from the same sink list; this is a logic invariant.");
                    let (min_node, cost_to_base_path) = base_path
                        .iter()
                        .map(|&node| (node, terminal_distances[node]))
                        .min_by(|a, b| {
                            if graph.get_costs(a.0).usage > 0 {
                                return Ordering::Greater;
                            }
                            if graph.get_costs(b.0).usage > 0 {
                                return Ordering::Less;
                            }
                            a.1.partial_cmp(&b.1).unwrap_or(Ordering::Greater)
                        })
                        .expect("The base path is empty.");

                    // This cost is the *shortest path cost* from the base path to the sink.
                    costs += cost_to_base_path;
                    nodes.insert(min_node);
                    (sink, min_node)
                })
                .collect::<HashMap<NodeId, NodeId>>();

            let mut steiner_nodes = HashMap::new();
            for sink in &net.sinks {
                let mut sink_uses_steiner_nodes = vec![net.signal];
                let m = min_points
                    .get(sink)
                    .ok_or_else(|| format!("No midpoint calculated for sink {sink}"))?;
                for n in &base_path {
                    if n == m {
                        sink_uses_steiner_nodes.push(*sink);
                        steiner_nodes.insert(*sink, sink_uses_steiner_nodes);
                        break;
                    }
                    if nodes.contains(n) {
                        sink_uses_steiner_nodes.push(*n);
                    }
                }
            }
            Ok(SteinerTreeCandidate {
                steiner_nodes,
                nodes,
                costs,
            })
        })
        .collect::<FabricResult<Vec<SteinerTreeCandidate>>>()?;

    let best_candidate = best_candidate
        .into_iter()
        .min_by(|a, b| a.costs.partial_cmp(&b.costs).unwrap_or(Ordering::Equal))
        .ok_or(FabricError::NoSteinerTreeFound)?;

    // 3. Final Calculation: Sequentially calculate the full result for the winner.
    best_candidate.nodes.iter().for_each(|x| graph.costs[*x].usage = 1);
    Ok(best_candidate.steiner_nodes)
}
