use std::sync::Mutex;

use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CorrectionLedgerError {
    #[error("SQLite correction ledger error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("SQLite correction ledger lock poisoned")]
    LockPoisoned,
    #[error("invalid correction source: {0}")]
    InvalidSource(String),
    #[error("invalid correction status: {0}")]
    InvalidStatus(String),
    #[error("invalid correction confidence: {0}")]
    InvalidConfidence(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionSource {
    User,
    VerifiedSystem,
}

impl CorrectionSource {
    fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::VerifiedSystem => "verified_system",
        }
    }

    fn parse(value: String) -> Result<Self, CorrectionLedgerError> {
        match value.as_str() {
            "user" => Ok(Self::User),
            "verified_system" => Ok(Self::VerifiedSystem),
            _ => Err(CorrectionLedgerError::InvalidSource(value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionStatus {
    Active,
    Superseded,
}

impl CorrectionStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Superseded => "superseded",
        }
    }

    fn parse(value: String) -> Result<Self, CorrectionLedgerError> {
        match value.as_str() {
            "active" => Ok(Self::Active),
            "superseded" => Ok(Self::Superseded),
            _ => Err(CorrectionLedgerError::InvalidStatus(value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionConfidence {
    Confirmed,
}

impl CorrectionConfidence {
    fn as_str(&self) -> &'static str {
        "confirmed"
    }

    fn parse(value: String) -> Result<Self, CorrectionLedgerError> {
        match value.as_str() {
            "confirmed" => Ok(Self::Confirmed),
            _ => Err(CorrectionLedgerError::InvalidConfidence(value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrectionRecord {
    pub correction_id: String,
    pub subject: String,
    pub assertion: String,
    pub rejected_assertion: String,
    pub evidence_target: String,
    pub outcome_target: String,
    pub source: CorrectionSource,
    pub status: CorrectionStatus,
    pub confidence: CorrectionConfidence,
    pub created_at: String,
    pub supersedes: Option<String>,
}

impl CorrectionRecord {
    pub fn confirmed(
        subject: impl Into<String>,
        assertion: impl Into<String>,
        rejected_assertion: impl Into<String>,
        evidence_target: impl Into<String>,
        outcome_target: impl Into<String>,
        source: CorrectionSource,
    ) -> Self {
        Self {
            correction_id: Uuid::new_v4().to_string(),
            subject: subject.into(),
            assertion: assertion.into(),
            rejected_assertion: rejected_assertion.into(),
            evidence_target: evidence_target.into(),
            outcome_target: outcome_target.into(),
            source,
            status: CorrectionStatus::Active,
            confidence: CorrectionConfidence::Confirmed,
            created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            supersedes: None,
        }
    }
}

pub struct CorrectionLedger {
    connection: Mutex<Connection>,
}

impl CorrectionLedger {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, CorrectionLedgerError> {
        Self::from_connection(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self, CorrectionLedgerError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self, CorrectionLedgerError> {
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS corrections (
                correction_id TEXT PRIMARY KEY,
                subject TEXT NOT NULL,
                assertion TEXT NOT NULL,
                rejected_assertion TEXT NOT NULL,
                evidence_target TEXT NOT NULL,
                outcome_target TEXT NOT NULL,
                source TEXT NOT NULL CHECK(source IN ('user', 'verified_system')),
                status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'superseded')),
                confidence TEXT NOT NULL DEFAULT 'confirmed' CHECK(confidence = 'confirmed'),
                created_at TEXT NOT NULL DEFAULT '',
                supersedes TEXT REFERENCES corrections(correction_id),
                superseded INTEGER NOT NULL DEFAULT 0 CHECK(superseded IN (0, 1))
             ) STRICT;",
        )?;
        Self::add_column_if_missing(&connection, "status", "TEXT NOT NULL DEFAULT 'active'")?;
        Self::add_column_if_missing(
            &connection,
            "confidence",
            "TEXT NOT NULL DEFAULT 'confirmed'",
        )?;
        Self::add_column_if_missing(&connection, "created_at", "TEXT NOT NULL DEFAULT ''")?;
        connection.execute(
            "UPDATE corrections SET status = 'superseded' WHERE superseded = 1 AND status = 'active'",
            [],
        )?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn add_column_if_missing(
        connection: &Connection,
        column: &str,
        definition: &str,
    ) -> Result<(), CorrectionLedgerError> {
        let mut statement = connection.prepare("PRAGMA table_info(corrections)")?;
        let exists = statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .any(|name| name == column);
        if !exists {
            connection.execute_batch(&format!(
                "ALTER TABLE corrections ADD COLUMN {column} {definition}"
            ))?;
        }
        Ok(())
    }

    pub fn record(&self, record: &CorrectionRecord) -> Result<(), CorrectionLedgerError> {
        self.connection.lock().map_err(|_| CorrectionLedgerError::LockPoisoned)?.execute(
            "INSERT INTO corrections (
                correction_id, subject, assertion, rejected_assertion, evidence_target, outcome_target,
                source, status, confidence, created_at, supersedes
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                record.correction_id, record.subject, record.assertion, record.rejected_assertion,
                record.evidence_target, record.outcome_target, record.source.as_str(),
                record.status.as_str(), record.confidence.as_str(), record.created_at, record.supersedes,
            ],
        )?;
        Ok(())
    }

    pub fn supersede(
        &self,
        correction_id: &str,
        replacement: &CorrectionRecord,
    ) -> Result<(), CorrectionLedgerError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| CorrectionLedgerError::LockPoisoned)?;
        let transaction = connection.transaction()?;
        let exists = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM corrections WHERE correction_id = ?1 AND status = 'active')",
            [correction_id],
            |row| row.get::<_, bool>(0),
        )?;
        if !exists {
            return Err(CorrectionLedgerError::Sqlite(rusqlite::Error::QueryReturnedNoRows));
        }
        transaction.execute(
            "INSERT INTO corrections (
                correction_id, subject, assertion, rejected_assertion, evidence_target, outcome_target,
                source, status, confidence, created_at, supersedes
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', ?8, ?9, ?10)",
            params![
                replacement.correction_id, replacement.subject, replacement.assertion,
                replacement.rejected_assertion, replacement.evidence_target, replacement.outcome_target,
                replacement.source.as_str(), replacement.confidence.as_str(), replacement.created_at,
                correction_id,
            ],
        )?;
        transaction.execute(
            "UPDATE corrections SET status = 'superseded', superseded = 1 WHERE correction_id = ?1",
            [correction_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn active_for(
        &self,
        subject: &str,
    ) -> Result<Vec<CorrectionRecord>, CorrectionLedgerError> {
        self.records_for(subject, "AND status = 'active'")
    }

    pub fn history_for(
        &self,
        subject: &str,
    ) -> Result<Vec<CorrectionRecord>, CorrectionLedgerError> {
        self.records_for(subject, "")
    }

    pub fn conflict_for(
        &self,
        subject: &str,
        assertion: &str,
    ) -> Result<Option<CorrectionRecord>, CorrectionLedgerError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| CorrectionLedgerError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT correction_id, subject, assertion, rejected_assertion, evidence_target, outcome_target,
                    source, status, confidence, created_at, supersedes
             FROM corrections
             WHERE subject = ?1 AND rejected_assertion = ?2 AND status = 'active'
             LIMIT 1",
        )?;
        let mut rows = statement.query([subject, assertion])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(Self::record_from_row(row)?))
    }

    fn records_for(
        &self,
        subject: &str,
        predicate: &str,
    ) -> Result<Vec<CorrectionRecord>, CorrectionLedgerError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| CorrectionLedgerError::LockPoisoned)?;
        let mut statement = connection.prepare(&format!(
            "SELECT correction_id, subject, assertion, rejected_assertion, evidence_target, outcome_target,
                    source, status, confidence, created_at, supersedes
             FROM corrections WHERE subject = ?1 {predicate} ORDER BY rowid"
        ))?;
        let rows = statement.query_map([subject], Self::record_from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    fn record_from_row(row: &Row<'_>) -> rusqlite::Result<CorrectionRecord> {
        let source = CorrectionSource::parse(row.get(6)?).map_err(to_sql_error)?;
        let status = CorrectionStatus::parse(row.get(7)?).map_err(to_sql_error)?;
        let confidence = CorrectionConfidence::parse(row.get(8)?).map_err(to_sql_error)?;
        Ok(CorrectionRecord {
            correction_id: row.get(0)?,
            subject: row.get(1)?,
            assertion: row.get(2)?,
            rejected_assertion: row.get(3)?,
            evidence_target: row.get(4)?,
            outcome_target: row.get(5)?,
            source,
            status,
            confidence,
            created_at: row.get(9)?,
            supersedes: row.get(10)?,
        })
    }
}

fn to_sql_error(error: CorrectionLedgerError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}
