use std::net::TcpStream;
use log;
use log::{LevelFilter, Log, Metadata, Record};
use chrono::prelude::*;
use std::io::prelude::*;

pub struct Logger {
    tcp_stream: Option<TcpStream>,
}

impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &Record<'_>) {
        // colourize
        // timestamp
        // print level

        if !self.enabled(record.metadata()) {
            return;
        }

        eprintln!("{} - {} - {}", Local::now().format("%Y-%m-%d %H:%M:%S"), record.level(), record.args());

        if let Some(ref mut stream) = &mut self.tcp_stream {
            writeln!(stream, "{} - {} - {}", Local::now().format("%Y-%m-%d %H:%M:%S"), record.level(), record.args()).ok();
        }
    }

    fn flush(&self) {
    }
}

impl Logger {
    /// creates a new stderr logger
    pub fn init() {
        log::set_max_level(LevelFilter::Trace);
        log::set_boxed_logger(Box::new(Logger {
            tcp_stream: None
        })).unwrap();
    }
}
