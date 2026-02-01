use std::fs;

use rand::seq::SliceRandom;

use crate::{fabric_graph::{bucket_luts, FabricGraph, Routing, RoutingExpanded}, fasm::routing_to_fasm, path_finder::{route, Config, Logging}, solver::Solver};


pub fn start_routing(
    graph_path: &str,
    routing_list: &str,
    solver: Solver,
    hist_factor: f32,
    output_path: &str,
    logger: &dyn Logging,
    max_iterations: usize,
) {
    let mut graph = FabricGraph::from_file(graph_path).unwrap();
    let mut route_plan = graph.route_plan_form_file(routing_list).unwrap();
    let config = Config::new(hist_factor, solver, max_iterations);
    println!("Map: {}, Costs: {}", graph.map.iter().fold(0, |a,b| a + b.len()), graph.costs.len());

    match route(&mut route_plan,&mut graph, config, logger) {
        Ok(x) => {
            println!("Success: {} ", x.iteration);
            let ex = route_plan.iter().map(|x| x.expand(&graph).unwrap()).collect::<Vec<_>>();
            let out = if output_path.ends_with("fasm") {
                routing_to_fasm(&ex)
            } else {
                serde_json::to_string_pretty(&ex).unwrap()
            };
            fs::write(output_path, out).unwrap();
            println!("Wrote the routing into {}", output_path);
        }
        Err(x) => {
            println!("Failure: {} ", x);
        }
    }
}

pub fn create_fasm(expanded_routing: &str, output_path: &str) {
    let route_plan = FabricGraph::route_plan_expanded_form_file(expanded_routing).unwrap();
    let fasm = routing_to_fasm(&route_plan);
    fs::write(output_path, fasm).unwrap();
}

pub fn create_test(graph_path: &str, output_path: &str, percentage: f32, destinations: usize) {
    let mut rng = rand::rng();
    let graph = FabricGraph::from_file(graph_path).unwrap();
    let (mut inputs, mut outputs) = bucket_luts(&graph.nodes);

    inputs.shuffle(&mut rng);
    outputs.shuffle(&mut rng);

    let input_count = (percentage * outputs.len() as f32) as usize;
    let output_count = input_count * destinations;
    let used_outs = inputs.iter().take(output_count).cloned().collect::<Vec<usize>>();

    let route_plan = outputs
        .iter()
        .take(input_count)
        .cloned()
        .zip(used_outs.chunks(destinations))
        .map(|(signal, sinks)| {
            Routing {
                sinks: sinks.to_vec(),
                signal,
                result: None,
                steiner_tree: None,
            }
            .expand(&graph)
            .unwrap()
        })
        .collect::<Vec<RoutingExpanded>>();

    let pretty = serde_json::to_string_pretty(&route_plan).unwrap();
    fs::write(output_path, pretty).unwrap();
    println!("Test route plan written to {}", output_path);
}
