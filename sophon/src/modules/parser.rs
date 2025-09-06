use prost::Message;

use crate::{sophon_manifest::SophonManifestProto, sophon_patch::SophonPatchProto};

use super::*;

pub struct SophonParser;

pub enum Manifest {
    Full(SophonManifestProto),
    Diff(SophonPatchProto),
}

impl SophonParser {
    pub fn new() -> Self {
        Self {}
    }
    
    pub fn parse_manifest_file(&self, bytes: Vec<u8>) -> Result<Manifest> {
        if let Ok(full) = SophonManifestProto::decode(&*bytes) {
            println!("Detected full manifest format");
            return Ok(Manifest::Full(full));
        }
    
        if let Ok(diff) = SophonPatchProto::decode(&*bytes) {
            println!("Detected patch manifest format");
            return Ok(Manifest::Diff(diff));
        }
    
        anyhow::bail!("Failed to parse manifest file: unknown format");
    }

}