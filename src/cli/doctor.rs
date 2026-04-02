use crate::config::Config;
use crate::error::Result;
use crate::schema::validation::validate_frontmatter;
use crate::storage::Repository;
use crate::storage::markdown::MarkdownRepository;
use clap::{Args, Subcommand};
use regex::Regex;

#[derive(Args)]
pub struct DoctorArgs {
    #[command(subcommand)]
    pub command: DoctorCommands,
}

#[derive(Subcommand)]
pub enum DoctorCommands {
    /// Validate all files against schemas
    Validate,
    /// Check for broken links
    Links,
}

pub fn run(args: &DoctorArgs, config: &Config) -> Result<()> {
    let repo = MarkdownRepository::new(config.vault_path.clone());

    match &args.command {
        DoctorCommands::Validate => {
            let all = repo.list_all(&config.registry)?;
            let mut errors = 0;

            for entity in &all {
                if let Some(type_def) = config.registry.get(&entity.entity_type)
                    && let Err(e) = validate_frontmatter(&entity.frontmatter, type_def)
                {
                    errors += 1;
                    println!("ERROR in {} ({}): {e}", entity.id, entity.entity_type);
                }
            }

            if errors == 0 {
                println!("All {} entities pass validation.", all.len());
            } else {
                println!(
                    "\n{errors} validation error(s) found in {} entities.",
                    all.len()
                );
            }
        }
        DoctorCommands::Links => {
            let all = repo.list_all(&config.registry)?;
            let all_ids: std::collections::HashSet<String> =
                all.iter().map(|e| e.id.clone()).collect();

            let link_re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
            let mut broken = 0;

            for entity in &all {
                for (field, val) in &entity.frontmatter {
                    if let Some(s) = val.as_str() {
                        for cap in link_re.captures_iter(s) {
                            let target = &cap[1];
                            if !all_ids.contains(target) {
                                broken += 1;
                                println!(
                                    "BROKEN LINK in {} ({}): field '{}' -> [[{}]]",
                                    entity.id, entity.entity_type, field, target
                                );
                            }
                        }
                    }
                }

                for cap in link_re.captures_iter(&entity.body) {
                    let target = &cap[1];
                    if !all_ids.contains(target) {
                        broken += 1;
                        println!(
                            "BROKEN LINK in {} ({}): body -> [[{}]]",
                            entity.id, entity.entity_type, target
                        );
                    }
                }
            }

            if broken == 0 {
                println!("No broken links found across {} entities.", all.len());
            } else {
                println!("\n{broken} broken link(s) found.");
            }
        }
    }

    Ok(())
}
