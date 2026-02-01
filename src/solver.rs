use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use crate::fabric_graph::{FabricGraph, Routing, RoutingResult};

#[derive(Debug, Clone)]
struct SteinerCandidate {
    base_path: Vec<usize>,
    mid_points: HashMap<usize, usize>,
    costs: f32,
}
#[derive(Eq, PartialEq, Deserialize, Debug, Clone, Serialize)]
pub enum Solver {
    Simple(SimpleSolver),
    Steiner(SteinerSolver),
    SimpleSteiner(SimpleSteinerSolver),
}

#[derive(Eq, PartialEq, Deserialize, Debug, Clone, Serialize)]
pub struct SimpleSolver;
pub trait SolveRouting {
    fn solve(&self, graph: &FabricGraph, routing: &mut Routing) -> Result<(), String>;
    fn identifier(&self) -> &'static str;
}
impl SolveRouting for SimpleSolver {
    fn solve(&self, graph: &FabricGraph, routing: &mut Routing) -> Result<(), String> {
        let results: Result<Vec<(usize, Vec<usize>)>, String> = routing
            .sinks
            .par_iter() // 1. Parallel iterator
            .map(|sink| {
                // 2. Perform Dijkstra for each sink in parallel
                match graph.dijkstra(routing.signal, *sink) {
                    Some((path, _cost)) => Ok((*sink, path)),
                    None => Err(format!(
                        "Could not find a route for sink: {} id: {}, from signal: {}, id: {}",
                        sink,
                        graph.nodes[*sink].id(),
                        routing.signal,
                        graph.nodes[routing.signal].id()
                    )),
                }
            })
            .collect(); // 3. Collect will stop at the first Err it encounters

        let paths_vec = results?;

        // 4. Combine the results back into your HashMap and HashSet
        let mut nodes = HashSet::new();
        let mut paths = HashMap::new();

        for (sink, path) in paths_vec {
            nodes.extend(&path);
            paths.insert(sink, path);
        }

        routing.result = Some(RoutingResult { paths, nodes });
        Ok(())
    }

    fn identifier(&self) -> &'static str {
        "Simple Solver"
    }
}
#[derive(Eq, PartialEq, Deserialize, Debug, Clone, Serialize)]
pub struct SteinerSolver;

impl SolveRouting for SteinerSolver {
    fn identifier(&self) -> &'static str {
        "Steiner Solver"
    }
    fn solve(&self, graph: &FabricGraph, routing: &mut Routing) -> Result<(), String> {
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
                let (base_path, mut costs) = match graph.dijkstra(start, base_sink) {
                    Some(res) => res,
                    None => return Err(format!("Could not find a base path start: {start}, base sink: {base_sink}")),
                };

                // Calculate the cost of connecting all other sinks to this base path
                let mid_points = routing
                    .sinks
                    .iter()
                    .map(|sink| {
                        let terminal_distances = match dists.get(sink) {
                            Some(dist) => dist,
                            None => return Err(format!("No precalculated distances for the sink: {sink}")),
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
                    (Err(err1), Err(err2)) => Err(format!("err: {}\n err: {}\n", err1, err2)),
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
                let (mut path_to_mid, _cost) = match graph.dijkstra(signal, *mid_point) {
                    Some(res) => res,
                    None => return Err(format!("Could not find a route for sink: {sink}")),
                };
                let (path_from_mid, _cost) = match graph.dijkstra(*mid_point, *sink) {
                    Some(res) => res,
                    None => return Err(format!("Could not find a route for sink: {sink}")),
                };
                nodes.extend(&path_from_mid);
                path_to_mid.extend(&path_from_mid[1..]);
                paths.insert(*sink, path_to_mid);
            }

            routing.result = Some(RoutingResult { paths, nodes });
            Ok(())
        } else {
            routing.result = None; // No sinks found
            Err("Error".to_string())
        }
    }
}

#[derive(Eq, PartialEq, Deserialize, Debug, Clone, Serialize)]
pub struct SimpleSteinerSolver;

impl SolveRouting for SimpleSteinerSolver {
    fn solve(&self, graph: &FabricGraph, routing: &mut Routing) -> Result<(), String> {
        if let Some(steiner_tree) = &routing.steiner_tree {
            let mut paths = HashMap::new();
            let mut nodes = HashSet::new();
            for (terminal, route) in &steiner_tree.steiner_nodes {
                let mut path = Vec::new();
                for steiner_node in route.windows(2) {
                    let (start, end) = (steiner_node[0], steiner_node[1]);
                    let (a, _b) = match graph.dijkstra(start, end) {
                        Some(res) => res,
                        None => return Err(format!("Could not find path between steinere nodes: {start}, {end}")),
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
            Err("No steiner Tree precalculated.".to_string())
        }
    }

    fn identifier(&self) -> &'static str {
        "SimpleSteinerSolver"
    }
}
