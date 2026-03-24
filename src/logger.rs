use std::io::Write;

use router::{FabricResult, LogInstance};

pub enum Loggers {
    No,
    Terminal,
}
impl router::Logging for Loggers {
    fn log(&self, log_instance: &LogInstance) -> FabricResult<()> {
        match self {
            Self::No => {}
            Self::Terminal => terminal_log(log_instance),
        }
        Ok(())
    }
}

fn terminal_log(log_instance: &LogInstance) {
    match log_instance {
        LogInstance::Text(t) => println!("{t}"),
        LogInstance::RouterIteration(iteration_result) => {
            print!(
                "\rIteration: {: >3}, Conflicts: {: >4}, Wire Efficiency: {:.3}\r",
                iteration_result.iteration, iteration_result.conflicts, iteration_result.wire_reuse
            );
            std::io::stdout().flush().unwrap();
        }
    }
}
