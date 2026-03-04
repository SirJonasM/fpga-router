use thiserror::Error;
use std::io;
// A shorthand for results in your library
pub type FabricResult<T> = Result<T, FabricError>;

#[derive(Error, Debug)]
pub enum FabricError {
    #[error("IO error while accessing '{path}': {source}")]
    Io {
        path: String,
        source: io::Error,
    },


    #[error("Routing failure: {0}")]
    RoutingFailed(String),

    // This variant wraps the ParseError with line-specific context
    #[error("Parsing failed on line {line_number}: {source}\n  Line: \"{content}\"")]
    LineError {
        line_number: usize,
        content: String,
        source: ParseError, 
    },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Mapping expanded Node {signal} of Routeplan to a internal graph node id failed.\n Reason: {reason}")]
    MappingExpandedRoutePlan {
        signal: String,
        reason: String
    },

    #[error("Parsing Failed: {0}")]
    Parse(#[from] ParseError),
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid line format: expected 6 parts, found {parts_found}")]
    InvalidLine { parts_found: usize, line: String },

    #[error("Missing coordinate prefix '{prefix}' in token: {token}")]
    MissingPrefix { prefix: char, token: String },

    #[error("Failed to parse '{component}' coordinate: {token}")]
    InvalidCoordinate { 
        component: &'static str, 
        token: String, 
        #[source] source: std::num::ParseIntError 
    },
}

