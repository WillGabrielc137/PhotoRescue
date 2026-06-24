use chrono::{DateTime, Utc};
use photorescue_domain::{CandidateStatus, ImageFormat, RecoveryCandidate, ScanRecord};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("falha no índice SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("UUID inválido no índice: {0}")]
    Uuid(#[from] uuid::Error),
    #[error("data inválida no índice: {0}")]
    Date(#[from] chrono::ParseError),
    #[error("valor desconhecido no índice: {0}")]
    InvalidValue(String),
}

#[derive(Debug, Clone)]
pub struct ScanIndex {
    path: PathBuf,
}

impl ScanIndex {
    pub fn initialize(path: impl AsRef<Path>) -> Result<Self, IndexError> {
        let index = Self {
            path: path.as_ref().to_path_buf(),
        };
        let connection = index.connection()?;
        connection.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = FULL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS scans (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                source_root TEXT NOT NULL,
                source_device TEXT NOT NULL,
                workspace_directory TEXT NOT NULL,
                started_at TEXT NOT NULL,
                finished_at TEXT,
                status TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS candidates (
                id TEXT PRIMARY KEY,
                scan_id TEXT NOT NULL,
                format TEXT NOT NULL,
                offset INTEGER NOT NULL,
                estimated_size INTEGER NOT NULL,
                confidence INTEGER NOT NULL,
                status TEXT NOT NULL,
                possible_original_name TEXT,
                recovered_path TEXT,
                FOREIGN KEY(scan_id) REFERENCES scans(id)
            );

            CREATE INDEX IF NOT EXISTS idx_candidates_scan
                ON candidates(scan_id, offset);
            ",
        )?;
        Ok(index)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn create_scan(&self, scan: &ScanRecord) -> Result<(), IndexError> {
        self.connection()?.execute(
            "INSERT INTO scans (
                id, source_id, source_root, source_device, workspace_directory,
                started_at, finished_at, status
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                scan.id.to_string(),
                scan.source_id,
                scan.source_root,
                scan.source_device,
                scan.workspace_directory,
                scan.started_at.to_rfc3339(),
                scan.finished_at.map(|date| date.to_rfc3339()),
                scan.status,
            ],
        )?;
        Ok(())
    }

    pub fn finish_scan(
        &self,
        scan_id: Uuid,
        status: &str,
        finished_at: DateTime<Utc>,
    ) -> Result<(), IndexError> {
        self.connection()?.execute(
            "UPDATE scans SET status = ?1, finished_at = ?2 WHERE id = ?3",
            params![status, finished_at.to_rfc3339(), scan_id.to_string()],
        )?;
        Ok(())
    }

    pub fn save_candidate(&self, candidate: &RecoveryCandidate) -> Result<(), IndexError> {
        self.connection()?.execute(
            "INSERT OR REPLACE INTO candidates (
                id, scan_id, format, offset, estimated_size, confidence, status,
                possible_original_name, recovered_path
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                candidate.id.to_string(),
                candidate.scan_id.to_string(),
                candidate.format.extension(),
                candidate.offset as i64,
                candidate.estimated_size as i64,
                candidate.confidence as i64,
                candidate.status.as_str(),
                candidate.possible_original_name,
                candidate.recovered_path,
            ],
        )?;
        Ok(())
    }

    pub fn update_candidate_status(
        &self,
        candidate_id: Uuid,
        status: CandidateStatus,
        recovered_path: Option<&Path>,
    ) -> Result<(), IndexError> {
        self.connection()?.execute(
            "UPDATE candidates SET status = ?1, recovered_path = ?2 WHERE id = ?3",
            params![
                status.as_str(),
                recovered_path.map(|path| path.display().to_string()),
                candidate_id.to_string()
            ],
        )?;
        Ok(())
    }

    pub fn list_candidates(&self, scan_id: Uuid) -> Result<Vec<RecoveryCandidate>, IndexError> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, scan_id, format, offset, estimated_size, confidence, status,
                    possible_original_name, recovered_path
             FROM candidates WHERE scan_id = ?1 ORDER BY offset",
        )?;
        let rows = statement.query_map([scan_id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
            ))
        })?;

        rows.map(|row| {
            let (
                id,
                row_scan_id,
                format,
                offset,
                estimated_size,
                confidence,
                status,
                possible_original_name,
                recovered_path,
            ) = row?;
            Ok(RecoveryCandidate {
                id: Uuid::parse_str(&id)?,
                scan_id: Uuid::parse_str(&row_scan_id)?,
                format: parse_format(&format)?,
                offset: offset as u64,
                estimated_size: estimated_size as u64,
                confidence: confidence as u8,
                status: parse_status(&status)?,
                possible_original_name,
                recovered_path,
            })
        })
        .collect()
    }

    pub fn find_candidate(
        &self,
        scan_id: Uuid,
        candidate_id: Uuid,
    ) -> Result<Option<RecoveryCandidate>, IndexError> {
        Ok(self
            .list_candidates(scan_id)?
            .into_iter()
            .find(|candidate| candidate.id == candidate_id))
    }

    fn connection(&self) -> Result<Connection, IndexError> {
        Ok(Connection::open(&self.path)?)
    }
}

fn parse_format(value: &str) -> Result<ImageFormat, IndexError> {
    match value {
        "jpg" | "jpeg" => Ok(ImageFormat::Jpeg),
        "png" => Ok(ImageFormat::Png),
        "webp" => Ok(ImageFormat::Webp),
        "heic" => Ok(ImageFormat::Heic),
        "bmp" => Ok(ImageFormat::Bmp),
        "gif" => Ok(ImageFormat::Gif),
        _ => Err(IndexError::InvalidValue(value.to_owned())),
    }
}

fn parse_status(value: &str) -> Result<CandidateStatus, IndexError> {
    match value {
        "found" => Ok(CandidateStatus::Found),
        "partial" => Ok(CandidateStatus::Partial),
        "recovered" => Ok(CandidateStatus::Recovered),
        "failed" => Ok(CandidateStatus::Failed),
        "corrupted" => Ok(CandidateStatus::Corrupted),
        _ => Err(IndexError::InvalidValue(value.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_persisted_values() {
        assert_eq!(parse_format("jpeg").unwrap(), ImageFormat::Jpeg);
        assert_eq!(
            parse_status("recovered").unwrap(),
            CandidateStatus::Recovered
        );
        assert_eq!(parse_status("partial").unwrap(), CandidateStatus::Partial);
        assert!(parse_status("unknown").is_err());
    }
}
