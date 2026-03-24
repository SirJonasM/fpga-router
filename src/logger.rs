use std::{
    fs::{self, File}, io::{BufWriter, Write}, path::Path, sync::Mutex
};

use router::{FabricError, FabricResult, LogInstance};

pub enum Loggers {
    No,
    Terminal,
    File(FileLog),
}
impl router::Logging for Loggers {
    fn log(&self, log_instance: &LogInstance) -> FabricResult<()> {
        match self {
            Self::No => {}
            Self::Terminal => terminal_log(log_instance),
            Self::File(file_log) => file_log.log(log_instance)?,
        }
        Ok(())
    }
}

fn terminal_log(log_instance: &LogInstance) {
    match log_instance {
        LogInstance::Text(t) => println!("{t}"),
        LogInstance::RouterIteration(iteration_result) => {
            print!(
                "\rIteration: {: >3}, Conflicts: {: >4}, Wire Efficiency: {:.3}",
                iteration_result.iteration, iteration_result.conflicts, iteration_result.wire_reuse
            );
            std::io::stdout().flush().unwrap();
        }
    }
}

pub struct FileLog {
    writer: Mutex<BufWriter<File>>,
}

impl FileLog {
    /// Creates a new `FileLog` by opening or creating a file if it does not exist.
    /// Opens in append mode
    /// # Errors
    /// Fails if the provided file cannot be opened
    pub fn new<P: AsRef<Path>>(path: &P) -> Result<Self, String> {
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|_| format!("Could not open log file: {}", path.as_ref().to_path_buf().display()))?;

        Ok(Self {
            writer: Mutex::new(BufWriter::new(file)),
        })
    }
    fn log(&self, log_instance: &LogInstance) -> FabricResult<()> {
        let json = serde_json::to_vec(log_instance).map_err(|_| "Failed to serialize iteration result".to_string())?;
        self.writer
            .lock()
            .map_err(|_| FabricError::LoggingError("Failed to lock log file mutex".to_string()))?
            .write_all(&json)
            .map_err(|_| FabricError::LoggingError("Failed to write to file.".to_string()))?;
        Ok(())
    }
}
