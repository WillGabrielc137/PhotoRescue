use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Jpeg,
    Png,
    Webp,
    Heic,
    Bmp,
    Gif,
}

impl ImageFormat {
    pub const ALL: [Self; 6] = [
        Self::Jpeg,
        Self::Png,
        Self::Webp,
        Self::Heic,
        Self::Bmp,
        Self::Gif,
    ];

    pub fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
            Self::Heic => "heic",
            Self::Bmp => "bmp",
            Self::Gif => "gif",
        }
    }

    pub fn mime_type(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Webp => "image/webp",
            Self::Heic => "image/heic",
            Self::Bmp => "image/bmp",
            Self::Gif => "image/gif",
        }
    }
}

impl std::fmt::Display for ImageFormat {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.extension())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateStatus {
    Found,
    Partial,
    Recovered,
    Failed,
    Corrupted,
}

impl CandidateStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Found => "found",
            Self::Partial => "partial",
            Self::Recovered => "recovered",
            Self::Failed => "failed",
            Self::Corrupted => "corrupted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanMode {
    Normal,
    Deep,
}

impl ScanMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Deep => "deep",
        }
    }
}

impl Default for ScanMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryCandidate {
    pub id: Uuid,
    pub scan_id: Uuid,
    pub format: ImageFormat,
    pub offset: u64,
    pub estimated_size: u64,
    pub confidence: u8,
    pub status: CandidateStatus,
    pub possible_original_name: Option<String>,
    pub recovered_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRecord {
    pub id: Uuid,
    pub source_id: String,
    pub source_root: String,
    pub source_device: String,
    pub workspace_directory: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeInfo {
    pub id: String,
    pub root_path: String,
    pub device_path: String,
    pub display_name: String,
    pub total_bytes: u64,
    pub sector_size: u32,
    pub is_removable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_extensions_are_stable() {
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::Heic.mime_type(), "image/heic");
    }

    #[test]
    fn status_serializes_for_the_database_contract() {
        assert_eq!(CandidateStatus::Corrupted.as_str(), "corrupted");
        assert_eq!(CandidateStatus::Partial.as_str(), "partial");
    }

    #[test]
    fn scan_mode_serializes_for_commands() {
        assert_eq!(ScanMode::Deep.as_str(), "deep");
    }
}
