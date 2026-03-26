use rand::seq::SliceRandom;

use crate::{IterationResult, SimpleLogging, SimpleSolver};
use crate::{
    FabricError, FabricResult, Logging,
    fasm::net_to_fasm,
    graph::fabric_graph::{FabricGraph, bucket_luts},
    graph::node::NodeId,
    netlist::{NetExternal, NetInternal, NetListExternal, NetListInternal},
    path_finder::{Config, path_finder},
    slack::SlackReport,
    solver::RouteNet,
    validate,
};

pub struct RoutingConfig<T: RouteNet, L: Logging> {
    pub graph: FabricGraph,
    pub net_list: NetListExternal,
    pub hist_factor: f32,
    pub max_iterations: usize,
    pub slack_report: Option<SlackReport>,
    pub solver: T,
    pub logger: L,
}

/// Tries to solve a `NetList`
/// # Example
/// ```
/// use router::{FabricGraph, route, NetListExternal, create_test, RoutingConfigBuilder};
///
/// let path = testing_utils::get_test_data_path("pips_4x4.txt");
/// let graph = FabricGraph::from_file(path).unwrap();
///
/// let mut config = RoutingConfigBuilder::default().graph(graph).with_test_netlist(0.1,2).unwrap().build().unwrap();
///
/// let _ = route(&mut config).unwrap();
/// ```
/// # Errors
/// Fails if files cannot be read or cannot be parsed or it cannot write to the output file.
/// Fails if the `max_iterations` are reached
pub fn route<T, L>(config: &mut RoutingConfig<T,L>) -> FabricResult<Vec<IterationResult>>
where
    T: RouteNet,
    L: Logging,
{
    let net_list_external = &mut config.net_list;
    let graph = &mut config.graph;
    if let Some(hash) = &net_list_external.hash {
        if hash != &graph.calculate_structure_hash() {
            eprintln!("Warning: The net-list was not created with this graph.");
        }
    } else {
        eprintln!("Warning: Cannot determine if the net-list was created with this graph. Missing field in net-list.");
    }
    if let Some(slack_report) = &config.slack_report {
        net_list_external.add_slack(slack_report);
    }
    let mut net_list = NetListInternal::from_external(graph, net_list_external)?;
    let router_config = Config::new(config.hist_factor, config.max_iterations);

    let x = path_finder(&mut net_list, graph, &router_config, &config.solver, &config.logger);
    if x.is_ok() {
        *net_list_external = net_list.to_external(graph);
    }
    x
}

/// Converts Expanded JSON-like structure to a FASM string
/// # Errors
/// This errors when the provided `NetListExternal` is not solved meaning it has a result field
/// being `None`
pub fn create_fasm(netlist: &NetListExternal) -> FabricResult<String> {
    net_to_fasm(netlist)
}

/// Creates a Test Netlist by using a `percentage` of all Lut-Outputs and for each `destinations`
/// Lut-Inputs
///
/// # Errors
/// Can produce File Io erros.
/// Fails if parameters are bad like trying to use more than 100% of Lut-Outputs
pub fn create_test(graph: &FabricGraph, percentage: f32, destinations: usize) -> FabricResult<NetListExternal> {
    let mut rng = rand::rng();
    let graph_hash = graph.calculate_structure_hash();
    let (mut inputs, mut outputs) = bucket_luts(graph);

    inputs.shuffle(&mut rng);
    outputs.shuffle(&mut rng);

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
    let input_count = (percentage * outputs.len() as f32) as usize;
    let output_count = input_count * destinations;

    if input_count > outputs.len() {
        return Err(FabricError::CreatingTestBadParameters);
    }
    if output_count > inputs.len() {
        return Err(FabricError::CreatingTestBadParameters);
    }

    let used_outs = inputs.iter().take(output_count).copied().collect::<Vec<NodeId>>();

    let net_list = outputs
        .iter()
        .take(input_count)
        .copied()
        .zip(used_outs.chunks(destinations))
        .map(|(signal, sinks)| {
            NetInternal {
                sinks: sinks.to_vec(),
                signal,
                result: None,
                intermediate_nodes: None,
                priority: None,
                criticallity: 0.0,
            }
            .to_external(graph)
        })
        .collect::<Vec<NetExternal>>();

    let net_list = NetListExternal {
        plan: net_list,
        hash: Some(graph_hash),
    };

    Ok(net_list)
}

/// Validates a routing for a given `FabricGraph`
///
/// # Errors
/// Fails when netlist is invalid
pub fn validate_routing(graph: &FabricGraph, netlist: &NetListExternal) -> FabricResult<()> {
    let netlist = NetListInternal::from_external(graph, netlist)?;
    validate::validate(&netlist, graph)?;
    Ok(())
}

pub struct RoutingConfigBuilder<T: RouteNet, L: Logging> {
    graph: Option<FabricGraph>,
    net_list: Option<NetListExternal>,
    hist_factor: f32,
    max_iterations: usize,
    slack_report: Option<SlackReport>,
    solver: T,
    logger: L,
}

impl RoutingConfigBuilder<SimpleSolver, SimpleLogging> {
    /// Initializes the builder with default values
    #[must_use]
    pub const fn new() -> Self {
        Self {
            graph: None,
            net_list: None,
            hist_factor: 0.1,
            max_iterations: 100,
            slack_report: None,
            solver: SimpleSolver,
            logger: SimpleLogging,
        }
    }
}

impl Default for RoutingConfigBuilder<SimpleSolver, SimpleLogging> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: RouteNet, L: Logging> RoutingConfigBuilder<T, L> {
    #[must_use]
    pub fn graph(mut self, graph: FabricGraph) -> Self {
        self.graph = Some(graph);
        self
    }

    #[must_use]
    pub fn net_list(mut self, net_list: NetListExternal) -> Self {
        self.net_list = Some(net_list);
        self
    }

    /// Helper to generate the test netlist using the provided graph
    /// # Errors
    ///
    pub fn with_test_netlist(mut self, percentage: f32, destinations: usize) -> FabricResult<Self> {
        if let Some(ref g) = self.graph {
            // Unwrapping here assuming valid test params; 
            // alternatively, handle the FabricResult accordingly.
            self.net_list = Some(create_test(g, percentage, destinations)?);
        }
        Ok(self)
    }

    #[must_use]
    pub const fn hist_factor(mut self, factor: f32) -> Self {
        self.hist_factor = factor;
        self
    }

    #[must_use]
    pub const fn max_iterations(mut self, iterations: usize) -> Self {
        self.max_iterations = iterations;
        self
    }

    #[must_use]
    pub fn slack_report(mut self, report: SlackReport) -> Self {
        self.slack_report = Some(report);
        self
    }

    pub fn solver<NewT: RouteNet>(self, solver: NewT) -> RoutingConfigBuilder<NewT, L> {
        RoutingConfigBuilder {
            graph: self.graph,
            net_list: self.net_list,
            hist_factor: self.hist_factor,
            max_iterations: self.max_iterations,
            slack_report: self.slack_report,
            solver,
            logger: self.logger,
        }
    }

    pub fn logger<NewL: Logging>(self, logger: NewL) -> RoutingConfigBuilder<T, NewL> {
        RoutingConfigBuilder {
            graph: self.graph,
            net_list: self.net_list,
            hist_factor: self.hist_factor,
            max_iterations: self.max_iterations,
            slack_report: self.slack_report,
            solver: self.solver,
            logger,
        }
    }

    /// Builds the Routing Config
    /// # Errors
    /// if no graoh or netlist was provided
    pub fn build(self) -> FabricResult<RoutingConfig<T, L>> {
        let graph = self.graph.ok_or("Graph is required to build RoutingConfig")?;
        
        // If net_list is still None, we could either error or try a default. 
        // Given your instructions, we'll error if neither manual nor test netlist was provided.
        let net_list = self.net_list.ok_or("NetList is required (provide manually or use with_test_netlist)")?;

        Ok(RoutingConfig {
            graph,
            net_list,
            hist_factor: self.hist_factor,
            max_iterations: self.max_iterations,
            slack_report: self.slack_report,
            solver: self.solver,
            logger: self.logger,
        })
    }
}
