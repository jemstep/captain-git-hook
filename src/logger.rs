use std::net::TcpStream;
use log;
use log::{LevelFilter, Log, Metadata, Record};
use chrono::prelude::*;
use std::io::prelude::*;
use std::sync::Mutex;

pub struct Logger {
    tcp_stream: Option<Mutex<TcpStream>>,
}

impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let formatted = format!("{} - {} - {}", Local::now().format("%Y-%m-%d %H:%M:%S"), record.level(), record.args());

        eprintln!("{}", formatted);

        self.tcp_stream.as_ref().map(|ref stream| {
            stream.lock()
                .map_err(|e| e.to_string())
                .and_then(|mut stream| writeln!(*stream, "{}", formatted).map_err(|e| e.to_string()))
                .unwrap_or_else(|e| eprintln!("Error: Failed to log over TCP - {}", e));
        });
    }

    fn flush(&self) {
    }
}

impl Logger {
    pub fn init(quiet: bool, verbosity: usize, tcp_target: Option<String>) {
        let level_filter = match (quiet, verbosity) {
            (true, _) => LevelFilter::Off,
            (false, 0) => LevelFilter::Info,
            (false, 1) => LevelFilter::Debug,
            (false, _) => LevelFilter::Trace,
        };
        
        log::set_max_level(level_filter);
        let tcp_stream = tcp_target
            .and_then(|ref uri| {
                let connect_result = TcpStream::connect(uri);
                if let Err(ref e) = connect_result {
                    eprintln!("Error: Failed to initialize TCP logging to {} - {}", uri, e);
                }
                connect_result.ok()
            })
            .map(|stream| Mutex::new(stream));
        
        let log_set_result = log::set_boxed_logger(Box::new(Logger {
            tcp_stream
        }));
        if log_set_result.is_err() {
            eprintln!("Error: Logger initialized twice!");
        }
    }
}
