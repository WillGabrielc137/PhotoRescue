use photorescue_domain::{CandidateStatus, ImageFormat, RecoveryCandidate};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RecoveredFile {
    pub candidate_id: Uuid,
    pub path: PathBuf,
    pub bytes_written: u64,
    pub sha256: String,
}

#[derive(Debug, Error)]
pub enum RecoveryError {
    #[error("a pasta de destino precisa estar em outra unidade; origem={source_volume}, destino={destination_volume}")]
    SameVolume {
        source_volume: String,
        destination_volume: String,
    },
    #[error("não foi possível identificar a unidade do caminho: {0}")]
    UnknownVolume(String),
    #[error("falha ao criar a pasta de destino: {0}")]
    CreateDestination(#[source] std::io::Error),
    #[error("falha ao ler os dados na posição {offset}: {source}")]
    ReadSource {
        offset: u64,
        #[source]
        source: std::io::Error,
    },
    #[error("falha ao gravar o arquivo recuperado: {0}")]
    WriteDestination(#[source] std::io::Error),
}

pub fn ensure_different_volume(
    source_root: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<(), RecoveryError> {
    let source = volume_key(source_root.as_ref())
        .ok_or_else(|| RecoveryError::UnknownVolume(source_root.as_ref().display().to_string()))?;
    let destination = volume_key(destination.as_ref())
        .ok_or_else(|| RecoveryError::UnknownVolume(destination.as_ref().display().to_string()))?;

    if source.eq_ignore_ascii_case(&destination) {
        return Err(RecoveryError::SameVolume {
            source_volume: source,
            destination_volume: destination,
        });
    }
    Ok(())
}

pub fn volume_key(path: &Path) -> Option<String> {
    let normalized = path.as_os_str().to_string_lossy().replace('/', "\\");
    let without_device_prefix = normalized.strip_prefix(r"\\.\").unwrap_or(&normalized);
    let bytes = without_device_prefix.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return Some(format!("{}:", (bytes[0] as char).to_ascii_uppercase()));
    }
    if let Some(unc) = without_device_prefix.strip_prefix(r"\\") {
        let mut parts = unc.split('\\').filter(|part| !part.is_empty());
        let server = parts.next()?;
        let share = parts.next()?;
        return Some(format!(r"\\{}\{}", server, share));
    }
    None
}

pub fn safe_recovery_name(format: ImageFormat, offset: u64) -> String {
    format!("recovered_{offset:016X}.{}", format.extension())
}

pub fn recovery_bucket_name(status: CandidateStatus) -> &'static str {
    match status {
        CandidateStatus::Found | CandidateStatus::Recovered => "Recuperados_Integralmente",
        CandidateStatus::Partial => "Recuperados_Parcialmente",
        CandidateStatus::Corrupted | CandidateStatus::Failed => "Possivelmente_Corrompidos",
    }
}

pub struct RecoveryService;

impl RecoveryService {
    pub fn recover_one<R: Read + Seek>(
        source: &mut R,
        candidate: &RecoveryCandidate,
        destination: &Path,
    ) -> Result<RecoveredFile, RecoveryError> {
        fs::create_dir_all(destination).map_err(RecoveryError::CreateDestination)?;
        let category_directory = destination.join(recovery_bucket_name(candidate.status));
        fs::create_dir_all(&category_directory).map_err(RecoveryError::CreateDestination)?;
        let final_path =
            unique_destination_path(&category_directory, candidate.format, candidate.offset);
        let temporary_path =
            category_directory.join(format!(".photorescue-{}.part", Uuid::new_v4()));

        let result = (|| {
            let mut target = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary_path)
                .map_err(RecoveryError::WriteDestination)?;

            source
                .seek(SeekFrom::Start(candidate.offset))
                .map_err(|source| RecoveryError::ReadSource {
                    offset: candidate.offset,
                    source,
                })?;

            let mut remaining = candidate.estimated_size;
            let mut buffer = vec![0_u8; 1024 * 1024];
            let mut hasher = Sha256::new();
            let mut bytes_written = 0_u64;

            while remaining > 0 {
                let wanted = remaining.min(buffer.len() as u64) as usize;
                source.read_exact(&mut buffer[..wanted]).map_err(|source| {
                    RecoveryError::ReadSource {
                        offset: candidate.offset + bytes_written,
                        source,
                    }
                })?;
                target
                    .write_all(&buffer[..wanted])
                    .map_err(RecoveryError::WriteDestination)?;
                hasher.update(&buffer[..wanted]);
                remaining -= wanted as u64;
                bytes_written += wanted as u64;
            }

            target.sync_all().map_err(RecoveryError::WriteDestination)?;
            drop(target);
            fs::rename(&temporary_path, &final_path).map_err(RecoveryError::WriteDestination)?;

            Ok(RecoveredFile {
                candidate_id: candidate.id,
                path: final_path,
                bytes_written,
                sha256: format!("{:x}", hasher.finalize()),
            })
        })();

        if result.is_err() {
            let _ = fs::remove_file(&temporary_path);
        }
        result
    }
}

fn unique_destination_path(destination: &Path, format: ImageFormat, offset: u64) -> PathBuf {
    let base = format!("recovered_{offset:016X}");
    for suffix in 0_u32.. {
        let name = if suffix == 0 {
            format!("{base}.{}", format.extension())
        } else {
            format!("{base}_{suffix:03}.{}", format.extension())
        };
        let candidate = destination.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("o espaço de nomes u32 foi esgotado")
}

#[cfg(test)]
mod tests {
    use super::*;
    use photorescue_domain::CandidateStatus;
    use std::io::Cursor;
    use tempfile::tempdir;

    #[test]
    fn blocks_destination_on_the_scanned_volume() {
        let error = ensure_different_volume(r"C:\", r"c:\Recovered").unwrap_err();
        assert!(matches!(error, RecoveryError::SameVolume { .. }));
    }

    #[test]
    fn accepts_a_different_destination_volume() {
        ensure_different_volume(r"C:\", r"E:\Recovered").unwrap();
    }

    #[test]
    fn generated_names_are_deterministic_and_safe() {
        assert_eq!(
            safe_recovery_name(ImageFormat::Jpeg, 0x1234),
            "recovered_0000000000001234.jpg"
        );
    }

    #[test]
    fn bucket_names_follow_candidate_status() {
        assert_eq!(
            recovery_bucket_name(CandidateStatus::Found),
            "Recuperados_Integralmente"
        );
        assert_eq!(
            recovery_bucket_name(CandidateStatus::Partial),
            "Recuperados_Parcialmente"
        );
        assert_eq!(
            recovery_bucket_name(CandidateStatus::Corrupted),
            "Possivelmente_Corrompidos"
        );
    }

    #[test]
    fn recovery_never_overwrites_an_existing_file() {
        let directory = tempdir().unwrap();
        let data = b"photo bytes".to_vec();
        let candidate = RecoveryCandidate {
            id: Uuid::new_v4(),
            scan_id: Uuid::new_v4(),
            format: ImageFormat::Png,
            offset: 0,
            estimated_size: data.len() as u64,
            confidence: 90,
            status: CandidateStatus::Found,
            possible_original_name: None,
            recovered_path: None,
        };
        let bucket = directory
            .path()
            .join(recovery_bucket_name(CandidateStatus::Found));
        fs::create_dir_all(&bucket).unwrap();
        let original = bucket.join(safe_recovery_name(ImageFormat::Png, 0));
        fs::write(&original, b"keep me").unwrap();

        let recovered = RecoveryService::recover_one(
            &mut Cursor::new(data.clone()),
            &candidate,
            directory.path(),
        )
        .unwrap();

        assert_ne!(recovered.path, original);
        assert!(recovered.path.starts_with(&bucket));
        assert_eq!(fs::read(original).unwrap(), b"keep me");
        assert_eq!(fs::read(recovered.path).unwrap(), data);
    }
}
