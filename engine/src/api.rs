use std::collections::{HashMap, HashSet};

use rand::seq::SliceRandom;

use crate::graph::fabric_graph::{Fabric, State, Tile, TileId, TileManager};
use crate::path_finder::{TimingAnalysis, timing_driven_path_finder};
use crate::{
    FabricError, FabricResult, Logging,
    fasm::net_to_fasm,
    graph::fabric_graph::{FabricGraph, bucket_luts},
    graph::node::NodeId,
    netlist::{NetExternal, NetInternal, NetListExternal, NetListInternal},
    path_finder::{Config, path_finder},
    solver::RouteNet,
    validate,
};
use crate::{IterationResult, SimpleLogging, SimpleSolver};

pub struct TimingDrivenRoutingConfig<R: RouteNet, L: Logging, T: TimingAnalysis> {
    pub graph: FabricGraph,
    pub net_list: NetListExternal,
    pub hist_factor: f32,
    pub max_iterations: usize,
    pub solver: R,
    pub logger: L,
    pub sta: T,
}

pub struct RoutingConfig<R: RouteNet, L: Logging> {
    pub fabric: Fabric,
    pub net_list: NetListExternal,
    pub hist_factor: f32,
    pub max_iterations: usize,
    pub solver: R,
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
pub fn route<R, L>(config: &mut RoutingConfig<R, L>) -> FabricResult<Vec<IterationResult>>
where
    R: RouteNet,
    L: Logging,
{
    let net_list_external = &mut config.net_list;
    let fabric = &mut config.fabric;
    if let Some(hash) = &net_list_external.hash {
        if hash != &fabric.graph.calculate_structure_hash() {
            eprintln!("Warning: The net-list was not created with this graph.");
        }
    } else {
        eprintln!("Warning: Cannot determine if the net-list was created with this graph. Missing field in net-list.");
    }
    let mut net_list = NetListInternal::from_external(&fabric.graph, net_list_external)?;

    for net in &net_list.plan {
        // Check the Source
        fabric.check_and_mark_node(net.signal);

        // Check all Sinks
        for sink_id in &net.sinks {
            fabric.check_and_mark_node(*sink_id);
        }
    }

    let net_list_flatten = net_list
        .plan
        .iter()
        .flat_map(|a| a.sinks.iter().map(|v| (a.signal, *v)))
        .collect::<HashSet<(NodeId, NodeId)>>();

    let mut optimized_net = HashSet::new();
    for (signal, sink) in &net_list_flatten {
        let signal_node = fabric.graph.get_node(*signal);
        if fabric.graph.dijkstra(*signal, *sink, 0.0).is_some() {
            optimized_net.insert((*signal, *sink));
            continue;
        }
        let state = match signal_node.id.as_str() {
            "VCC0" => Some(State::High),
            "GND0" => Some(State::Low),
            _ => None,
        };
        let sink_node = fabric.graph.get_node(*sink);
        let state = state.ok_or(FabricError::Other(format!("Cannot find a routing for net {} -> {}",sink_node.id(),signal_node.id())))?;
        let tile = TileId(sink_node.x, sink_node.y);
        let new_source_name = fabric
            .tile_manager
            .request_constant(tile, state)
            .ok_or(FabricError::Other("Fabric exhausted: No free LUTs for constants".into()))?;

        let node_id_str = format!("X{}Y{}.{}", new_source_name.0.0, new_source_name.0.1, new_source_name.1);
        let node = fabric.graph.get_node_id(&node_id_str).unwrap();
        // 4. Re-run Dijkstra with the new local source
        let _ = fabric.graph.dijkstra(*node, *sink, 0.0).ok_or(FabricError::Other(format!(
            "Even local constant {:?} couldn't reach sink",
            new_source_name
        )))?;
        optimized_net.insert((*node, *sink));
    }

    // 1. Group sinks by their signal (source)
    let mut grouped_nets: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

    for (signal, sink) in optimized_net {
        grouped_nets.entry(signal).or_default().push(sink);
    }

    // 2. Map the groups back into NetInternal structures
    let new_plan: Vec<NetInternal> = grouped_nets
        .into_iter()
        .map(|(signal, sinks)| NetInternal {
            signal,
            sinks,
            result: None,
            intermediate_nodes: None,
            priority: None,    // Set to default as requested
            criticallity: 0.0, // Set to default as requested
        })
        .collect();
    net_list = NetListInternal { plan: new_plan };

    let router_config = Config::new(config.hist_factor, config.max_iterations);

    let x = path_finder(&mut net_list, fabric, &router_config, &config.solver, &config.logger);
    if x.is_ok() {
        *net_list_external = net_list.to_external(&fabric.graph);
    }
    let luts_borrowed_fasm = fabric.tile_manager.generate_constant_fasm();
    println!("{}", luts_borrowed_fasm.join("\n"));
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

/// Tries to route the given netlist in timing driven appraoch
/// # Errors
///
pub fn route_timing_driven<R, L, T>(config: &mut TimingDrivenRoutingConfig<R, L, T>) -> FabricResult<Vec<IterationResult>>
where
    R: RouteNet,
    L: Logging,
    T: TimingAnalysis,
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
    let mut net_list = NetListInternal::from_external(graph, net_list_external)?;
    let router_config = Config::new(config.hist_factor, config.max_iterations);

    let x = timing_driven_path_finder(
        &mut net_list,
        graph,
        &router_config,
        &config.solver,
        &config.logger,
        &config.sta,
    );
    if x.is_ok() {
        *net_list_external = net_list.to_external(graph);
    }
    x
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

pub struct RoutingConfigBuilder<R: RouteNet, L: Logging> {
    tile_manager: Option<TileManager>,
    graph: Option<FabricGraph>,
    net_list: Option<NetListExternal>,
    hist_factor: f32,
    max_iterations: usize,
    solver: R,
    logger: L,
}

impl RoutingConfigBuilder<SimpleSolver, SimpleLogging> {
    /// Initializes the builder with default values
    #[must_use]
    pub const fn new() -> Self {
        Self {
            graph: None,
            tile_manager: None,
            net_list: None,
            hist_factor: 0.1,
            max_iterations: 100,
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

impl<R: RouteNet, L: Logging> RoutingConfigBuilder<R, L> {
    #[must_use]
    pub fn graph(mut self, graph: FabricGraph) -> Self {
        self.graph = Some(graph);
        self
    }
    #[must_use]
    pub fn tile_manager(mut self, tile_manager: TileManager) -> Self {
        self.tile_manager = Some(tile_manager);
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

    pub fn solver<NewT: RouteNet>(self, solver: NewT) -> RoutingConfigBuilder<NewT, L> {
        RoutingConfigBuilder {
            graph: self.graph,
            tile_manager: self.tile_manager,
            net_list: self.net_list,
            hist_factor: self.hist_factor,
            max_iterations: self.max_iterations,
            solver,
            logger: self.logger,
        }
    }

    pub fn logger<NewL: Logging>(self, logger: NewL) -> RoutingConfigBuilder<R, NewL> {
        RoutingConfigBuilder {
            graph: self.graph,
            tile_manager: self.tile_manager,
            net_list: self.net_list,
            hist_factor: self.hist_factor,
            max_iterations: self.max_iterations,
            solver: self.solver,
            logger,
        }
    }
    /// Builds the Routing Config
    /// # Errors
    /// if no graoh or netlist was provided
    pub fn build(self) -> FabricResult<RoutingConfig<R, L>> {
        let graph = self.graph.ok_or("Graph is required to build RoutingConfig")?;
        let tile_manager = self.tile_manager.ok_or("Graph is required to build RoutingConfig")?;
        let fabric = Fabric { tile_manager, graph };

        // If net_list is still None, we could either error or try a default.
        // Given your instructions, we'll error if neither manual nor test netlist was provided.
        let net_list = self
            .net_list
            .ok_or("NetList is required (provide manually or use with_test_netlist)")?;

        Ok(RoutingConfig {
            fabric,
            net_list,
            hist_factor: self.hist_factor,
            max_iterations: self.max_iterations,
            solver: self.solver,
            logger: self.logger,
        })
    }

    /// Builds the Routing Config
    /// # Errors
    /// if no graoh or netlist was provided
    pub fn build_timing_driven<T: TimingAnalysis>(self, sta: T) -> FabricResult<TimingDrivenRoutingConfig<R, L, T>> {
        let graph = self.graph.ok_or("Graph is required to build RoutingConfig")?;
        let net_list = self
            .net_list
            .ok_or("NetList is required (provide manually or use with_test_netlist)")?;

        Ok(TimingDrivenRoutingConfig {
            graph,
            net_list,
            hist_factor: self.hist_factor,
            max_iterations: self.max_iterations,
            solver: self.solver,
            logger: self.logger,
            sta,
        })
    }
}
