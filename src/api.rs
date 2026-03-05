use std::fs;

use rand::seq::SliceRandom;

use crate::{
    fabric_graph::{bucket_luts, FabricGraph}, fasm::routing_to_fasm, node::NodeId, path_finder::{route, Config}, route_plan::{Net, NetExternal, NetList, NetListExternal}, solver::SolveRouting, validate, FabricError, FabricResult, Logging 
};

pub fn start_routing<T, L>(
    graph_path: &str,
    routing_list: &str,
    solver: &T,
    hist_factor: f32,
    output_path: &str,
    logger: &L,
    max_iterations: usize,
) -> FabricResult<()>
where
    T: SolveRouting,
    L: Logging
{
    let mut graph = FabricGraph::from_file(graph_path)?;
    let route_plan_external = NetListExternal::from_file(routing_list)?;
    let mut route_plan = NetList::from_external(&graph,&route_plan_external)?;
    let config = Config::new(hist_factor, max_iterations);

    match route(&mut route_plan, &mut graph, &config, solver, logger) {
        Ok(_x) => {
            let ex = route_plan.to_external(&graph);
            let out = if output_path.ends_with("fasm") {
                routing_to_fasm(&ex)
            } else {
                serde_json::to_string_pretty(&ex.plan)?
            };
            fs::write(output_path, out).map_err(|e| FabricError::Io {
                path: output_path.to_string(),
                source: e,
            })?;
            Ok(())
        }
        Err(_) => Err("Test".into()),
    }
}

pub fn create_fasm(expanded_routing: &str, output_path: &str) -> Result<(), String> {
    let route_plan = NetListExternal::from_file(expanded_routing)
        .map_err(|_| format!("Error reading routeplan: {expanded_routing}"))?;
    let fasm = routing_to_fasm(&route_plan);
    fs::write(output_path, fasm).map_err(|_| format!("Error writing to file: {output_path}"))?;
    Ok(())
}

pub fn create_test(graph_path: &str, output_path: &str, percentage: f32, destinations: usize) -> FabricResult<()> {
    let mut rng = rand::rng();
    let graph = FabricGraph::from_file(graph_path)?;
    let (mut inputs, mut outputs) = bucket_luts(&graph.nodes);

    inputs.shuffle(&mut rng);
    outputs.shuffle(&mut rng);

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
    let input_count = (percentage * outputs.len() as f32) as usize;
    let output_count = input_count * destinations;
    
    if input_count > outputs.len() {
        return Err(FabricError::CreatingTestBadParameters)
    }
    if output_count > inputs.len() {
        return Err(FabricError::CreatingTestBadParameters)
    }

    let used_outs = inputs.iter().take(output_count).copied().collect::<Vec<NodeId>>();

    let route_plan = outputs
        .iter()
        .take(input_count)
        .copied()
        .zip(used_outs.chunks(destinations))
        .map(|(signal, sinks)| {
            Net {
                sinks: sinks.to_vec(),
                signal,
                result: None,
                steiner_tree: None,
                priority: None,
            }
            .to_external(&graph)
        })
        .collect::<Vec<NetExternal>>();

    let pretty = serde_json::to_string_pretty(&route_plan)?;
    fs::write(output_path, pretty).map_err(|e| FabricError::Io {path: output_path.to_string(), source: e})?;
    Ok(())
}

pub fn validate_routing(graph_path: &str, routing_list: &str) -> FabricResult<()> {
    let graph = FabricGraph::from_file(graph_path).map_err(|_| format!("Error reading file: {graph_path}"))?;
    let route_plan = NetListExternal::from_file(routing_list)?;
    let route_plan = NetList::from_external(&graph, &route_plan)
        .map_err(|_| format!("Error reading file: {routing_list}"))?;
    validate::validate(&route_plan, &graph)?;
    Ok(())
}
