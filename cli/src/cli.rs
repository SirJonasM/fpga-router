use clap::{Parser, Subcommand, ValueEnum};
use router::{Fabric, FabricGraph, NetInternal, RouteNet, SimpleSolver, SimpleSteinerSolver, SteinerSolver};

#[derive(ValueEnum, Clone, Debug)]
pub enum SolverType {
    Simple,
    Steiner,
    SimpleSteiner,
}
#[derive(ValueEnum, Clone, Debug)]
pub enum LoggerType {
    No,
    Terminal,
}

// --- Subcommand Arguments ---
#[derive(Parser, Debug)]
pub struct CreateTestArgs {
    #[arg(short, long)]
    pub output: String,
    #[arg(short, long)]
    pub graph: String,
    #[arg(short, long)]
    pub destinations: usize,
    #[arg(short, long, default_value_t = 0.2)]
    pub percentage: f32,
}

#[derive(Parser, Debug)]
pub struct RouteArgs {
    #[arg(short, long)]
    /// Can be `json` or `fasm`
    pub output: String,
    #[arg(short, long)]
    pub net_list: String,
    #[arg(short, long)]
    pub graph: String,
    #[arg(short, long)]
    pub bel: String,
    #[arg(short = 'S', long, value_enum, default_value_t = SolverType::Simple)]
    pub solver: SolverType,
    #[arg(long, default_value_t = 0.1)]
    pub hist_factor: f32,
    #[arg(short='L', long, value_enum, default_value_t=LoggerType::Terminal )]
    pub logger: LoggerType,
    #[arg(short='l', long, value_enum, default_value=None )]
    pub log_file: Option<String>,
    #[arg(short = 'i', long, default_value_t = 2000)]
    pub max_iterations: usize,
    #[arg(short, long)]
    pub ffs: Option<String>,
    #[arg(short, long)]
    pub timing_model: Option<String>,
}

#[derive(Parser, Debug)]
pub struct ValidateArgs {
    #[arg(short, long)]
    pub graph: String,
    #[arg(short, long)]
    pub net_list: String,
}

#[derive(Parser, Debug)]
pub struct RouteStaArgs {
    #[arg(short, long)]
    pub graph: String,
    #[arg(short, long)]
    pub bel: String,
    #[arg(short, long)]
    pub net_list: String,
    #[arg(short, long)]
    pub output: String, // This will be the final FASM output
    #[arg(short = 'S', long, value_enum, default_value_t = SolverType::Simple)]
    pub solver: SolverType,
    #[arg(long, default_value_t = 0.1)]
    pub hist_factor: f32,
    #[arg(short='L', long, value_enum, default_value_t=LoggerType::Terminal )]
    pub logger: LoggerType,
    #[arg(short='l', long, value_enum, default_value=None )]
    pub log_file: Option<String>,
    #[arg(short = 'i', long, default_value_t = 2000)]
    pub max_iterations: usize,
    #[arg(long, default_value = "5000")]
    pub target_ps: u32,
    #[arg(short, long)]
    pub ffs: String,
    #[arg(short, long)]
    pub timings: String,
}

// --- CLI Structure ---

#[derive(Parser, Debug)]
#[command(version, about = "FPGA Routing Utility")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Creates a test `route_plan`
    CreateTest(CreateTestArgs),
    /// Starts the router
    Route(RouteArgs),

    Validate(ValidateArgs),

    RouteSta(RouteStaArgs),
}

pub enum Solver {
    Simple(SimpleSolver),
    SimpleSteiner(SimpleSteinerSolver),
    Steiner(SteinerSolver),
}

impl RouteNet for Solver {
    fn solve(&self, graph: &Fabric, routing: &mut NetInternal) -> router::FabricResult<()> {
        match self {
            Self::Simple(simple_solver) => simple_solver.solve(graph, routing),
            Self::SimpleSteiner(simple_steiner_solver) => simple_steiner_solver.solve(graph, routing),
            Self::Steiner(steiner_solver) => steiner_solver.solve(graph, routing),
        }
    }

    fn pre_process(&self, graph: &mut Fabric, route_plan: &mut [NetInternal]) -> router::FabricResult<()> {
        match self {
            Self::Simple(simple_solver) => simple_solver.pre_process(graph, route_plan),
            Self::SimpleSteiner(simple_steiner_solver) => simple_steiner_solver.pre_process(graph, route_plan),
            Self::Steiner(steiner_solver) => steiner_solver.pre_process(graph, route_plan),
        }
    }

    fn identifier(&self) -> &'static str {
        match self {
            Self::Simple(simple_solver) => simple_solver.identifier(),
            Self::SimpleSteiner(simple_steiner_solver) => simple_steiner_solver.identifier(),
            Self::Steiner(steiner_solver) => steiner_solver.identifier(),
        }
    }
}
