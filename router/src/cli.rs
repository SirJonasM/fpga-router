use clap::{Parser, Subcommand, ValueEnum};

#[derive(ValueEnum, Clone, Debug)]
pub enum SolverType {
    Simple,
    Steiner,
    SimpleSteiner,
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
    pub output: String,
    #[arg(short, long)]
    pub routing_list: String,
    #[arg(short, long)]
    pub graph: String,
    #[arg(short, long, value_enum, default_value_t = SolverType::Simple)]
    pub solver: SolverType,
    #[arg(short, long, default_value_t = 0.1)]
    pub hist_factor: f32,
    #[arg(short, long, value_enum, default_value=None )]
    pub log_file: Option<String>,
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
    /// Creates a test route_plan
    CreateTest(CreateTestArgs),
    /// Starts the router
    Route(RouteArgs),
}
