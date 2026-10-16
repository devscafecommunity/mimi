use crate::ai::{
    adapter_to_cli, invoke_ai_with_fallback, invoke_prompt, resolve_adapter_config,
};
use std::sync::Arc;
use std::time::Duration;

use mimi_core::{
    circuit_breakers, connect_zenoh_if_requested, flush_with_retry, intervention_context,
    odlaguna_deadline, pandora_context_with_fallback, run_daemon, run_isolated_skill_supervised,
    skill_run_succeeded, start_scoped_odlaguna_supervisor, CritiqueCommentaryMessage,
    IsolatedSkillPayload, OdlagunaWatchdog, PandoraClient, PriscillaAdvisor, PriscillaConfig,
    TaskDraftMessage,
};
use uuid::Uuid;
use crate::cli::error::{CliError, CliResult, EXIT_SUCCESS};
use crate::cli::{Commands, Formatter, GlobalOpts, OutputFormat, Priority};
use crate::config::ConfigLoader;
use tracing::info;

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
                limit,
                ..
            }) => self.handle_query(&query, filter, include_context, limit).await,

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
                    DebugSubcommand::SkillRun {
                        image,
                        command,
                        timeout,
                    } => self.handle_debug_skill_run(&image, &command, timeout).await,
                    DebugSubcommand::CircuitStats => self.handle_debug_circuit_stats().await,
                    DebugSubcommand::PriscillaCritique {
                        text,
                        tokens,
                        skill_id,
                        with_patterns,
                        pattern_window,
                    } => self
                        .handle_debug_priscilla_critique(
                            &text,
                            tokens,
                            skill_id,
                            with_patterns,
                            pattern_window,
                        )
                        .await,
                }
            },

            Some(Commands::Daemon) => self.handle_daemon().await,

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
        timeout_secs: u32,
        _is_async: bool,
        _follow: bool,
        dry_run: bool,
    ) -> CliResult<i32> {
        info!(
            "Executing task: {} (priority: {}, timeout: {}s)",
            task, priority, timeout_secs
        );

        if dry_run {
            let msg = format!(
                "DRY RUN: Would execute task '{}' with priority {}",
                task, priority
            );
            println!("{}", self.formatter.format_success(&msg));
            return Ok(EXIT_SUCCESS);
        }

        let file_config = ConfigLoader::load(self.global_opts.config.clone())?;
        let adapter_config = resolve_adapter_config(&file_config).map_err(adapter_to_cli)?;

        let task_id = Uuid::new_v4().to_string();
        let timeout_dur = Duration::from_secs(timeout_secs.max(1) as u64);
        let mut watchdog = OdlagunaWatchdog::new();

        let breakers = circuit_breakers();

        let pandora = Arc::new(
            PandoraClient::new()
                .await
                .map_err(|e| CliError::ExecutionError(e.to_string()))?,
        );

        let zenoh_bus = connect_zenoh_if_requested()
            .await
            .map_err(|e| CliError::ExecutionError(e.to_string()))?;
        // No background supervisor on plain exec — local `odlaguna_deadline` is enough and
        // an infinite tokio task would keep the CLI process alive after the task finishes.

        let intervention = intervention_context(pandora.as_ref(), zenoh_bus.as_deref());

        let exec_out = odlaguna_deadline(
            &mut watchdog,
            &task_id,
            timeout_dur,
            None,
            intervention,
            || async {
                let context_bundle =
                    pandora_context_with_fallback(&breakers.pandora, &pandora, task, 10, 2).await;

                let system_context = context_bundle.as_ref().map(|b| b.prompt_block.clone());

                let response = invoke_ai_with_fallback(
                    &breakers.ai,
                    &file_config,
                    &adapter_config,
                    task,
                    system_context,
                )
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

                let _ = flush_with_retry(pandora.as_ref(), 2).await;

                Ok((response, context_bundle))
            },
        )
        .await;

        circuit_breakers().check_alerts();

        match exec_out {
            Ok((response, Some(bundle))) => {
                self.print_adapter_response(&response, Some(&bundle))?;
                Ok(EXIT_SUCCESS)
            },
            Ok((response, None)) => {
                self.print_adapter_response(&response, None)?;
                Ok(EXIT_SUCCESS)
            },
            Err(mimi_core::OdlagunaDeadlineError::Timeout) => Err(CliError::ExecutionError(
                format!("task exceeded Odlaguna deadline ({}s)", timeout_secs),
            )),
            Err(mimi_core::OdlagunaDeadlineError::Operation(e)) => {
                Err(CliError::ExecutionError(e.to_string()))
            },
        }
    }

    async fn handle_query(
        &self,
        query: &str,
        _filter: Option<String>,
        include_context: bool,
        limit: u32,
    ) -> CliResult<i32> {
        info!("Querying: {} (include_context={})", query, include_context);

        let file_config = ConfigLoader::load(self.global_opts.config.clone())?;
        let adapter_config = resolve_adapter_config(&file_config).map_err(adapter_to_cli)?;

        let breakers = circuit_breakers();

        let (context_bundle, pandora) = if include_context {
            let pandora = PandoraClient::new()
                .await
                .map_err(|e| CliError::ExecutionError(e.to_string()))?;
            let bundle = pandora_context_with_fallback(
                &breakers.pandora,
                &pandora,
                query,
                limit as usize,
                2,
            )
            .await;
            (bundle, Some(pandora))
        } else {
            (None, None)
        };

        let system_context = context_bundle.as_ref().map(|b| b.prompt_block.clone());

        let response = invoke_ai_with_fallback(
            &breakers.ai,
            &file_config,
            &adapter_config,
            query,
            system_context,
        )
        .await
        .map_err(adapter_to_cli)?;

        if let Some(pandora) = pandora {
            let _ = flush_with_retry(&pandora, 2).await;
        }

        circuit_breakers().check_alerts();

        self.print_adapter_response(&response, context_bundle.as_ref())?;
        Ok(EXIT_SUCCESS)
    }

    fn print_adapter_response(
        &self,
        response: &crate::ai::adapter::AiResponse,
        context: Option<&mimi_core::ContextBundle>,
    ) -> CliResult<()> {
        match self.global_opts.output {
            OutputFormat::Json => {
                let mut payload = serde_json::json!({
                    "content": response.content,
                    "model": response.model,
                    "tokens_used": response.tokens_used,
                    "cached": response.cached,
                });
                if let Some(bundle) = context {
                    payload["context"] = serde_json::json!({
                        "query_context_id": bundle.query_context_id,
                        "hot_count": bundle.hot.len(),
                        "expanded_count": bundle.expanded.len(),
                        "hot": bundle.hot,
                        "expanded": bundle.expanded,
                    });
                }
                println!("{}", serde_json::to_string_pretty(&payload).map_err(|e| {
                    CliError::UnknownError(format!("failed to encode JSON: {}", e))
                })?);
            },
            OutputFormat::Yaml => {
                println!("content: {}", response.content);
                println!("model: {}", response.model);
                println!("tokens_used: {}", response.tokens_used);
                println!("cached: {}", response.cached);
            },
            OutputFormat::Text => {
                println!("{}", response.content);
            },
        }
        Ok(())
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
        let pandora = PandoraClient::new()
            .await
            .map_err(|e| CliError::ExecutionError(e.to_string()))?;

        let stats = pandora.l1_cache_stats().await;
        let heatmap_len = pandora.heatmap_handle().lock().await.len();

        match self.global_opts.output {
            OutputFormat::Json => {
                let hit_rate = stats.hit_rate();
                let payload = serde_json::json!({
                    "l1_cache": {
                        "hits": stats.hits,
                        "misses": stats.misses,
                        "evictions": stats.evictions,
                        "hit_rate": hit_rate,
                        "m2_target_met": hit_rate >= mimi_core::M2_L1_HIT_RATE_TARGET
                            || (stats.hits + stats.misses) == 0,
                    },
                    "heatmap_nodes": heatmap_len,
                    "neo4j_connected": pandora.is_neo4j_connected(),
                });
                println!("{}", serde_json::to_string_pretty(&payload).unwrap());
            },
            _ => {
                let hit_rate = stats.hit_rate();
                println!(
                    "L1 cache: hits={} misses={} evictions={} hit_rate={:.1}% (M2 target {:.0}%)",
                    stats.hits,
                    stats.misses,
                    stats.evictions,
                    hit_rate * 100.0,
                    mimi_core::M2_L1_HIT_RATE_TARGET * 100.0
                );
                println!("Heatmap nodes in memory: {}", heatmap_len);
                println!(
                    "Neo4j: {}",
                    if pandora.is_neo4j_connected() {
                        "connected"
                    } else {
                        "offline (L1 + heatmap only)"
                    }
                );
            },
        }

        Ok(EXIT_SUCCESS)
    }

    async fn handle_debug_adapter_test(&self, adapter: &str) -> CliResult<i32> {
        let file_config = ConfigLoader::load(self.global_opts.config.clone())?;
        let mut adapter_config = resolve_adapter_config(&file_config).map_err(adapter_to_cli)?;
        adapter_config.adapter_type = adapter.to_string();
        adapter_config.validate().map_err(adapter_to_cli)?;

        let shared = crate::ai::AdapterFactory::create(&adapter_config)
            .await
            .map_err(adapter_to_cli)?;
        {
            let guard = shared.lock().await;
            guard.health_check().await.map_err(adapter_to_cli)?;
        }

        let response = invoke_prompt(
            &adapter_config,
            "Respond with exactly one word: ready",
        )
        .await
        .map_err(adapter_to_cli)?;

        println!(
            "{}",
            self.formatter.format_success(&format!(
                "Adapter '{}' OK — model {} ({} tokens)",
                adapter,
                response.model,
                response.tokens_used
            ))
        );
        println!("{}", response.content);
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

    async fn handle_debug_circuit_stats(&self) -> CliResult<i32> {
        let snaps = circuit_breakers().snapshots();
        match self.global_opts.output {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&snaps).unwrap());
            },
            _ => {
                println!("Pandora breaker: {:?}", snaps.pandora);
                println!("AI breaker: {:?}", snaps.ai);
            },
        }
        Ok(EXIT_SUCCESS)
    }

    async fn handle_debug_skill_run(
        &self,
        image: &str,
        command: &[String],
        timeout_secs: u32,
    ) -> CliResult<i32> {
        let task_id = Uuid::new_v4().to_string();
        let timeout = Duration::from_secs(timeout_secs.max(1) as u64);
        let mut watchdog = OdlagunaWatchdog::new();

        let pandora = Arc::new(
            PandoraClient::new()
                .await
                .map_err(|e| CliError::ExecutionError(e.to_string()))?,
        );
        let zenoh_bus = connect_zenoh_if_requested()
            .await
            .map_err(|e| CliError::ExecutionError(e.to_string()))?;
        let _odlaguna_supervisor = start_scoped_odlaguna_supervisor(pandora.clone(), zenoh_bus.clone())
            .map_err(|e| CliError::ExecutionError(e.to_string()))?;
        let intervention = intervention_context(pandora.as_ref(), zenoh_bus.as_deref());

        let spec = IsolatedSkillPayload {
            image: image.to_string(),
            command: command.to_vec(),
            memory_mb: 256,
            cpu_percent: 50,
            network_isolated: true,
        };

        let result = run_isolated_skill_supervised(
            &mut watchdog,
            &task_id,
            spec,
            timeout,
            intervention,
        )
            .await
            .map_err(|e| CliError::ExecutionError(e.to_string()))?;

        skill_run_succeeded(&result).map_err(|e| CliError::ExecutionError(e.to_string()))?;

        match self.global_opts.output {
            OutputFormat::Json => {
                let payload = serde_json::json!({
                    "container_id": result.container_id,
                    "status": result.status,
                    "exit_code": result.exit_code,
                    "logs": result.logs,
                });
                println!("{}", serde_json::to_string_pretty(&payload).unwrap());
            },
            _ => {
                println!(
                    "{}",
                    self.formatter.format_success(&format!(
                        "Skill run ok (container_id={}, exit={:?})",
                        result.container_id,
                        result.exit_code
                    ))
                );
                if !result.logs.is_empty() {
                    println!("--- logs ---\n{}", result.logs);
                }
            },
        }

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

    async fn handle_daemon(&self) -> CliResult<i32> {
        run_daemon()
            .await
            .map_err(|e| CliError::ExecutionError(e.to_string()))?;
        Ok(EXIT_SUCCESS)
    }

    async fn handle_debug_priscilla_critique(
        &self,
        text: &str,
        tokens: u32,
        skill_id: Option<String>,
        with_patterns: bool,
        pattern_window: u32,
    ) -> CliResult<i32> {
        let task_id = Uuid::new_v4().to_string();
        let draft = TaskDraftMessage {
            task_id,
            intent: text.to_string(),
            estimated_tokens: tokens,
            skill_id,
        };
        let advisor = PriscillaAdvisor::new(PriscillaConfig::default());
        let critique: CritiqueCommentaryMessage = if with_patterns {
            let pandora = PandoraClient::new()
                .await
                .map_err(|e| CliError::ExecutionError(e.to_string()))?;
            advisor
                .critique_with_pandora(&draft, &pandora, pattern_window)
                .await
        } else {
            advisor.critique(&draft)
        };

        match self.global_opts.output {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&critique).unwrap());
            },
            _ => {
                println!(
                    "{}",
                    self.formatter.format_success(&format!(
                        "Priscilla critique for task {} (necessity={:.2}, cost={:.2})",
                        critique.task_id,
                        critique.necessity_score,
                        critique.cost_score
                    ))
                );
                if !critique.concerns.is_empty() {
                    println!("Concerns:");
                    for c in &critique.concerns {
                        println!("  - {}", c);
                    }
                }
                if !critique.suggestions.is_empty() {
                    println!("Suggestions:");
                    for s in &critique.suggestions {
                        println!("  - {}", s);
                    }
                }
            },
        }

        Ok(EXIT_SUCCESS)
    }
}
