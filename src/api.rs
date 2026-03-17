use std::process::Command;
use std::{fs, path::Path};

use rand::seq::SliceRandom;

use crate::logger::LogInstance;
use crate::{
    FabricError, FabricResult, Logging,
    fabric_graph::{FabricGraph, bucket_luts},
    fasm::routing_to_fasm,
    netlist::{NetExternal, NetInternal, NetListExternal, NetListInternal},
    node::NodeId,
    path_finder::{Config, route},
    slack::SlackReport,
    solver::SolveRouting,
    validate,
};

pub struct RoutingConfig<P: AsRef<Path>> {
    pub graph_file: P,
    pub net_list_file: P,
    pub output_file: P,
    pub hist_factor: f32,
    pub max_iterations: usize,
}

/// Tries to solve a `NetList`
///
/// # Errors
/// Fails if files cannot be read or cannot be parsed or it cannot write to the output file.
/// Fails if the `max_iterations` are reached
pub fn start_routing<T, L, P>(config: RoutingConfig<P>, slack_report: Option<P>, solver: &T, logger: &L) -> FabricResult<()>
where
    T: SolveRouting,
    L: Logging,
    P: AsRef<Path>,
{
    let output_file_ref = config.output_file.as_ref();
    let mut graph = FabricGraph::from_file(config.graph_file)?;
    let mut route_plan_external = NetListExternal::from_file(config.net_list_file)?;
    if let Some(slack_report) = slack_report {
        let slack_report = SlackReport::from_file(slack_report)?;
        route_plan_external.add_slack(&slack_report);
    }
    let mut route_plan = NetListInternal::from_external(&graph, &route_plan_external)?;
    let config = Config::new(config.hist_factor, config.max_iterations);

    match route(&mut route_plan, &mut graph, &config, solver, logger) {
        Ok(_x) => {
            let ex = route_plan.to_external(&graph);
            let out = if let Some(s) = output_file_ref.extension()
                && s == "fasm"
            {
                routing_to_fasm(&ex)
            } else {
                serde_json::to_string_pretty(&ex.plan)?
            };
            fs::write(output_file_ref, out).map_err(|e| FabricError::Io {
                path: output_file_ref.to_path_buf(),
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
pub fn create_test<P: AsRef<Path>>(graph_file: P, output_file: P, percentage: f32, destinations: usize) -> FabricResult<()> {
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
    fs::write(&output_file, pretty).map_err(|e| FabricError::Io {
        path: output_file.as_ref().to_path_buf(),
        source: e,
    })?;
    Ok(())
}

/// Starts a routing and runs a Static Timing Analysis to meet timing requirements.
/// # Errors
/// This fails if `max_iterations` or `max_sta_cycles` are reached
/// It also writes to files so `io::Errors` can occure
pub fn route_sta<T, L, P>(
    routing_config: RoutingConfig<P>,
    max_sta_cycles: usize,
    target_ps: u32,
    solver: &T,
    logger: &L,
) -> FabricResult<()>
where
    T: SolveRouting,
    L: Logging,
    P: AsRef<Path>,
{
    let mut graph = FabricGraph::from_file(routing_config.graph_file)?;
    let net_list = NetListExternal::from_file(routing_config.net_list_file)?;
    let mut net_list = NetListInternal::from_external(&graph, &net_list)?;
    let config = Config::new(routing_config.hist_factor, routing_config.max_iterations);

    let slack_file = "current_slack.csv";

    for current_cycle in 0..max_sta_cycles {
        logger.log(&LogInstance::from(format!("\n=== STA Routing Cycle {current_cycle} ===")))?;

        // 2. ROUTE: Run the actual Pathfinder iterations
        // You might need to expose a function that takes objects, not paths
        route(&mut net_list, &mut graph, &config, solver, logger)?;
        let mut net_list = net_list.to_external(&graph);
        fs::write(&routing_config.output_file, routing_to_fasm(&net_list)).map_err(|e| FabricError::Io {
            path: routing_config.output_file.as_ref().to_path_buf(),
            source: e,
        })?;

        // 4. ANALYZE: Call Python Mock STA
        logger.log(&LogInstance::from("Running STA Analysis..."))?;
        match run_mock_sta(&routing_config.output_file, slack_file, target_ps) {
            Ok(r) => {
                logger.log(&LogInstance::Text(r))?;
                return Ok(());
            }
            Err(MockError::Slack(out)) => logger.log(&LogInstance::Text(out))?,
            Err(MockError::Other(err)) => return Err(FabricError::StaFailed(err)),
        }

        // 5. EVALUATE: Load report and check bounds
        let report = SlackReport::from_file(slack_file)?;

        if report.get_worst_slack() > 0.0 {
            logger.log(&LogInstance::from("Success: Timing met and congestion resolved."))?;
            break;
        }

        graph.reset_usage();
        net_list.add_slack(&report);
    }
    Err(FabricError::TimingNotMet)
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

enum MockError {
    Slack(String),
    Other(String),
}

// This is just for the moment as there is no current implementation of the STA
fn run_mock_sta<P: AsRef<Path>>(fasm_in: &P, csv_out: &str, target: u32) -> Result<String, MockError> {
    let output = Command::new("python3")
        .arg("mock_slack.py") // Name of your python script
        .arg(fasm_in.as_ref().to_str().unwrap())
        .arg(csv_out)
        .arg("--target")
        .arg(target.to_string())
        .output()
        .map_err(|e| MockError::Other(format!("{e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(MockError::Slack(format!("Result: {stdout}\n STA Script Error: {err}")));
    }
    Ok(stdout)
}
