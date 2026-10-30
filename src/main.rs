use clap::{Parser, Subcommand};
use proven::arena::TriAgentArena;
use proven::ast_graph::{
    extract_scoped_context_with_options, scan_repository_changes,
    scan_repository_changes_with_options,
};
use proven::benchmark::BenchmarkEngine;
use proven::graph_rag::CodeGraphRAG;
use proven::memory_cls::GraphMemoryManager;
use proven::mutation_gate::MicroMutationGate;
use std::path::Path;

fn print_json<T: serde::Serialize>(val: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(val).unwrap_or_else(|_| "{}".to_string())
    );
}

#[derive(Parser)]
#[command(
    name = "proven",
    version,
    about = "AST context extraction, call-graph analysis, and mutation testing CLI for coding agents"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan repository diff/files and list untested functions
    Scan {
        #[arg(long, alias = "repo", default_value = ".")]
        path: String,
        #[arg(long, default_value = "main")]
        base: String,
        #[arg(long, default_value = "HEAD")]
        head: String,
        #[arg(long)]
        fan_out: bool,
        #[arg(long)]
        json: bool,
    },
    /// Refresh the repository AST topology and runtime verification map
    Refresh {
        #[arg(long, alias = "repo", default_value = ".")]
        path: String,
        #[arg(long, default_value = "main")]
        base: String,
        #[arg(long, default_value = "HEAD")]
        head: String,
        #[arg(long)]
        json: bool,
    },
    /// Extract scoped AST context, call-graph attention, and failure memory for a target
    Context {
        #[arg(long, alias = "targets")]
        target: Option<String>,
        #[arg(long)]
        file: Option<String>,
        #[arg(long, alias = "repo", default_value = ".")]
        path: String,
        #[arg(long)]
        blind: bool,
        #[arg(long)]
        spec: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Run target tests and AST mutation verification
    VerifyProof {
        #[arg(long, alias = "targets")]
        target: String,
        #[arg(long)]
        test: String,
        #[arg(long)]
        file: Option<String>,
        #[arg(long, alias = "repo", default_value = ".")]
        path: String,
        #[arg(long)]
        mutation_check: bool,
        #[arg(long, default_value_t = 3)]
        max_mutants: usize,
        #[arg(long)]
        pytest_args: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Explain multi-hop topological attention (GraphRAG) for a target symbol
    ExplainAttention {
        #[arg(long)]
        target: String,
        #[arg(long, default_value_t = 3)]
        hops: usize,
        #[arg(long, default_value_t = 5)]
        top_k: usize,
        #[arg(long, default_value = "topological")]
        mode: String,
        #[arg(long, alias = "repo", default_value = ".")]
        path: String,
        #[arg(long)]
        visualize: bool,
        #[arg(long)]
        json: bool,
    },
    /// Predict change impact blast radius
    Impact {
        #[arg(long)]
        target: String,
        #[arg(long, default_value = "deterministic")]
        mode: String,
        #[arg(long, alias = "repo", default_value = ".")]
        path: String,
        #[arg(long)]
        json: bool,
    },
    /// Deterministic Graph Bug Triage & Reviewer Recommendation
    Triage {
        #[arg(allow_hyphen_values = true)]
        query_pos: Option<String>,
        #[arg(long = "query", allow_hyphen_values = true)]
        query_flag: Option<String>,
        #[arg(long, default_value_t = 3)]
        max_devs: usize,
        #[arg(long, default_value_t = 5)]
        max_files: usize,
        #[arg(long, alias = "repo", default_value = ".")]
        path: String,
        #[arg(long)]
        json: bool,
    },
    /// Referee a Tri-Agent Adversarial Arena round (Blue vs 3 Red Personas)
    Arena {
        #[arg(long)]
        target: String,
        #[arg(long)]
        test: String,
        #[arg(long, default_value_t = 1)]
        round: usize,
        #[arg(long)]
        mutation_check: bool,
        #[arg(long)]
        pytest_args: Option<String>,
        #[arg(long, alias = "repo", default_value = ".")]
        path: String,
        #[arg(long)]
        json: bool,
    },
    /// Run Neocortex Sleep Consolidation to synthesize lessons into axioms & Meta-Axioms
    Sleep {
        #[arg(long, alias = "repo", default_value = ".")]
        path: String,
        #[arg(long, default_value_t = 10)]
        threshold: usize,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        meta: bool,
        #[arg(long)]
        json: bool,
    },
    /// Manage CLS Hierarchical Memory Graph
    Memory {
        #[arg(long, alias = "repo")]
        path: Option<String>,
        #[command(subcommand)]
        sub: MemorySubcommands,
    },
    /// Empirical 4-Regime Benchmark & Agent Challenge Arena
    Benchmark {
        #[command(subcommand)]
        sub: BenchmarkSubcommands,
    },
}

#[derive(Subcommand)]
enum MemorySubcommands {
    List {
        #[arg(long)]
        target: Option<String>,
        #[arg(long, alias = "repo", default_value = ".")]
        path: String,
        #[arg(long)]
        json: bool,
    },
    Pin {
        #[arg(long)]
        target: String,
        #[arg(long)]
        lesson: String,
        #[arg(long, default_value = "BOUNDARY_OFF_BY_ONE")]
        failure_mode: String,
        #[arg(long, default_value = "MANUAL_PIN")]
        mutant_desc: String,
        #[arg(long, alias = "repo", default_value = ".")]
        path: String,
        #[arg(long)]
        json: bool,
    },
    Consolidate {
        #[arg(long, alias = "repo", default_value = ".")]
        path: String,
        #[arg(long, default_value_t = 10)]
        threshold: usize,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        json: bool,
    },
    MetaPromote {
        #[arg(long, alias = "repo", default_value = ".")]
        path: String,
        #[arg(long)]
        json: bool,
    },
    MetaList {
        #[arg(long, alias = "repo", default_value = ".")]
        path: String,
        #[arg(long)]
        json: bool,
    },
    RecordSuccess {
        #[arg(long, alias = "target")]
        function_id: String,
        #[arg(long, default_value_t = 1)]
        mutants_killed: usize,
        #[arg(long, default_value_t = 1)]
        total_mutants: usize,
        #[arg(long, alias = "repo", default_value = ".")]
        path: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum BenchmarkSubcommands {
    Run {
        #[arg(long, alias = "repo", default_value = ".")]
        path: String,
        #[arg(long, default_value = "all")]
        regime: String,
        #[arg(long)]
        external_repo: Option<String>,
        #[arg(long)]
        report: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Init {
        #[arg(long, default_value = ".proven/benchmark_arena")]
        dir: String,
        #[arg(long)]
        json: bool,
    },
    Grade {
        #[arg(long, default_value = ".proven/benchmark_arena")]
        dir: String,
        #[arg(long)]
        report: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Report {
        #[arg(long, alias = "repo", default_value = ".")]
        path: String,
        #[arg(long, default_value = "docs/BENCHMARK_REPORT.md")]
        output: String,
        #[arg(long)]
        json: bool,
    },
}

fn execute_command(cmd: Commands) {
    match cmd {
        Commands::Scan {
            path,
            base,
            head,
            fan_out,
            json: _,
        } => {
            let res = scan_repository_changes_with_options(Path::new(&path), &base, &head, fan_out);
            print_json(&res);
        }
        Commands::Refresh {
            path,
            base,
            head,
            json: _,
        } => {
            let res = scan_repository_changes(Path::new(&path), &base, &head);
            print_json(&res);
        }
        Commands::Context {
            target,
            file,
            path,
            blind,
            spec,
            json: _,
        } => {
            let res = extract_scoped_context_with_options(
                Path::new(&path),
                target.as_deref(),
                file.as_deref(),
                true,
                blind,
                spec.as_deref(),
            );
            print_json(&res);
        }
        Commands::VerifyProof {
            target,
            test,
            file,
            path,
            mutation_check,
            max_mutants,
            pytest_args,
            json: _,
        } => {
            let res = MicroMutationGate::verify_proof(
                Path::new(&path),
                &target,
                &test,
                file.as_deref(),
                mutation_check,
                max_mutants,
                pytest_args.as_deref(),
            );
            print_json(&res);
        }
        Commands::ExplainAttention {
            target,
            hops,
            top_k,
            mode,
            path,
            visualize,
            json,
        } => {
            let res = CodeGraphRAG::extract_subgraph_attention(
                Path::new(&path),
                &target,
                hops,
                top_k,
                &mode,
            );
            if json && !visualize {
                print_json(&res);
            } else {
                println!("{}\n\n{}", res.ascii_tree, res.prompt_block);
            }
        }
        Commands::Impact {
            target,
            mode,
            path,
            json: _,
        } => {
            let res = CodeGraphRAG::predict_impact(Path::new(&path), &target, &mode);
            print_json(&res);
        }
        Commands::Triage {
            query_pos,
            query_flag,
            max_devs,
            max_files,
            path,
            json: _,
        } => {
            let query = query_flag.or(query_pos).unwrap_or_default();
            let res = CodeGraphRAG::triage_bug(Path::new(&path), &query, max_devs, max_files);
            print_json(&res);
        }
        Commands::Arena {
            target,
            test,
            round,
            mutation_check: _,
            pytest_args,
            path,
            json: _,
        } => {
            let res = TriAgentArena::referee_round(
                Path::new(&path),
                &target,
                &test,
                round,
                pytest_args.as_deref(),
            );
            print_json(&res);
        }
        Commands::Sleep {
            path,
            threshold,
            force,
            meta,
            json: _,
        } => {
            let res =
                GraphMemoryManager::consolidate_axioms(Path::new(&path), threshold, force, meta);
            print_json(&res);
        }
        Commands::Memory {
            path: parent_path,
            sub,
        } => {
            let resolve_p = |sub_p: String| -> String {
                if let Some(pp) = &parent_path
                    && sub_p == "." {
                        return pp.clone();
                    }
                sub_p
            };
            match sub {
                MemorySubcommands::List {
                    target,
                    path,
                    json: _,
                } => {
                    let eff_p = resolve_p(path);
                    let res = GraphMemoryManager::get_pinned_lessons(
                        Path::new(&eff_p),
                        target.as_deref(),
                    );
                    print_json(&res);
                }
                MemorySubcommands::Pin {
                    target,
                    lesson,
                    failure_mode,
                    mutant_desc,
                    path,
                    json: _,
                } => {
                    let eff_p = resolve_p(path);
                    let res = GraphMemoryManager::pin_lesson(
                        Path::new(&eff_p),
                        &target,
                        &lesson,
                        &failure_mode,
                        &mutant_desc,
                    );
                    print_json(&res);
                }
                MemorySubcommands::Consolidate {
                    path,
                    threshold,
                    force,
                    json: _,
                } => {
                    let eff_p = resolve_p(path);
                    let res = GraphMemoryManager::consolidate_axioms(
                        Path::new(&eff_p),
                        threshold,
                        force,
                        false,
                    );
                    print_json(&res);
                }
                MemorySubcommands::MetaPromote { path, json: _ } => {
                    let eff_p = resolve_p(path);
                    let res = GraphMemoryManager::promote_to_meta_axioms(Path::new(&eff_p));
                    print_json(&res);
                }
                MemorySubcommands::MetaList { path, json: _ } => {
                    let eff_p = resolve_p(path);
                    let res = GraphMemoryManager::load_meta_axioms(Path::new(&eff_p));
                    print_json(&res);
                }
                MemorySubcommands::RecordSuccess {
                    function_id,
                    mutants_killed,
                    total_mutants,
                    path,
                    json: _,
                } => {
                    let eff_p = resolve_p(path);
                    let res = GraphMemoryManager::record_success(
                        Path::new(&eff_p),
                        &function_id,
                        mutants_killed,
                        total_mutants,
                    );
                    print_json(&res);
                }
            }
        }
        Commands::Benchmark { sub } => match sub {
            BenchmarkSubcommands::Run {
                path,
                regime,
                external_repo,
                report,
                json,
            } => {
                let res = match regime.as_str() {
                    "small" => BenchmarkEngine::evaluate_small_regime(Path::new(&path)),
                    "large" => BenchmarkEngine::evaluate_large_scale_regime(),
                    "dirty" => BenchmarkEngine::evaluate_dirty_zero_test_regime(),
                    "cross-repo" => BenchmarkEngine::evaluate_cross_repo_regime(),
                    _ => BenchmarkEngine::evaluate_all_regimes(
                        Path::new(&path),
                        external_repo.as_deref(),
                    ),
                };
                if let Some(rp) = report {
                    let _ = BenchmarkEngine::write_benchmark_report(&res, Path::new(&rp));
                }
                if json || regime != "all" {
                    print_json(&res);
                } else {
                    println!("{}", BenchmarkEngine::render_cli_summary(&res));
                }
            }
            BenchmarkSubcommands::Init { dir, json: _ } => {
                let res = BenchmarkEngine::init_arena(Path::new(&dir));
                print_json(&res);
            }
            BenchmarkSubcommands::Grade {
                dir,
                report,
                json: _,
            } => {
                let res = BenchmarkEngine::grade_arena(Path::new(&dir), report.as_deref());
                print_json(&res);
            }
            BenchmarkSubcommands::Report { path, output, json } => {
                let summary = BenchmarkEngine::evaluate_all_regimes(Path::new(&path), None);
                let res = BenchmarkEngine::write_benchmark_report(&summary, Path::new(&output));
                if json {
                    print_json(&res);
                } else {
                    println!("Wrote benchmark report to {}", output);
                }
            }
        },
    }
}

fn main() {
    let mut raw_args: Vec<String> = std::env::args().collect();
    if raw_args.get(1).map(|s| s.as_str()) == Some("agent") {
        raw_args.remove(1);
    }
    let cli = Cli::parse_from(raw_args);
    execute_command(cli.command);
}

