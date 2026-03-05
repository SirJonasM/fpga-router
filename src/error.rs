use thiserror::Error;
use std::io;

use crate::node::NodeId;

// A shorthand for results in your library
pub type FabricResult<T> = Result<T, FabricError>;

#[derive(Error, Debug)]
pub enum FabricError {
    #[error("IO error while accessing '{path}': {source}")]
    Io {
        path: String,
        source: io::Error,
    },
    #[error("Cannot give each Node an own id because value space is too small.")]
    NodeIdValueSpaceTooSmall,

    #[error("Creating test failed because of bad parameters.")]
    CreatingTestBadParameters,

    #[error("Iteration Failed")]
    IterationError{source: Box<FabricError>},

    #[error("Routing has reached its maximum iterations.")]
    RoutingMaxIterationsReached,

    // This variant wraps the ParseError with line-specific context
    #[error("Parsing failed on line {line_number}: {source}\n  Line: \"{content}\"")]
    LineError {
        line_number: usize,
        content: String,
        source: ParseError, 
    },

    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Mapping expanded Node {signal} of Routeplan to a internal graph node id failed.\n Reason: {reason}")]
    MappingExternelNet {
        signal: String,
        reason: String
    },

    #[error("Edge does not exist in Graph: {start} -> {end}")]
    EdgeDoesNotExist {start: NodeId, end: NodeId},

    #[error("Parsing Failed: {0}")]
    Parse(#[from] ParseError),

    #[error("Failed to log: {0}")]
    LoggingError(String),

    #[error("Failed to preprocess route for signal {signal}: {source}")]
    RoutePreProcessing{signal: NodeId, #[source] source: Box<FabricError>},

    #[error("Path finding for Start: {start} and Sink: {sink} failed.")]
    PathfindingFailed {start: NodeId, sink: NodeId},

    #[error("Steiner tree conflict: Node {node_id} is already in use by another route.")]
    ResourceConflict { node_id: NodeId },

    #[error("No valid Steiner tree could be constructed for the given sinks.")]
    NoSteinerTreeFound,

    #[error("Some Error: {0}")]
    Other(String)
}

#[derive(Error, Debug, PartialEq)]
pub enum ParseError {
    #[error("Wrong Pips line format. Expecting 6 parts.")]
    InvalidLineFormat,

    #[error("Failed to parse {part}")]
    InvalidCoordinates {
        token: String,
        part: String,
        #[source] source: Box<ParseError>,
    },

    #[error("Failed to parse start node id: {id} cords: {cords}")]
    InvalidStartNode {id: String, cords: String, source: Box<ParseError>},

    #[error("Failed to parse end node id: {id} cords: {cords}")]
    InvalidEndNode {id: String, cords: String, source: Box<ParseError>},

    #[error("Missing coordinate prefix '{prefix}' in token: {token}")]
    MissingPrefix { prefix: char, token: String },

    #[error("Failed to parse '{component}' coordinate: {token}")]
    InvalidCoordinate { 
        component: &'static str, 
        token: String, 
        #[source] source: std::num::ParseIntError 
    },
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
