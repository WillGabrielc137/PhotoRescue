use chrono::Utc;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LogError {
    #[error("falha ao abrir o arquivo de log: {0}")]
    Open(#[source] std::io::Error),
    #[error("falha ao gravar o log: {0}")]
    Write(#[source] std::io::Error),
    #[error("o serviço de log ficou indisponível")]
    Poisoned,
}

#[derive(Clone)]
pub struct ScanLogger {
    file: Arc<Mutex<File>>,
}

impl ScanLogger {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, LogError> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(LogError::Open)?;
        Ok(Self {
            file: Arc::new(Mutex::new(file)),
        })
    }

    pub fn info(&self, message: impl AsRef<str>) -> Result<(), LogError> {
        self.write("INFO", message.as_ref())
    }

    pub fn error(&self, message: impl AsRef<str>) -> Result<(), LogError> {
        self.write("ERROR", message.as_ref())
    }

    fn write(&self, level: &str, message: &str) -> Result<(), LogError> {
        let safe_message = message.replace(['\r', '\n'], " ");
        let line = format!("{} [{}] {}\n", Utc::now().to_rfc3339(), level, safe_message);
        let mut file = self.file.lock().map_err(|_| LogError::Poisoned)?;
        file.write_all(line.as_bytes()).map_err(LogError::Write)?;
        file.flush().map_err(LogError::Write)
    }
}
