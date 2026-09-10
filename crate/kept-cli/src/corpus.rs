//! Gold-corpus validation / snapshot / replay CLI surface, split out of
//! `main.rs` so each command domain stays isolated (ARCH review: CLI god file).

use std::path::PathBuf;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum CorpusCommands {
    Validate {
        corpus: PathBuf,
        #[arg(long)]
        report: String,
    },
    Snapshot {
        #[command(subcommand)]
        cmd: CorpusSnapshotCommands,
    },
    Replay {
        snapshot: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum CorpusSnapshotCommands {
    Save {
        corpus: PathBuf,
        snapshot: PathBuf,
    },
}

/// Import, validate, snapshot, and replay reviewed gold assertions.
pub fn handle_corpus(cmd: CorpusCommands) -> anyhow::Result<()> {
    match cmd {
        CorpusCommands::Validate { corpus, report } => {
            let mut errors = Vec::new();
            let mut accepted = 0usize;
            for line in std::fs::read_to_string(corpus)?.lines() {
                let value: serde_json::Value = serde_json::from_str(line)?;
                let mut missing = Vec::new();
                for path in [
                    "source.provenanceId",
                    "source.locator",
                    "context.fragmentHash",
                    "normalizedCandidate",
                    "assertionKind",
                    "review.decision",
                    "review.reason",
                    "split",
                ] {
                    let present = path
                        .split('.')
                        .try_fold(&value, |current, key| current.get(key))
                        .is_some();
                    if !present {
                        missing.push(path);
                    }
                }
                if missing.is_empty()
                    && value
                        .get("review")
                        .and_then(|v| v.get("decision"))
                        .and_then(serde_json::Value::as_str)
                        == Some("accepted")
                {
                    accepted += 1;
                }
                if !missing.is_empty() {
                    errors.push(serde_json::json!({"missing":missing}));
                }
            }
            if report == "json" {
                println!(
                    "{}",
                    serde_json::json!({"accepted":accepted,"rejected":errors.len(),"errors":errors,"targetSplit":{"build":7000,"validation":1500,"holdout":1500}})
                );
            }
            Ok(())
        },
        CorpusCommands::Snapshot {
            cmd: CorpusSnapshotCommands::Save { corpus, snapshot },
        } => {
            let records: Vec<serde_json::Value> = std::fs::read_to_string(corpus)?
                .lines()
                .map(serde_json::from_str)
                .collect::<Result<_, _>>()?;
            if let Some(parent) = snapshot.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(
                snapshot,
                serde_json::to_vec_pretty(
                    &serde_json::json!({"contract":"gold-corpus-v1","records":records}),
                )?,
            )?;
            Ok(())
        },
        CorpusCommands::Replay { snapshot, json } => {
            let value: serde_json::Value = serde_json::from_slice(&std::fs::read(snapshot)?)?;
            let mut dictionary = Vec::new();
            if let Some(records) = value.get("records").and_then(serde_json::Value::as_array) {
                for record in records {
                    if record
                        .get("review")
                        .and_then(|v| v.get("decision"))
                        .and_then(serde_json::Value::as_str)
                        == Some("accepted")
                    {
                        if let Some(candidate) = record
                            .get("normalizedCandidate")
                            .and_then(serde_json::Value::as_str)
                        {
                            if !dictionary.iter().any(|v| v == candidate) {
                                dictionary.push(candidate.to_string());
                            }
                        }
                    }
                }
            }
            if json {
                println!("{}", serde_json::json!({"dictionary":dictionary}));
            }
            Ok(())
        },
    }
}
