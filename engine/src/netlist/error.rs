use thiserror::Error;

use crate::fabric::node::Node;

pub type MapExternalResult<T> = Result<T, MapExternalError>;

#[derive(Error, Debug)]
pub enum MapExternalError {
    #[error("Mapping expanded Net with signal '{signal}' to internal Net failed.")]
    Net {signal: Node, #[source]source: Box<Self>},
    #[error("Mapping the signal failed.")]
    Signal,
    #[error("Mapping the signal failed.")]
    Sink(#[source] Box<Self>),
    #[error("Mapping expanded Net Result Internal Net Result failed.")]
    NetResult(#[source] Box<Self>),
    #[error("Mapping expanded Net Result Internal structure failed.")]
    NetResultNodes(#[source] Box<Self>),
    #[error("Mapping expanded Net Result Paths to Internal structure failed.")]
    NetResultPaths(#[source] Box<Self>),
    #[error("Mapping expanded Net Id: {0} to Internal id failed.")]
    Id(String),
}
