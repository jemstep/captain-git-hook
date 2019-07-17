use chrono::prelude::*;
use log;
use log::{Level, LevelFilter, Log, Metadata, Record};
use serde::Serialize;
use serde_json;
use std::fmt::Display;
use std::io::prelude::*;
use std::net::TcpStream;
use std::sync::Mutex;
use structopt::StructOpt;
use uuid::Uuid;

#[derive(Debug, StructOpt)]
pub struct LoggingOpt {
    /// Silence all output
    #[structopt(short = "q", long = "quiet")]
    pub quiet: bool,
    /// Verbose mode (-v, -vv, -vvv, etc)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    pub verbose: usize,
    /// URL for logging over TCP
    #[structopt(long = "log-url")]
    pub log_url: Option<String>,
    /// User IP address for logging context
    #[structopt(long = "ip")]
    pub ip: Option<String>,
    /// Username for logging context
    #[structopt(long = "user")]
    pub user: Option<String>,
    /// Repository name for logging context
    #[structopt(long = "repo")]
    pub repo: Option<String>,
}

pub struct Logger {
    tcp_stream: Option<Mutex<TcpStream>>,
    context: LoggingContext,
}

impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        eprintln!("{}", record.args());

        self.tcp_stream.as_ref().map(|ref stream| {
            stream
                .lock()
                .map_err(|e| e.to_string())
                .and_then(|mut stream| {
                    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
                    let message = LogMessage {
                        timestamp: timestamp.to_string(),
                        level: record.level(),
                        context: self.context.clone(),
                        message: format!("{}", record.args()),
                    };
                    let message_json =
                        serde_json::to_string(&message).map_err(|e| e.to_string())?;
                    writeln!(stream, "{}", message_json).map_err(|e| e.to_string())
                })
                .unwrap_or_else(|e| eprintln!("Error: Failed to log over TCP - {}", e));
        });
    }

    fn flush(&self) {
        std::io::stderr()
            .flush()
            .unwrap_or_else(|e| eprintln!("Error: Failed to flush StdErr logging stream - {}", e));

        self.tcp_stream.as_ref().map(|ref stream| {
            stream
                .lock()
                .map_err(|e| e.to_string())
                .and_then(|mut stream| stream.flush().map_err(|e| e.to_string()))
                .unwrap_or_else(|e| eprintln!("Error: Failed to flush TCP logging stream - {}", e));
        });
    }
}

impl Logger {
    pub fn init(opt: LoggingOpt) {
        let log_level = match (opt.quiet, opt.verbose) {
            (true, _) => LevelFilter::Off,
            (false, 0) => LevelFilter::Info,
            (false, 1) => LevelFilter::Debug,
            (false, _) => LevelFilter::Trace,
        };

        log::set_max_level(log_level);
        let context = LoggingContext {
            run_id: Uuid::new_v4(),
            user_id: opt.user,
            user_ip: opt.ip,
            repo: opt.repo,
        };

        let tcp_stream = opt
            .log_url
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
            context,
        }));
        if log_set_result.is_err() {
            eprintln!("Error: Logger initialized twice!");
        }
    }

    pub fn test_init() {
        Logger::init(LoggingOpt {
            quiet: false,
            verbose: 2,
            log_url: None,
            user: None,
            ip: None,
            repo: None,
        });
    }
}

pub fn print_header(text: impl Display, quiet: bool) {
    if !quiet {
        let seperator =
            "********************************************************************************";
        println!("\n{0}\n{1}\n{0}\n", seperator, text);
    }
}

#[derive(Serialize)]
struct LogMessage {
    timestamp: String,
    level: Level,
    context: LoggingContext,
    message: String,
}

#[derive(Serialize, Clone)]
pub struct LoggingContext {
    run_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo: Option<String>,
}
