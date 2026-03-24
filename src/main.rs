#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]

mod cli;
use clap::Parser;
use router::{
    FileLog, Loggers, RoutingConfig, SimpleSolver, SimpleSteinerSolver, SteinerSolver, create_fasm, create_test, start_routing,
    validate_routing,
};

use crate::cli::{Cli, Commands, CreateTestArgs, FasmArgs, LoggerType, Solver, SolverType, ValidateArgs};

fn main() -> Result<(), u32> {
    match Cli::parse().command {
        Commands::CreateTest(args) => command_create_test(&args),
        Commands::Fasm(args) => command_fasm(&args),
        Commands::Route(args) => command_route(args),
        Commands::RouteSta(args) => command_route_sta(args),
        Commands::Validate(args) => command_validate(&args),
    }
}

fn command_create_test(args: &CreateTestArgs) -> Result<(), u32> {
    match create_test(&args.graph, &args.output, args.percentage, args.destinations) {
        Ok(()) => {
            println!("Created Test route plan in: {}", args.output);
            Ok(())
        }
        Err(err) => {
            eprintln!("Failed to create test File: {err}");
            Err(1)
        }
    }
}
fn command_fasm(args: &FasmArgs) -> Result<(), u32> {
    match create_fasm(&args.routing, &args.output) {
        Ok(()) => {
            println!("Created Fasm in: {}", args.output);
            Ok(())
        }
        Err(err) => {
            println!("Failed to create FASM File: {err}");
            Err(1)
        }
    }
}

fn command_route(args: cli::RouteArgs) -> Result<(), u32> {
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
            let Ok(file_log) = FileLog::new(&file) else { return Err(1) };
            Loggers::File(file_log)
        }
    };

    let config = RoutingConfig {
        graph_file: args.graph,
        net_list_file: args.net_list,
        output_file: args.output,
        hist_factor: args.hist_factor,
        max_iterations: args.max_iterations,
    };

    match start_routing(config, args.slack_report, &solver, &logger) {
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

fn command_route_sta(args: cli::RouteStaArgs) -> Result<(), u32> {
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
            let Ok(file_log) = FileLog::new(&file) else { return Err(1) };
            Loggers::File(file_log)
        }
    };
    let config = RoutingConfig {
        graph_file: args.graph,
        net_list_file: args.net_list,
        output_file: args.output,
        hist_factor: args.hist_factor,
        max_iterations: args.max_iterations,
    };

    match router::route_sta(config, args.max_sta_cycles, args.target_ps, &solver, &logger) {
        Ok(()) => {
            println!("Routing is valid and in timing bounds.");
            Ok(())
        }
        Err(err) => {
            println!("Routing is invalid due to: {err}");
            Err(1)
        }
    }
}

fn command_validate(args: &ValidateArgs) -> Result<(), u32> {
    match validate_routing(&args.routing, &args.graph) {
        Ok(()) => {
            println!("Routing is valid.");
            Ok(())
        }
        Err(err) => {
            println!("Routing is invalid due to: {err}");
            Err(1)
        }
    }
}
