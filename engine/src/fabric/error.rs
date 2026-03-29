use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ParseError {
    #[error("Parsing failed content: '{content}'")]
    LineError {
        content: String,
        #[source] source: Box<Self>,
    },

    #[error("Wrong Pips line format. Expecting 6 parts.")]
    InvalidLineFormat,

    #[error("Failed to parse {part}")]
    InvalidCoordinates {
        token: String,
        part: String,
        #[source]
        source: Box<Self>,
    },

    #[error("Failed to parse start node id: {id} cords: {cords}")]
    InvalidStartNode { id: String, cords: String, source: Box<Self> },

    #[error("Failed to parse end node id: {id} cords: {cords}")]
    InvalidEndNode { id: String, cords: String, source: Box<Self> },

    #[error("Missing coordinate prefix '{prefix}' in token: {token}")]
    MissingPrefix { prefix: char, token: String },

    #[error("Failed to parse '{component}' coordinate: {token}")]
    InvalidCoordinate {
        component: &'static str,
        token: String,
        #[source]
        source: std::num::ParseIntError,
    },
}
