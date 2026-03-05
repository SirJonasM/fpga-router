//! # FPGA Path Finder
//!
//! This crate implements a Path Finder algorithm for FPGA routing**. 
//! It provides tools to model the FPGA fabric, nodes, and routing paths,
//! as well as algorithms for finding and validating optimal routes.  

#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]

pub(crate) mod dijkstra;
pub(crate) mod fabric_graph;
pub(crate) mod node;
pub(crate) mod path_finder;
pub(crate) mod fasm;
pub(crate) mod solver;
pub(crate) mod api;
pub(crate) mod logger;
pub(crate) mod validate;
pub(crate) mod error;

// Public API
pub use api::*;
pub use path_finder::Logging;
pub use path_finder::IterationResult;
pub use error::{FabricError, FabricResult};
pub use fabric_graph::{FabricGraph, Routing};

pub use solver::{SolveRouting, SimpleSolver, SimpleSteinerSolver, SteinerSolver};
pub use logger::{Loggers,FileLog};
