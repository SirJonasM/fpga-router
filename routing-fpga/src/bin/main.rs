use clap::Parser;
use routing_fpga::FabricGraph;
pub fn main(){
    let args = Args::parse();
    let graph = FabricGraph::from_file(&args.graph).unwrap();
    let routing_list = 
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args{
    #[arg(short, long)]
    output: String,
    #[arg(short, long)]
    routing_list: String,
    #[arg(short, long)]
    graph: String
}
