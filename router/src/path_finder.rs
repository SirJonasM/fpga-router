//! Module `path_finder`
//!
//! Implements the pathfinding algorithm for FPGA routing using iterative
//! conflict-driven optimization. This module contains functions to execute
//! routing iterations, log results, and validate routing correctness.
#![macro_use]
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant};

use crate::fabric_graph::{FabricGraph, SteinerTreeCandidate};
use crate::fabric_graph::{Routing, SteinerTree};
use crate::solver::{SimpleSolver, SimpleSteinerSolver, SolveRouting, Solver};

/// Trait for logging pathfinding iterations.
pub trait Logging {
    /// Logs the current iteration result.
    fn log(&self, log_instance: &IterationResult);
}

/// Test case parameters for running a routing algorithm.
#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct Config {
    /// Unique ID for the test case
    pub id: u64,
    /// Historical cost factor for congestion handling
    pub hist_factor: f32,
    /// Solver to use (Simple or Steiner)
    pub solver: Solver,
    /// The maximum iterations the path finder algorithm will try to solve the routing
    pub max_iterations: usize,
}

static COUNTER: AtomicU64 = AtomicU64::new(0);
impl Config {
    pub fn new(hist_factor: f32, solver: Solver, max_iterations: usize) -> Self {
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self { id, hist_factor, solver , max_iterations}
    }
}
impl Default for Config {
    fn default() -> Self {
        Self::new(0.1, Solver::Simple(SimpleSolver), 1000)
    }
}
fn pre_process(graph: &mut FabricGraph, route_plan: &mut [Routing]) {
    let mut nodes = HashSet::new();
    for route in route_plan.iter_mut() {
        let x = route.pre_calc_steiner_tree(graph).unwrap();

        if x.nodes.iter().any(|a| nodes.contains(a)) {
            panic!("Steiner Node is already used.")
        }
        nodes.extend(x.nodes.clone());
        route.steiner_tree = Some(x);
    }
    graph.reset_usage()
}

/// Execute routing for a given `TestCase` and `FabricGraph`.
///
/// # Arguments
/// * `logger` - Object implementing `Logging` to capture iteration results
/// * `test_case` - Parameters for this routing run
/// * `graph` - FPGA fabric graph
/// * `route_plan` - Array of routing requests to process
///
/// # Returns
/// - `Ok(IterationResult)` if routing succeeds with zero conflicts
/// - `Err(IterationResult)` if routing reaches `MAX_ITERATION` without resolving all conflicts
pub fn route(
    route_plan: &mut [Routing],
    graph: &mut FabricGraph,
    config: Config,
    logger: &dyn Logging,
) -> Result<IterationResult, IterationResult> {
    let hist_fac = config.hist_factor;

    let mut i = 0;
    let mut last_conflicts = 0;
    let mut same_conflicts = 0;
    if config.solver == Solver::SimpleSteiner(SimpleSteinerSolver) {
        pre_process(graph, route_plan);
    }
    let max_iterations = config.max_iterations;
    loop {
        let mut result = match iteration(graph, route_plan, &config.solver, hist_fac) {
            Ok(iteration_result) => iteration_result,
            Err(err) => panic!("Error in interation {}: {}", i, err),
        };
        result.iteration = i;
        result.test_case = config.clone();

        logger.log(&result);

        if result.conflicts == last_conflicts {
            same_conflicts += 1;
        }
        if result.conflicts == 0 {
            return Ok(result);
        };

        if i == max_iterations {
            return Err(result);
        }
        last_conflicts = result.conflicts;
        if same_conflicts == 200
            && let Solver::SimpleSteiner(_) = config.solver
        {
            pre_process(graph, route_plan);
        }
        i += 1;
    }
}

/// Perform a single iteration of routing for all routing requests.
///
/// Updates node usages, calculates conflicts, and returns iteration statistics.
pub fn iteration(
    graph: &mut FabricGraph,
    routing: &mut [Routing],
    solver: &Solver,
    hist_fac: f32,
) -> Result<IterationResult, String> {
    let time1 = Instant::now();
    for route in &mut *routing {
        match solver {
            Solver::Simple(simple_solver) => simple_solver.solve(graph, route),
            Solver::Steiner(steiner_solver) => steiner_solver.solve(graph, route),
            Solver::SimpleSteiner(simple_steiner_solver) => simple_steiner_solver.solve(graph, route),
        }?;
        if let Some(result) = &route.result {
            result.nodes.iter().for_each(|index| {
                graph.costs[*index].usage += 1;
            })
        }
    }
    let mut conflicts = 0;
    for node in &mut graph.costs {
        if node.update(hist_fac) {
            conflicts += 1;
        }
    }
    let duration = time1.elapsed();
    let result = analyze_result(conflicts, duration, graph, routing);
    Ok(result)
}

/// Analyze the routing result for metrics like longest path, total wire usage, and wire reuse.
fn analyze_result(conflicts: usize, duration: Duration, graph: &mut FabricGraph, steiner: &[Routing]) -> IterationResult {
    let mut result = IterationResult {
        iteration: 0,
        conflicts,
        test_case: Config {
            id: 0,
            hist_factor: 0.0,
            solver: Solver::Simple(SimpleSolver),
            max_iterations: 1000,
        },
        longest_path: (0, 0),
        longest_path_cost: 0.0,
        average_path: 0.0,
        total_wire_use: 0,
        wire_reuse: 0.0,
        duration: duration.as_micros(),
    };
    let mut total_wire_use = 0;
    for s in steiner {
        if let Some(steiner_result) = &s.result {
            let mut usages = HashMap::new();
            let paths = &steiner_result.paths;
            for (sink, path) in paths {
                let mut cost = 0.0;
                assert_eq!(path[0], s.signal);
                assert_eq!(path[path.len() - 1], *sink);
                for pair in path.windows(2) {
                    let (start, end) = (pair[0], pair[1]);
                    let edge = match graph.map[start].iter().find(|a| a.node_id == end) {
                        Some(edge) => edge,
                        None => panic!("Graph did not contain the edge: a: {}, b: {}", start, end),
                    };
                    cost += edge.cost;
                }

                if result.longest_path_cost < cost {
                    result.longest_path = (s.signal, *sink);
                    result.longest_path_cost = cost;
                }
            }

            for node in steiner_result.paths.values().flatten() {
                usages.entry(node).and_modify(|x| *x += 1).or_insert(1);
            }
            result.wire_reuse += usages.values().sum::<i32>() as f32 / usages.len() as f32;
            total_wire_use += steiner_result.nodes.len();
        }
    }
    result.wire_reuse /= steiner.len() as f32;
    result.total_wire_use = total_wire_use;
    result
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct IterationResult {
    pub iteration: usize,
    pub test_case: Config,
    pub conflicts: usize,
    pub longest_path: (usize, usize),
    pub longest_path_cost: f32,
    pub average_path: f32,
    pub total_wire_use: usize,
    pub wire_reuse: f32,
    pub duration: u128,
}

impl IterationResult {
    pub fn csv_header() -> &'static str {
        "iteration,test_id,percentage,dst,hist_factor,solver,conflicts,longest_path_start,longest_path_end,longest_path_cost,average_path,total_wire_use,wire_reuse,duration"
    }
}

impl Display for IterationResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let solver = match &self.test_case.solver {
            Solver::Simple(simple_solver) => simple_solver.identifier().to_string(),
            Solver::Steiner(steiner_solver) => steiner_solver.identifier().to_string(),
            Solver::SimpleSteiner(simple_steiner_solver) => simple_steiner_solver.identifier().to_string(),
        };
        write!(
            f,
            "{},{},{},{},{},{},{},{},{},{},{},{}",
            self.iteration,
            self.test_case.id,
            self.test_case.hist_factor,
            solver,
            self.conflicts,
            self.longest_path.0,
            self.longest_path.1,
            self.longest_path_cost,
            self.average_path,
            self.total_wire_use,
            self.wire_reuse,
            self.duration
        )
    }
}

use std::collections::{HashSet, VecDeque};

/// Validate SteinerTrees on a FabricGraph.
///
/// Conditions:
/// 1. Each SteinerTree has a result (steiner_nodes, nodes, cost).
/// 2. From the "signal" node you can reach all sinks, using only `result.nodes`.
/// 3. No node belongs to more than one signal's Steiner tree.
/// 4. All node IDs referenced exist inside the FabricGraph.
pub fn validate_routing(graph: &FabricGraph, routing: &[Routing]) -> Result<(), String> {
    let mut used_nodes_global: HashSet<usize> = HashSet::new();
    let node_count = graph.nodes.len();

    for (tree_idx, tree) in routing.iter().enumerate() {
        let result = tree
            .result
            .as_ref()
            .ok_or_else(|| format!("Tree {} has no SteinerTreeResult", tree_idx))?;

        // --- Check: all nodes exist ---
        for &n in &result.nodes {
            if n >= node_count {
                return Err(format!("Tree {} contains invalid node index {} (out of range)", tree_idx, n));
            }
        }

        // --- Check: no node is used in multiple signals ---
        for &n in &result.nodes {
            if !used_nodes_global.insert(n) {
                return Err(format!(
                    "Node {} is used by more than one signal (conflict at tree {})",
                    n, tree_idx
                ));
            }
        }

        // --- Check: signal exists ---
        if tree.signal >= node_count {
            return Err(format!("Tree {} uses invalid signal node {}", tree_idx, tree.signal));
        }

        // --- Reachability check: signal -> every sink using only result.nodes ---
        for &sink in &tree.sinks {
            if sink >= node_count {
                return Err(format!("Tree {} has invalid sink {}", tree_idx, sink));
            }

            if !is_reachable_within_set(graph, tree.signal, sink, &result.nodes) {
                println!("Sink in nodes: {}", result.nodes.contains(&sink));
                return Err(format!(
                    "Tree {}: sink {} is NOT reachable from signal {} using tree nodes",
                    tree_idx, sink, tree.signal,
                ));
            }
        }
    }

    Ok(())
}

/// BFS restricted to `allowed` node set.
fn is_reachable_within_set(graph: &FabricGraph, start: usize, target: usize, allowed: &HashSet<usize>) -> bool {
    if start == target {
        return true;
    }
    if !allowed.contains(&start) || !allowed.contains(&target) {
        return false;
    }

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    visited.insert(start);
    queue.push_back(start);

    while let Some(u) = queue.pop_front() {
        for edge in &graph.map[u] {
            let v = edge.node_id;

            if !allowed.contains(&v) {
                continue;
            }
            if visited.insert(v) {
                if v == target {
                    return true;
                }
                queue.push_back(v);
            }
        }
    }
    false
}

impl Routing {
    pub fn pre_calc_steiner_tree(&self, graph: &mut FabricGraph) -> Result<SteinerTree, String> {
        let dists = self
            .sinks
            .par_iter()
            .map(|sink| (*sink, graph.dijkstra_all(*sink)))
            .collect::<HashMap<usize, Vec<f32>>>();
        let signal = self.signal;
        let base_paths: Vec<(usize, usize)> = self.sinks.iter().map(|&sink| (signal, sink)).collect();

        let mut errors = Vec::new();

        // 1. Parallel reduction to find the single best SteinerCandidate
        let best_candidate = base_paths
            .into_par_iter()
            .map(|(start, base_sink)| {
                // --- Computation to find the MINIMUM COST ---
                // Calculate the cost of the base path (Dijkstra is still necessary here)
                let (base_path, mut costs) = match graph.dijkstra(start, base_sink) {
                    Some(result) => result,
                    None => {
                        return Err(format!(
                            "Could not determine a route for the Base bath: start: {}, sink: {}",
                            start, base_sink
                        ));
                    }
                };

                let mut nodes = HashSet::new();
                // Calculate the cost of connecting all other sinks to this base path
                let min_points = self
                    .sinks
                    .iter()
                    .cloned()
                    .map(|sink| {
                        let terminal_distances = match dists.get(&sink) {
                            Some(dist) => dist,
                            None => return Err(format!("No distances pre caclulated for the sink: {}.", sink)),
                        };

                        // Find the connection node (min_node) on the base_path
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
                            .unwrap();

                        // This cost is the *shortest path cost* from the base path to the sink.
                        costs += cost_to_base_path;
                        nodes.insert(min_node);
                        Ok((sink, min_node))
                    })
                    .collect::<Result<HashMap<usize, usize>, String>>()?;

                let mut steiner_nodes = HashMap::new();
                for sink in &self.sinks {
                    let mut sink_uses_steiner_nodes = vec![self.signal];
                    let m = match min_points.get(sink) {
                        Some(m) => m,
                        None => return Err(format!("No midpoint calculated for sink {sink}")),
                    };
                    for n in &base_path {
                        if n == m {
                            sink_uses_steiner_nodes.push(*sink);
                            steiner_nodes.insert(*sink, sink_uses_steiner_nodes);
                            break;
                        }
                        if nodes.contains(n) {
                            sink_uses_steiner_nodes.push(*n)
                        }
                    }
                }
                // Return only the lightweight candidate struct
                Ok(SteinerTreeCandidate {
                    nodes,
                    steiner_nodes,
                    costs,
                })
            })
            .collect::<Vec<Result<SteinerTreeCandidate, String>>>();

        let best_candidate = best_candidate
            .into_iter()
            .filter_map(|a| a.map_err(|e| errors.push(e)).ok())
            // 2. Reduce the candidates to find the one with the minimum cost.
            .min_by(|a, b| {
                if a.costs < b.costs {
                    Ordering::Less
                } else if a.costs > b.costs {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            });

        // 3. Final Calculation: Sequentially calculate the full result for the winner.
        match best_candidate {
            Some(best) => {
                for x in &best.nodes {
                    graph.costs[*x].usage = 1;
                }
                Ok(SteinerTree {
                    nodes: best.nodes,
                    steiner_nodes: best.steiner_nodes,
                })
            }
            None => {
                println!("{:#?}", errors);
                Err("No Steiner tree was found".to_string())
            }
        }
    }
}
