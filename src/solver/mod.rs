mod simple;
mod simple_steiner;
mod steiner;

pub use simple::SimpleSolver;
pub use simple_steiner::SimpleSteinerSolver;
pub use steiner::SteinerSolver;

use crate::{FabricResult, fabric_graph::FabricGraph, netlist::NetInternal};

/// A trait for implementing custom routing algorithms within the fabric.
///
/// Implementors of this trait can define how individual signals are routed
/// and how the global routing plan is prepared before execution.
pub trait RouteNet {
    /// Executes the routing algorithm for a single net.
    ///
    /// This method is responsible for finding a path in the [`FabricGraph`]
    /// and updating the [`Routing`] structure with the results.
    ///
    /// # Errors
    ///
    /// Returns [`FabricError::PathfindingFailed`] if a valid route cannot be found
    /// given the current graph constraints.
    fn solve(&self, graph: &FabricGraph, routing: &mut NetInternal) -> FabricResult<()>;

    /// Prepares the graph or the route plan before the main solving phase.
    ///
    /// This is typically used for global optimizations, such as pre-calculating
    /// Steiner points or identifying high-congestion areas.
    ///
    /// # Errors
    ///
    /// Returns [`FabricError::RoutePreProcessing`] if the pre-processing logic
    /// detects overlapping requirements that cannot be resolved.
    fn pre_process(&self, graph: &mut FabricGraph, route_plan: &mut [NetInternal]) -> FabricResult<()>;

    /// Returns a unique string constant identifying the solver implementation.
    ///
    /// This is used for logging, telemetry, and CLI selection (e.g., "steiner", "simple").
    fn identifier(&self) -> &'static str;
}
