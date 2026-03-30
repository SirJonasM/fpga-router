#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]

mod cli;
mod display_helper;
mod logger;
use fpga_timing_analyzer::{Pip, TimingConstraints, TimingModel, analysis::TimingAnalysisResult, generate_slack_report};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::BufReader,
    path::Path,
};

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use router::{
    Fabric, FabricError, FabricGraph, FabricResult, NetListExternal, RoutingConfig, RoutingConfigBuilder, SimpleSolver,
    SimpleSteinerSolver, SlackReport, SteinerSolver, TileManager, TimingAnalysis, create_fasm, create_test, route,
    route_timing_driven, validate_routing,
};

use crate::{
    cli::{Cli, Commands, CreateTestArgs, LoggerType, Solver, SolverType, ValidateArgs},
    display_helper::{display_failed_routing, display_results, display_run_create_test, display_metadata_route},
    logger::Loggers,
};

fn main() -> Result<()> {
    match Cli::parse().command {
        Commands::CreateTest(args) => command_create_test(&args),
        Commands::Route(args) => command_route(&args),
        Commands::Validate(args) => command_validate(&args),
    }?;
    Ok(())
}

fn command_create_test(args: &CreateTestArgs) -> Result<()> {
    let graph =
        FabricGraph::from_file(&args.graph, None).with_context(|| format!("Failed to load fabric graph from {}", args.graph))?;
    let _ = clearscreen::clear();
    display_run_create_test(args);
    let net_list = create_test(&graph, args.percentage, args.destinations).with_context(|| "Failed to create test File")?;

    let pretty = serde_json::to_string_pretty(&net_list)
        .with_context(|| "Failed to serialize the net-list into a readable JSON format")?;
    fs::write(&args.output, pretty).with_context(|| format!("Failed to write the generated test net-list to {}", args.output))?;
    println!("Created Test net-list.");
    Ok(())
}

fn command_route(args: &cli::RouteArgs) -> Result<()> {
    let (mut config, sta) = parse_arguments(args)?;

    let _ = clearscreen::clear();
    display_metadata_route(args, &config.solver);
    let routing_result = if args.timing_driven {
        route_timing_driven(&mut config, &sta)
    } else {
        route(&mut config)
    };
    let result = match routing_result {
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

    let swapped_inputs = result.0.swapped_inputs(&config.net_list);
    display_results(&result.1, &swapped_inputs);
    let path = Path::new(&args.output);
    let serialized_net_list = match path.extension().and_then(|s| s.to_str()) {
        Some("fasm") => {
            let ffs = args.ffs.as_ref().map_or_else(
                || Ok("# No FFS provided".to_string()),
                |path| fs::read_to_string(path).context("Error reading FFS file"),
            )?;
            let fasm = create_fasm(&result.0, &config.fabric.tile_manager)
                .with_context(|| "Failed to generate FASM output from the routed net-list")?;
            format!("{fasm}\n{ffs}")
        }
        Some("json") => {
            serde_json::to_string_pretty(&config.net_list).with_context(|| "Failed to serialize net-list for FASM generation")?
        }
        _ => {
            println!("Unknown file extension defaulting to fasm.");
            let ffs = args.ffs.as_ref().map_or_else(
                || Ok("# No FFS provided".to_string()),
                |path| fs::read_to_string(path).context("Error reading FFS file"),
            )?;
            let fasm = create_fasm(&config.net_list, &config.fabric.tile_manager)
                .with_context(|| "Failed to generate FASM output from the routed net-list")?;
            format!("{fasm}\n{ffs}")
        }
    };
    fs::write(path, serialized_net_list).with_context(|| format!("Failed to write routing results to {}", args.output))?;
    Ok(())
}

fn command_validate(args: &ValidateArgs) -> Result<()> {
    let graph =
        FabricGraph::from_file(&args.graph, None).with_context(|| format!("Failed to load graph from file: {}", args.graph))?;
    let route_plan = NetListExternal::from_file(&args.net_list)
        .with_context(|| format!("Validation aborted: could not load net-list {}", args.net_list))?;
    validate_routing(&graph, &route_plan).with_context(|| "Routing is invalid due to")?;

    println!("Routing is valid.");
    Ok(())
}

#[derive(Deserialize, Debug)]
struct Sta {
    timing_model: TimingModel,
    timing_constraints: TimingConstraints,
    #[serde(skip)]
    pub graph: Option<Vec<Pip>>,
}

impl TimingAnalysis for Sta {
    fn timing_analysis(&self, fabric: &Fabric, net_list: &router::NetListInternal) -> FabricResult<SlackReport> {
        let ex = net_list.to_external(&fabric.graph);
        let mut fasm = create_fasm(&ex, &fabric.tile_manager)?;
        let ffs = fs::read_to_string("ffs.fasm").unwrap();
        fasm.push('\n');
        fasm.push_str(&ffs);
        let graph = self
            .graph
            .as_ref()
            .ok_or_else(|| FabricError::Other("Graph was none".into()))?;
        let timing_analyisis_report = generate_slack_report(&fasm, graph, &self.timing_model).unwrap();
        let slack_report =
            slack_report_from_timing_analyis(&timing_analyisis_report, &fabric.graph, self.timing_constraints.clk_period)
                .map_err(|e| FabricError::Csv(Box::new(e)))?;
        Ok(slack_report)
    }
}

fn slack_report_from_timing_analyis(
    timing_analyisis: &TimingAnalysisResult,
    graph: &FabricGraph,
    target_period_ps: f64,
) -> FabricResult<SlackReport> {
    let mut slacks = HashMap::new();
    let mut criticalities = HashMap::new();
    let mut worst_node_str = (String::new(), String::new());

    let mut min_slack_val = f64::INFINITY;
    let max_arrival = timing_analyisis.max_paths.iter().map(|p| p.1).fold(0.0, f64::max);

    for (_, arrival, src, sink, path) in &timing_analyisis.max_paths {
        let slack = target_period_ps - arrival;
        let criticality = (arrival / max_arrival).clamp(0.0, 1.0);

        // MAP THE SINK:
        // If the sink is a 'FF_SINK', the router actually needs to know
        // the wire-end (the LUT input) that fed it.
        let router_sink = if sink.pin.contains("FF_SINK") {
            // Get the node immediately BEFORE the FF_SINK in the timing path
            path.iter().rev().nth(2).map_or_else(
                || format!("X{}Y{}.{}", sink.tile.0, sink.tile.1, sink.pin),
                |tn| format!("X{}Y{}.{}", tn.tile.0, tn.tile.1, tn.pin),
            )
        } else {
            format!("X{}Y{}.{}", sink.tile.0, sink.tile.1, sink.pin)
        };

        let source_str = format!("X{}Y{}.{}", src.tile.0, src.tile.1, src.pin);
        let source_id = *graph
            .get_node_id(&source_str)
            .ok_or_else(|| FabricError::InvalidStringNodeId(source_str.clone()))?;

        let sink_id = *graph
            .get_node_id(&router_sink)
            .ok_or_else(|| FabricError::InvalidStringNodeId(router_sink.clone()))?;

        if min_slack_val > slack {
            min_slack_val = slack;
            worst_node_str = (source_str, router_sink);
        }

        #[allow(clippy::cast_possible_truncation)]
        {
            criticalities.insert((source_id, sink_id), criticality as f32);
            slacks.insert((source_id, sink_id), slack as f32);
        }
    }
    #[allow(clippy::cast_possible_truncation)]
    let worst_slack = (
        (
            *graph
                .get_node_id(&worst_node_str.0)
                .ok_or(FabricError::InvalidStringNodeId(worst_node_str.0))?,
            *graph
                .get_node_id(&worst_node_str.1)
                .ok_or(FabricError::InvalidStringNodeId(worst_node_str.1))?,
        ),
        min_slack_val as f32,
    );

    Ok(SlackReport {
        slacks,
        criticalities,
        worst_slack,
    })
}
fn parse_arguments(args: &cli::RouteArgs) -> Result<(RoutingConfig<Solver, Loggers>, Sta)> {
    let solver = match args.solver {
        SolverType::Simple => Solver::Simple(SimpleSolver),
        SolverType::Steiner => Solver::Steiner(SteinerSolver),
        SolverType::SimpleSteiner => Solver::SimpleSteiner(SimpleSteinerSolver),
    };
    let logger = match &args.logger {
        LoggerType::No => Loggers::No,
        LoggerType::Terminal => Loggers::Terminal,
    };
    let file = File::open(&args.timings)?;
    let reader = BufReader::new(file);
    let mut sta: Sta = serde_json::from_reader(reader)?;
    let timing_model = &sta.timing_model;
    sta.graph = Some(fpga_timing_analyzer::pips_parser(&args.graph));

    let graph_timing_model = router::TimingModel {
        lut_delay: timing_model.lut_delay,
        pip_delay: timing_model.pip_delay,
        fanout_delay: timing_model.fanout_delay,
        clock_to_output_delay: timing_model.clock_to_output_delay,
        clock_tree_delay: timing_model.clock_tree_delay,
    };

    let graph = FabricGraph::from_file(&args.graph, Some(graph_timing_model))
        .with_context(|| format!("Router initialization failed: unable to load graph {}", args.graph))?;
    let net_list = NetListExternal::from_file(&args.net_list)
        .with_context(|| format!("Router initialization failed: unable to load net-list {}", args.net_list))?;

    let tile_manager = TileManager::from_file(&args.bel)?;
    let config = RoutingConfigBuilder::default()
        .hist_factor(args.hist_factor)
        .max_iterations(args.max_iterations)
        .net_list(net_list)
        .solver(solver)
        .logger(logger)
        .graph(graph)
        .tile_manager(tile_manager)
        .build()
        .context("Failed to build Routing Config.")?;
    Ok((config, sta))
}
