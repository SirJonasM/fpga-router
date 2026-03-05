use crate::{FabricError, FabricResult, IterationResult};
use std::{
    fs::{self, File},
    io::BufWriter,
    io::Write,
    sync::Mutex,
};

/// Trait for logging pathfinding iterations.
pub trait Logging {
    /// Logs the current iteration result.
    ///
    /// # Errors
    /// Should return an `LoggingError`
    fn log(&self, log_instance: &IterationResult) -> FabricResult<()>;
}

pub enum Loggers {
    No,
    Terminal,
    File(FileLog),
}
impl crate::Logging for Loggers {
    fn log(&self, log_instance: &IterationResult) -> FabricResult<()> {
        match self {
            Self::No => {}
            Self::Terminal => println!("{log_instance}"),
            Self::File(file_log) => file_log.log(log_instance)?,
        }
        Ok(())
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
    pub fn new(path: &str) -> Result<Self, String> {
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|_| format!("Could not open log file: {path}"))?;

        Ok(Self {
            writer: Mutex::new(BufWriter::new(file)),
        })
    }
    fn log(&self, log_instance: &IterationResult) -> FabricResult<()> {
        let json = serde_json::to_vec(log_instance).map_err(|_| "Failed to serialize iteration result".to_string())?;
        self.writer
            .lock()
            .map_err(|_| FabricError::LoggingError("Failed to lock log file mutex".to_string()))?
            .write_all(&json)
            .map_err(|_| FabricError::LoggingError("Failed to write to file.".to_string()))?;
        Ok(())
    }
}
