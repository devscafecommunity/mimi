use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize logging based on verbosity level
pub fn init_logging(verbose: u8, log_level: Option<&str>, no_color: bool) {
    let level = if let Some(level) = log_level {
        level.to_string()
    } else {
        match verbose {
            0 => "info".to_string(),
            1 => "debug".to_string(),
            _ => "trace".to_string(),
        }
    };

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level))
        .add_directive("hyper=info".parse().unwrap())
        .add_directive("tokio=info".parse().unwrap());

    let fmt_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(verbose > 1)
        .with_file(verbose > 1)
        .with_line_number(verbose > 1)
        .with_ansi(!no_color);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();
}
