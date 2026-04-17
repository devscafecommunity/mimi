use crate::cli::{Commands, GlobalOpts, Formatter};
use crate::cli::error::{CliError, CliResult, EXIT_SUCCESS};

pub struct CommandHandler {
    global_opts: GlobalOpts,
    formatter: Formatter,
}

impl CommandHandler {
    pub fn new(global_opts: GlobalOpts) -> Self {
        let formatter = Formatter::new(global_opts.output, global_opts.no_color);
        CommandHandler {
            global_opts,
            formatter,
        }
    }

    pub async fn handle(&self, _command: Option<Commands>) -> CliResult<i32> {
        Ok(EXIT_SUCCESS)
    }
}
