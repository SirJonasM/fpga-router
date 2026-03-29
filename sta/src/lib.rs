use serde::{Deserialize, Serialize};

pub mod analysis;
pub mod design;
pub mod parsers;
pub mod report;

pub use analysis::perform_timing_analysis;
pub use design::{build_design, design_stats, design_to_json_map, make_fabric, node_key};
pub use parsers::{
    fasm_parser_string, parse_all_timing_constraints, parse_all_timing_models, parse_timing_constraints,
    parse_timing_model, pips_parser,
};
pub use report::report_violations;

pub struct Edge {
    pub delay: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingConstraints {
    pub setup_time: f64,
    pub hold_time: f64,
    pub clk_period: f64,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Flop {
    pub tile: (u32, u32),
    pub lut: u32,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct Configuration {
    pub tile: (u32, u32),
    pub src_pin: String,
    pub dst_pin: String,
}

#[derive(Hash, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Node {
    pub tile: (u32, u32),
    pub pin: String,
}

impl Node {
    pub fn key(&self) -> String {
        format!("X{}Y{}.{}", self.tile.0, self.tile.1, self.pin)
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.key())
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("tile", &self.tile)
            .field("pin", &self.pin)
            .finish()
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct Pip {
    pub src: Node,
    pub dst: Node,
    pub delay: f64,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub delay: f64,
}

#[derive(PartialEq, Clone, Serialize, Deserialize)]
pub struct TimingNode {
    pub tile: (u32, u32),
    pub pin: String,
    pub max_delay: f64,
    pub min_delay: f64,
}

impl TimingNode {
    pub fn from_node(node: &Node, max_delay: f64, min_delay: f64) -> Self {
        TimingNode {
            tile: node.tile,
            pin: node.pin.clone(),
            max_delay,
            min_delay,
        }
    }
}


impl std::fmt::Display for TimingNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "X{}Y{}.{} (max: {}, min: {})",
            self.tile.0, self.tile.1, self.pin, self.max_delay, self.min_delay
        )
    }
}

impl std::fmt::Debug for TimingNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimingNode")
            .field("tile", &self.tile)
            .field("pin", &self.pin)
            .field("max_delay", &self.max_delay)
            .field("min_delay", &self.min_delay)
            .finish()
    }
}

use std::error::Error;

use crate::analysis::TimingAnalysisResult;

#[derive(Deserialize, Debug, Default)]
pub struct TimingModel {
    pub lut_delay: f64,
    pub pip_delay: f64,
    pub fanout_delay: f64,
    pub clock_to_output_delay: f64,
    pub clock_tree_delay: f64,
}

/// Analyzes a design and returns a CSV string containing the slack report.
pub fn generate_slack_report(
    fasm: &str,
    pips: &[Pip],
    timing_model: &TimingModel,
) -> Result<TimingAnalysisResult, Box<dyn Error>> {
    // 1. Parse Input Files
    // We assume these parsers exist in your crate based on the previous code
    let (configurations, flops) = fasm_parser_string(fasm).unwrap();

    // 2. Build the Design Graph
    // This maps the physical FASM/PIPs into a directed graph with delays
    let design = build_design(pips, &configurations, &flops, &timing_model);

    // 3. Perform Timing Analysis
    // Using the broadened perform_timing_analysis we discussed
    Ok(perform_timing_analysis(&design, &flops))
}

