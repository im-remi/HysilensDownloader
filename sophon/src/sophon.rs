use std::path::Path;

use crate::modules::{Manifest, SophonChunks, SophonDownloader, SophonMerger, SophonParser};
use anyhow::Result;

pub struct SophonClient {
    downloader: SophonDownloader,
    merger: SophonMerger,
    parser: SophonParser,
    manifest_url: String,
    manifest_file: String,
    chunk_url: String
}

impl SophonClient {
    pub fn new(
        manifest_url: &str,
        manifest_file: &str,
        chunk_url: &str
    ) -> Self {
        Self {
            downloader: SophonDownloader::new(),
            merger: SophonMerger {},
            parser: SophonParser::new(),
            manifest_url: manifest_url.to_string(),
            manifest_file: manifest_file.to_string(),
            chunk_url: chunk_url.to_string()
        }
    }
    
    pub async fn download_game(&self, output_dir: &str) -> Result<()> {
        let manifest = self.downloader.download_and_extract_manifest(&self.manifest_url, &self.manifest_file, &output_dir, false).await?;
        let manifest_proto = self.parser.parse_manifest_file(manifest)?;
        let chunks = SophonChunks::new(self.downloader.clone(), self.merger, &self.chunk_url);
        match manifest_proto {
            Manifest::Full(proto) => {
                let _ = std::fs::remove_file(Path::new(output_dir).join(format!("{}~", self.manifest_file)));
                chunks.parse_manifest_proto(proto, output_dir).await?
            },
            Manifest::Diff(proto) => chunks.parse_manifest_diff_proto(proto, output_dir).await?
        }
        
        Ok(())
    }
    
    
}