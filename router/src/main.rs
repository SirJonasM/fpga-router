mod cli;
use clap::Parser;
use cli::*;
use rand::seq::SliceRandom;
use router::{Config, FabricGraph, Logging, Routing, RoutingExpanded, SimpleSolver, SimpleSteinerSolver, Solver, SteinerSolver};
use std::io::Write;
use std::{
    fs::{self, File},
    io::BufWriter,
    sync::Mutex,
};

// --- Logic Helpers ---

struct SysOutLog;
struct SimpleLogger {
    // We wrap the writer in a Mutex so we can modify it via an immutable &self
    writer: Mutex<BufWriter<File>>,
}
impl Logging for SysOutLog {
    fn log(&self, log_instance: &router::IterationResult) {
        println!("{:?}", log_instance)
    }
}

impl SimpleLogger {
    pub fn new(path: &str) -> Self {
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("Could not open log file");

        Self {
            writer: Mutex::new(BufWriter::new(file)),
        }
    }
}

impl Logging for SimpleLogger {
    fn log(&self, log_instance: &router::IterationResult) {
        // Lock the mutex. If another thread is logging, this will wait its turn.
        let mut guard = self.writer.lock().expect("Failed to lock log file mutex");

        // Serialize and write
        if let Ok(json) = serde_json::to_string(log_instance) {
            // Use writeln! to handle the newline and the buffer
            let _ = writeln!(guard, "{}", json);
        }
    }
}

fn bucket_luts(nodes: &[router::Node]) -> (Vec<usize>, Vec<usize>) {
    let mut lut_inputs = vec![];
    let mut lut_outputs = vec![];
    for (i, node) in nodes.iter().enumerate() {
        if node.id.starts_with('L') {
            if node.id.chars().nth(3) == Some('O') {
                lut_outputs.push(i);
            } else if node.id.chars().nth(3) == Some('I') {
                lut_inputs.push(i);
            }
        }
    }
    (lut_inputs, lut_outputs)
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::CreateTest(args) => create_test(&args.graph, &args.output, args.percentage, args.destinations),
        Commands::Route(args) => {
            let solver = match args.solver {
                SolverType::Simple => Solver::Simple(SimpleSolver),
                SolverType::Steiner => Solver::Steiner(SteinerSolver),
                SolverType::SimpleSteiner => Solver::SimpleSteiner(SimpleSteinerSolver),
            };
            if let Some(log_file) = &args.log_file {
                let logger = SimpleLogger::new(log_file);
                start_routing(
                    &args.graph,
                    &args.routing_list,
                    solver,
                    args.hist_factor,
                    &args.output,
                    &logger,
                )
            } else {
                start_routing(
                    &args.graph,
                    &args.routing_list,
                    solver,
                    args.hist_factor,
                    &args.output,
                    &SysOutLog,
                )
            };
        }
    }
}

fn start_routing(
    graph_path: &str,
    routing_list: &str,
    solver: Solver,
    hist_factor: f32,
    output_path: &str,
    logger: &dyn Logging,
) {
    let mut graph = FabricGraph::from_file(graph_path).unwrap();
    let mut route_plan = graph.route_plan_form_file(routing_list).unwrap();
    let config = Config::new(hist_factor, solver);

    let result = router::route(logger, config, &mut graph, &mut route_plan).unwrap();

    println!("{result:#?}");
    let ex = route_plan.iter().map(|x| x.expand(&graph).unwrap()).collect::<Vec<_>>();

    let pretty = serde_json::to_string_pretty(&ex).unwrap();
    fs::write(output_path, pretty).unwrap();
    
}

fn create_test(graph_path: &str, output_path: &str, percentage: f32, destinations: usize) {
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
        .zip(used_outs.chunks(destinations)) // Use args.destinations instead of hardcoded 4
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
