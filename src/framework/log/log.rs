use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer, Registry};
use tracing_subscriber::filter::{LevelFilter, FilterFn};
use crate::framework::config::config::Log as LogConfig;
use tracing::{error, Level, Metadata};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogOutput {
    Terminal,
    File,
    Both,
}

impl LogOutput {
    pub fn from_config_str(config: &str) -> Self {
        match config.to_ascii_lowercase().as_str() {
            s if s.contains("terminal") && s.contains("file") => LogOutput::Both,
            s if s.contains("file") => LogOutput::File,
            _ => LogOutput::Terminal,
        }
    }
    
    pub fn needs_file(&self) -> bool {
        matches!(self, LogOutput::File | LogOutput::Both)
    }
    
    pub fn needs_terminal(&self) -> bool {
        matches!(self, LogOutput::Terminal | LogOutput::Both)
    }
}

#[derive(Debug, Clone)]
pub struct LogSettings {
    pub debug: LogOutput,
    pub info: LogOutput,
    pub net: LogOutput,
    pub warn: LogOutput,
    pub err: LogOutput,
}

impl From<&LogConfig> for LogSettings {
    fn from(config: &LogConfig) -> Self {
        Self {
            debug: LogOutput::from_config_str(&config.debug),
            info: LogOutput::from_config_str(&config.info),
            net: LogOutput::from_config_str(&config.net),
            warn: LogOutput::from_config_str(&config.warn),
            err: LogOutput::from_config_str(&config.err),
        }
    }
}


// Log guard holds the non-blocking writer guards to keep background threads alive
pub struct LogGuard {
    _file_guards: Vec<WorkerGuard>,
}

pub struct LogManager;

impl LogManager {
    // Create terminal filter from settings
    fn create_terminal_filter(settings: LogSettings) -> impl Fn(&Metadata) -> bool {
        move |metadata: &Metadata| {
            match *metadata.level() {
                Level::ERROR => matches!(settings.err, LogOutput::Terminal | LogOutput::Both),
                Level::WARN => matches!(settings.warn, LogOutput::Terminal | LogOutput::Both),
                Level::INFO => matches!(settings.info, LogOutput::Terminal | LogOutput::Both) ||
                              matches!(settings.net, LogOutput::Terminal | LogOutput::Both),
                Level::DEBUG => matches!(settings.debug, LogOutput::Terminal | LogOutput::Both),
                Level::TRACE => false,
            }
        }
    }
    
    // Create file filter from settings
    fn create_file_filter(settings: LogSettings) -> impl Fn(&Metadata) -> bool {
        move |metadata: &Metadata| {
            match *metadata.level() {
                Level::ERROR => matches!(settings.err, LogOutput::File | LogOutput::Both),
                Level::WARN => matches!(settings.warn, LogOutput::File | LogOutput::Both),
                Level::INFO => matches!(settings.info, LogOutput::File | LogOutput::Both) ||
                              matches!(settings.net, LogOutput::File | LogOutput::Both),
                Level::DEBUG => matches!(settings.debug, LogOutput::File | LogOutput::Both),
                Level::TRACE => false,
            }
        }
    }
    
    pub fn init_logger(
        log_config: &LogConfig,
        server_name: String,
        server_id: u32,
        log_base_path: Option<PathBuf>,
    ) -> (bool, Option<LogGuard>) {
        let settings = LogSettings::from(log_config);
        let mut file_guards = Vec::new();
        
        // Determine the maximum log level needed
        let max_level = if matches!(settings.debug, LogOutput::Terminal | LogOutput::File | LogOutput::Both) {
            LevelFilter::DEBUG
        } else if matches!(settings.info, LogOutput::Terminal | LogOutput::File | LogOutput::Both) ||
                  matches!(settings.net, LogOutput::Terminal | LogOutput::File | LogOutput::Both) {
            LevelFilter::INFO
        } else if matches!(settings.warn, LogOutput::Terminal | LogOutput::File | LogOutput::Both) {
            LevelFilter::WARN
        } else if matches!(settings.err, LogOutput::Terminal | LogOutput::File | LogOutput::Both) {
            LevelFilter::ERROR
        } else {
            LevelFilter::OFF
        };
        
        // Check if any level needs terminal output
        let needs_terminal = settings.debug.needs_terminal() ||
                            settings.info.needs_terminal() ||
                            settings.net.needs_terminal() ||
                            settings.warn.needs_terminal() ||
                            settings.err.needs_terminal();
        
        // Check if any level needs file output
        let needs_file = settings.debug.needs_file() ||
                        settings.info.needs_file() ||
                        settings.net.needs_file() ||
                        settings.warn.needs_file() ||
                        settings.err.needs_file();
        
        // Build the subscriber based on what outputs are needed
        if needs_terminal && needs_file {
            // Both terminal and file output
            let base_path = log_base_path.unwrap_or_else(|| PathBuf::from("logs"));
            let log_dir = base_path.join(&server_name);
            
            // Create log directory
            if let Err(e) = std::fs::create_dir_all(&log_dir) {
                error!("Failed to create log directory: {}", e);
                return (false, None);
            }
            
            // Create file appender with rotation
            let filename = format!("{}_{:03}", server_name, server_id);
            let file_appender = tracing_appender::rolling::daily(log_dir, filename);
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            file_guards.push(guard);
            
            let terminal_filter = Self::create_terminal_filter(settings.clone());
            let file_filter = Self::create_file_filter(settings);
            
            let terminal_layer = tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_writer(std::io::stdout);
            
            let file_layer = tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_ansi(false)  // Disable ANSI colors for file output
                .with_writer(non_blocking);
            
            let subscriber = Registry::default()
                .with(max_level)
                .with(terminal_layer)
                .with(file_layer);
            
            subscriber.init();
        } else if needs_terminal {
            // Only terminal output
            let terminal_filter = Self::create_terminal_filter(settings);
            let terminal_layer = tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_writer(std::io::stdout);
            
            let subscriber = Registry::default()
                .with(max_level)
                .with(terminal_layer);
            
            subscriber.init();
        } else if needs_file {
            // Only file output
            let base_path = log_base_path.unwrap_or_else(|| PathBuf::from("logs"));
            let log_dir = base_path.join(&server_name);
            
            // Create log directory
            if let Err(e) = std::fs::create_dir_all(&log_dir) {
                error!("Failed to create log directory: {}", e);
                return (false, None);
            }
            
            // Create file appender with rotation
            let filename = format!("{}_{:03}", server_name, server_id);
            let file_appender = tracing_appender::rolling::daily(log_dir, filename);
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            file_guards.push(guard);
            
            let file_filter = Self::create_file_filter(settings);
            let file_layer = tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_ansi(false)  // Disable ANSI colors for file output
                .with_writer(non_blocking);
            
            let subscriber = Registry::default()
                .with(max_level)
                .with(file_layer);
            
            subscriber.init();
        } else {
            // No output configured
            return (false, None);
        }
        
        (true, Some(LogGuard {
            _file_guards: file_guards,
        }))
    }
}