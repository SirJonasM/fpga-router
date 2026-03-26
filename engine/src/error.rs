use std::{io, path::PathBuf};
use thiserror::Error;

use crate::{IterationResult, graph::error::ParseError, netlist::error::MapExternalError, path_finder::CongestionReportExtern};

// A shorthand for results in your library
pub type FabricResult<T> = Result<T, FabricError>;

#[derive(Error, Debug)]
pub enum FabricError {
    #[error("The String does not represent a valid Node Id '{0}'.")]
    InvalidStringNodeId(String),
    #[error("Tried to unwrap the result field in Net but it was none.")]
    NetNotSolved,
    #[error("IO error while accessing '{path}'")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Cannot give each Node an own id because value space is too small.")]
    NodeIdValueSpaceTooSmall,

    #[error("Creating test failed because of bad parameters.")]
    CreatingTestBadParameters,

    #[error("Iteration Failed")]
    IterationError { source: Box<Self> },

    #[error("Routing has reached its maximum iterations.")]
    RoutingMaxIterationsReached {
        congestion_report: CongestionReportExtern,
        iteration_report: Vec<IterationResult>,
    },

    #[error("Error in line {line_number}.")]
    ParseError {
        line_number: usize,
        #[source]
        source: ParseError,
    },

    #[error("Runnning the STA script failed due to: {0}")]
    StaFailed(String),

    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Serialization error: {0}")]
    Csv(#[from] csv::Error),

    #[error("Failed to Map External Net to Internal representaion.")]
    MapExternalNet(#[from] MapExternalError),

    #[error("Edge does not exist in Graph: {start} -> {end}")]
    EdgeDoesNotExist { start: String, end: String },

    #[error("Failed to log: {0}")]
    LoggingError(String),

    #[error("Failed to preprocess route for signal {signal}: {source}")]
    RoutePreProcessing {
        signal: String,
        #[source]
        source: Box<Self>,
    },

    #[error("Path finding for Start: {start} and Sink: {sink} failed.")]
    PathfindingFailed { start: String, sink: String },

    #[error("Steiner tree conflict: Node {node_id} is already in use by another route.")]
    ResourceConflict { node_id: String },

    #[error("No valid Steiner tree could be constructed for the given sinks.")]
    NoSteinerTreeFound,

    #[error("Timing could not be met in given maximum sta cycles.")]
    TimingNotMet,

    #[error("Some Error: {0}")]
    Other(String),
}

// ADD this manual implementation
impl From<String> for FabricError {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}

// Highly recommended: adds support for ? on string literals ("error message".into())
impl From<&str> for FabricError {
    fn from(s: &str) -> Self {
        Self::Other(s.to_string())
    }
}
