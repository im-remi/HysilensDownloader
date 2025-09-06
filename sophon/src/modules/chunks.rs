use std::path::Path;
use std::sync::Arc;
use futures::{stream::FuturesUnordered, StreamExt};
use tokio::sync::Semaphore;

use crate::{modules::SophonDownloader, sophon_manifest::SophonManifestProto, sophon_patch::SophonPatchProto};
use super::*;

pub struct SophonChunks {
    downloader: SophonDownloader,
    merger: SophonMerger,
    chunk_url: String,
}

impl SophonChunks {
    pub fn new(downloader: SophonDownloader, merger: SophonMerger, chunk_url: &str) -> Self {
        Self {
            downloader,
            merger,
            chunk_url: chunk_url.to_string(),
        }
    }

    pub async fn parse_manifest_proto(&self, proto: SophonManifestProto, output_dir: &str) -> Result<()> {
        let pb = crate::utils::create_progress_bar(proto.assets.len());
    
        let asset_semaphore = Arc::new(Semaphore::new(5));
    
        let mut asset_futures = FuturesUnordered::new();
    
        for asset in proto.assets {
            let asset_semaphore = asset_semaphore.clone();
            let downloader = &self.downloader;
            let merger = &self.merger;
            let chunk_url = self.chunk_url.clone();
            let output_dir = output_dir.to_string();
            let pb = pb.clone();
    
            asset_futures.push(async move {
                let _asset_permit = asset_semaphore.acquire_owned().await.unwrap();
    
                let target_path = format!("{}/{}", output_dir, asset.asset_name);
                let target_path_obj = Path::new(&target_path);
    
                if target_path_obj.exists() { 
                    if let Ok(metadata) = target_path_obj.metadata() {
                        if metadata.len() == asset.asset_size as u64 {
                            return Ok::<_, anyhow::Error>(());
                        }
                    }
                }
                
                if asset.asset_type != 0 {
                    std::fs::create_dir_all(target_path_obj)?;
                    pb.inc(1);
                    return Ok(());
                }
    
                let chunk_semaphore = Arc::new(Semaphore::new(10)); 
                let mut chunk_futures = FuturesUnordered::new();
    
                for chunk in &asset.asset_chunks {
                    let chunk_name = chunk.chunk_name.clone();
                    let offset = chunk.chunk_on_file_offset;
                    let chunk_url = chunk_url.clone();
                    let output_dir = output_dir.clone();
                    let downloader = downloader.clone();
                    let chunk_semaphore = chunk_semaphore.clone();
    
                    chunk_futures.push(async move {
                        let _permit = chunk_semaphore.acquire_owned().await.unwrap();
                        let bytes = downloader
                            .download_and_extract_chunk(&chunk_url, &chunk_name, &output_dir, true)
                            .await?;
                        Ok::<_, anyhow::Error>((offset, bytes))
                    });
                }
    
                let mut downloaded_chunks = Vec::new();
                while let Some(result) = chunk_futures.next().await {
                    downloaded_chunks.push(result?);
                }
    
                downloaded_chunks.sort_by_key(|(offset, _)| *offset);
                let chunk_refs: Vec<&[u8]> = downloaded_chunks.iter().map(|(_, bytes)| bytes.as_slice()).collect();
    
                if let Some(parent_dir) = target_path_obj.parent() {
                    std::fs::create_dir_all(parent_dir)?;
                }
    
                merger.merge_chunks(&chunk_refs, &target_path)?;
                pb.inc(1);
    
                Ok::<(), anyhow::Error>(())
            });
        }
    
        while let Some(asset_result) = asset_futures.next().await {
            asset_result?;
        }
    
        Ok(())
    }
    
    pub async fn parse_manifest_diff_proto(&self, proto: SophonPatchProto, output_dir: &str) -> Result<()> {
        use std::collections::HashSet;
        
        let mut version_tags = HashSet::new();
        for asset in &proto.patch_assets {
            if !asset.asset_infos.is_empty() {
                for chunk in &asset.asset_infos {
                    version_tags.insert(chunk.version_tag.clone());
                }
                break
            }
        }
        
        println!("Available version tags:");
        for tag in &version_tags {
            println!("- {}", tag);
        }
        let version_tag = crate::utils::read_input("Enter version tag: ");
        let total_chunks: usize = proto
                .patch_assets
                .iter()
                .flat_map(|asset| &asset.asset_infos)
                .filter(|chunk| chunk.version_tag == version_tag)
                .count();
        
        let pb = crate::utils::create_progress_bar(total_chunks);
        
        for asset in &proto.patch_assets {
            for chunk in &asset.asset_infos {
                if chunk.version_tag != version_tag {
                    continue;
                }
                if let Some(chunk_inner) = &chunk.chunk {
                    let _ = self.downloader.download_chunk(&self.chunk_url, &chunk_inner.patch_name, output_dir, false).await?;
                }
                pb.inc(1);
            }
            
        }
        Ok(())
    }
}
