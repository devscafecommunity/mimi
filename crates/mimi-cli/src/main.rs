use clap::{Parser, Subcommand};
use tracing::info;

mod repl;

#[derive(Parser)]
#[command(name = "mimi")]
#[command(about = "MiMi - Multimodal Instruction Master Interface", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Run the MiMi system")]
    Run {
        #[arg(short, long)]
        config: Option<String>,
    },
    #[command(about = "Start interactive REPL mode")]
    Repl {
        #[arg(long)]
        history_file: Option<String>,

        #[arg(long)]
        max_history: Option<usize>,

        #[arg(long)]
        no_history: bool,

        #[arg(long)]
        startup_script: Option<String>,
    },
    #[command(about = "Show version information")]
    Version,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let cli = Cli::parse();

    if cli.verbose {
        info!("Verbose mode enabled");
    }

    match cli.command {
        Some(Commands::Run { config }) => {
            info!("Starting MiMi system");
            if let Some(config_path) = config {
                info!("Using config: {}", config_path);
            }
        },
        Some(Commands::Repl {
            history_file,
            max_history,
            no_history,
            startup_script,
        }) => {
            let repl_config = repl::ReplConfig {
                history_file: history_file.unwrap_or_else(|| {
                    format!(
                        "{}/.mimi/repl_history",
                        std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
                    )
                }),
                max_history: max_history.unwrap_or(1000),
                no_history,
                startup_script,
                completion: true,
            };

            if let Err(e) = repl::run_repl(repl_config).await {
                eprintln!("REPL error: {}", e);
            }
        },
        Some(Commands::Version) => {
            println!("mimi {}", env!("CARGO_PKG_VERSION"));
        },
        None => {
            println!("MiMi v{}", env!("CARGO_PKG_VERSION"));
            println!("Use --help for usage information");
        },
    }

    Ok(())
}
