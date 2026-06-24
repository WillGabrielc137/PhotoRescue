use crate::state::{AppState, ScanSession};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Utc;
use photorescue_domain::{
    CandidateStatus, ImageFormat, RecoveryCandidate, ScanMode, ScanRecord, VolumeInfo,
};
use photorescue_index::ScanIndex;
use photorescue_logging::ScanLogger;
use photorescue_recovery::{ensure_different_volume, RecoveryService};
use photorescue_scanner::{ScanConfig, ScanCounts, ScanEvent, SignatureScanner};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartScanRequest {
    source_id: String,
    workspace_directory: String,
    #[serde(default)]
    scan_mode: ScanMode,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartScanResponse {
    scan_id: Uuid,
    session_directory: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ScanProgressPayload {
    scan_id: Uuid,
    bytes_scanned: u64,
    total_bytes: u64,
    found_count: usize,
    complete_count: usize,
    partial_count: usize,
    corrupted_count: usize,
    read_error_count: usize,
    scan_mode: ScanMode,
    status: String,
    message: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RecoveryProgressPayload {
    scan_id: Uuid,
    completed: usize,
    total: usize,
    candidate_id: Option<Uuid>,
    success: bool,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewData {
    data_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoverRequest {
    scan_id: Uuid,
    candidate_ids: Vec<Uuid>,
    destination: String,
}

#[tauri::command]
pub fn list_volumes() -> Result<Vec<VolumeInfo>, String> {
    photorescue_platform::list_volumes().map_err(user_error)
}

#[tauri::command]
pub fn is_elevated() -> bool {
    photorescue_platform::is_elevated()
}

#[tauri::command]
pub fn restart_elevated() -> Result<(), String> {
    photorescue_platform::restart_elevated().map_err(user_error)
}

#[tauri::command]
pub fn start_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    request: StartScanRequest,
) -> Result<StartScanResponse, String> {
    let scan_mode = request.scan_mode;
    let source = photorescue_platform::list_volumes()
        .map_err(user_error)?
        .into_iter()
        .find(|volume| volume.id.eq_ignore_ascii_case(&request.source_id))
        .ok_or_else(|| "A unidade selecionada não está mais disponível.".to_owned())?;

    let workspace = PathBuf::from(&request.workspace_directory);
    if !workspace.is_absolute() {
        return Err("Escolha uma pasta de trabalho com caminho absoluto.".to_owned());
    }
    ensure_different_volume(&source.root_path, &workspace).map_err(user_error)?;
    fs::create_dir_all(&workspace)
        .map_err(|error| format!("Não foi possível preparar a pasta de trabalho: {error}"))?;

    let scan_id = Uuid::new_v4();
    let session_directory = workspace
        .join("PhotoRescue")
        .join(format!("scan-{scan_id}"));
    fs::create_dir_all(&session_directory)
        .map_err(|error| format!("Não foi possível criar a sessão segura: {error}"))?;

    let logs_directory = session_directory.join("Logs");
    fs::create_dir_all(&logs_directory)
        .map_err(|error| format!("Nao foi possivel criar a pasta de logs: {error}"))?;

    let index =
        ScanIndex::initialize(session_directory.join("photorescue.sqlite")).map_err(user_error)?;
    let logger = ScanLogger::open(logs_directory.join("photorescue.log")).map_err(user_error)?;
    let scan_record = ScanRecord {
        id: scan_id,
        source_id: source.id.clone(),
        source_root: source.root_path.clone(),
        source_device: source.device_path.clone(),
        workspace_directory: session_directory.display().to_string(),
        started_at: Utc::now(),
        finished_at: None,
        status: "running".to_owned(),
    };
    index.create_scan(&scan_record).map_err(user_error)?;
    logger
        .info(format!(
            "Iniciando varredura somente leitura em {}; modo={}; removivel={}; filtros={}",
            source.device_path,
            scan_mode.as_str(),
            source.is_removable,
            supported_filters()
        ))
        .map_err(user_error)?;

    let session = Arc::new(ScanSession {
        source: source.clone(),
        index,
        logger,
        cancel: Arc::new(AtomicBool::new(false)),
    });
    state.insert(scan_id, session.clone())?;

    app.emit(
        "scan-progress",
        ScanProgressPayload {
            scan_id,
            bytes_scanned: 0,
            total_bytes: source.total_bytes,
            found_count: 0,
            complete_count: 0,
            partial_count: 0,
            corrupted_count: 0,
            read_error_count: 0,
            scan_mode,
            status: "starting".to_owned(),
            message: "Abrindo a unidade em modo somente leitura…".to_owned(),
        },
    )
    .map_err(user_error)?;

    std::thread::spawn(move || run_scan(app, scan_id, session, scan_mode));

    Ok(StartScanResponse {
        scan_id,
        session_directory: session_directory.display().to_string(),
    })
}

fn run_scan(app: AppHandle, scan_id: Uuid, session: Arc<ScanSession>, scan_mode: ScanMode) {
    let started = Instant::now();
    let mut source = match photorescue_platform::open_volume_read_only(
        &session.source.device_path,
        session.source.total_bytes,
        session.source.sector_size,
    ) {
        Ok(source) => source,
        Err(error) => {
            let message = format!(
                "Não foi possível ler a unidade. Reinicie como administrador. Detalhe: {error}"
            );
            let _ = session.logger.error(&message);
            let _ = session.index.finish_scan(scan_id, "failed", Utc::now());
            let _ = emit_scan_status(
                &app,
                scan_id,
                0,
                session.source.total_bytes,
                ScanCounts::default(),
                scan_mode,
                "failed",
                message,
            );
            return;
        }
    };

    let config = ScanConfig::for_mode(scan_mode);
    let _ = session.logger.info(format!(
        "Configuracao de varredura: bloco={} bytes; max_arquivo={} bytes; parciais={}; tolera_erros={}; pular_intervalos={}",
        config.chunk_size,
        config.max_file_size,
        config.allow_partial,
        config.tolerate_read_errors,
        config.skip_carved_ranges
    ));
    let scanner = SignatureScanner::new(scan_id, config);
    let cancel = session.cancel.clone();
    let result = scanner.scan(
        &mut source,
        session.source.total_bytes,
        |event| match event {
            ScanEvent::Candidate(candidate) => {
                if let Err(error) = session.index.save_candidate(&candidate) {
                    let _ = session.logger.error(format!(
                        "Falha ao indexar candidato em 0x{:X}: {error}",
                        candidate.offset
                    ));
                    return;
                }
                let _ = session.logger.info(format!(
                    "Candidato {} em 0x{:X}, {} bytes, status={}, confianca={}%",
                    candidate.format,
                    candidate.offset,
                    candidate.estimated_size,
                    candidate.status.as_str(),
                    candidate.confidence
                ));
                let _ = app.emit("candidate-found", candidate);
            }
            ScanEvent::Progress {
                bytes_scanned,
                total_bytes,
                counts,
            } => {
                let _ = emit_scan_status(
                    &app,
                    scan_id,
                    bytes_scanned,
                    total_bytes,
                    counts,
                    scan_mode,
                    "scanning",
                    "Lendo setores e procurando assinaturas de imagem…".to_owned(),
                );
            }
            ScanEvent::ReadError { offset, message } => {
                let _ = session.logger.error(format!(
                    "Erro de leitura em 0x{offset:X}; continuando no modo {}: {message}",
                    scan_mode.as_str()
                ));
            }
        },
        || cancel.load(Ordering::Relaxed),
    );

    match result {
        Ok(summary) => {
            let status = if summary.cancelled {
                "cancelled"
            } else {
                "completed"
            };
            let message = if summary.cancelled {
                "Varredura cancelada com segurança.".to_owned()
            } else {
                format!(
                    "Varredura concluída: {} candidato(s) encontrado(s).",
                    summary.counts.found_count
                )
            };
            let elapsed = started.elapsed();
            let _ = session.logger.info(format!(
                "{} completos={}; parciais={}; corrompidos={}; erros_leitura={}; duracao_aprox={:.1}s",
                message,
                summary.counts.complete_count,
                summary.counts.partial_count,
                summary.counts.corrupted_count,
                summary.counts.read_error_count,
                elapsed.as_secs_f64()
            ));
            let _ = session.index.finish_scan(scan_id, status, Utc::now());
            let _ = emit_scan_status(
                &app,
                scan_id,
                summary.bytes_scanned,
                session.source.total_bytes,
                summary.counts,
                scan_mode,
                status,
                message,
            );
        }
        Err(error) => {
            let message = format!("A varredura foi interrompida: {error}");
            let _ = session.logger.error(&message);
            let _ = session.index.finish_scan(scan_id, "failed", Utc::now());
            let _ = emit_scan_status(
                &app,
                scan_id,
                0,
                session.source.total_bytes,
                ScanCounts::default(),
                scan_mode,
                "failed",
                message,
            );
        }
    }
}

fn emit_scan_status(
    app: &AppHandle,
    scan_id: Uuid,
    bytes_scanned: u64,
    total_bytes: u64,
    counts: ScanCounts,
    scan_mode: ScanMode,
    status: &str,
    message: String,
) -> Result<(), tauri::Error> {
    app.emit(
        "scan-progress",
        ScanProgressPayload {
            scan_id,
            bytes_scanned,
            total_bytes,
            found_count: counts.found_count,
            complete_count: counts.complete_count,
            partial_count: counts.partial_count,
            corrupted_count: counts.corrupted_count,
            read_error_count: counts.read_error_count,
            scan_mode,
            status: status.to_owned(),
            message,
        },
    )
}

#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>, scan_id: Uuid) -> Result<(), String> {
    state.get(scan_id)?.cancel.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub fn list_candidates(
    state: State<'_, AppState>,
    scan_id: Uuid,
) -> Result<Vec<RecoveryCandidate>, String> {
    state
        .get(scan_id)?
        .index
        .list_candidates(scan_id)
        .map_err(user_error)
}

#[tauri::command]
pub fn preview_candidate(
    state: State<'_, AppState>,
    scan_id: Uuid,
    candidate_id: Uuid,
) -> Result<PreviewData, String> {
    const MAX_PREVIEW_BYTES: u64 = 20 * 1024 * 1024;
    let session = state.get(scan_id)?;
    let candidate = session
        .index
        .find_candidate(scan_id, candidate_id)
        .map_err(user_error)?
        .ok_or_else(|| "O arquivo candidato não foi encontrado.".to_owned())?;
    if candidate.estimated_size > MAX_PREVIEW_BYTES {
        return Err("A imagem é grande demais para pré-visualização em memória.".to_owned());
    }

    let mut source = photorescue_platform::open_volume_read_only(
        &session.source.device_path,
        session.source.total_bytes,
        session.source.sector_size,
    )
    .map_err(user_error)?;
    source
        .seek(SeekFrom::Start(candidate.offset))
        .map_err(user_error)?;
    let mut data = vec![0_u8; candidate.estimated_size as usize];
    source.read_exact(&mut data).map_err(user_error)?;
    Ok(PreviewData {
        data_url: format!(
            "data:{};base64,{}",
            candidate.format.mime_type(),
            STANDARD.encode(data)
        ),
    })
}

#[tauri::command]
pub fn recover_candidates(
    app: AppHandle,
    state: State<'_, AppState>,
    request: RecoverRequest,
) -> Result<(), String> {
    let session = state.get(request.scan_id)?;
    let destination = PathBuf::from(&request.destination);
    if !destination.is_absolute() {
        return Err("Escolha uma pasta de destino com caminho absoluto.".to_owned());
    }
    ensure_different_volume(&session.source.root_path, &destination).map_err(user_error)?;

    let all_candidates = session
        .index
        .list_candidates(request.scan_id)
        .map_err(user_error)?;
    let selected: Vec<_> = all_candidates
        .into_iter()
        .filter(|candidate| request.candidate_ids.contains(&candidate.id))
        .collect();
    if selected.is_empty() {
        return Err("Selecione ao menos um arquivo para recuperar.".to_owned());
    }

    std::thread::spawn(move || run_recovery(app, request.scan_id, session, selected, destination));
    Ok(())
}

fn run_recovery(
    app: AppHandle,
    scan_id: Uuid,
    session: Arc<ScanSession>,
    candidates: Vec<RecoveryCandidate>,
    destination: PathBuf,
) {
    let total = candidates.len();
    let mut source = match photorescue_platform::open_volume_read_only(
        &session.source.device_path,
        session.source.total_bytes,
        session.source.sector_size,
    ) {
        Ok(source) => source,
        Err(error) => {
            let _ = app.emit(
                "recovery-progress",
                RecoveryProgressPayload {
                    scan_id,
                    completed: total,
                    total,
                    candidate_id: None,
                    success: false,
                    message: format!("Não foi possível reabrir a origem: {error}"),
                },
            );
            return;
        }
    };

    for (index, candidate) in candidates.iter().enumerate() {
        let result = RecoveryService::recover_one(&mut source, candidate, &destination);
        let (success, message) = match result {
            Ok(recovered) => {
                let _ = session.index.update_candidate_status(
                    candidate.id,
                    CandidateStatus::Recovered,
                    Some(&recovered.path),
                );
                let message = format!(
                    "{} recuperado com {} bytes (SHA-256 {}).",
                    recovered.path.display(),
                    recovered.bytes_written,
                    recovered.sha256
                );
                let _ = session.logger.info(&message);
                (true, message)
            }
            Err(error) => {
                let _ = session.index.update_candidate_status(
                    candidate.id,
                    CandidateStatus::Failed,
                    None,
                );
                let message = format!(
                    "Falha ao recuperar o candidato em 0x{:X}: {error}",
                    candidate.offset
                );
                let _ = session.logger.error(&message);
                (false, message)
            }
        };

        let _ = app.emit(
            "recovery-progress",
            RecoveryProgressPayload {
                scan_id,
                completed: index + 1,
                total,
                candidate_id: Some(candidate.id),
                success,
                message,
            },
        );
    }
}

fn user_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn supported_filters() -> String {
    ImageFormat::ALL
        .iter()
        .map(|format| format.extension())
        .collect::<Vec<_>>()
        .join(",")
}
