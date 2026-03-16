use clap::{Parser, Subcommand, ValueEnum};

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
    File,
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
    pub routing_list: String,
    #[arg(short, long)]
    pub graph: String,
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
    #[arg(short = 's', long)]
    pub slack_report: Option<String>,
}
#[derive(Parser, Debug)]
pub struct FasmArgs {
    #[arg(short, long)]
    pub output: String,
    #[arg(short, long)]
    pub routing: String,
}

#[derive(Parser, Debug)]
pub struct ValidateArgs{
    #[arg(short, long)]
    pub graph: String,
    #[arg(short, long)]
    pub routing: String,
}

#[derive(Parser, Debug)]
pub struct RouteStaArgs {
    #[arg(short, long)]
    pub graph: String,
    #[arg(short, long)]
    pub routing_list: String,
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
    #[arg(long, default_value = "10")]
    pub max_sta_cycles: usize,
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
    /// parses the router output to fasm
    Fasm(FasmArgs),

    Validate(ValidateArgs),

    RouteSta(RouteStaArgs),
}
