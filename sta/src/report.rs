use crate::{Node, TimingConstraints, TimingNode};

pub fn report_violations(
    min_path: &[(f64, f64, Node, Node, Vec<TimingNode>)],
    max_path: &[(f64, f64, Node, Node, Vec<TimingNode>)],
    constraints: &TimingConstraints
) {
    log::info!("--- Timing Constraint Violation Report ---");
    log::info!(
        "Constraints: Clock Period={} ps Setup={} ps, Hold={} ps",
        constraints.clk_period,
        constraints.setup_time,
        constraints.hold_time
    );

    let mut min_delay_path: (f64, f64, Node, Node, Vec<TimingNode>) = (f64::INFINITY, 0.0, Node { tile: (0, 0), pin: String::new() }, Node { tile: (0, 0), pin: String::new() }, Vec::new());
    let mut max_delay_path: (f64, f64, Node, Node, Vec<TimingNode>) = (f64::INFINITY, 0.0, Node { tile: (0, 0), pin: String::new() }, Node { tile: (0, 0), pin: String::new() }, Vec::new());
    log::info!("Amount of paths analyzed: {} min paths, {} max paths", min_path.len(), max_path.len());
    log::trace!("--- Min Paths ---");

    for path in min_path.iter() {
        let (min_d, _max_d, src, sink, path_trace) = path;
        let hold_slack = min_d - constraints.hold_time;

        // Track overall critical path
        if min_delay_path.0 == 0.0 || *min_d < min_delay_path.0 {
            min_delay_path = path.clone();
        }


        log::trace!(
            "MIN PATH: {} -> {}",
            src,
            sink
        );
        log::trace!(
            "   Hold Slack: {:.2} ps | Min Delay: {:.2} ps | Required: > {:.2} ps",
            hold_slack,
            min_d,
            constraints.hold_time
        );
        log::trace!(
            "   Path: {}",
            path_trace.iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(" -> ")
        );
        log::trace!("");
    }

    log::trace!("--- Max Paths ---");

    for path in max_path.iter() {
        let (_min_d, max_d, src, sink, path_trace) = path;
        let setup_slack = constraints.clk_period - constraints.setup_time - max_d;

        
        // Track overall critical path
        if *max_d > max_delay_path.1 {
            max_delay_path = path.clone();
        }
        log::trace!(
            "MAX PATH: {} -> {}",
            src,
            sink
         );
        log::trace!(
            "   Setup Slack: {:.2} ps | Max Delay: {:.2} ps | Required: < {:.2} ps",
            setup_slack,
            max_d,
            constraints.clk_period - constraints.setup_time
        );
        log::trace!(
            "   Worst Path: {}",
            path_trace.iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(" -> ")
        );
        log::trace!("");
    }

    log::info!("--- Critical Path Summary ---");

    let (min_d, _max_d, start, end, path) = min_delay_path;
    log::info!("---------------------------------------------------------");
    log::info!("MINIMUM PATH REPORT");
    log::info!("---------------------------------------------------------");
    log::info!("Min Path Delay (incl. clock skew): {:.2} ps", min_d);
    let hold_slack = min_d - constraints.hold_time;
    
    if hold_slack < 0.0 {
        log::warn!(
            "WARNING: Hold slack violation detected! Hold Slack: {:.2} ps | Required: > {:.2} ps | Found: {:.2} ps",
            hold_slack,
            constraints.hold_time,
            min_d
        );
    } else {
        log::info!(
            "Hold Slack: {:.2} ps",
            hold_slack
        );
    }
    
    log::info!("Path from {} to {}", start, end);
    log::debug!("Path Elements:");
    for p in &path {
        log::debug!("{}", p);
    }
    log::info!("---------------------------------------------------------");

    let (_min_d, max_d, start, end, path) = max_delay_path;
    
    log::info!("---------------------------------------------------------");
    log::info!("MAXIMUM PATH REPORT");
    log::info!("---------------------------------------------------------");
    log::info!("Max Path Delay (incl. clock skew): {:.2} ps", max_d);
    let setup_slack = constraints.clk_period - constraints.setup_time - max_d;

    
    if setup_slack < 0.0 {
        log::warn!(
            "WARNING: Setup slack violation detected! Setup Slack: {:.2} ps | Required: < {:.2} ps | Found: {:.2} ps",
             setup_slack,
            constraints.clk_period - constraints.setup_time,
            max_d
        );
    } else {
        log::info!(
            "Setup Slack: {:.2} ps",
            setup_slack
        );  
    }

    log::info!(
        "Minimum Clock Period: {:.2} ps (incl. setup time)",
        max_d + constraints.setup_time
    );

    log::info!(
        "Maximum Clock Frequency: {:.2} MHz",
        1_000_000.0 / (max_d + constraints.setup_time)
    );

    log::info!("Path from {} to {}", start, end);
    log::debug!("Path Elements:");
    for p in &path {
        log::debug!("{}", p);
    }
    log::info!("Path length: {} hops", path.len());
    log::info!("---------------------------------------------------------");
}