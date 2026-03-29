use std::{time::Instant, fs::{self, File}, io::Write, sync::{Arc, Mutex}};
use simplelog::*;

// ... imports remain same ...
use fpga_timing_analyzer::{
    build_design, design_to_json_map, parse_all_timing_constraints, parse_all_timing_models, parsers::fasm_parser, perform_timing_analysis, pips_parser, report_violations
};

// Custom writer that allows redirecting output to a file dynamically
struct DynamicWriter {
    target: Arc<Mutex<Option<File>>>,
}

impl Write for DynamicWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(mut lock) = self.target.lock() {
            if let Some(ref mut file) = *lock {
                return file.write(buf);
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Ok(mut lock) = self.target.lock() {
            if let Some(ref mut file) = *lock {
                return file.flush();
            }
        }
        Ok(())
    }
}

fn main() {
    // Create output directory
    fs::create_dir_all("output").expect("idk why reading output dir failed");

    // Shared file handle for dynamic logging
    let report_file_handle: Arc<Mutex<Option<File>>> = Arc::new(Mutex::new(None));
    let logger_writer = DynamicWriter { target: report_file_handle.clone() };

    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Info, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(LevelFilter::Trace, Config::default(), File::create("output/timing_analysis.log").unwrap()),
            WriteLogger::new(LevelFilter::Info, Config::default(), logger_writer),
        ]
    ).unwrap();

    let start = Instant::now();

    let (configurations, flops) = fasm_parser("sequential_16bit_en.fasm").unwrap();
    
    let flops_json = serde_json::to_string_pretty(&flops).expect("Failed to serialize flops");
    std::fs::write("output/flops.json", flops_json).expect("Failed to write flops to file");
    
    let pips = pips_parser("pips_8x8.txt");

    let timing_models = parse_all_timing_models("src/timing_model_files");
    let timing_constraints_list = parse_all_timing_constraints("src/timing_constraints_files");

    if timing_models.is_empty() {
        log::error!("No timing model files found in src/timing_model_files/");
    }
    if timing_constraints_list.is_empty() {
        log::error!("No timing constraint files found in src/timing_constraints_files/");
    }

    for (model_name, timing_model) in &timing_models {
        log::info!("Loading Timing Model: {}", model_name);
        
        let design = build_design(&pips, &configurations, &flops, &timing_model);
        
        let json_string = serde_json::to_string_pretty(&design_to_json_map(&design)).expect("Failed to serialize design");
        std::fs::write("output/design.json", json_string).expect("Failed to write design to file");

        for (constraint_name, timing_constraints) in &timing_constraints_list {
            // Update the log file for this testcase
            let report_name = format!("output/report_{}_{}.log", model_name.replace(".json", ""), constraint_name.replace(".json", ""));
            if let Ok(mut lock) = report_file_handle.lock() {
                *lock = Some(File::create(&report_name).expect("Failed to create report file"));
            }

            log::info!("============================================================");
            log::info!("Running Analysis");
            log::info!("  Model:       {}", model_name);
            log::info!("  Constraints: {}", constraint_name);
            log::info!("============================================================");

            let result = perform_timing_analysis(&design, &flops);
            println!("{result:#?}");
            
            report_violations(&result.min_paths, &result.max_paths, timing_constraints); 

            let csv = result.to_slack_csv(timing_constraints.clk_period);
            fs::write("slack.csv", csv).unwrap();

            log::info!("ANALYSIS END");
            log::info!("============================================================");
        }
    }
    
    log::info!("Time elapsed in main() is: {:#?}", start.elapsed());
}
