//! # FPGA Path Finder
//!
//! This crate implements a Path Finder algorithm for FPGA routing**. 
//! It provides tools to model the FPGA fabric, nodes, and routing paths,
//! as well as algorithms for finding and validating optimal routes.  
//!
//! ## Features
//! - FPGA fabric modeling (`FabricGraph`, `Node`)
//! - Path finding algorithms (`path_finder`, `path_finding_algo`)
//! - Solver implementations (`SimpleSolver`, `SteinerSolver`)
//! - Validation of routing results
//! - Optional JSON export for routing data (`graph_to_json` feature)


// #![deny(clippy::nursery)]
// #![deny(clippy::pedantic)]

pub(crate)mod dijkstra;
pub(crate)mod fabric_graph;
pub(crate)mod node;
pub(crate)mod path_finder;
pub(crate)mod fasm;
pub(crate)mod solver;
pub(crate)mod api;
pub(crate)mod logger;
pub (crate) mod validate;



// Public API
pub use api::*;
pub use path_finder::Logging;
pub use path_finder::IterationResult;

pub use solver::{Solver, SimpleSolver, SimpleSteinerSolver, SteinerSolver};
pub use logger::{Loggers,FileLog};
