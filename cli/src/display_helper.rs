use router::{CongestionReportExtern, IterationResult, RouteNet, Swap};

use crate::cli::{CreateTestArgs, RouteArgs};

pub fn display_results(results: &[IterationResult], swaps: &[Swap]) {
    if !swaps.is_empty() {
        println!("\nInput Swaps Performed:");
        println!("{:-<110}", "");
        println!(
            "{:<20} | {:<40} -> {:<40}",
            "Signal", "Old Sink (Input)", "New Sink (Input)"
        );
        println!("{:-<110}", "");
        
        for swap in swaps {
            println!(
                "{:<20} | {:<40} -> {:<40}",
                swap.signal.id, 
                swap.sink_old.id, 
                swap.sink_new.id
            );
        }
        println!("{:-<110}\n", "");
    }

    // 2. Print Iteration Results
    println!("{:-<110}", "");
    println!(
        "{:<4} | {:<10} | {:<10} | {:<10} | {:<10} | {:<10} | {:<10}",
        "Iter", "Conflicts", "Max Cost", "Avg Cost", "Wire Use", "Reuse %", "Time"
    );
    println!("{:-<110}", "");

    for res in results {
        println!(
            "{:<4} | {:<10} | {:<10.1} | {:<10.1} | {:<10} | {:<10.2} | {:?}",
            res.iteration,
            res.conflicts,
            res.longest_path_cost,
            res.average_path_cost,
            res.total_wire_use,
            res.wire_reuse * 100.0,
            res.duration
        );
    }
    println!("{:-<110}", "");

    if let Some(last) = results.last() {
        println!("Critical Path: {} -> {}", last.longest_path.0, last.longest_path.1);
    }
}

pub fn display_run_metadata_route<T: RouteNet>(config: &RouteArgs, solver: &T) {
    println!("{:=<60}", "");
    println!(" FPGA ROUTER CONFIGURATION");
    println!("{:-<60}", "");
    println!("{:<20}: {}", "Solver Engine", solver.identifier());
    println!("{:<20}: {}", "Graph File", config.graph);
    println!("{:<20}: {}", "Bel File", config.bel);
    println!("{:<20}: {}", "Netlist File", config.net_list);
    println!("{:<20}: {}", "Max Iterations", config.max_iterations);
    println!("{:<20}: {}", "History Factor", config.hist_factor);
    println!("{:<20}: {}", "Flip-Flop file", config.ffs);
    println!("{:<20}: {}", "Timings file", config.timings);
    println!("{:<20}: {}", "Timing Driven", if config.timing_driven {"Yes"} else {"No"});
    println!("{:=<60}\n", "");
}
pub fn display_run_create_test(config: &CreateTestArgs) {
    println!("{:=<60}", "");
    println!(" FPGA CREATE TEST CONFIGURATION");
    println!("{:-<60}", "");
    println!("{:<20}: {}", "Graph File", config.graph);
    println!("{:<20}: {}", "Output File", config.output);
    println!("{:<20}: {}", "LUT Percentage", config.percentage);
    println!("{:<20}: {}", "LUT Destinations", config.destinations);

    println!("{:=<60}\n", "");
}

pub fn display_failed_routing(congestion_report: &CongestionReportExtern, iteration_report: &[IterationResult]) {
    // 1. Clear Screen
    let _ = clearscreen::clear();

    // --- SECTION 1: CONVERGENCE HISTORY ---
    println!("{:=^80}", " ROUTING FAILURE REPORT ");
    println!("\n### Iteration History");
    println!("{:-<80}", "");
    println!(
        "{:<4} | {:<10} | {:<10} | {:<10} | {:<10} | {:<10}",
        "Iter", "Conflicts", "Max Cost", "Wire Use", "Reuse %", "Time"
    );
    println!("{:-<80}", "");

    for res in iteration_report {
        println!(
            "{:<4} | {:<10} | {:<10.1} | {:<10} | {:<10.2} | {:?}",
            res.iteration,
            res.conflicts,
            res.longest_path_cost,
            res.total_wire_use,
            res.wire_reuse * 100.0,
            res.duration
        );
    }

    // --- SECTION 2: CONGESTION HOTSPOTS (The "Overbooked" Wires) ---
    println!("\n### Top Congested Resources (Resource -> [Nets involved])");
    println!("{:-<80}", "");

    // Sort by number of nets sharing the same resource
    let mut congestion_list: Vec<_> = congestion_report.congestion.iter().collect();
    congestion_list.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    for (resource, nets) in congestion_list.iter().take(10) {
        println!("{:<25} | {} nets: {:?}", resource, nets.len(), nets);
    }

    // --- SECTION 3: PROBLEMATIC NETS (The "Stubborn" Nets) ---
    println!("\n### Most Critical / Problematic Nets (Net -> Congestion Score)");
    println!("{:-<80}", "");

    let mut net_scores: Vec<_> = congestion_report.net_congestion.iter().collect();
    net_scores.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));

    for (net_name, score) in net_scores.iter().take(10) {
        // High score here usually means this net refused to move or was forced into high-congestion
        println!("{net_name:<25} | Score: {score:.4}");
    }

    println!("\n{:=^80}", " END OF REPORT ");
    println!("Hint: If conflicts are high, try increasing 'hist_factor' or 'max_iterations'.");
}
