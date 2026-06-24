mod carvers;

use photorescue_domain::{CandidateStatus, RecoveryCandidate, ScanMode};
use std::io::{Read, Seek, SeekFrom};
use thiserror::Error;
use uuid::Uuid;

use carvers::{carve_at, detect_format};

const SIGNATURE_OVERLAP: usize = 32;

#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub mode: ScanMode,
    pub chunk_size: usize,
    pub max_file_size: u64,
    pub max_candidates: usize,
    pub allow_partial: bool,
    pub tolerate_read_errors: bool,
    pub skip_carved_ranges: bool,
    pub progress_interval: u64,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            mode: ScanMode::Normal,
            chunk_size: 8 * 1024 * 1024,
            max_file_size: 512 * 1024 * 1024,
            max_candidates: 100_000,
            allow_partial: false,
            tolerate_read_errors: false,
            skip_carved_ranges: true,
            progress_interval: 16 * 1024 * 1024,
        }
    }
}

impl ScanConfig {
    pub fn for_mode(mode: ScanMode) -> Self {
        match mode {
            ScanMode::Normal => Self::default(),
            ScanMode::Deep => Self {
                mode,
                chunk_size: 4 * 1024 * 1024,
                max_file_size: 768 * 1024 * 1024,
                max_candidates: 250_000,
                allow_partial: true,
                tolerate_read_errors: true,
                skip_carved_ranges: false,
                progress_interval: 8 * 1024 * 1024,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ScanCounts {
    pub found_count: usize,
    pub complete_count: usize,
    pub partial_count: usize,
    pub corrupted_count: usize,
    pub read_error_count: usize,
}

impl ScanCounts {
    fn record_candidate(&mut self, status: CandidateStatus) {
        self.found_count += 1;
        match status {
            CandidateStatus::Found => self.complete_count += 1,
            CandidateStatus::Partial => self.partial_count += 1,
            CandidateStatus::Corrupted => self.corrupted_count += 1,
            CandidateStatus::Recovered | CandidateStatus::Failed => {}
        }
    }
}

#[derive(Debug, Clone)]
pub enum ScanEvent {
    Progress {
        bytes_scanned: u64,
        total_bytes: u64,
        counts: ScanCounts,
    },
    Candidate(RecoveryCandidate),
    ReadError {
        offset: u64,
        message: String,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct ScanSummary {
    pub bytes_scanned: u64,
    pub counts: ScanCounts,
    pub cancelled: bool,
}

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("falha de leitura na posição {offset}: {source}")]
    Read {
        offset: u64,
        #[source]
        source: std::io::Error,
    },
    #[error("o tamanho do bloco de leitura deve ser maior que 64 bytes")]
    InvalidChunkSize,
    #[error("limite de {0} candidatos atingido; refine a varredura")]
    CandidateLimit(usize),
}

pub struct SignatureScanner {
    scan_id: Uuid,
    config: ScanConfig,
}

impl SignatureScanner {
    pub fn new(scan_id: Uuid, config: ScanConfig) -> Self {
        Self { scan_id, config }
    }

    pub fn scan<R, F, C>(
        &self,
        reader: &mut R,
        total_bytes: u64,
        mut on_event: F,
        is_cancelled: C,
    ) -> Result<ScanSummary, ScanError>
    where
        R: Read + Seek,
        F: FnMut(ScanEvent),
        C: Fn() -> bool,
    {
        if self.config.chunk_size <= SIGNATURE_OVERLAP * 2 {
            return Err(ScanError::InvalidChunkSize);
        }

        let mut buffer = vec![0_u8; self.config.chunk_size];
        let mut scan_offset = 0_u64;
        let mut counts = ScanCounts::default();
        let mut last_reported = 0_u64;

        while scan_offset < total_bytes {
            if is_cancelled() {
                return Ok(ScanSummary {
                    bytes_scanned: scan_offset,
                    counts,
                    cancelled: true,
                });
            }

            if let Err(source) = reader.seek(SeekFrom::Start(scan_offset)) {
                if self.config.tolerate_read_errors {
                    counts.read_error_count += 1;
                    on_event(ScanEvent::ReadError {
                        offset: scan_offset,
                        message: source.to_string(),
                    });
                    scan_offset = scan_offset
                        .saturating_add(self.config.chunk_size as u64)
                        .min(total_bytes);
                    on_event(ScanEvent::Progress {
                        bytes_scanned: scan_offset,
                        total_bytes,
                        counts,
                    });
                    continue;
                }
                return Err(ScanError::Read {
                    offset: scan_offset,
                    source,
                });
            }

            let wanted = (total_bytes - scan_offset).min(buffer.len() as u64) as usize;
            let bytes_read = match reader.read(&mut buffer[..wanted]) {
                Ok(bytes_read) => bytes_read,
                Err(source) if self.config.tolerate_read_errors => {
                    counts.read_error_count += 1;
                    on_event(ScanEvent::ReadError {
                        offset: scan_offset,
                        message: source.to_string(),
                    });
                    scan_offset = scan_offset
                        .saturating_add(self.config.chunk_size as u64)
                        .min(total_bytes);
                    on_event(ScanEvent::Progress {
                        bytes_scanned: scan_offset,
                        total_bytes,
                        counts,
                    });
                    continue;
                }
                Err(source) => {
                    return Err(ScanError::Read {
                        offset: scan_offset,
                        source,
                    });
                }
            };

            if bytes_read == 0 {
                break;
            }

            let mut cursor = 0_usize;
            let mut jumped_to_candidate_end = false;

            while cursor < bytes_read {
                if is_cancelled() {
                    return Ok(ScanSummary {
                        bytes_scanned: scan_offset + cursor as u64,
                        counts,
                        cancelled: true,
                    });
                }

                let available = &buffer[cursor..bytes_read];
                if let Some(format) = detect_format(available) {
                    let absolute_offset = scan_offset + cursor as u64;
                    if let Some(carved) = carve_at(
                        reader,
                        absolute_offset,
                        format,
                        self.config
                            .max_file_size
                            .min(total_bytes.saturating_sub(absolute_offset)),
                        self.config.allow_partial,
                    ) {
                        counts.record_candidate(carved.status);
                        if counts.found_count > self.config.max_candidates {
                            return Err(ScanError::CandidateLimit(self.config.max_candidates));
                        }

                        let candidate = RecoveryCandidate {
                            id: Uuid::new_v4(),
                            scan_id: self.scan_id,
                            format,
                            offset: absolute_offset,
                            estimated_size: carved.length,
                            confidence: carved.confidence,
                            status: carved.status,
                            possible_original_name: None,
                            recovered_path: None,
                        };
                        on_event(ScanEvent::Candidate(candidate));

                        if self.config.skip_carved_ranges {
                            scan_offset = absolute_offset.saturating_add(carved.length.max(1));
                            jumped_to_candidate_end = true;
                            break;
                        }
                    }
                }
                cursor += 1;
            }

            if !jumped_to_candidate_end {
                let advance = bytes_read.saturating_sub(SIGNATURE_OVERLAP).max(1);
                scan_offset = scan_offset.saturating_add(advance as u64);
            }

            if scan_offset.saturating_sub(last_reported) >= self.config.progress_interval
                || scan_offset >= total_bytes
            {
                last_reported = scan_offset;
                on_event(ScanEvent::Progress {
                    bytes_scanned: scan_offset.min(total_bytes),
                    total_bytes,
                    counts,
                });
            }
        }

        Ok(ScanSummary {
            bytes_scanned: scan_offset.min(total_bytes),
            counts,
            cancelled: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use photorescue_domain::{CandidateStatus, ImageFormat, ScanMode};
    use std::io::Cursor;

    fn minimal_webp() -> Vec<u8> {
        let mut data = b"RIFF".to_vec();
        data.extend_from_slice(&4_u32.to_le_bytes());
        data.extend_from_slice(b"WEBP");
        data
    }

    #[test]
    fn scanner_finds_a_real_signature_inside_raw_bytes() {
        let scan_id = Uuid::new_v4();
        let mut raw = vec![0xAA; 4096];
        raw.extend_from_slice(&minimal_webp());
        raw.extend_from_slice(&vec![0xBB; 4096]);
        let total = raw.len() as u64;
        let mut reader = Cursor::new(raw);
        let mut candidates = Vec::new();
        let scanner = SignatureScanner::new(
            scan_id,
            ScanConfig {
                chunk_size: 512,
                ..ScanConfig::default()
            },
        );

        let summary = scanner
            .scan(
                &mut reader,
                total,
                |event| {
                    if let ScanEvent::Candidate(candidate) = event {
                        candidates.push(candidate);
                    }
                },
                || false,
            )
            .unwrap();

        assert_eq!(summary.counts.found_count, 1);
        assert_eq!(summary.counts.complete_count, 1);
        assert_eq!(candidates[0].offset, 4096);
        assert_eq!(candidates[0].format, ImageFormat::Webp);
    }

    #[test]
    fn deep_scan_classifies_a_jpeg_without_footer_as_partial() {
        let scan_id = Uuid::new_v4();
        let mut raw = vec![0xAA; 256];
        raw.extend_from_slice(&[
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00, 0xFF, 0xDA, 0x00, 0x02, 0x11, 0x22,
            0x33, 0x44, 0x55, 0x66,
        ]);
        raw.extend_from_slice(&vec![0xBB; 256]);
        let total = raw.len() as u64;
        let mut reader = Cursor::new(raw);
        let mut candidates = Vec::new();
        let scanner = SignatureScanner::new(
            scan_id,
            ScanConfig {
                chunk_size: 128,
                ..ScanConfig::for_mode(ScanMode::Deep)
            },
        );

        let summary = scanner
            .scan(
                &mut reader,
                total,
                |event| {
                    if let ScanEvent::Candidate(candidate) = event {
                        candidates.push(candidate);
                    }
                },
                || false,
            )
            .unwrap();

        assert_eq!(summary.counts.found_count, 1);
        assert_eq!(summary.counts.partial_count, 1);
        assert_eq!(candidates[0].status, CandidateStatus::Partial);
        assert_eq!(candidates[0].format, ImageFormat::Jpeg);
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requer uma unidade Windows real e execução como administrador"]
    fn scanner_crosses_raw_volume_chunk_boundaries() {
        let requested_id =
            std::env::var("PHOTORESCUE_TEST_VOLUME").unwrap_or_else(|_| "D:".to_owned());
        let volume = photorescue_platform::list_volumes()
            .unwrap()
            .into_iter()
            .find(|volume| volume.id.eq_ignore_ascii_case(&requested_id))
            .expect("unidade de teste não encontrada");
        let mut reader = photorescue_platform::open_volume_read_only(
            &volume.device_path,
            volume.total_bytes,
            volume.sector_size,
        )
        .unwrap();
        let scan_length = volume.total_bytes.min(24 * 1024 * 1024);
        let scanner = SignatureScanner::new(Uuid::new_v4(), ScanConfig::default());

        let summary = scanner
            .scan(&mut reader, scan_length, |_| {}, || false)
            .unwrap();

        assert_eq!(summary.bytes_scanned, scan_length);
        assert!(!summary.cancelled);
    }
}
