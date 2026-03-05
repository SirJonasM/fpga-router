//! Module `path_finder`
//!
//! Implements the pathfinding algorithm for FPGA routing using iterative
//! conflict-driven optimization. This module contains functions to execute
//! routing iterations, log results, and validate routing correctness.
#![macro_use]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant};

use crate::fabric_graph::FabricGraph;
use crate::fabric_graph::Routing;
use crate::solver::SolveRouting;
use crate::{FabricError, FabricResult};

/// Trait for logging pathfinding iterations.
pub trait Logging {
    /// Logs the current iteration result.
    fn log(&self, log_instance: &IterationResult) -> Result<(), String>;
}

/// Test case parameters for running a routing algorithm.
#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct Config {
    /// Unique ID for the test case
    pub id: u64,
    /// Historical cost factor for congestion handling
    pub hist_factor: f32,
    /// The maximum iterations the path finder algorithm will try to solve the routing
    pub max_iterations: usize,
}

static COUNTER: AtomicU64 = AtomicU64::new(0);
impl Config {
    pub fn new(hist_factor: f32, max_iterations: usize) -> Self {
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            id,
            hist_factor,
            max_iterations,
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        Self::new(0.1, 1000)
    }
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
pub fn route<T, L>(
    route_plan: &mut [Routing],
    graph: &mut FabricGraph,
    config: &Config,
    solver: &T,
    logger: &L,
) -> FabricResult<()>
where
    T: SolveRouting,
    L: Logging,
{
    let hist_fac = config.hist_factor;

    let mut i = 0;
    let mut last_conflicts = 0;
    let mut same_conflicts = 0;
    solver.pre_process(graph, route_plan)?;
    let max_iterations = config.max_iterations;
    loop {
        let mut result =
            iteration(graph, route_plan, solver, hist_fac).map_err(|e| FabricError::IterationError { source: e.into() })?;
        result.iteration = i;
        result.test_case = config.clone();

        logger.log(&result)?;

        if result.conflicts == last_conflicts {
            same_conflicts += 1;
        }
        if result.conflicts == 0 {
            return Ok(());
        }

        if i == max_iterations {
            return Err(FabricError::RoutingMaxIterationsReached);
        }

        last_conflicts = result.conflicts;
        if same_conflicts == 200 {
            solver.pre_process(graph, route_plan)?;
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
    solver: &dyn SolveRouting,
    hist_fac: f32,
) -> FabricResult<IterationResult> {
    let time1 = Instant::now();
    for route in &mut *routing {
        solver.solve(graph, route).map_err(|_e| "Test")?;
        if let Some(result) = &route.result {
            result.nodes.iter().for_each(|index| {
                graph.costs[*index].usage += 1;
            });
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
fn analyze_result(conflicts: usize, duration: Duration, graph: &FabricGraph, steiner: &[Routing]) -> IterationResult {
    let mut result = IterationResult {
        iteration: 0,
        conflicts,
        test_case: Config {
            id: 0,
            hist_factor: 0.0,
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
                    let edge = graph.map[start]
                        .iter()
                        .find(|a| a.node_id == end)
                        .map_or_else(|| panic!("Graph did not contain the edge: a: {start}, b: {end}"), |edge| edge);
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
            #[allow(clippy::cast_precision_loss)]
            let wire_reuse = usages.values().sum::<i32>() as f32;
            #[allow(clippy::cast_precision_loss)]
            let wire_reuse2 = usages.len() as f32;

            result.wire_reuse += wire_reuse / wire_reuse2;
            total_wire_use += steiner_result.nodes.len();
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let wire_reuse3 = steiner.len() as f32;
    result.wire_reuse /= wire_reuse3;
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
    pub const CSV_HEADER: &'static str = "iteration,test_id,percentage,dst,hist_factor,solver,conflicts,longest_path_start,longest_path_end,longest_path_cost,average_path,total_wire_use,wire_reuse,duration";
}

impl Display for IterationResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{},{},{},{},{},{},{},{},{},{},{}",
            self.iteration,
            self.test_case.id,
            self.test_case.hist_factor,
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
