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

use crate::graph::fabric_graph::Fabric;
use crate::graph::{fabric_graph::FabricGraph, node::NodeId};
use crate::netlist::NetInternal;
use crate::solver::RouteNet;
use crate::{FabricError, FabricResult, Logging, netlist::NetListInternal};
use crate::{LogInstance, SlackReport};

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

pub trait TimingAnalysis {
    /// This runs the timing analysis that is called by the `timing_driven_path_finder` and returns the Slack Report
    /// # Errors
    ///
    fn timing_analysis(&self, graph: &FabricGraph, net_list: &NetListInternal) -> FabricResult<SlackReport>;
}

/// Execute routing using the `path_finder` algorihm
///
/// # Arguments
/// * `net_list` - Array of routing requests to process
/// * `graph` - FPGA fabric graph
/// * `config` - Configuration parameters
/// * `solver` - Solver that implements `Solve` to solve a `NetInternal` (Router)
/// * `logger` - Object implementing `Logging` to capture iteration results
///
/// # Errors
///
pub fn path_finder<R, L>(
    net_list: &mut NetListInternal,
    fabric: &mut Fabric,
    config: &Config,
    solver: &R,
    logger: &L,
) -> FabricResult<Vec<IterationResult>>
where
    R: RouteNet,
    L: Logging,
{
    let hist_fac = config.hist_factor;
    let mut iteration_report = Vec::new();

    let mut i = 0;
    let mut last_conflicts = 0;
    let mut same_conflicts = 0;
    solver.pre_process(&mut fabric.graph, &mut net_list.plan)?;
    let max_iterations = config.max_iterations;

    loop {
        let time1 = Instant::now();
        let conflicts = iteration(&mut fabric.graph, &mut net_list.plan, solver, hist_fac)
            .map_err(|e| FabricError::IterationError { source: e.into() })?;
        let duration = time1.elapsed();
        let result = analyze_result(i, conflicts, duration, &fabric.graph, net_list, config);

        if result.conflicts == last_conflicts {
            same_conflicts += 1;
        }
        last_conflicts = result.conflicts;

        if result.conflicts == 0 {
            logger.log(&LogInstance::RouterIteration(&result))?;
            iteration_report.push(result);
            return Ok(iteration_report);
        }

        if i == max_iterations {
            logger.log(&LogInstance::RouterIteration(&result))?;
            let congestion_report = congestion_report(net_list);
            let congestion_report = CongestionReportExtern::from_intern(&congestion_report, &fabric.graph);
            return Err(FabricError::RoutingMaxIterationsReached {
                congestion_report,
                iteration_report,
            });
        }

        if same_conflicts == 200 {
            solver.pre_process(&mut fabric.graph, &mut net_list.plan)?;
        }
        logger.log(&LogInstance::RouterIteration(&result))?;
        iteration_report.push(result);
        i += 1;
    }
}

pub fn timing_driven_path_finder<R, L, T>(
    net_list: &mut NetListInternal,
    graph: &mut FabricGraph,
    config: &Config,
    solver: &R,
    logger: &L,
    sta: &T,
) -> FabricResult<Vec<IterationResult>>
where
    R: RouteNet,
    L: Logging,
    T: TimingAnalysis,
{
    let hist_fac = config.hist_factor;
    let mut iteration_report = Vec::new();

    let mut i = 0;
    let mut last_conflicts = 0;
    let mut same_conflicts = 0;
    solver.pre_process(graph, &mut net_list.plan)?;
    let max_iterations = config.max_iterations;

    loop {
        let time1 = Instant::now();
        let conflicts = iteration(graph, &mut net_list.plan, solver, hist_fac)
            .map_err(|e| FabricError::IterationError { source: e.into() })?;
        let duration = time1.elapsed();
        let result = analyze_result(i, conflicts, duration, graph, net_list, config);

        let slack_report = sta.timing_analysis(graph, net_list)?;
        net_list.set_slack(&slack_report);

        if result.conflicts == last_conflicts {
            same_conflicts += 1;
        }
        last_conflicts = result.conflicts;

        if result.conflicts == 0 {
            logger.log(&LogInstance::RouterIteration(&result))?;
            iteration_report.push(result);
            return Ok(iteration_report);
        }

        if i == max_iterations {
            logger.log(&LogInstance::RouterIteration(&result))?;
            let congestion_report = congestion_report(net_list);
            let congestion_report = CongestionReportExtern::from_intern(&congestion_report, graph);
            return Err(FabricError::RoutingMaxIterationsReached {
                congestion_report,
                iteration_report,
            });
        }

        if same_conflicts == 200 {
            solver.pre_process(graph, &mut net_list.plan)?;
        }
        logger.log(&LogInstance::RouterIteration(&result))?;
        iteration_report.push(result);
        i += 1;
    }
}

fn congestion_report(net_list: &NetListInternal) -> CongestionReportIntern {
    let mut congestion: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    let mut net_congestion: HashMap<NodeId, f32> = HashMap::new();

    for net in &net_list.plan {
        let signal = net.signal;
        if let Some(net_res) = &net.result {
            for node_id in &net_res.nodes {
                congestion
                    .entry(*node_id)
                    .and_modify(|node_usage| node_usage.push(signal))
                    .or_insert_with(|| vec![signal]);
            }
        }
    }
    for net in &net_list.plan {
        let signal_id = net.signal;
        if let Some(net_result) = &net.result {
            #[allow(clippy::cast_precision_loss)]
            {
                let signal_congestion = net_result
                    .nodes
                    .iter()
                    .map(|node_id| congestion.get(node_id).map_or(0.0, |con| con.len() as f32))
                    .sum::<f32>()
                    / net_result.nodes.len() as f32;
                net_congestion.insert(signal_id, signal_congestion);
            }
        }
    }
    congestion.retain(|_k, v| v.len() > 1);
    CongestionReportIntern {
        congestion,
        net_congestion,
    }
}

#[derive(Debug)]
pub struct CongestionReportIntern {
    pub congestion: HashMap<NodeId, Vec<NodeId>>,
    pub net_congestion: HashMap<NodeId, f32>,
}

impl CongestionReportExtern {
    #[must_use]
    pub fn from_intern(intern: &CongestionReportIntern, graph: &FabricGraph) -> Self {
        let congestion = intern
            .congestion
            .iter()
            .map(|(key, value)| {
                let mapped_key = graph.get_node(*key).id();
                let mapped_value = value.iter().map(|id| graph.get_node(*id).id()).collect();
                (mapped_key, mapped_value)
            })
            .collect::<HashMap<String, Vec<String>>>();
        let congestion_signals = intern
            .net_congestion
            .iter()
            .map(|(key, value)| {
                let mapped_key = graph.get_node(*key).id();
                (mapped_key, *value)
            })
            .collect::<HashMap<String, f32>>();

        Self {
            congestion,
            net_congestion: congestion_signals,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CongestionReportExtern {
    pub congestion: HashMap<String, Vec<String>>,
    pub net_congestion: HashMap<String, f32>,
}
/// Perform a single iteration of routing for all routing requests.
///
/// Updates node usages and calculates conflicts
pub fn iteration(
    graph: &mut FabricGraph,
    routing: &mut [NetInternal],
    solver: &dyn RouteNet,
    hist_fac: f32,
) -> FabricResult<usize> {
    let mut routing_failed = vec![];
    for net in &mut *routing {
        if let Err(e) = solver.solve(graph, net)
            && let FabricError::PathfindingFailed { start, sink } = e
        {
            routing_failed.push((start, sink));
        }
        if let Some(result) = &net.result {
            result.nodes.iter().for_each(|index| {
                graph.get_costs_mut(*index).usage += 1;
            });
        }
    }
    if !routing_failed.is_empty() {
        return Err(FabricError::Other(
            routing_failed
                .iter()
                .map(|(a, b)| format!("{} -> {}",a.id(), b.id()))
                .collect::<Vec<String>>()
                .join("\n"),
        ));
    }
    let mut conflicts = 0;
    for node in &mut graph.costs {
        if node.update(hist_fac) {
            conflicts += 1;
        }
    }
    Ok(conflicts)
}

fn analyze_result(
    iteration: usize,
    conflicts: usize,
    duration: Duration,
    graph: &FabricGraph,
    net_list: &NetListInternal,
    config: &Config,
) -> IterationResult {
    let mut total_wire_segments = 0;
    let mut total_path_cost = 0.0;
    let mut path_count = 0;

    let mut max_path_info = ((String::new(), String::new()), f32::MIN);
    let mut total_sharing_efficiency = 0.0;

    for net in &net_list.plan {
        let Some(routing) = &net.result else { continue };

        // 1. Calculate Path Costs (Longest and Accumulator for Average)
        for (sink, path) in &routing.paths {
            let mut current_path_cost = 0.0;

            // Calculate cost using windows for edge lookups
            for pair in path.windows(2) {
                let edge = graph.get_edge_panic(pair[0], pair[1]);
                current_path_cost += edge.cost;
            }

            if current_path_cost > max_path_info.1 {
                max_path_info = (
                    (graph.get_node(net.signal).id(), graph.get_node(*sink).id()),
                    current_path_cost,
                );
            }

            total_path_cost += current_path_cost;
            path_count += 1;
        }

        // 2. Calculate Wire Sharing Efficiency for this net
        // unique_nodes: physical wires used. total_points: sum of nodes across all paths.
        let unique_nodes = routing.nodes.len(); // Assuming this contains unique nodes in the Steiner tree
        let total_nodes_in_all_paths: usize = routing.paths.values().map(Vec::len).sum();

        if unique_nodes > 0 {
            #[allow(clippy::cast_precision_loss)]
            {
                total_sharing_efficiency += total_nodes_in_all_paths as f32 / unique_nodes as f32;
            }
        }

        total_wire_segments += unique_nodes;
    }

    #[allow(clippy::cast_precision_loss)]
    let avg_path_cost = if path_count > 0 {
        total_path_cost / path_count as f32
    } else {
        0.0
    };
    #[allow(clippy::cast_precision_loss)]
    let avg_wire_sharing = if net_list.plan.is_empty() {
        0.0
    } else {
        total_sharing_efficiency / net_list.plan.len() as f32
    };

    let longest_path = (max_path_info.0.0, max_path_info.0.1);

    IterationResult {
        iteration,
        conflicts,
        longest_path,
        longest_path_cost: max_path_info.1,
        average_path_cost: avg_path_cost,
        total_wire_use: total_wire_segments,
        wire_reuse: avg_wire_sharing,
        duration,
        test_case: config.clone(),
    }
}

#[derive(Deserialize, Debug, Clone, Serialize, Default)]
pub struct IterationResult {
    pub iteration: usize,
    pub test_case: Config,
    pub conflicts: usize,
    pub longest_path: (String, String),
    pub longest_path_cost: f32,
    pub average_path_cost: f32,
    pub total_wire_use: usize,
    pub wire_reuse: f32,
    pub duration: Duration,
}

impl Display for IterationResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Iteration: {}, Conflicts: {}, Wire Efficency: {}, ",
            self.iteration, self.conflicts, self.wire_reuse
        )
    }
}
