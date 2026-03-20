use serde::Serialize;

use crate::{FabricError, FabricResult, IterationResult};
use std::{
    fmt::Display,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
    sync::Mutex,
};

#[derive(Debug, Clone, Serialize)]
pub enum LogInstance<'a> {
    Text(String),
    RouterIteration(&'a IterationResult),
}

impl<'a> From<&str> for LogInstance<'a> {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl<'a> From<String> for LogInstance<'a> {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl<'a> Display for LogInstance<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogInstance::Text(s) => write!(f, "{}", s),
            LogInstance::RouterIteration(iteration_result) => write!(f, "{}", iteration_result),
        }
    }
}

/// Trait for logging pathfinding iterations.
pub trait Logging {
    /// Logs the current iteration result.
    ///
    /// # Errors
    /// Should return an `LoggingError`
    fn log(&self, log_instance: &LogInstance) -> FabricResult<()>;
}

pub enum Loggers {
    No,
    Terminal,
    File(FileLog),
}
impl crate::Logging for Loggers {
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
            .map_err(|_| format!("Could not open log file: {:?}", path.as_ref().to_path_buf()))?;

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
