//! Document conversion and sync command handler.

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum DocCommands {
    Sync {
        #[arg(short, long)]
        dir: PathBuf,
        #[arg(long)]
        notion_db: String,
    },
    Convert {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
}

pub async fn handle_doc(cmd: DocCommands) -> anyhow::Result<()> {
    use kept_doc::converter::markdown::MarkdownConverter;
    use kept_doc::converter::{FromPlatform, ToPlatform};

    match cmd {
        DocCommands::Sync { .. } => {
            anyhow::bail!("Live Notion sync is not available in offline kept build.");
        }
        DocCommands::Convert { input, output } => {
            let content = std::fs::read_to_string(&input)?;
            let doc = MarkdownConverter::from_platform(content)
                .map_err(|e| anyhow::anyhow!("Failed to read markdown: {}", e))?;

            if let Some(ext) = output.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("json") {
                    let json = serde_json::to_string_pretty(&doc)?;
                    std::fs::write(output, json)?;
                    println!("Converted successfully to JSON IR!");
                    return Ok(());
                }
            }

            let md = MarkdownConverter::from_universal(&doc)
                .map_err(|e| anyhow::anyhow!("Failed to write markdown: {}", e))?;
            std::fs::write(output, md)?;
            println!("Converted successfully to Markdown!");
        }
    }
    Ok(())
}
