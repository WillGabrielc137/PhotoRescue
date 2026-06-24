use photorescue_domain::VolumeInfo;
use photorescue_index::ScanIndex;
use photorescue_logging::ScanLogger;
use std::collections::HashMap;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use uuid::Uuid;

pub struct ScanSession {
    pub source: VolumeInfo,
    pub index: ScanIndex,
    pub logger: ScanLogger,
    pub cancel: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct AppState {
    sessions: Mutex<HashMap<Uuid, Arc<ScanSession>>>,
}

impl AppState {
    pub fn insert(&self, id: Uuid, session: Arc<ScanSession>) -> Result<(), String> {
        self.sessions
            .lock()
            .map_err(|_| "O estado interno ficou indisponível.".to_owned())?
            .insert(id, session);
        Ok(())
    }

    pub fn get(&self, id: Uuid) -> Result<Arc<ScanSession>, String> {
        self.sessions
            .lock()
            .map_err(|_| "O estado interno ficou indisponível.".to_owned())?
            .get(&id)
            .cloned()
            .ok_or_else(|| "Sessão de varredura não encontrada.".to_owned())
    }
}
