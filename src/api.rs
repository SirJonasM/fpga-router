use std::fs;

use rand::seq::SliceRandom;

use crate::{
    fabric_graph::{FabricGraph, Routing, RoutingExpanded, bucket_luts},
    fasm::routing_to_fasm,
    path_finder::{Config, Logging, route},
    solver::Solver,
    validate,
};

pub fn start_routing(
    graph_path: &str,
    routing_list: &str,
    solver: Solver,
    hist_factor: f32,
    output_path: &str,
    logger: &dyn Logging,
    max_iterations: usize,
) -> Result<(), String> {
    let mut graph = FabricGraph::from_file(graph_path).map_err(|_| format!("Error reading file: {graph_path}"))?;
    let mut route_plan = graph.route_plan_form_file(routing_list).map_err(|_| format!("Error reading file: {routing_list}"))?;
    let config = Config::new(hist_factor, solver, max_iterations);
    println!(
        "Map: {}, Costs: {}",
        graph.map.iter().fold(0, |a, b| a + b.len()),
        graph.costs.len()
    );

    match route(&mut route_plan, &mut graph, config, logger) {
        Ok(x) => {
            println!("Success: {} ", x.iteration);
            let ex = route_plan.iter().map(|x| x.expand(&graph)).collect::<Vec<_>>();
            let out = if output_path.ends_with("fasm") {
                routing_to_fasm(&ex)
            } else {
                serde_json::to_string_pretty(&ex).map_err(|_|"Error serializing route plan")?
            };
            fs::write(output_path, out).map_err(|_| format!("Error writing to file: {output_path}"))?;
            println!("Wrote the routing into {output_path}");
            Ok(())
        }
        Err(x) => Err(format!("Failure: {x} ")),
    }
}

pub fn create_fasm(expanded_routing: &str, output_path: &str) -> Result<(), String> {
    let route_plan = FabricGraph::route_plan_expanded_form_file(expanded_routing)
        .map_err(|_| format!("Error reading routeplan: {expanded_routing}"))?;
    let fasm = routing_to_fasm(&route_plan);
    fs::write(output_path, fasm).map_err(|_| format!("Error writing to file: {output_path}"))?;
    Ok(())
}

pub fn create_test(graph_path: &str, output_path: &str, percentage: f32, destinations: usize) -> Result<(), String> {
    let mut rng = rand::rng();
    let graph = FabricGraph::from_file(graph_path).map_err(|_| format!("Error reading file: {graph_path}"))?;
    let (mut inputs, mut outputs) = bucket_luts(&graph.nodes);

    inputs.shuffle(&mut rng);
    outputs.shuffle(&mut rng);

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
    let input_count = (percentage * outputs.len() as f32) as usize;
    let output_count = input_count * destinations;
    let used_outs = inputs.iter().take(output_count).copied().collect::<Vec<usize>>();

    let route_plan = outputs
        .iter()
        .take(input_count)
        .copied()
        .zip(used_outs.chunks(destinations))
        .map(|(signal, sinks)| {
            Routing {
                sinks: sinks.to_vec(),
                signal,
                result: None,
                steiner_tree: None,
            }
            .expand(&graph)
        })
        .collect::<Vec<RoutingExpanded>>();

    let pretty = serde_json::to_string_pretty(&route_plan).map_err(|_| "Error in serializing route-plan to json".to_string())?;
    fs::write(output_path, pretty).map_err(|_| format!("Error writing route plan to file: {output_path}"))?;
    println!("Test route plan written to {output_path}");
    Ok(())
}

pub fn validate_routing(graph_path: &str, routing_list: &str) -> Result<(), String> {
    let graph = FabricGraph::from_file(graph_path).map_err(|_| format!("Error reading file: {graph_path}"))?;
    let route_plan = graph
        .route_plan_form_file(routing_list)
        .map_err(|_| format!("Error reading file: {routing_list}"))?;
    validate::validate(&route_plan, &graph)
}
