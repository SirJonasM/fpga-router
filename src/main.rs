
mod cli;
use clap::Parser;
use router::{
    FileLog, Loggers, SimpleSolver, SimpleSteinerSolver, Solver, SteinerSolver, create_fasm, create_test, start_routing,
    validate_routing,
};

use crate::cli::{Cli, Commands, LoggerType, SolverType};

fn main() -> Result<(), u32> {
    let cli = Cli::parse();

    match cli.command {
        Commands::CreateTest(args) => match create_test(&args.graph, &args.output, args.percentage, args.destinations) {
            Ok(()) => {
                println!("Created Fasm in: {}", args.output);
                Ok(())
            }
            Err(err) => {
                println!("Failed to test  File: {err}");
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
                    Loggers::File(FileLog::new(&file))
                }
            };

            match start_routing(
                &args.graph,
                &args.routing_list,
                solver,
                args.hist_factor,
                &args.output,
                &logger,
                args.max_iterations,
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
