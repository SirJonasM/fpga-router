use rand::seq::SliceRandom;

use crate::IterationResult;
use crate::{
    FabricError, FabricResult, Logging,
    fasm::routing_to_fasm,
    graph::fabric_graph::{FabricGraph, bucket_luts},
    graph::node::NodeId,
    netlist::{NetExternal, NetInternal, NetListExternal, NetListInternal},
    path_finder::{Config, path_finder},
    slack::SlackReport,
    solver::RouteNet,
    validate,
};

pub struct RoutingConfig {
    pub graph: FabricGraph,
    pub net_list: NetListExternal,
    pub hist_factor: f32,
    pub max_iterations: usize,
    pub slack_report: Option<SlackReport>,
}

/// Tries to solve a `NetList`
///
/// # Errors
/// Fails if files cannot be read or cannot be parsed or it cannot write to the output file.
/// Fails if the `max_iterations` are reached
pub fn route<T, L>(config: &mut RoutingConfig, solver: &T, logger: &L) -> FabricResult<Vec<IterationResult>>
where
    T: RouteNet,
    L: Logging,
{
    let net_list_external = &mut config.net_list;
    let graph = &mut config.graph;
    if let Some(hash) = &net_list_external.hash {
        if hash != &graph.calculate_structure_hash() {
            eprintln!("Warning: The net-list was not created with this graph.");
        }
    } else {
        eprintln!("Warning: Cannot determine if the net-list was created with this graph. Missing field in net-list.");
    }
    if let Some(slack_report) = &config.slack_report {
        net_list_external.add_slack(slack_report);
    }
    let mut net_list = NetListInternal::from_external(graph, net_list_external)?;
    let config = Config::new(config.hist_factor, config.max_iterations);

    let x = path_finder(&mut net_list, graph, &config, solver, logger);
    if x.is_ok() {
        *net_list_external = net_list.to_external(graph);
    }
    x
}

/// Can be used to create a `FASM` file from a netlist
/// # Errors
/// Fails if files do not exist or deserialization does not succeed.
pub fn create_fasm(netlist: &NetListExternal) -> FabricResult<String> {
    let fasm = routing_to_fasm(netlist);
    Ok(fasm)
}

/// Creates a Test Netlist by using a `percentage` of all Lut-Outputs and for each `destinations`
/// Lut-Inputs
///
/// # Errors
/// Can produce File Io erros.
/// Fails if parameters are bad like trying to use more than 100% of Lut-Outputs
pub fn create_test(graph: &FabricGraph, percentage: f32, destinations: usize) -> FabricResult<NetListExternal> {
    let mut rng = rand::rng();
    let graph_hash = graph.calculate_structure_hash();
    let (mut inputs, mut outputs) = bucket_luts(graph);

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

    let net_list = outputs
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
            .to_external(graph)
        })
        .collect::<Vec<NetExternal>>();

    let net_list = NetListExternal {
        plan: net_list,
        hash: Some(graph_hash),
    };

    Ok(net_list)
}

/// Validates a routing for a given `FabricGraph`
///
/// # Errors
/// Fails when netlist is invalid
pub fn validate_routing(graph: &FabricGraph, netlist: &NetListExternal) -> FabricResult<()> {
    let netlist = NetListInternal::from_external(graph, netlist)?;
    validate::validate(&netlist, graph)?;
    Ok(())
}
