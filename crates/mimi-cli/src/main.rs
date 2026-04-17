use clap::{Parser, Subcommand};
use tracing::info;

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
