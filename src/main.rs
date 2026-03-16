mod cli;
use clap::Parser;
use router::{
    FabricGraph, FileLog, Loggers, NetInternal, SimpleSolver, SimpleSteinerSolver, SolveRouting, SteinerSolver, create_fasm,
    create_test, start_routing, validate_routing,
};

use crate::cli::{Cli, Commands, LoggerType, SolverType};

fn main() -> Result<(), u32> {
    match Cli::parse().command {
        Commands::CreateTest(args) => match create_test(&args.graph, &args.output, args.percentage, args.destinations) {
            Ok(()) => {
                println!("Created Test route plan in: {}", args.output);
                Ok(())
            }
            Err(err) => {
                eprintln!("Failed to create test File: {err}");
                Err(1)
            }
        },
        Commands::Fasm(args) => match create_fasm(&args.routing, &args.output) {
            Ok(()) => {
                println!("Created Fasm in: {}", args.output);
                Ok(())
            }
            Err(err) => {
                println!("Failed to create FASM File: {err}");
                Err(1)
            }
        },
        Commands::Route(args) => {
            let solver = match args.solver {
                SolverType::Simple => Solver::Simple(SimpleSolver),
                SolverType::Steiner => Solver::Steiner(SteinerSolver),
                SolverType::SimpleSteiner => Solver::SimpleSteiner(SimpleSteinerSolver),
            };
            let logger = match &args.logger {
                LoggerType::No => Loggers::No,
                LoggerType::Terminal => Loggers::Terminal,
                LoggerType::File => {
                    let file = args.log_file.unwrap();
                    let file_log = match FileLog::new(&file) {
                        Ok(f) => f,
                        Err(_) => return Err(1),
                    };
                    Loggers::File(file_log)
                }
            };
            
            match start_routing(
                &args.graph,
                &args.routing_list,
                &solver,
                args.hist_factor,
                &args.output,
                &logger,
                args.max_iterations,
                args.slack_report,
            ) {
                Ok(()) => {
                    println!("Finished routing.");
                    Ok(())
                }
                Err(err) => {
                    println!("Failed to route: {err}");
                    Err(1)
                }
            }
        }
        Commands::Validate(args) => match validate_routing(&args.routing, &args.graph) {
            Ok(()) => {
                println!("Routing is valid.");
                Ok(())
            }
            Err(err) => {
                println!("Routing is invalid due to: {err}");
                Err(1)
            }
        },
    }
}

enum Solver {
    Simple(SimpleSolver),
    SimpleSteiner(SimpleSteinerSolver),
    Steiner(SteinerSolver),
}

impl SolveRouting for Solver {
    fn solve(&self, graph: &FabricGraph, routing: &mut NetInternal) -> router::FabricResult<()> {
        match self {
            Solver::Simple(simple_solver) => simple_solver.solve(graph, routing),
            Solver::SimpleSteiner(simple_steiner_solver) => simple_steiner_solver.solve(graph, routing),
            Solver::Steiner(steiner_solver) => steiner_solver.solve(graph, routing),
        }
    }

    fn pre_process(&self, graph: &mut FabricGraph, route_plan: &mut [NetInternal]) -> router::FabricResult<()> {
        match self {
            Solver::Simple(simple_solver) => simple_solver.pre_process(graph, route_plan),
            Solver::SimpleSteiner(simple_steiner_solver) => simple_steiner_solver.pre_process(graph, route_plan),
            Solver::Steiner(steiner_solver) => steiner_solver.pre_process(graph, route_plan),
        }
    }

    fn identifier(&self) -> &'static str {
        match self {
            Solver::Simple(simple_solver) => simple_solver.identifier(),
            Solver::SimpleSteiner(simple_steiner_solver) => simple_steiner_solver.identifier(),
            Solver::Steiner(steiner_solver) => steiner_solver.identifier(),
        }
    }
}
