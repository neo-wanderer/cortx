use crate::config::Config;
use crate::error::Result;
use crate::storage::Repository;
use crate::storage::markdown::MarkdownRepository;
use crate::value::Value;
use clap::Args;
use std::collections::HashMap;

#[derive(Args)]
pub struct CreateArgs {
    /// Entity type (resolved from types.yaml)
    pub entity_type: String,

    #[arg(long)]
    pub id: Option<String>,

    #[arg(long)]
    pub title: Option<String>,

    #[arg(long)]
    pub name: Option<String>,

    #[arg(long)]
    pub tags: Option<String>,

    #[arg(long = "set", num_args = 1)]
    pub fields: Vec<String>,
}

pub fn run(args: &CreateArgs, config: &Config) -> Result<()> {
    let repo = MarkdownRepository::new(config.vault_path.clone());

    let mut fm = HashMap::new();

    let id = args.id.clone().unwrap_or_else(|| {
        let today = chrono::Local::now().format("%Y%m%d");
        let short = &uuid::Uuid::new_v4().to_string()[..8];
        format!("{}-{today}-{short}", args.entity_type)
    });

    fm.insert("id".into(), Value::String(id.clone()));
    fm.insert("type".into(), Value::String(args.entity_type.clone()));

    if let Some(title) = &args.title {
        fm.insert("title".into(), Value::String(title.clone()));
    }
    if let Some(name) = &args.name {
        fm.insert("name".into(), Value::String(name.clone()));
    }

    if let Some(type_def) = config.registry.get(&args.entity_type)
        && type_def.fields.contains_key("status")
        && !args.fields.iter().any(|f| f.starts_with("status="))
    {
        fm.insert("status".into(), Value::String("open".into()));
    }

    if let Some(tags) = &args.tags {
        let tag_list: Vec<Value> = tags
            .split(',')
            .map(|t| Value::String(t.trim().to_string()))
            .collect();
        fm.insert("tags".into(), Value::Array(tag_list));
    } else {
        fm.insert("tags".into(), Value::Array(vec![]));
    }

    for kv in &args.fields {
        if let Some((k, v)) = kv.split_once('=') {
            let value = parse_cli_value(v);
            fm.insert(k.to_string(), value);
        }
    }

    let today = chrono::Local::now().date_naive();
    fm.insert("created_at".into(), Value::Date(today));
    fm.insert("updated_at".into(), Value::Date(today));

    let entity = repo.create(fm, "", &config.registry)?;
    println!("Created {} ({})", entity.id, entity.entity_type);
    if let Some(path) = &entity.file_path {
        println!("  File: {}", path.display());
    }

    Ok(())
}

pub fn parse_cli_value(v: &str) -> Value {
    match v {
        "today" => return Value::Date(chrono::Local::now().date_naive()),
        "yesterday" => {
            return Value::Date(chrono::Local::now().date_naive() - chrono::Duration::days(1));
        }
        "tomorrow" => {
            return Value::Date(chrono::Local::now().date_naive() + chrono::Duration::days(1));
        }
        _ => {}
    }
    if let Some(date_val) = Value::parse_as_date(v) {
        return date_val;
    }
    if v.starts_with('[') && v.ends_with(']') {
        let inner = &v[1..v.len() - 1];
        let arr: Vec<Value> = inner
            .split(',')
            .map(|s| Value::String(s.trim().trim_matches('"').to_string()))
            .collect();
        return Value::Array(arr);
    }
    Value::String(v.to_string())
}
