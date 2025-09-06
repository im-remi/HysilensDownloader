
use std::{path::{Path, PathBuf}, sync::OnceLock};

#[derive(Debug, thiserror::Error)]
pub enum PatchError {
    #[error("Embedded hpatchz extraction failed: {0}")]
    EmbeddedExtractionFailed(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to run patch command for file {0}")]
    PatchCommandFailed(String)
}

pub struct HDiff {
    executable: PathBuf
}

static HDIFFZ_INSTANCE: OnceLock<HDiff> = OnceLock::new();

impl HDiff {
    pub fn new() -> Result<Self, PatchError> {
        let executable = Self::extract_embedded_binary()?;
        Ok(Self {
            executable
        })
    }
    pub fn diff(&self, old_path: &Path, new_path: &Path, out_patch: &Path) -> Result<(), PatchError> {
       let args = [old_path.to_str().unwrap(), new_path.to_str().unwrap(), out_patch.to_str().unwrap()];
    
       let output = crate::utils::run_command_with_nixos_wrapper(&self.executable, &args)
           .map_err(|_| PatchError::PatchCommandFailed(out_patch.display().to_string()))?;
    
       if !output.status.success() {
           return Err(PatchError::PatchCommandFailed(out_patch.display().to_string()));
       }
    
       Ok(())
    }
    
    pub fn instance() -> Result<&'static Self, PatchError> {
        HDIFFZ_INSTANCE.get_or_try_init(Self::new)
    }
    
    fn extract_embedded_binary() -> Result<PathBuf, PatchError> {
        const BINARY: &[u8] = include_bytes!("../../../bins/hdiffz");
        
        let temp_path = crate::utils::get_temp_files_path().map_err(|e| PatchError::EmbeddedExtractionFailed(format!("Failed to get temp path: {}", e)))?;
        
        let temp_dir = PathBuf::from(&temp_path);
        let exe_path = temp_dir.join("hdiffz");
        
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