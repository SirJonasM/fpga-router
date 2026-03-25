#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]

//! # FPGA Path Finder
//!
//! This crate implements a Path Finder algorithm for FPGA routing**.
//! It provides tools to model the FPGA fabric, nodes, and routing paths,
//! as well as algorithms for finding and validating optimal routes.  

pub(crate) mod api;
pub(crate) mod dijkstra;
pub(crate) mod error;
pub(crate) mod fasm;
pub(crate) mod graph;
pub(crate) mod netlist;
pub(crate) mod path_finder;
pub(crate) mod slack;
pub(crate) mod solver;
pub(crate) mod validate;

// Error handling
pub use error::{FabricError, FabricResult};

// Public API
pub use api::*;
pub use graph::fabric_graph::FabricGraph;
pub use netlist::{NetExternal, NetListExternal, NetInternal, NetResultExternal};
pub use path_finder::{IterationResult, CongestionReportExtern};
pub use slack::SlackReport;
pub use fasm::routing_to_fasm;

use serde::Serialize;
pub use solver::{RouteNet, SimpleSolver, SimpleSteinerSolver, SteinerSolver};


/// Trait for logging pathfinding iterations.
pub trait Logging {
    /// Logs the current iteration result.
    ///
    /// # Errors
    /// Should return an `LoggingError`
    fn log(&self, log_instance: &LogInstance) -> FabricResult<()>;
}

#[derive(Debug, Clone, Serialize)]
pub enum LogInstance<'a> {
    Text(String),
    RouterIteration(&'a IterationResult),
}

impl From<&str> for LogInstance<'_> {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl From<String> for LogInstance<'_> {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

pub struct SimpleLogging;
impl Logging for SimpleLogging {
    fn log(&self, log_instance: &LogInstance) -> FabricResult<()> {
        println!("{log_instance:?}");
        Ok(())
    }
}
