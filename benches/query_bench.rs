use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

use cortx::frontmatter::serialize_entity;
use cortx::query::evaluator::evaluate;
use cortx::query::parser::parse_query;
use cortx::schema::registry::TypeRegistry;
use cortx::storage::Repository;
use cortx::storage::markdown::MarkdownRepository;
use cortx::value::Value;

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
    }

    group.finish();
}

criterion_group!(benches, bench_query_scan);
criterion_main!(benches);
