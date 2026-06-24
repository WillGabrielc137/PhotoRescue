use photorescue_domain::VolumeInfo;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::process::{Command, Stdio};
use thiserror::Error;

#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const FALLBACK_SECTOR_SIZE: u32 = 4096;
#[cfg(windows)]
const DRIVE_REMOVABLE: u32 = 2;

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetDiskFreeSpaceW(
        root_path_name: *const u16,
        sectors_per_cluster: *mut u32,
        bytes_per_sector: *mut u32,
        number_of_free_clusters: *mut u32,
        total_number_of_clusters: *mut u32,
    ) -> i32;

    fn GetDriveTypeW(root_path_name: *const u16) -> u32;
}

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("o PhotoRescue ainda oferece acesso bruto apenas no Windows")]
    UnsupportedPlatform,
    #[error("não foi possível abrir {device} em modo somente leitura: {source}")]
    OpenDevice {
        device: String,
        #[source]
        source: std::io::Error,
    },
    #[error("não foi possível solicitar elevação: {0}")]
    Elevation(#[source] std::io::Error),
}

pub fn list_volumes() -> Result<Vec<VolumeInfo>, PlatformError> {
    #[cfg(not(windows))]
    {
        Err(PlatformError::UnsupportedPlatform)
    }

    #[cfg(windows)]
    {
        let mut volumes = Vec::new();
        for letter in b'A'..=b'Z' {
            let root = format!("{}:\\", letter as char);
            let path = Path::new(&root);
            if !path.exists() {
                continue;
            }
            let total_bytes = fs2::total_space(path).unwrap_or(0);
            let sector_size = volume_sector_size(path).unwrap_or(FALLBACK_SECTOR_SIZE);
            let is_removable = volume_is_removable(path);
            let id = format!("{}:", letter as char);
            volumes.push(VolumeInfo {
                id: id.clone(),
                root_path: root,
                device_path: format!(r"\\.\{id}"),
                display_name: format!("Unidade {id}"),
                total_bytes,
                sector_size,
                is_removable,
            });
        }
        Ok(volumes)
    }
}

pub fn open_volume_read_only(
    device_path: &str,
    total_bytes: u64,
    sector_size: u32,
) -> Result<AlignedVolumeReader<File>, PlatformError> {
    let file = OpenOptions::new()
        .read(true)
        .write(false)
        .open(device_path)
        .map_err(|source| PlatformError::OpenDevice {
            device: device_path.to_owned(),
            source,
        })?;
    Ok(AlignedVolumeReader::new(
        file,
        total_bytes,
        sector_size.max(1) as u64,
    ))
}

pub struct AlignedVolumeReader<R> {
    inner: R,
    position: u64,
    length: u64,
    alignment: u64,
    cache_start: u64,
    cache_valid: usize,
    cache: Vec<u8>,
}

impl<R: Read + Seek> AlignedVolumeReader<R> {
    pub fn new(inner: R, length: u64, alignment: u64) -> Self {
        Self {
            inner,
            position: 0,
            length,
            alignment: alignment.max(1),
            cache_start: 0,
            cache_valid: 0,
            cache: Vec::new(),
        }
    }

    fn refill_cache(&mut self, requested: usize) -> std::io::Result<()> {
        let aligned_start = align_down(self.position, self.alignment);
        let leading = self.position.saturating_sub(aligned_start);
        let remaining = self.length.saturating_sub(aligned_start);
        let desired = leading.saturating_add(requested as u64).min(remaining);
        let physical_length = align_up(desired, self.alignment).min(remaining);
        let physical_length = usize::try_from(physical_length).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "bloco de leitura maior que o espaço de endereçamento",
            )
        })?;

        self.cache.resize(physical_length, 0);
        self.inner.seek(SeekFrom::Start(aligned_start))?;

        let mut filled = 0;
        while filled < physical_length {
            let bytes_read = self.inner.read(&mut self.cache[filled..])?;
            if bytes_read == 0 {
                break;
            }
            filled += bytes_read;
        }

        self.cache_start = aligned_start;
        self.cache_valid = filled;
        Ok(())
    }
}

impl<R: Read + Seek> Read for AlignedVolumeReader<R> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if output.is_empty() || self.position >= self.length {
            return Ok(0);
        }

        let wanted = output
            .len()
            .min(self.length.saturating_sub(self.position) as usize);
        let mut copied = 0;

        while copied < wanted {
            let cache_end = self.cache_start.saturating_add(self.cache_valid as u64);
            if self.position < self.cache_start || self.position >= cache_end {
                self.refill_cache(wanted - copied)?;
            }

            let cache_offset = (self.position - self.cache_start) as usize;
            let available = self.cache_valid.saturating_sub(cache_offset);
            if available == 0 {
                break;
            }

            let count = available.min(wanted - copied);
            output[copied..copied + count]
                .copy_from_slice(&self.cache[cache_offset..cache_offset + count]);
            copied += count;
            self.position += count as u64;
        }

        Ok(copied)
    }
}

impl<R: Read + Seek> Seek for AlignedVolumeReader<R> {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        let next = match position {
            SeekFrom::Start(offset) => i128::from(offset),
            SeekFrom::End(delta) => i128::from(self.length) + i128::from(delta),
            SeekFrom::Current(delta) => i128::from(self.position) + i128::from(delta),
        };
        if !(0..=i128::from(self.length)).contains(&next) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "posição de leitura fora dos limites da unidade",
            ));
        }
        self.position = next as u64;
        Ok(self.position)
    }
}

fn align_down(value: u64, alignment: u64) -> u64 {
    value - (value % alignment)
}

fn align_up(value: u64, alignment: u64) -> u64 {
    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value.saturating_add(alignment - remainder)
    }
}

#[cfg(windows)]
fn volume_sector_size(root: &Path) -> Option<u32> {
    let mut wide: Vec<u16> = OsStr::new(root.as_os_str())
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut sectors_per_cluster = 0;
    let mut bytes_per_sector = 0;
    let mut free_clusters = 0;
    let mut total_clusters = 0;
    let success = unsafe {
        GetDiskFreeSpaceW(
            wide.as_mut_ptr(),
            &mut sectors_per_cluster,
            &mut bytes_per_sector,
            &mut free_clusters,
            &mut total_clusters,
        )
    };
    (success != 0 && bytes_per_sector > 0).then_some(bytes_per_sector)
}

#[cfg(windows)]
fn volume_is_removable(root: &Path) -> bool {
    let wide: Vec<u16> = OsStr::new(root.as_os_str())
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe { GetDriveTypeW(wide.as_ptr()) == DRIVE_REMOVABLE }
}

pub fn is_elevated() -> bool {
    #[cfg(not(windows))]
    {
        false
    }

    #[cfg(windows)]
    {
        let script = concat!(
            "$p=[Security.Principal.WindowsPrincipal]",
            "[Security.Principal.WindowsIdentity]::GetCurrent();",
            "if($p.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))",
            "{exit 0}else{exit 1}"
        );
        let mut command = Command::new("powershell.exe");
        command
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command.creation_flags(CREATE_NO_WINDOW);
        command
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

pub fn restart_elevated() -> Result<(), PlatformError> {
    #[cfg(not(windows))]
    {
        Err(PlatformError::UnsupportedPlatform)
    }

    #[cfg(windows)]
    {
        let executable = std::env::current_exe().map_err(PlatformError::Elevation)?;
        let escaped = executable.display().to_string().replace('\'', "''");
        let script = format!("Start-Process -FilePath '{escaped}' -Verb RunAs");
        let mut command = Command::new("powershell.exe");
        command
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-Command",
                &script,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command.creation_flags(CREATE_NO_WINDOW);
        command.spawn().map_err(PlatformError::Elevation)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    struct StrictAlignedSource {
        inner: Cursor<Vec<u8>>,
        alignment: u64,
    }

    impl Read for StrictAlignedSource {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            if !(buffer.len() as u64).is_multiple_of(self.alignment) {
                return Err(std::io::Error::from_raw_os_error(87));
            }
            self.inner.read(buffer)
        }
    }

    impl Seek for StrictAlignedSource {
        fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
            if let SeekFrom::Start(offset) = position {
                if offset % self.alignment != 0 {
                    return Err(std::io::Error::from_raw_os_error(87));
                }
            }
            self.inner.seek(position)
        }
    }

    #[test]
    fn logical_unaligned_reads_use_aligned_physical_io() {
        let data: Vec<u8> = (0..16 * 1024).map(|index| (index % 251) as u8).collect();
        let source = StrictAlignedSource {
            inner: Cursor::new(data.clone()),
            alignment: 512,
        };
        let mut reader = AlignedVolumeReader::new(source, data.len() as u64, 512);
        let offset = 8 * 1024 - 32;
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let mut output = vec![0_u8; 1024];
        reader.read_exact(&mut output).unwrap();
        assert_eq!(output, data[offset..offset + output.len()]);
    }

    #[test]
    fn reads_across_the_end_of_an_aligned_cache_window() {
        let data: Vec<u8> = (0..4096).map(|index| (index % 239) as u8).collect();
        let source = StrictAlignedSource {
            inner: Cursor::new(data.clone()),
            alignment: 512,
        };
        let mut reader = AlignedVolumeReader::new(source, data.len() as u64, 512);
        reader.seek(SeekFrom::Start(500)).unwrap();
        let mut output = vec![0_u8; 700];
        reader.read_exact(&mut output).unwrap();
        assert_eq!(output, data[500..1200]);
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requer uma unidade Windows real e execução como administrador"]
    fn raw_volume_accepts_unaligned_logical_reads() {
        let requested_id =
            std::env::var("PHOTORESCUE_TEST_VOLUME").unwrap_or_else(|_| "D:".to_owned());
        let volume = list_volumes()
            .unwrap()
            .into_iter()
            .find(|volume| volume.id.eq_ignore_ascii_case(&requested_id))
            .expect("unidade de teste não encontrada");
        let mut reader =
            open_volume_read_only(&volume.device_path, volume.total_bytes, volume.sector_size)
                .unwrap();

        reader.seek(SeekFrom::Start(8 * 1024 * 1024 - 32)).unwrap();
        let mut output = vec![0_u8; 4096];
        reader.read_exact(&mut output).unwrap();
        assert_eq!(output.len(), 4096);
    }
}
