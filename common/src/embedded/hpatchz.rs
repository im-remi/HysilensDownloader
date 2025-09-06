
use std::{path::PathBuf, sync::OnceLock};

#[derive(Debug, thiserror::Error)]
pub enum PatchError {
    #[error("{0} doesn't exist, skipping")]
    NotFound(String),
    #[error("Embedded hpatchz extraction failed: {0}")]
    EmbeddedExtractionFailed(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to run patch command for file {0}")]
    PatchCommandFailed(String)
}

pub struct HPatchz {
    executable: PathBuf
}

static HPATCHZ_INSTANCE: OnceLock<HPatchz> = OnceLock::new();

impl HPatchz {
    pub fn new() -> Result<Self, PatchError> {
        let executable = Self::extract_embedded_binary()?;
        Ok(Self {
            executable
        })
    }
    
    pub fn patch(&self, source_file: &PathBuf, patch_file: &PathBuf, target_file: &PathBuf) -> Result<(), PatchError> {
        if !patch_file.exists() {
            return Err(PatchError::NotFound(format!(
                "Patch file not found: {}",
                patch_file.display()
            )));
        }
        
        let output = crate::utils::run_command_with_nixos_wrapper(
            &self.executable,
            &[
                &source_file.display().to_string(),
                &patch_file.display().to_string(),
                &target_file.display().to_string(),
                "-f",
            ],
        );

        
        if let Ok(out) = output {
            if out.status.success() {
                self.remove_file(&patch_file);
                if source_file != target_file && source_file.exists() {
                    self.remove_file(&source_file);
                }
            } else {
                return Err(PatchError::PatchCommandFailed(format!(
                    "{}, exited with code {:?}",
                    source_file.display(),
                    out.status.code()
                )));
            }
        } else {
            return Err(PatchError::PatchCommandFailed(source_file.display().to_string()));
        }
        Ok(())
    }
    
    pub fn remove_file(&self, path: &PathBuf) {
        if let Err(e) = std::fs::remove_file(path) {
            eprintln!("Warning: failed to remove {}: {}", path.display(), e);
        }
    } 
    
    pub fn instance() -> Result<&'static Self, PatchError> {
        HPATCHZ_INSTANCE.get_or_try_init(Self::new)
    }
    
    fn extract_embedded_binary() -> Result<PathBuf, PatchError> {
        const BINARY: &[u8] = include_bytes!("../../../bins/hpatchz");
        
        let temp_path = crate::utils::get_temp_files_path().map_err(|e| PatchError::EmbeddedExtractionFailed(format!("Failed to get temp path: {}", e)))?;
        
        let temp_dir = PathBuf::from(&temp_path);
        let exe_path = temp_dir.join("hpatchz");
        
        std::fs::write(&exe_path, BINARY).map_err(|e| 
            PatchError::EmbeddedExtractionFailed(format!("Failed to write hpatchz: {}", e))
        )?;
        
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