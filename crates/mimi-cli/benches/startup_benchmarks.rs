use clap::Parser;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mimi_cli::cli::commands::Cli;

fn bench_cli_parse(c: &mut Criterion) {
    c.bench_function("cli_parse_version", |b| {
        b.iter(|| {
            let cli = Cli::try_parse_from(["mimi", "--version"]).unwrap();
            black_box(cli);
        });
    });

    c.bench_function("cli_parse_query", |b| {
        b.iter(|| {
            let cli = Cli::try_parse_from([
                "mimi",
                "-v",
                "query",
                "What is the current system status?",
                "--limit",
                "10",
            ])
            .unwrap();
            black_box(cli);
        });
    });

    c.bench_function("cli_parse_exec", |b| {
        b.iter(|| {
            let cli = Cli::try_parse_from([
                "mimi",
                "exec",
                "run diagnostics",
                "--priority",
                "high",
                "--timeout",
                "120",
            ])
            .unwrap();
            black_box(cli);
        });
    });

    c.bench_function("cli_parse_config_list", |b| {
        b.iter(|| {
            let cli = Cli::try_parse_from(["mimi", "config", "list"]).unwrap();
            black_box(cli);
        });
    });
}

criterion_group!(benches, bench_cli_parse);
criterion_main!(benches);
