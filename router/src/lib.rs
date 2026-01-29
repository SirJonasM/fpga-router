//! # FPGA PathFinder
//!
//! This crate implements a **PathFinder algorithm for FPGA routing**. 
//! It provides tools to model the FPGA fabric, nodes, and routing paths,
//! as well as algorithms for finding and validating optimal routes.  
//!
//! ## Features
//! - FPGA fabric modeling (`FabricGraph`, `Node`)
//! - Path finding algorithms (`path_finder`, `path_finding_algo`)
//! - Solver implementations (`SimpleSolver`, `SteinerSolver`)
//! - Validation of routing results
//! - Optional JSON export for routing data (`graph_to_json` feature)

// mod typst_table;
mod dijkstra;
mod fabric_graph;
mod node;
mod path_finder;
mod path_finding_algo;
mod fasm;
mod solver;



// Public API

/// The FPGA fabric graph, representing nodes and connections.
pub use fabric_graph::{FabricGraph, Routing, RoutingExpanded};

/// Represents a node in the FPGA fabric.
pub use node::Node;

/// Path finding utilities and structures.
pub use path_finder::{IterationResult, Logging, Config, route, validate_routing};

/// Solver implementations for routing optimization.
pub use solver::{SimpleSolver, SimpleSteinerSolver, SolveRouting, Solver, SteinerSolver};

pub use fasm::routing_to_fasm;

