use std::{fs::{self, File}, io::BufWriter, sync::Mutex, io::Write};
use crate::IterationResult;

pub enum Loggers {
    No,
    Terminal,
    File(FileLog),
}
impl crate::Logging for Loggers {
    fn log(&self, log_instance: &IterationResult) -> Result<(), String>{
        match self {
            Self::No => {},
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
    fn log(&self, log_instance: &IterationResult) -> Result<(), String>{
        let json = serde_json::to_vec(log_instance).map_err(|_| "Failed to serialize iteration result".to_string())?;
        self.writer.lock().expect("Failed to lock log file mutex").write_all(&json).map_err(|_| "Failed to write to file.")?;
        Ok(())
    }
}
