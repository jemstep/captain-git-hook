use std::net::TcpStream;
use log;
use log::{LevelFilter, Level, Log, Metadata, Record};
use chrono::prelude::*;
use std::io::prelude::*;
use std::sync::Mutex;
use std::fmt::Display;
use serde::Serialize;
use serde_json;
use uuid::Uuid;

pub struct Logger {
    tcp_stream: Option<Mutex<TcpStream>>,
    context: LoggingContext
}

impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");

        eprintln!("{}", record.args());

        let message = LogMessage {
            timestamp: timestamp.to_string(),
            level: record.level(),
            context: self.context.clone(),
            message: format!("{}", record.args())
        };

        self.tcp_stream.as_ref().map(|ref stream| {
            stream.lock()
                .map_err(|e| e.to_string())
                .and_then(|stream| serde_json::to_writer(&(*stream), &message).map_err(|e| e.to_string()))
                .unwrap_or_else(|e| eprintln!("Error: Failed to log over TCP - {}", e));
        });
    }

    fn flush(&self) {
        std::io::stderr().flush()
            .unwrap_or_else(|e| eprintln!("Error: Failed to flush StdErr logging stream - {}", e));

        self.tcp_stream.as_ref().map(|ref stream| {
            stream.lock()
                .map_err(|e| e.to_string())
                .and_then(|mut stream| stream.flush().map_err(|e| e.to_string()))
                .unwrap_or_else(|e| eprintln!("Error: Failed to flush TCP logging stream - {}", e));
        });
    }
}

impl Logger {
    pub fn init(level: LevelFilter, tcp_target: Option<String>) {
        log::set_max_level(level);
        let context = LoggingContext {
            run_id: Uuid::new_v4(),
            trigger: None,
            user_id: None,
            user_ip: None
        };
        
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
            tcp_stream,
            context
        }));
        if log_set_result.is_err() {
            eprintln!("Error: Logger initialized twice!");
        }
    }
}

pub fn print_header(text: impl Display, quiet: bool) {
    if !quiet {
        let seperator = "********************************************************************************";
        println!("\n{0}\n{1}\n{0}\n", seperator, text);
    }
}


#[derive(Serialize)]
struct LogMessage {
    timestamp: String,
    level: Level,
    context: LoggingContext,
    message: String
}

#[derive(Serialize, Clone)]
pub struct LoggingContext {
    run_id: Uuid,
    trigger: Option<String>,
    user_id: Option<String>,
    user_ip: Option<String>,
}
