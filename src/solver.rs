use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use crate::{
    FabricError, FabricResult,
    fabric_graph::{FabricGraph, Routing, RoutingResult, SteinerTreeCandidate},
};

#[derive(Debug, Clone)]
struct SteinerCandidate {
    base_path: Vec<usize>,
    mid_points: HashMap<usize, usize>,
    costs: f32,
}

#[derive(Eq, PartialEq, Deserialize, Debug, Clone, Serialize)]
pub struct SimpleSolver;
#[derive(Eq, PartialEq, Deserialize, Debug, Clone, Serialize)]
pub struct SteinerSolver;
#[derive(Eq, PartialEq, Deserialize, Debug, Clone, Serialize)]
pub struct SimpleSteinerSolver;

/// A trait for implementing custom routing algorithms within the fabric.
///
/// Implementors of this trait can define how individual signals are routed
/// and how the global routing plan is prepared before execution.
pub trait SolveRouting {
    /// Executes the routing algorithm for a single net.
    ///
    /// This method is responsible for finding a path in the [`FabricGraph`]
    /// and updating the [`Routing`] structure with the results.
    ///
    /// # Errors
    ///
    /// Returns [`FabricError::PathfindingFailed`] if a valid route cannot be found
    /// given the current graph constraints.
    fn solve(&self, graph: &FabricGraph, routing: &mut Routing) -> FabricResult<()>;

    /// Prepares the graph or the route plan before the main solving phase.
    ///
    /// This is typically used for global optimizations, such as pre-calculating
    /// Steiner points or identifying high-congestion areas.
    ///
    /// # Errors
    ///
    /// Returns [`FabricError::ResourceConflict`] if the pre-processing logic
    /// detects overlapping requirements that cannot be resolved.
    fn pre_process(&self, graph: &mut FabricGraph, route_plan: &mut [Routing]) -> FabricResult<()>;

    /// Returns a unique string constant identifying the solver implementation.
    ///
    /// This is used for logging, telemetry, and CLI selection (e.g., "steiner", "simple").
    fn identifier(&self) -> &'static str;
}

impl SolveRouting for SimpleSolver {
    fn pre_process(&self, _graph: &mut FabricGraph, _route_plan: &mut [Routing]) -> FabricResult<()> {
        Ok(())
    }
    fn solve(&self, graph: &FabricGraph, routing: &mut Routing) -> FabricResult<()> {
        let signal = routing.signal;
        let paths: HashMap<usize, Vec<usize>> = routing
            .sinks
            .par_iter()
            .map(|sink| {
                let (path, _cost) = graph.dijkstra(signal, *sink).ok_or(FabricError::PathfindingFailed {
                    start: signal,
                    sink: *sink,
                })?;
                Ok((*sink, path))
            })
            .collect::<Result<HashMap<usize, Vec<usize>>, FabricError>>()?;

        let nodes = paths.values().flatten().copied().collect::<HashSet<usize>>();

        routing.result = Some(RoutingResult { paths, nodes });
        Ok(())
    }

    fn identifier(&self) -> &'static str {
        "Simple Solver"
    }
}

impl SolveRouting for SteinerSolver {
    fn identifier(&self) -> &'static str {
        "Steiner Solver"
    }
    fn pre_process(&self, _graph: &mut FabricGraph, _route_plan: &mut [Routing]) -> FabricResult<()> {
        Ok(())
    }
    fn solve(&self, graph: &FabricGraph, routing: &mut Routing) -> FabricResult<()> {
        let dists = routing
            .sinks
            .par_iter()
            .map(|sink| (*sink, graph.dijkstra_all(*sink)))
            .collect::<HashMap<usize, Vec<f32>>>();
        let signal = routing.signal;
        let base_paths: Vec<(usize, usize)> = routing.sinks.iter().map(|&sink| (signal, sink)).collect();

        // 1. Parallel reduction to find the single best SteinerCandidate
        let best_candidate: Result<SteinerCandidate, String> = base_paths
            .into_par_iter()
            .map(|(start, base_sink)| {
                // --- Computation to find the MINIMUM COST ---
                // Calculate the cost of the base path (Dijkstra is still necessary here)
                let Some((base_path, mut costs)) = graph.dijkstra(start, base_sink) else {
                    return Err(format!("Could not find a base path start: {start}, base sink: {base_sink}"));
                };

                // Calculate the cost of connecting all other sinks to this base path
                let mid_points = routing
                    .sinks
                    .iter()
                    .map(|sink| {
                        let Some(terminal_distances) = dists.get(sink) else {
                            return Err(format!("No precalculated distances for the sink: {sink}"));
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
                    .collect::<Result<HashMap<usize, usize>, String>>();
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
                let Some((mut path_to_mid, _cost)) = graph.dijkstra(signal, *mid_point) else {
                    return Err(format!("Could not find a route for sink: {sink}").into());
                };
                let Some((path_from_mid, _cost)) = graph.dijkstra(*mid_point, *sink) else {
                    return Err(format!("Could not find a route for sink: {sink}").into());
                };
                nodes.extend(&path_from_mid);
                path_to_mid.extend(&path_from_mid[1..]);
                paths.insert(*sink, path_to_mid);
            }

            routing.result = Some(RoutingResult { paths, nodes });
            Ok(())
        } else {
            routing.result = None; // No sinks found
            Err("Error".into())
        }
    }
}

impl SolveRouting for SimpleSteinerSolver {
    fn pre_process(&self, graph: &mut FabricGraph, route_plan: &mut [Routing]) -> FabricResult<()> {
        let mut used_nodes = HashSet::new();
        for route in route_plan.iter_mut() {
            let signal_id = route.signal;
            let steiner_tree = pre_calc_steiner_tree(graph, route).map_err(|e| FabricError::RoutePreProcessing {
                signal: signal_id,
                source: e.into(),
            })?;

            for &node_id in &steiner_tree.values().flatten().copied().collect::<HashSet<usize>>() {
                if !used_nodes.insert(node_id) {
                    return Err(FabricError::RoutePreProcessing {
                        signal: signal_id,
                        source: Box::new(FabricError::ResourceConflict { node_id }),
                    });
                }
            }
            route.steiner_tree = Some(steiner_tree);
        }
        graph.reset_usage();
        Ok(())
    }
    fn solve(&self, graph: &FabricGraph, routing: &mut Routing) -> FabricResult<()> {
        if let Some(steiner_tree) = &routing.steiner_tree {
            let mut paths = HashMap::new();
            let mut nodes = HashSet::new();
            for (terminal, route) in steiner_tree {
                let mut path = Vec::new();
                for steiner_node in route.windows(2) {
                    let (start, end) = (steiner_node[0], steiner_node[1]);
                    let Some((a, _b)) = graph.dijkstra(start, end) else {
                        return Err(format!("Could not find path between steinere nodes: {start}, {end}").into());
                    };
                    nodes.extend(&a);
                    path.extend(&a[..a.len() - 1]);
                }
                path.push(*terminal);
                paths.insert(*terminal, path);
            }
            routing.result = Some(RoutingResult { paths, nodes });
            Ok(())
        } else {
            Err("No steiner Tree precalculated.".into())
        }
    }

    fn identifier(&self) -> &'static str {
        "SimpleSteinerSolver"
    }
}

fn pre_calc_steiner_tree(graph: &mut FabricGraph, routing: &Routing) -> FabricResult<HashMap<usize, Vec<usize>>> {
    let dists = routing
        .sinks
        .par_iter()
        .map(|sink| (*sink, graph.dijkstra_all(*sink)))
        .collect::<HashMap<usize, Vec<f32>>>();
    let signal = routing.signal;
    let base_paths: Vec<(usize, usize)> = routing.sinks.iter().map(|&sink| (signal, sink)).collect();

    // 1. Parallel reduction to find the single best SteinerCandidate
    let best_candidate: Vec<SteinerTreeCandidate> = base_paths
        .into_par_iter()
        .map(|(start, base_sink)| {
            let (base_path, mut costs) = graph
                .dijkstra(start, base_sink)
                .ok_or(FabricError::PathfindingFailed { start, sink: base_sink })?;

            let mut nodes = HashSet::new();
            let min_points = routing
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
                            if graph.costs[a.0].usage > 0 {
                                return Ordering::Greater;
                            }
                            if graph.costs[b.0].usage > 0 {
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
                .collect::<HashMap<usize, usize>>();

            let mut steiner_nodes = HashMap::new();
            for sink in &routing.sinks {
                let mut sink_uses_steiner_nodes = vec![routing.signal];
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
