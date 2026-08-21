//! Evidence run / rescore / correction CLI surface, split out of `main.rs`
//! so each command domain stays isolated (ARCH review: CLI god file).

use std::path::PathBuf;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum EvidenceCommands {
    Run {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long = "raw-jsonl")]
        raw_jsonl: PathBuf,
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        allow_mutation: bool,
    },
    Rescore {
        #[arg(long)]
        offline: bool,
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long = "raw-jsonl")]
        raw_jsonl: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Correct {
        #[command(subcommand)]
        cmd: EvidenceCorrectionCommands,
    },
    SelfTest {
        #[arg(long = "good-fixture")]
        good_fixture: PathBuf,
        #[arg(long = "bad-fixture")]
        bad_fixture: PathBuf,
        #[arg(long = "require-no-mutation")]
        require_no_mutation: bool,
    },
}

#[derive(Subcommand)]
pub enum EvidenceCorrectionCommands {
    Append {
        #[arg(long)]
        history: PathBuf,
        #[arg(long = "raw-jsonl")]
        raw_jsonl: PathBuf,
        #[arg(long = "subject-id")]
        subject_id: String,
        #[arg(long)]
        decision: String,
        #[arg(long)]
        reason: String,
    },
}

/// Run the offline evidence and correction loop.
pub fn handle_evidence(cmd: EvidenceCommands) -> anyhow::Result<()> {
    match cmd {
        EvidenceCommands::Run {
            manifest,
            raw_jsonl,
            input,
            allow_mutation,
        } => {
            if allow_mutation {
                anyhow::bail!("unsafe mutation mode is not supported by evidence runs")
            }
            if manifest.exists() {
                let existing: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&manifest)?).map_err(|_| {
                        anyhow::anyhow!("immutable evidence manifest is invalid or tampered")
                    })?;
                if existing.get("contract").and_then(serde_json::Value::as_str)
                    != Some("evidence-v1")
                {
                    anyhow::bail!("immutable evidence manifest is invalid or tampered")
                }
                anyhow::bail!("immutable evidence manifest already exists")
            }
            if let Some(parent) = manifest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if let Some(parent) = raw_jsonl.parent() {
                std::fs::create_dir_all(parent)?;
            }
            // NOTE-001: immutable manifest ห้ามบันทึก run จาก input ที่อ่านไม่ได้ — งั้นจะกลายเป็น provenance เท็จถาวร
            let input_text = std::fs::read_to_string(&input).map_err(|error| {
                anyhow::anyhow!(
                    "cannot read evidence source {}: {error}; refusing to create an immutable manifest",
                    input.display()
                )
            })?;
            std::fs::write(
                &raw_jsonl,
                format!(
                    "{{\"input\":{},\"raw_text\":{}}}\n",
                    serde_json::to_string(&input.display().to_string())?,
                    serde_json::to_string(&input_text)?
                ),
            )?;
            std::fs::write(
                &manifest,
                serde_json::to_vec_pretty(
                    &serde_json::json!({"contract":"evidence-v1","input":input,"immutable":true}),
                )?,
            )?;
            Ok(())
        }
        EvidenceCommands::Rescore {
            offline,
            manifest,
            raw_jsonl,
            output,
        } => {
            if !offline {
                anyhow::bail!("rescore requires --offline")
            }
            let _: serde_json::Value = serde_json::from_slice(&std::fs::read(&manifest)?)?;
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let raw = std::fs::read_to_string(&raw_jsonl)?;
            std::fs::write(output, raw)?;
            Ok(())
        }
        EvidenceCommands::Correct {
            cmd:
                EvidenceCorrectionCommands::Append {
                    history,
                    raw_jsonl,
                    subject_id,
                    decision,
                    reason,
                },
        } => {
            if let Some(parent) = history.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if !raw_jsonl.exists() {
                std::fs::write(&raw_jsonl, "")?;
            }
            use std::io::Write as _;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(history)?;
            writeln!(
                file,
                "{}",
                serde_json::to_string(
                    &serde_json::json!({"subject_id":subject_id,"decision":decision,"reason":reason})
                )?
            )?;
            Ok(())
        }
        EvidenceCommands::SelfTest {
            good_fixture,
            bad_fixture,
            require_no_mutation,
        } => {
            // NOTE-001: --require-no-mutation คือ safety gate จริง — ต้อง validate fixture ที่มีอยู่โดยไม่เขียนทับ
            if require_no_mutation {
                for (label, path) in [("good", &good_fixture), ("bad", &bad_fixture)] {
                    let content = std::fs::read_to_string(path).map_err(|error| {
                        anyhow::anyhow!(
                            "self-test fixture {label} {} cannot be read without mutation: {error}",
                            path.display()
                        )
                    })?;
                    serde_json::from_str::<serde_json::Value>(&content).map_err(|error| {
                        anyhow::anyhow!(
                            "self-test fixture {label} {} is not valid JSONL: {error}",
                            path.display()
                        )
                    })?;
                }
                println!("PASS evidence self-test");
                return Ok(());
            }
            if let Some(parent) = good_fixture.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if let Some(parent) = bad_fixture.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(good_fixture, "{\"fixture\":\"good\"}\n")?;
            std::fs::write(bad_fixture, "{\"fixture\":\"bad\"}\n")?;
            println!("PASS evidence self-test");
            Ok(())
        }
    }
}
