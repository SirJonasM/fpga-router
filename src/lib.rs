//! # FPGA Path Finder
//!
//! This crate implements a Path Finder algorithm for FPGA routing**.
//! It provides tools to model the FPGA fabric, nodes, and routing paths,
//! as well as algorithms for finding and validating optimal routes.  

#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]

pub(crate) mod api;
pub(crate) mod dijkstra;
pub(crate) mod error;
pub(crate) mod fabric_graph;
pub(crate) mod fasm;
pub(crate) mod logger;
pub(crate) mod netlist;
pub(crate) mod node;
pub(crate) mod path_finder;
pub(crate) mod solver;
pub(crate) mod validate;
pub(crate) mod slack;

// Error handling
pub use error::{FabricError, FabricResult};

// Public API
pub use api::*;
pub use fabric_graph::FabricGraph;
pub use netlist::{NetExternal, NetInternal, NetListExternal, NetListInternal, NetResultExternal};
pub use path_finder::IterationResult;

pub use logger::{FileLog, Loggers, Logging};
pub use solver::{SimpleSolver, SimpleSteinerSolver, SolveRouting, SteinerSolver};
