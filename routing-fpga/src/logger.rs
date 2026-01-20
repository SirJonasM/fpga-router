use std::sync::{Arc, Mutex};
use std::fs::OpenOptions;
use std::io::Write;

pub fn create_logger(log_file: &str) -> Arc<Mutex<std::fs::File>> {
    Arc::new(Mutex::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
            .expect("Cannot open results.txt"),
    ))
}


#[macro_export]
macro_rules! logln {
    ($logger:expr, $($arg:tt)*) => {{
        let mut file = $logger.lock().unwrap();
        writeln!(file, $($arg)*).unwrap();
    }};
}
