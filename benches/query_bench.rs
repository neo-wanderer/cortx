use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

use cortx::entity::Entity;
use cortx::frontmatter::serialize_entity;
use cortx::query::evaluator::evaluate;
use cortx::query::parser::parse_query;
use cortx::schema::registry::TypeRegistry;
use cortx::storage::Repository;
use cortx::storage::markdown::MarkdownRepository;
use cortx::value::Value;

/// Sort specification for benchmarking
#[derive(Debug, Clone)]
struct SortSpec {
    field: String,
    descending: bool,
}

/// Compare two optional values for sorting (nulls to end)
fn compare_values(a: Option<&Value>, b: Option<&Value>, descending: bool) -> Ordering {
    match (a, b) {
        (Some(av), Some(bv)) => {
            let cmp = av.partial_cmp(bv).unwrap_or(Ordering::Equal);
            if descending { cmp.reverse() } else { cmp }
        }
        (None, None) => Ordering::Equal,
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
    }
}

/// Sort entities by the given specifications
fn sort_entities(entities: &mut [&Entity], specs: &[SortSpec]) {
    entities.sort_by(|a, b| {
        for spec in specs {
            let cmp = compare_values(a.get(&spec.field), b.get(&spec.field), spec.descending);
            if cmp != Ordering::Equal {
                return cmp;
            }
        }
        Ordering::Equal
    });
}

fn generate_vault(dir: &Path, n: usize) {
    let task_folder = dir.join("1_Projects/tasks");
    fs::create_dir_all(&task_folder).unwrap();
    fs::copy("types.yaml", dir.join("types.yaml")).unwrap();

    for folder in &[
        "0_Inbox",
        "2_Areas",
        "3_Resources",
        "3_Resources/notes",
        "4_Archive",
        "5_People",
        "5_Companies",
    ] {
        fs::create_dir_all(dir.join(folder)).unwrap();
    }

    let statuses = ["open", "in_progress", "waiting", "done"];
    let all_tags = [
        "home", "work", "urgent", "backend", "frontend", "api", "docs",
    ];

    for i in 0..n {
        let id = format!("task-bench-{i:06}");
        let status = statuses[i % statuses.len()];
        let tag1 = all_tags[i % all_tags.len()];
        let tag2 = all_tags[(i + 3) % all_tags.len()];

        let day = (i % 28) + 1;
        let month = if i % 2 == 0 { "03" } else { "05" };
        let due = format!("2026-{month}-{day:02}");

        let mut fm = HashMap::new();
        fm.insert("id".into(), Value::String(id.clone()));
        fm.insert("type".into(), Value::String("task".into()));
        fm.insert("title".into(), Value::String(format!("Benchmark task {i}")));
        fm.insert("status".into(), Value::String(status.into()));
        fm.insert("due".into(), Value::parse_as_date(&due).unwrap());
        fm.insert(
            "tags".into(),
            Value::Array(vec![Value::String(tag1.into()), Value::String(tag2.into())]),
        );
        fm.insert(
            "created_at".into(),
            Value::parse_as_date("2026-01-01").unwrap(),
        );
        fm.insert(
            "updated_at".into(),
            Value::parse_as_date("2026-04-01").unwrap(),
        );

        let body = format!(
            "# Task {i}\n\nThis is benchmark task number {i}.\nIt contains some body text for search purposes.\n"
        );
        let content = serialize_entity(&fm, &body);
        fs::write(task_folder.join(format!("{id}.md")), content).unwrap();
    }
}

fn bench_query_scan(c: &mut Criterion) {
    let registry = TypeRegistry::from_yaml_file(Path::new("types.yaml")).unwrap();

    let mut group = c.benchmark_group("query_scan");
    group.sample_size(10);

    for size in [100, 500, 1000, 5000, 10000, 20000] {
        let dir = TempDir::new().unwrap();
        generate_vault(dir.path(), size);
        let repo = MarkdownRepository::new(dir.path().to_path_buf());

        group.bench_with_input(BenchmarkId::new("list_all", size), &size, |b, _| {
            b.iter(|| {
                let entities = repo.list_all(&registry).unwrap();
                assert_eq!(entities.len(), size);
            });
        });

        let simple_expr = parse_query(r#"type = "task" and status = "open""#).unwrap();
        group.bench_with_input(BenchmarkId::new("filter_simple", size), &size, |b, _| {
            b.iter(|| {
                let all = repo.list_all(&registry).unwrap();
                let matches: Vec<_> = all.iter().filter(|e| evaluate(&simple_expr, e)).collect();
                assert!(!matches.is_empty());
            });
        });

        let complex_expr = parse_query(
            r#"type = "task" and status != "done" and due < "2026-04-01" and tags contains "home""#,
        )
        .unwrap();
        group.bench_with_input(BenchmarkId::new("filter_complex", size), &size, |b, _| {
            b.iter(|| {
                let all = repo.list_all(&registry).unwrap();
                let _matches: Vec<_> = all.iter().filter(|e| evaluate(&complex_expr, e)).collect();
            });
        });

        let text_expr = parse_query(r#"type = "task" and text ~ "benchmark task""#).unwrap();
        group.bench_with_input(
            BenchmarkId::new("filter_text_search", size),
            &size,
            |b, _| {
                b.iter(|| {
                    let all = repo.list_all(&registry).unwrap();
                    let matches: Vec<_> = all.iter().filter(|e| evaluate(&text_expr, e)).collect();
                    assert!(!matches.is_empty());
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("distinct_tags", size), &size, |b, _| {
            b.iter(|| {
                let all = repo.list_all(&registry).unwrap();
                let mut unique = std::collections::BTreeSet::new();
                for entity in &all {
                    if let Some(Value::Array(tags)) = entity.get("tags") {
                        for tag in tags {
                            unique.insert(tag.to_string());
                        }
                    }
                }
                assert!(!unique.is_empty());
            });
        });

        // Sort benchmarks
        let all = repo.list_all(&registry).unwrap();
        let task_expr = parse_query(r#"type = "task""#).unwrap();
        let matches: Vec<_> = all.iter().filter(|e| evaluate(&task_expr, e)).collect();
        let baseline_len = matches.len();

        group.bench_with_input(BenchmarkId::new("sort_single_asc", size), &size, |b, _| {
            b.iter_with_setup(
                || matches.clone(),
                |mut entities| {
                    sort_entities(
                        &mut entities,
                        &[SortSpec {
                            field: "due".into(),
                            descending: false,
                        }],
                    );
                    assert_eq!(entities.len(), baseline_len);
                    entities
                },
            );
        });

        group.bench_with_input(BenchmarkId::new("sort_single_desc", size), &size, |b, _| {
            b.iter_with_setup(
                || matches.clone(),
                |mut entities| {
                    sort_entities(
                        &mut entities,
                        &[SortSpec {
                            field: "due".into(),
                            descending: true,
                        }],
                    );
                    assert_eq!(entities.len(), baseline_len);
                    entities
                },
            );
        });

        group.bench_with_input(BenchmarkId::new("sort_multi_field", size), &size, |b, _| {
            b.iter_with_setup(
                || matches.clone(),
                |mut entities| {
                    sort_entities(
                        &mut entities,
                        &[
                            SortSpec {
                                field: "status".into(),
                                descending: false,
                            },
                            SortSpec {
                                field: "due".into(),
                                descending: true,
                            },
                        ],
                    );
                    assert_eq!(entities.len(), baseline_len);
                    entities
                },
            );
        });
    }

    group.finish();
}

/// Build a vault where `n` tasks all reference one shared project.
/// Returns the TempDir (holds the vault alive) and the vault path.
fn build_vault_with_refs(n: usize) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let vault_path = dir.path().to_path_buf();

    // Minimal schema
    fs::write(
        vault_path.join("types.yaml"),
        r#"types:
  task:
    folder: "tasks"
    required: [type, title]
    fields:
      type: { const: task }
      title: { type: string }
      project: { type: link, ref: project }
  project:
    folder: "projects"
    required: [type, title]
    fields:
      type: { const: project }
      title: { type: string }
"#,
    )
    .unwrap();

    let registry = TypeRegistry::from_yaml_file(&vault_path.join("types.yaml")).unwrap();
    let repo = MarkdownRepository::new(vault_path.clone()).with_link_validation(false);

    // Create the central project
    let mut pfm = HashMap::new();
    pfm.insert("type".into(), Value::String("project".into()));
    pfm.insert("title".into(), Value::String("Central Project".into()));
    repo.create("Central Project", pfm, "", &registry).unwrap();

    // Create n tasks referencing it
    for i in 0..n {
        let mut tfm = HashMap::new();
        tfm.insert("type".into(), Value::String("task".into()));
        let title = format!("Task {i:05}");
        tfm.insert("title".into(), Value::String(title.clone()));
        tfm.insert("project".into(), Value::String("Central Project".into()));
        repo.create(&title, tfm, "", &registry).unwrap();
    }

    (dir, vault_path)
}

fn rename_bench(c: &mut Criterion) {
    use cortx::cli::rename::{RenameArgs, run as rename_run};
    use cortx::config::Config;

    let mut group = c.benchmark_group("rename_cascade");
    group.sample_size(10);

    for size in [100, 500, 5000] {
        group.bench_function(format!("N={size}"), |b| {
            b.iter_batched(
                || {
                    let (dir, vault_path) = build_vault_with_refs(size);
                    let registry =
                        TypeRegistry::from_yaml_file(&vault_path.join("types.yaml")).unwrap();
                    let config = Config {
                        vault_path: vault_path.clone(),
                        registry,
                    };
                    let args = RenameArgs {
                        old_title: "Central Project".into(),
                        new_title: "Main Project".into(),
                        dry_run: false,
                        skip_body: true, // skip body scan to isolate frontmatter cascade
                    };
                    (dir, config, args)
                },
                |(dir, config, args)| {
                    rename_run(&args, &config).unwrap();
                    drop(dir);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(benches, bench_query_scan, rename_bench);
criterion_main!(benches);
