mod cli;
use clap::Parser;
use cli::*;
use router::{create_fasm, create_test, start_routing, validate_routing, FileLog, Loggers, SimpleSolver, SimpleSteinerSolver, Solver, SteinerSolver};


fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::CreateTest(args) => create_test(&args.graph, &args.output, args.percentage, args.destinations),
        Commands::Fasm(args) => create_fasm(&args.routing, &args.output),
        Commands::Route(args) => {
            let solver = match args.solver {
                SolverType::Simple => Solver::Simple(SimpleSolver),
                SolverType::Steiner => Solver::Steiner(SteinerSolver),
                SolverType::SimpleSteiner => Solver::SimpleSteiner(SimpleSteinerSolver),
            };
            let logger = match &args.logger {
                LoggerType::No => Loggers::No,
                LoggerType::Terminal => Loggers::Terminal,
                LoggerType::File => {
                    let file = args.log_file.unwrap();
                    Loggers::File(FileLog::new(&file))
                }
            };

            start_routing(
                &args.graph,
                &args.routing_list,
                solver,
                args.hist_factor,
                &args.output,
                &logger,
                args.max_iterations,
            )
        }
        Commands::Validate(args) => {
            match validate_routing(&args.routing, &args.graph){
                Ok(()) => println!("Routing is valid."),
                Err(err) => println!("Routing is invalid due to: {}", err)
            }
        }
    }
}

