use mimi_cli::cli::{Cli, CommandHandler};
use mimi_cli::init_logging;
use clap::Parser;
use std::process;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(
        cli.global.verbose,
        cli.global.log_level.as_deref(),
        cli.global.no_color,
    );

    // Create handler
    let handler = CommandHandler::new(cli.global);

    // Execute command
    match handler.handle(cli.command).await {
        Ok(exit_code) => {
            process::exit(exit_code);
        }
        Err(e) => {
            eprintln!("{}", e.user_message());
            process::exit(e.exit_code());
        }
    }
}
