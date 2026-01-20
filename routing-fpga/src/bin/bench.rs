use std::cmp::Ordering;

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use routing_fpga::{
    route, validate_routing, FabricGraph, IterationResult, Logging, SimpleSolver, SimpleSteinerSolver, Solver, SteinerSolver, TestCase
};

fn get_graph() -> FabricGraph {
    match FabricGraph::from_file("../pips.txt") {
        Ok(graph) => graph,
        Err(err) => panic!("{}", err),
    }
}
struct Logger;

impl Logging for Logger {
    fn log(&self, _log_instance: &routing_fpga::IterationResult) {}
}

fn get_test_cases() -> Vec<TestCase> {
    let destinations = [1, 2, 3, 4];
    let percentages = [20, 40, 60, 80, 100];
    let solvers = [
        Solver::Simple(SimpleSolver),
        Solver::Steiner(SteinerSolver),
        Solver::SimpleSteiner(SimpleSteinerSolver)
    ];
    let hist_factors: [f32; 3] = [0.1, 0.001, 1.0];
    let mut test_cases = percentages
        .into_iter()
        .flat_map(|percentage| {
            destinations
                .into_iter()
                .map(|dst| (percentage, dst))
                .collect::<Vec<(usize, usize)>>()
        })
        .flat_map(|(percentage, dst)| {
            solvers
                .iter()
                .map(|solver| (percentage, dst, solver.clone()))
                .collect::<Vec<(usize, usize, Solver)>>()
        })
        .enumerate()
        .flat_map(|(id, (percentage, dst, solver))| {
            hist_factors
                .into_iter()
                .map(|hist_factor| TestCase {
                    id: id as u64,
                    percentage,
                    dst,
                    hist_factor,
                    solver: solver.clone(),
                })
                .collect::<Vec<TestCase>>()
        })
        .collect::<Vec<TestCase>>();
    test_cases.sort_by(|test1, test2| {
        test1
            .percentage
            .cmp(&test2.percentage)
            .then(test1.dst.cmp(&test2.dst).then(Ordering::Equal))
    });
    test_cases
}

fn main() {
    let logger = Logger;
    let test_cases = get_test_cases();
    let result = test_cases
        .into_par_iter()
        .map(|test_case| {
            let r = match run_test(&test_case, &logger){
                Ok(i) => i,
                Err(i) => i,
            };
            r.to_string()
        })
        .collect::<Vec<String>>();

    println!("{}", result.join("\n"));
}

fn run_test(test_case: &TestCase, logger: &Logger) -> Result<IterationResult, IterationResult> {
    println!(
        "Starting to solve: {}, {}, {:?}, {}",
        test_case.percentage, test_case.dst, test_case.solver, test_case.hist_factor
    );
    let mut graph = get_graph();
    let mut routing = graph.route_plan(test_case.percentage as f32 / 100.0, test_case.dst);
    let result = route(logger, test_case.clone(), &mut graph, &mut routing);
    if result.is_ok() {
        println!(
            "Solved: {}, {}, {:?}, {}",
            test_case.percentage, test_case.dst, test_case.solver, test_case.hist_factor
        );
        validate_routing(&graph, &routing).unwrap()
    } else {
        println!(
            "Failed: {}, {}, {:?}, {}",
            test_case.percentage, test_case.dst, test_case.solver, test_case.hist_factor
        );
    };
    result
}

