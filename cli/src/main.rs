#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]

mod cli;
mod display_helper;
mod logger;
use std::{fs, path::Path, process::Command};

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use router::{
    FabricError, FabricGraph, FabricResult, NetListExternal, RoutingConfigBuilder, SimpleSolver, SimpleSteinerSolver, SlackReport, SteinerSolver, TileManager, TimingAnalysis, create_fasm, create_test, route, route_timing_driven, validate_routing
};

use crate::{
    cli::{Cli, Commands, CreateTestArgs, FasmArgs, LoggerType, Solver, SolverType, ValidateArgs},
    display_helper::{
        display_failed_routing, display_results, display_run_create_fasm, display_run_create_test, display_run_metadata_route,
        display_run_metadata_route_sta,
    },
    logger::Loggers,
};

fn main() -> Result<()> {
    match Cli::parse().command {
        Commands::CreateTest(args) => command_create_test(&args),
        Commands::Fasm(args) => command_fasm(&args),
        Commands::Route(args) => command_route(&args),
        Commands::RouteSta(args) => command_route_sta(&args),
        Commands::Validate(args) => command_validate(&args),
    }?;
    Ok(())
}

fn command_create_test(args: &CreateTestArgs) -> Result<()> {
    let graph =
        FabricGraph::from_file(&args.graph).with_context(|| format!("Failed to load fabric graph from {}", args.graph))?;
    let _ = clearscreen::clear();
    display_run_create_test(args);
    let net_list = create_test(&graph, args.percentage, args.destinations).with_context(|| "Failed to create test File")?;

    let pretty = serde_json::to_string_pretty(&net_list)
        .with_context(|| "Failed to serialize the net-list into a readable JSON format")?;
    fs::write(&args.output, pretty).with_context(|| format!("Failed to write the generated test net-list to {}", args.output))?;
    println!("Created Test net-list.");
    Ok(())
}
fn command_fasm(args: &FasmArgs) -> Result<()> {
    let _ = clearscreen::clear();
    display_run_create_fasm(args);
    let route_plan =
        NetListExternal::from_file(&args.net_list).with_context(|| format!("Failed to load net-list from {}", args.net_list))?;
    let fasm = create_fasm(&route_plan).with_context(|| "Failed to create FASM File")?;
    fs::write(&args.output, fasm).with_context(|| format!("Failed to save the generated FASM file to {}", args.output))?;
    println!("Created Fasm in: {}", args.output);
    Ok(())
}

fn command_route(args: &cli::RouteArgs) -> Result<()> {
    let solver = match args.solver {
        SolverType::Simple => Solver::Simple(SimpleSolver),
        SolverType::Steiner => Solver::Steiner(SteinerSolver),
        SolverType::SimpleSteiner => Solver::SimpleSteiner(SimpleSteinerSolver),
    };
    let logger = match &args.logger {
        LoggerType::No => Loggers::No,
        LoggerType::Terminal => Loggers::Terminal,
    };
    let graph = FabricGraph::from_file(&args.graph)
        .with_context(|| format!("Router initialization failed: unable to load graph {}", args.graph))?;
    let net_list = NetListExternal::from_file(&args.net_list)
        .with_context(|| format!("Router initialization failed: unable to load net-list {}", args.net_list))?;
    let tile_manager = TileManager::from_file(&args.bel);
    let mut config = RoutingConfigBuilder::default()
        .hist_factor(args.hist_factor)
        .max_iterations(args.max_iterations)
        .net_list(net_list)
        .solver(solver)
        .logger(logger)
        .graph(graph)
        .tile_manager(tile_manager)
        .build()?;

    let _ = clearscreen::clear();
    display_run_metadata_route(args, &config.solver);
    let result = match route(&mut config) {
        Ok(result) => result,

        Err(router::FabricError::RoutingMaxIterationsReached {
            congestion_report,
            iteration_report,
        }) => {
            display_failed_routing(&congestion_report, &iteration_report);
            return Err(anyhow!("Routing Failed: Maximum iterations reached."));
        }
        Err(err) => {
            return Err(err).with_context(|| "Routing engine encounterd critical error.");
        }
    };

    display_results(&result);
    let path = Path::new(&args.output);
    let serialized_net_list = match path.extension().and_then(|s| s.to_str()) {
        Some("fasm") => {
            create_fasm(&config.net_list).with_context(|| "Failed to generate FASM output from the routed net-list")?
        }
        Some("json") => {
            serde_json::to_string_pretty(&config.net_list).with_context(|| "Failed to serialize net-list for FASM generation")?
        }
        _ => {
            println!("Unknown file extension defaulting to fasm.");
            create_fasm(&config.net_list).with_context(|| "Failed to generate default FASM output")?
        }
    };
    fs::write(path, serialized_net_list).with_context(|| format!("Failed to write routing results to {}", args.output))?;
    Ok(())
}

fn command_route_sta(args: &cli::RouteStaArgs) -> Result<()> {
    let solver = match args.solver {
        SolverType::Simple => Solver::Simple(SimpleSolver),
        SolverType::Steiner => Solver::Steiner(SteinerSolver),
        SolverType::SimpleSteiner => Solver::SimpleSteiner(SimpleSteinerSolver),
    };
    let logger = match &args.logger {
        LoggerType::No => Loggers::No,
        LoggerType::Terminal => Loggers::Terminal,
    };
    let graph = FabricGraph::from_file(&args.graph)
        .with_context(|| format!("Router initialization failed: unable to load graph {}", args.graph))?;
    let net_list = NetListExternal::from_file(&args.net_list)
        .with_context(|| format!("Router initialization failed: unable to load net-list {}", args.net_list))?;

    let mut config = RoutingConfigBuilder::default()
        .hist_factor(args.hist_factor)
        .max_iterations(args.max_iterations)
        .net_list(net_list)
        .solver(solver)
        .logger(logger)
        .graph(graph)
        .build_timing_driven(Sta)?;

    let _ = clearscreen::clear();
    display_run_metadata_route_sta(args, &config.solver);
    let result = match route_timing_driven(&mut config) {
        Ok(result) => result,

        Err(router::FabricError::RoutingMaxIterationsReached {
            congestion_report,
            iteration_report,
        }) => {
            display_failed_routing(&congestion_report, &iteration_report);
            return Err(anyhow!("Routing Failed: Maximum iterations reached."));
        }
        Err(err) => {
            return Err(err).with_context(|| "Routing engine encounterd critical error.");
        }
    };

    display_results(&result);
    let path = Path::new(&args.output);
    let serialized_net_list = match path.extension().and_then(|s| s.to_str()) {
        Some("fasm") => {
            serde_json::to_string_pretty(&config.net_list).with_context(|| "Failed to serialize net-list for FASM generation")?
        }
        Some("json") => {
            create_fasm(&config.net_list).with_context(|| "Failed to generate FASM output from the routed net-list")?
        }
        _ => {
            println!("Unknown file extension defaulting to fasm.");
            create_fasm(&config.net_list).with_context(|| "Failed to generate default FASM output")?
        }
    };
    fs::write(path, serialized_net_list).with_context(|| format!("Failed to write routing results to {}", args.output))?;
    Ok(())
}

fn command_validate(args: &ValidateArgs) -> Result<()> {
    let graph = FabricGraph::from_file(&args.graph).with_context(|| format!("Failed to load graph from file: {}", args.graph))?;
    let route_plan = NetListExternal::from_file(&args.net_list)
        .with_context(|| format!("Validation aborted: could not load net-list {}", args.net_list))?;
    validate_routing(&graph, &route_plan).with_context(|| "Routing is invalid due to")?;

    println!("Routing is valid.");
    Ok(())
}

// This is just for the moment as there is no current implementation of the STA
fn run_mock_sta(fasm_in: &str, csv_out: &str, target: u32) -> FabricResult<String> {
    let output = Command::new("python3")
        .arg("mock_slack.py") // Name of your python script
        .arg(fasm_in)
        .arg(csv_out)
        .arg("--target")
        .arg(target.to_string())
        .output()
        .map_err(|_| FabricError::STAInternalError)?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

struct Sta;
impl TimingAnalysis for Sta {
    fn timing_analysis(&self, graph: &FabricGraph, net_list: &router::NetListInternal) -> FabricResult<SlackReport> {
        let ex = net_list.to_external(graph);
        let fasm = create_fasm(&ex)?;
        fs::write(".fasm", &fasm).map_err(|_| FabricError::STAInternalError)?;
        let _ = run_mock_sta(".fasm", ".slack", 10)?;
        let slack_report = SlackReport::from_file(".slack", graph)?;
        Ok(slack_report)
    }
}
