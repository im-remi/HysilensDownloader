use anyhow::Result;
use std::env;
use std::path::Path;
use sophon::SophonClient;
use sophon::utils::read_input;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    let manifest_url = args.get(1).cloned().unwrap_or_else(|| read_input("Enter manifest URL: "));
    let manifest_file = args.get(2).cloned().unwrap_or_else(|| read_input("Enter manifest file name: "));
    let chunk_url = args.get(3).cloned().unwrap_or_else(|| read_input("Enter chunk URL: "));
    let output_dir = args.get(4).cloned().unwrap_or_else(|| read_input("Enter output directory: "));
    
    if !Path::new(&output_dir).exists() {
        std::fs::create_dir_all(&output_dir)?;
    }

    let client = SophonClient::new(&manifest_url, &manifest_file, &chunk_url);
    client.download_game(&output_dir).await?;

    println!("Download complete!");
    Ok(())
}
