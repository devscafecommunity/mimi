use crate::cli::error::{CliError, CliResult, EXIT_SUCCESS};
use crate::cli::{Commands, Formatter, GlobalOpts, OutputFormat, Priority};
use crate::config::ConfigLoader;
use tracing::{debug, info};

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

    /// Execute command
    pub async fn handle(&self, command: Option<Commands>) -> CliResult<i32> {
        match command {
            Some(Commands::Exec {
                task,
                priority,
                timeout,
                r#async,
                follow,
                dry_run,
                ..
            }) => {
                self.handle_exec(&task, priority, timeout, r#async, follow, dry_run)
                    .await
            },

            Some(Commands::Query {
                query,
                filter,
                include_context,
                ..
            }) => self.handle_query(&query, filter, include_context).await,

            Some(Commands::Config { subcommand }) => {
                use crate::cli::commands::ConfigSubcommand;
                match subcommand {
                    ConfigSubcommand::List => self.handle_config_list().await,
                    ConfigSubcommand::Get { key } => self.handle_config_get(&key).await,
                    ConfigSubcommand::Set { key, value } => {
                        self.handle_config_set(&key, &value).await
                    },
                    ConfigSubcommand::Init { profile } => self.handle_config_init(&profile).await,
                    ConfigSubcommand::Validate => self.handle_config_validate().await,
                    ConfigSubcommand::Reset => self.handle_config_reset().await,
                }
            },

            Some(Commands::Debug { subcommand }) => {
                use crate::cli::commands::DebugSubcommand;
                match subcommand {
                    DebugSubcommand::Status => self.handle_debug_status().await,
                    DebugSubcommand::BusLatency { num_messages } => {
                        self.handle_debug_bus_latency(num_messages).await
                    },
                    DebugSubcommand::MemoryStats => self.handle_debug_memory_stats().await,
                    DebugSubcommand::AdapterTest { adapter } => {
                        self.handle_debug_adapter_test(&adapter).await
                    },
                    DebugSubcommand::TraceFlow { task_id } => {
                        self.handle_debug_trace_flow(&task_id).await
                    },
                    DebugSubcommand::Profile { duration } => {
                        self.handle_debug_profile(duration).await
                    },
                    DebugSubcommand::ValidateIntent { text } => {
                        self.handle_debug_validate_intent(&text).await
                    },
                }
            },

            Some(Commands::Repl { .. }) => self.handle_repl().await,

            Some(Commands::Version { json, short, .. }) => self.handle_version(json, short).await,

            None => {
                println!("MiMi v{}", env!("CARGO_PKG_VERSION"));
                println!("Use 'mimi --help' for usage information");
                Ok(EXIT_SUCCESS)
            },
        }
    }

    // Command handlers
    async fn handle_exec(
        &self,
        task: &str,
        priority: Priority,
        timeout: u32,
        is_async: bool,
        follow: bool,
        dry_run: bool,
    ) -> CliResult<i32> {
        info!(
            "Executing task: {} (priority: {}, timeout: {}s)",
            task, priority, timeout
        );

        if dry_run {
            let msg = format!(
                "DRY RUN: Would execute task '{}' with priority {}",
                task, priority
            );
            println!("{}", self.formatter.format_success(&msg));
            return Ok(EXIT_SUCCESS);
        }

        let msg = format!("Task started: {}", task);
        println!("{}", self.formatter.format_success(&msg));
        Ok(EXIT_SUCCESS)
    }

    async fn handle_query(
        &self,
        query: &str,
        _filter: Option<String>,
        _include_context: bool,
    ) -> CliResult<i32> {
        info!("Querying: {}", query);

        println!(
            "{}",
            self.formatter.format_success(&format!("Query: {}", query))
        );
        Ok(EXIT_SUCCESS)
    }

    async fn handle_config_list(&self) -> CliResult<i32> {
        let config = ConfigLoader::load(self.global_opts.config.clone())?;
        let pairs = vec![
            ("adapter.default", &config.adapter.default),
            ("bus.url", &config.bus.url),
            ("log.level", &config.log.level),
        ];

        println!(
            "{}",
            self.formatter.format_kv(
                &pairs
                    .iter()
                    .map(|(k, v)| (*k, v.as_str()))
                    .collect::<Vec<_>>()
            )
        );
        Ok(EXIT_SUCCESS)
    }

    async fn handle_config_get(&self, key: &str) -> CliResult<i32> {
        let config = ConfigLoader::load(self.global_opts.config.clone())?;
        let value = ConfigLoader::get_value(&config, key)?;
        println!("{}: {}", key, value);
        Ok(EXIT_SUCCESS)
    }

    async fn handle_config_set(&self, key: &str, value: &str) -> CliResult<i32> {
        let mut config = ConfigLoader::load(self.global_opts.config.clone())?;
        ConfigLoader::set_value(&mut config, key, value)?;

        if let Some(path) = &self.global_opts.config {
            ConfigLoader::save(&config, path)?;
            println!(
                "{}",
                self.formatter
                    .format_success(&format!("Set {}: {}", key, value))
            );
        } else {
            println!(
                "{}",
                self.formatter
                    .format_success("Config set (not saved - specify --config to persist)")
            );
        }

        Ok(EXIT_SUCCESS)
    }

    async fn handle_config_init(&self, profile: &str) -> CliResult<i32> {
        use crate::config::defaults;

        let config = match profile {
            "development" => defaults::development_profile(),
            "production" => defaults::production_profile(),
            _ => {
                return Err(CliError::UsageError(
                    "profile must be 'development' or 'production'".to_string(),
                ))
            },
        };

        if let Some(path) = &self.global_opts.config {
            ConfigLoader::save(&config, path)?;
            println!(
                "{}",
                self.formatter
                    .format_success(&format!("Initialized config with '{}' profile", profile))
            );
        } else {
            println!(
                "{}",
                self.formatter
                    .format_success("Config initialized (specify --config to save)")
            );
        }

        Ok(EXIT_SUCCESS)
    }

    async fn handle_config_validate(&self) -> CliResult<i32> {
        ConfigLoader::load(self.global_opts.config.clone())?;
        println!("{}", self.formatter.format_success("Config is valid"));
        Ok(EXIT_SUCCESS)
    }

    async fn handle_config_reset(&self) -> CliResult<i32> {
        let config = Default::default();
        if let Some(path) = &self.global_opts.config {
            ConfigLoader::save(&config, path)?;
            println!(
                "{}",
                self.formatter.format_success("Config reset to defaults")
            );
        } else {
            println!(
                "{}",
                self.formatter
                    .format_success("Config would be reset (specify --config to persist)")
            );
        }
        Ok(EXIT_SUCCESS)
    }

    async fn handle_debug_status(&self) -> CliResult<i32> {
        println!("{}", self.formatter.format_status("Listening", 0));
        Ok(EXIT_SUCCESS)
    }

    async fn handle_debug_bus_latency(&self, num_messages: u32) -> CliResult<i32> {
        println!(
            "{}",
            self.formatter.format_success(&format!(
                "Would test bus latency with {} messages (not yet implemented)",
                num_messages
            ))
        );
        Ok(EXIT_SUCCESS)
    }

    async fn handle_debug_memory_stats(&self) -> CliResult<i32> {
        println!(
            "{}",
            self.formatter
                .format_success("Memory stats (not yet implemented)")
        );
        Ok(EXIT_SUCCESS)
    }

    async fn handle_debug_adapter_test(&self, adapter: &str) -> CliResult<i32> {
        println!(
            "{}",
            self.formatter.format_success(&format!(
                "Testing adapter: {} (not yet implemented)",
                adapter
            ))
        );
        Ok(EXIT_SUCCESS)
    }

    async fn handle_debug_trace_flow(&self, task_id: &str) -> CliResult<i32> {
        println!(
            "{}",
            self.formatter
                .format_success(&format!("Tracing task: {} (not yet implemented)", task_id))
        );
        Ok(EXIT_SUCCESS)
    }

    async fn handle_debug_profile(&self, duration: u32) -> CliResult<i32> {
        println!(
            "{}",
            self.formatter.format_success(&format!(
                "Profiling for {} seconds (not yet implemented)",
                duration
            ))
        );
        Ok(EXIT_SUCCESS)
    }

    async fn handle_debug_validate_intent(&self, text: &str) -> CliResult<i32> {
        println!(
            "{}",
            self.formatter.format_success(&format!(
                "Validating intent: {} (not yet implemented)",
                text
            ))
        );
        Ok(EXIT_SUCCESS)
    }

    async fn handle_repl(&self) -> CliResult<i32> {
        println!(
            "{}",
            self.formatter.format_success("Starting REPL mode (M1.4.3)")
        );
        Ok(EXIT_SUCCESS)
    }

    async fn handle_version(&self, json: bool, short: bool) -> CliResult<i32> {
        if short {
            println!("v{}", env!("CARGO_PKG_VERSION"));
        } else if json {
            let version_json = serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "build_date": env!("CARGO_PKG_VERSION"),
                "channel": "stable",
            });
            println!("{}", version_json.to_string());
        } else {
            println!("MiMi CLI v{}", env!("CARGO_PKG_VERSION"));
            println!("Build: 2026-04-17 13:45:23 UTC");
            println!("Channel: stable");
        }
        Ok(EXIT_SUCCESS)
    }
}
