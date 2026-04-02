use clap::{Args, Subcommand};
use crate::config::Config;
use crate::error::{CortxError, Result};
use crate::storage::markdown::MarkdownRepository;
use crate::storage::Repository;

#[derive(Args)]
pub struct NoteArgs {
    #[command(subcommand)]
    pub command: NoteCommands,
}

#[derive(Subcommand)]
pub enum NoteCommands {
    /// List headings in a note
    Headings {
        id: String,
    },
    /// Insert content after a heading
    InsertAfterHeading {
        id: String,
        #[arg(long)]
        heading: String,
        #[arg(long)]
        content: String,
    },
    /// Replace a named block
    ReplaceBlock {
        id: String,
        #[arg(long = "block-id")]
        block_id: String,
        #[arg(long)]
        content: String,
    },
    /// Read specific lines
    ReadLines {
        id: String,
        #[arg(long)]
        start: usize,
        #[arg(long)]
        end: usize,
    },
}

pub fn run(args: &NoteArgs, config: &Config) -> Result<()> {
    let repo = MarkdownRepository::new(config.vault_path.clone());

    match &args.command {
        NoteCommands::Headings { id } => {
            let entity = repo.get_by_id(id, &config.registry)?;
            let headings = extract_headings(&entity.body);
            println!("Headings in {id}:\n");
            for (line_num, heading) in &headings {
                println!("  Line {line_num}: {heading}");
            }
        }
        NoteCommands::InsertAfterHeading { id, heading, content } => {
            let entity = repo.get_by_id(id, &config.registry)?;
            let new_body = insert_after_heading(&entity.body, heading, content)?;

            let path = entity
                .file_path
                .as_ref()
                .ok_or_else(|| CortxError::Storage("no file path".into()))?;
            let file_content =
                crate::frontmatter::serialize_entity(&entity.frontmatter, &new_body);
            std::fs::write(path, file_content)?;
            println!("Inserted content after '{heading}' in {id}");
        }
        NoteCommands::ReplaceBlock { id, block_id, content } => {
            let entity = repo.get_by_id(id, &config.registry)?;
            let new_body = replace_block(&entity.body, block_id, content)?;

            let path = entity
                .file_path
                .as_ref()
                .ok_or_else(|| CortxError::Storage("no file path".into()))?;
            let file_content =
                crate::frontmatter::serialize_entity(&entity.frontmatter, &new_body);
            std::fs::write(path, file_content)?;
            println!("Replaced block '{block_id}' in {id}");
        }
        NoteCommands::ReadLines { id, start, end } => {
            let entity = repo.get_by_id(id, &config.registry)?;
            let lines: Vec<&str> = entity.body.lines().collect();
            let start_idx = start.saturating_sub(1);
            let end_idx = (*end).min(lines.len());
            for (i, line) in lines[start_idx..end_idx].iter().enumerate() {
                println!("{:4}: {line}", start_idx + i + 1);
            }
        }
    }

    Ok(())
}

fn extract_headings(body: &str) -> Vec<(usize, String)> {
    body.lines()
        .enumerate()
        .filter_map(|(i, line)| {
            if line.starts_with('#') {
                Some((i + 1, line.to_string()))
            } else {
                None
            }
        })
        .collect()
}

fn insert_after_heading(body: &str, heading: &str, content: &str) -> Result<String> {
    let lines: Vec<&str> = body.lines().collect();
    let heading_idx = lines
        .iter()
        .position(|line| line.trim() == heading.trim())
        .ok_or_else(|| CortxError::Storage(format!("heading '{heading}' not found")))?;

    let mut result = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        result.push(*line);
        if i == heading_idx {
            result.push(content);
        }
    }

    Ok(result.join("\n") + "\n")
}

fn replace_block(body: &str, block_id: &str, content: &str) -> Result<String> {
    let open_tag = format!("<!-- block:id={block_id} -->");
    let close_tag = format!("<!-- /block:id={block_id} -->");

    let open_pos = body
        .find(&open_tag)
        .ok_or_else(|| CortxError::Storage(format!("block '{block_id}' not found")))?;
    let close_pos = body
        .find(&close_tag)
        .ok_or_else(|| CortxError::Storage(format!("closing tag for block '{block_id}' not found")))?;

    let before = &body[..open_pos + open_tag.len()];
    let after = &body[close_pos..];

    Ok(format!("{before}\n{content}\n{after}"))
}
