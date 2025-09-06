use std::path::PathBuf;
use std::{
    path::Path,
    sync::OnceLock,
};
use thiserror::Error;

static SEVEN_ZIP_INSTANCE: OnceLock<SevenZip> = OnceLock::new();

#[derive(Error, Debug)]
pub enum SevenZipError {
    #[error("7-zip failed to run using Command")]
    CommandError(#[source] std::io::Error),
    #[error("7-zip extraction failed: '{0}'")]
    ExtractionFailed(String),
    #[error("Embedded 7z.exe extraction failed: {0}")]
    EmbeddedExtractionFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Default)]
pub struct SevenZip {
    executable: PathBuf,
}

impl SevenZip {
    pub fn new() -> Result<Self, SevenZipError> {
        let executable = Self::extract_embedded_binary()?;
        Ok(Self { executable })
    }

    pub fn instance() -> Result<&'static SevenZip, SevenZipError> {
        SEVEN_ZIP_INSTANCE.get_or_try_init(SevenZip::new)
    }

    pub fn extract_to(&self, archive: &Path, destination: &Path) -> Result<(), SevenZipError> {
        let output = crate::utils::run_command_with_nixos_wrapper(
            &self.executable,
            &[
                "x",
                &archive.display().to_string(),
                &format!("-o{}", destination.display()),
                "-aoa",
            ],
        )
        .map_err(SevenZipError::CommandError)?;

        if !output.status.success() {
            let stderr_msg = String::from_utf8_lossy(&output.stderr);
            return Err(SevenZipError::ExtractionFailed(stderr_msg.to_string()));
        }

        Ok(())
    }


    fn extract_embedded_binary() -> Result<PathBuf, SevenZipError> {
        const SEVENZ_BIN: &[u8] = include_bytes!("../../../bins/7z");
        
        let temp_path = crate::utils::get_temp_files_path()
            .map_err(|e| SevenZipError::EmbeddedExtractionFailed(format!("Failed to get temp path: {e}")))?;
        
        let temp_dir = PathBuf::from(&temp_path);
        let exe_path = temp_dir.join("7z");
        
        std::fs::write(&exe_path, SEVENZ_BIN).map_err(|e| {
            SevenZipError::EmbeddedExtractionFailed(format!("Failed to write 7z: {e}"))
        })?;
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&exe_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&exe_path, perms)?;
        }
    
        Ok(exe_path)
    }

}
