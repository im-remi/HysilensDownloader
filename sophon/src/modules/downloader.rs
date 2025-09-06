use std::path::Path;

use reqwest::Client;
use super::*;

#[derive(Clone)]
pub struct SophonDownloader {
    client: Client
}

impl SophonDownloader {
    pub fn new() -> Self {
        Self {
            client: Client::new()
        }
    }
    async fn download_and_extract(
        &self,
        url_base: &str,
        file_name: &str,
        extract_path: &str,
        cleanup: bool, 
    ) -> Result<Vec<u8>> {
        let url = format!("{}/{}", url_base, file_name);
        let bytes = self.client.get(&url).send().await?.bytes().await?;
    
        let tmp_path = dirs::cache_dir().unwrap().join(file_name);
        std::fs::write(&tmp_path, &bytes)?;
    
        let extract_path = Path::new(&extract_path);
        crate::SevenZip::instance()?.extract_to(&tmp_path, extract_path)?;
        std::fs::remove_file(&tmp_path)?;
    
        let extracted_file_path = extract_path.join(format!("{}~", file_name));
        let data = std::fs::read(&extracted_file_path)?;
    
        if cleanup {
            std::fs::remove_file(&extracted_file_path)?;
        }
    
        Ok(data)
    }

    pub async fn download_and_extract_manifest(
        &self,
        manifest_url: &str,
        manifest_file: &str,
        extract_path: &str,
        cleanup: bool
    ) -> Result<Vec<u8>> {
        self.download_and_extract(manifest_url, manifest_file, extract_path, cleanup).await
    }

    pub async fn download_and_extract_chunk(
        &self,
        chunk_url: &str,
        chunk_file: &str,
        extract_path: &str,
        cleanup: bool
    ) -> Result<Vec<u8>> {
        self.download_and_extract(chunk_url, chunk_file, extract_path, cleanup).await
    }
    
    pub async fn download_chunk(
        &self,
        chunk_url: &str,
        chunk_file: &str,
        download_path: &str,
        cleanup: bool
    ) -> Result<Vec<u8>> {
        let download_path = Path::new(download_path).join("ldiff/");
        let file_path = download_path.join(chunk_file);
        if file_path.exists() {
            return Ok(vec![]);
        }
        let url = format!("{}/{}", chunk_url, chunk_file);

        let bytes = self.client.get(&url).send().await?.bytes().await?;
        
        std::fs::create_dir_all(download_path)?; 

        if !cleanup{
            std::fs::write(&file_path, &bytes)?;
        }
        
        Ok(bytes.to_vec())
    }
}
      