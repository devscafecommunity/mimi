use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// MiMi - Multimodal Instruction Master Interface
#[derive(Parser, Debug)]
#[command(name = "mimi")]
#[command(version = "0.1.0")]
#[command(about = "Multimodal Instruction Master Interface for autonomous task execution")]
#[command(author = "MiMi Team")]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOpts,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Global options available to all commands
#[derive(Parser, Debug)]
pub struct GlobalOpts {
    /// Increase verbosity level (-v for debug, -vv for trace)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress non-essential output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Configuration file path
    #[arg(short, long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Log level: trace, debug, info, warn, error
    #[arg(long, global = true, value_name = "LEVEL")]
    pub log_level: Option<String>,

    /// Output format: text, json, yaml
    #[arg(long, global = true, value_name = "FORMAT", default_value = "text")]
    pub output: OutputFormat,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,
}

/// Output formats
#[derive(ValueEnum, Clone, Debug, Copy)]
pub enum OutputFormat {
    /// Human-readable text output
    #[value(name = "text")]
    Text,
    /// JSON structured output
    #[value(name = "json")]
    Json,
    /// YAML formatted output
    #[value(name = "yaml")]
    Yaml,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Text => write!(f, "text"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Yaml => write!(f, "yaml"),
        }
    }
}

/// Task priority levels
#[derive(ValueEnum, Clone, Debug, Copy)]
pub enum Priority {
    #[value(name = "low")]
    Low,
    #[value(name = "normal")]
    Normal,
    #[value(name = "high")]
    High,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Low => write!(f, "low"),
            Priority::Normal => write!(f, "normal"),
            Priority::High => write!(f, "high"),
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Execute a task or skill
    Exec {
        /// Task description or command
        #[arg(value_name = "TASK")]
        task: String,

        /// Task priority
        #[arg(short, long, default_value = "normal")]
        priority: Priority,

        /// Task timeout in seconds
        #[arg(short, long, default_value = "300")]
        timeout: u32,

        /// Run asynchronously (return task ID)
        #[arg(long)]
        r#async: bool,

        /// Follow async task completion
        #[arg(long)]
        follow: bool,

        /// Save result to file
        #[arg(long)]
        save_result: Option<PathBuf>,

        /// Output format override
        #[arg(long)]
        format: Option<OutputFormat>,

        /// Preview task without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// Query MiMi memory and status
    Query {
        /// Query text
        #[arg(value_name = "QUERY")]
        query: String,

        /// Filter results (jq syntax)
        #[arg(short, long)]
        filter: Option<String>,

        /// Sort results by field
        #[arg(short, long)]
        sort: Option<String>,

        /// Limit number of results
        #[arg(long, default_value = "20")]
        limit: u32,

        /// Pagination offset
        #[arg(long, default_value = "0")]
        offset: u32,

        /// Include contextual information
        #[arg(long)]
        include_context: bool,

        /// Memory type: short, long, working, all
        #[arg(long, default_value = "all")]
        memory_type: String,

        /// Results from date (ISO 8601)
        #[arg(long)]
        from: Option<String>,

        /// Results until date (ISO 8601)
        #[arg(long)]
        to: Option<String>,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        subcommand: ConfigSubcommand,
    },

    /// Debugging and diagnostics
    Debug {
        #[command(subcommand)]
        subcommand: DebugSubcommand,
    },

    /// Start interactive REPL mode
    Repl {
        /// History file location
        #[arg(long)]
        history_file: Option<PathBuf>,

        /// Maximum history entries
        #[arg(long, default_value = "1000")]
        max_history: usize,

        /// Disable history persistence
        #[arg(long)]
        no_history: bool,

        /// Startup script to run
        #[arg(long)]
        startup_script: Option<PathBuf>,

        /// Enable command completion
        #[arg(long)]
        completion: bool,
    },

    /// Show version information
    Version {
        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Show short version only
        #[arg(long)]
        short: bool,

        /// Show full build details
        #[arg(long)]
        full: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigSubcommand {
    /// Set a configuration value
    Set {
        /// Config key (e.g., adapter.default)
        key: String,
        /// Config value
        value: String,
    },

    /// Get a configuration value
    Get {
        /// Config key
        key: String,
    },

    /// List all configuration values
    List,

    /// Reset to defaults
    Reset,

    /// Validate configuration file
    Validate,

    /// Initialize with preset profile
    Init {
        /// Profile: development, production
        profile: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum DebugSubcommand {
    /// Show system status and health
    Status,

    /// Test message bus latency
    BusLatency {
        /// Number of messages to send
        #[arg(value_name = "NUM")]
        num_messages: u32,
    },

    /// Show memory graph statistics
    MemoryStats,

    /// Test AI adapter connectivity
    AdapterTest {
        /// Adapter name: gemini, ollama
        adapter: String,
    },

    /// Trace execution flow of task
    TraceFlow {
        /// Task ID
        task_id: String,
    },

    /// CPU/memory profiling
    Profile {
        /// Duration in seconds
        duration: u32,
    },

    /// Test intent parsing
    ValidateIntent {
        /// Text to parse
        text: String,
    },
}
