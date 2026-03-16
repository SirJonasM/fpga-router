use std::{collections::HashMap, fs, path::Path};

use rand::seq::SliceRandom;
use serde::Deserialize;

use crate::{
    FabricError, FabricResult, Logging,
    fabric_graph::{FabricGraph, bucket_luts},
    fasm::routing_to_fasm,
    netlist::{NetExternal, NetInternal, NetListExternal, NetListInternal},
    node::NodeId,
    path_finder::{Config, route},
    solver::SolveRouting,
    validate,
};

/// Tries to solve a `NetList`
///
/// # Errors
/// Fails if files cannot be read or cannot be parsed or it cannot write to the output file.
/// Fails if the `max_iterations` are reached
pub fn start_routing<T, L>(
    graph_file: &str,
    netlist_file: &str,
    solver: &T,
    hist_factor: f32,
    output_file: &str,
    logger: &L,
    max_iterations: usize,
    slack_report: Option<String>,
) -> FabricResult<()>
where
    T: SolveRouting,
    L: Logging,
{
    let mut graph = FabricGraph::from_file(graph_file)?;
    let mut route_plan_external = NetListExternal::from_file(netlist_file)?;
    if let Some(slack_report) = slack_report {
        let slack_report = SlackReport::from_file(slack_report)?;
        route_plan_external.add_slack(slack_report)
    };
    let mut route_plan = NetListInternal::from_external(&graph, &route_plan_external)?;
    let config = Config::new(hist_factor, max_iterations);

    match route(&mut route_plan, &mut graph, &config, solver, logger) {
        Ok(_x) => {
            let ex = route_plan.to_external(&graph);
            let out = if output_file.ends_with("fasm") {
                routing_to_fasm(&ex)
            } else {
                serde_json::to_string_pretty(&ex.plan)?
            };
            fs::write(output_file, out).map_err(|e| FabricError::Io {
                path: output_file.to_string(),
                source: e,
            })?;
            Ok(())
        }
        Err(_) => Err("Test".into()),
    }
}

/// Can be used to create a `FASM` file from a netlist
/// # Errors
/// Fails if files do not exist or deserialization does not succeed.
pub fn create_fasm(netlist_file: &str, output_file: &str) -> FabricResult<()> {
    let route_plan = NetListExternal::from_file(netlist_file).map_err(|_| format!("Error reading routeplan: {netlist_file}"))?;
    let fasm = routing_to_fasm(&route_plan);
    fs::write(output_file, fasm).map_err(|_| format!("Error writing to file: {output_file}"))?;
    Ok(())
}

/// Creates a Test Netlist by using a `percentage` of all Lut-Outputs and for each `destinations`
/// Lut-Inputs
///
/// # Errors
/// Can produce File Io erros.
/// Fails if parameters are bad like trying to use more than 100% of Lut-Outputs
pub fn create_test(graph_file: &str, output_file: &str, percentage: f32, destinations: usize) -> FabricResult<()> {
    let mut rng = rand::rng();
    let graph = FabricGraph::from_file(graph_file)?;
    let (mut inputs, mut outputs) = bucket_luts(&graph.nodes);

    inputs.shuffle(&mut rng);
    outputs.shuffle(&mut rng);

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
    let input_count = (percentage * outputs.len() as f32) as usize;
    let output_count = input_count * destinations;

    if input_count > outputs.len() {
        return Err(FabricError::CreatingTestBadParameters);
    }
    if output_count > inputs.len() {
        return Err(FabricError::CreatingTestBadParameters);
    }

    let used_outs = inputs.iter().take(output_count).copied().collect::<Vec<NodeId>>();

    let route_plan = outputs
        .iter()
        .take(input_count)
        .copied()
        .zip(used_outs.chunks(destinations))
        .map(|(signal, sinks)| {
            NetInternal {
                sinks: sinks.to_vec(),
                signal,
                result: None,
                intermediate_nodes: None,
                priority: None,
                criticallity: 0.0,
            }
            .to_external(&graph)
        })
        .collect::<Vec<NetExternal>>();
    let route_plan = NetListExternal { plan: route_plan };

    let pretty = serde_json::to_string_pretty(&route_plan)?;
    fs::write(output_file, pretty).map_err(|e| FabricError::Io {
        path: output_file.to_string(),
        source: e,
    })?;
    Ok(())
}

/// Validates a routing for a given `FabricGraph`
///
/// # Errors
/// Fails when bad files or invalid
pub fn validate_routing(graph_file: &str, netlist_file: &str) -> FabricResult<()> {
    let graph = FabricGraph::from_file(graph_file).map_err(|_| format!("Error reading file: {graph_file}"))?;
    let route_plan = NetListExternal::from_file(netlist_file)?;
    let route_plan =
        NetListInternal::from_external(&graph, &route_plan).map_err(|_| format!("Error reading file: {netlist_file}"))?;
    validate::validate(&route_plan, &graph)?;
    Ok(())
}

/// The raw record format expected from the Timing Team's CSV
#[derive(Debug, Deserialize)]
struct SlackRecord {
    #[serde(rename = "source_wire")]
    source_wire: String,
    #[serde(rename = "slack_ps")]
    slack_ps: f32,
}

pub struct SlackReport {
    /// Mapping of Wire Name -> Slack in picoseconds
    pub slacks: HashMap<String, f32>,
}

impl SlackReport {
    /// Parses the CSV file from the timing team
    pub fn from_file<P: AsRef<Path>>(path: P) -> FabricResult<Self> {
        let mut rdr = csv::Reader::from_path(path)?;
        let mut slacks = HashMap::new();

        for result in rdr.deserialize() {
            let record: SlackRecord = result?;
            slacks.insert(record.source_wire, record.slack_ps);
        }

        Ok(SlackReport { slacks })
    }

    /// Helper to find the worst (most negative) slack for normalization
    pub fn get_worst_slack(&self) -> f32 {
        self.slacks
            .values()
            .cloned()
            .fold(0.0, |min, val| if val < min { val } else { min })
    }
    /// Returns a criticality value between 0.0 and 1.0 for a given wire.
    /// 1.0 = This is the most critical net in the design (worst slack).
    /// 0.0 = This net meets timing or is not in the report.
    pub fn calculate_criticality(&self, source_wire: &str) -> Option<f32> {
        let worst_slack = self.get_worst_slack();

        // If worst_slack is 0 or positive, the whole design meets timing.
        // Everyone gets 0.0 criticality.
        if worst_slack >= 0.0 {
            return None
        }

        if let Some(&slack) = self.slacks.get(source_wire) {
            if slack < 0.0 {
                // Formula: (current_slack / worst_negative_slack)
                // Example: (-500 / -1000) = 0.5 criticality
                // We use .min(1.0) just in case of rounding errors
                let base_crit = (slack / worst_slack).min(1.0);

                // Optional: Sharpening exponent.
                // Using crit^3 is common in FPGA tools to make the router
                // focus HARD on the top 10% of failing nets.
                return Some(base_crit.powf(3.0));
            }
        }

        None
    }
}
