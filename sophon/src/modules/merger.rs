use std::{fs::OpenOptions, io::Write};

use super::*;

#[derive(Clone, Copy)]
pub struct SophonMerger;

impl SophonMerger {
    pub fn merge_chunks(&self, chunks: &[&[u8]], target_path: &str) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(target_path)?;
    
        for chunk in chunks {
            file.write_all(chunk)?;
        }
    
        Ok(())
    }
}