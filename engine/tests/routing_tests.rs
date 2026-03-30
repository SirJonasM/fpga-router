use std::sync::atomic::{AtomicUsize, Ordering};

use router::{FabricGraph, Logging, RoutingConfigBuilder, SimpleSteinerSolver, SteinerSolver, TileManager, create_fasm, create_test, route};
use testing_utils::get_test_data_path;

struct MockLogger {
    pub calls_text: AtomicUsize,
    pub calls_iteration: AtomicUsize,
}
impl Logging for MockLogger {
    fn log(&self, log_instance: &router::LogInstance) -> router::FabricResult<()> {
        match log_instance {
            router::LogInstance::Text(_) => self.calls_text.fetch_add(1, Ordering::Relaxed),
            router::LogInstance::RouterIteration(_) => self.calls_iteration.fetch_add(1, Ordering::Relaxed),
            router::LogInstance::RouterStaIteration(_) => self.calls_iteration.fetch_add(1, Ordering::Relaxed),
        };
        Ok(())
    }
}

#[test]
fn test_create_test() {
    let graph = FabricGraph::from_file(&get_test_data_path("pips_8x8.txt"), None).unwrap();
    let result = create_test(&graph, 0.1, 3);
    assert!(result.is_ok())
}
#[test]
fn test_create_test_bad_percentage() {
    let graph = FabricGraph::from_file(&get_test_data_path("pips_8x8.txt"), None).unwrap();
    let result = create_test(&graph, 100.1, 3);
    assert!(result.is_err())
}

#[test]
fn test_create_test_bad_destinations() {
    let graph = FabricGraph::from_file(&get_test_data_path("pips_8x8.txt"), None).unwrap();
    let result = create_test(&graph, 0.1, 10000);
    assert!(result.is_err())
}

#[test]
fn test_routing_simple() {
    let graph = FabricGraph::from_file(&get_test_data_path("pips_8x8.txt"), None).unwrap();
    let tile_manager = TileManager::from_file(&get_test_data_path("bel.txt")).unwrap();
    let mut config = RoutingConfigBuilder::default()
        .graph(graph)
        .with_test_netlist(0.2, 3)
        .unwrap()
        .tile_manager(tile_manager)
        .build()
        .unwrap();
    let result = route(&mut config);
    assert!(result.is_ok())
}

#[test]
fn test_routing_simple_steiner() {
    let graph = FabricGraph::from_file(&get_test_data_path("pips_8x8.txt"), None).unwrap();
    let tile_manager = TileManager::from_file(&get_test_data_path("bel.txt")).unwrap();
    let mut config = RoutingConfigBuilder::default()
        .graph(graph)
        .with_test_netlist(0.2, 1)
        .unwrap()
        .tile_manager(tile_manager)
        .solver(SimpleSteinerSolver)
        .build()
        .unwrap();
    let result = route(&mut config);
    assert!(result.is_ok())
}

#[test]
fn test_routing_steiner() {
    let graph = FabricGraph::from_file(&get_test_data_path("pips_8x8.txt"), None).unwrap();
    let tile_manager = TileManager::from_file(&get_test_data_path("bel.txt")).unwrap();
    let mut config = RoutingConfigBuilder::default()
        .graph(graph)
        .tile_manager(tile_manager)
        .with_test_netlist(0.2, 3)
        .unwrap()
        .solver(SteinerSolver)
        .build()
        .unwrap();
    let result = route(&mut config);
    assert!(result.is_ok())
}

#[test]
fn test_routing_simple_logging() {
    let graph = FabricGraph::from_file(&get_test_data_path("pips_8x8.txt"), None).unwrap();
    let tile_manager = TileManager::from_file(&get_test_data_path("bel.txt")).unwrap();
    let logger = MockLogger {
        calls_text: AtomicUsize::new(0),
        calls_iteration: AtomicUsize::new(0),
    };
    let mut config = RoutingConfigBuilder::default()
        .graph(graph)
        .tile_manager(tile_manager)
        .with_test_netlist(0.2, 3)
        .unwrap()
        .logger(logger)
        .build()
        .unwrap();
    let result = route(&mut config).unwrap();
    let len = result.1.len();
    assert!(config.logger.calls_iteration.load(Ordering::Relaxed) == len);
}

#[test]
fn test_create_fasm() {
    let graph = FabricGraph::from_file(&get_test_data_path("pips_8x8.txt"), None).unwrap();
    let tile_manager = TileManager::from_file(&get_test_data_path("bel.txt")).unwrap();
    let mut config = RoutingConfigBuilder::default()
        .graph(graph)
        .tile_manager(tile_manager)
        .with_test_netlist(0.2, 3)
        .unwrap()
        .build()
        .unwrap();
    let _ = route(&mut config).unwrap();
    let net_list = config.net_list;
    let _ = create_fasm(&net_list, &config.fabric.tile_manager).unwrap();
}
