use std::{fs::{self, File}, io::BufWriter, sync::Mutex, io::Write};
use crate::IterationResult;

pub enum Loggers {
    No,
    Terminal,
    File(FileLog),
}
impl crate::Logging for Loggers {
    fn log(&self, log_instance: &IterationResult) {
        match self {
            Loggers::No => {}
            Loggers::Terminal => println!("{}", log_instance),
            Loggers::File(file_log) => file_log.log(log_instance),
        }
    }
}

pub struct FileLog {
    writer: Mutex<BufWriter<File>>,
}

impl FileLog {
    pub fn new(path: &str) -> Self {
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("Could not open log file");

        Self {
            writer: Mutex::new(BufWriter::new(file)),
        }
    }
    fn log(&self, log_instance: &IterationResult) {
        // Lock the mutex. If another thread is logging, this will wait its turn.
        let mut guard = self.writer.lock().expect("Failed to lock log file mutex");

        // Serialize and write
        if let Ok(json) = serde_json::to_string(log_instance) {
            // Use writeln! to handle the newline and the buffer
            let _ = writeln!(guard, "{}", json);
        }
    }
}
