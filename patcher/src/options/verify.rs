use std::{io::{BufRead, BufReader}, path::Path};
use rayon::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct FileEntry {
    #[serde(rename = "remoteName")]
    remote_name: String,
    md5: String,
    #[serde(rename = "fileSize")]
    file_size: u64,
}

pub fn verify_files(client_folder: &Path) -> std::io::Result<()> {
    let pkg_version_path = client_folder.join("pkg_version");
    let file = std::fs::File::open(&pkg_version_path)?;
    let reader = BufReader::new(file);

    let lines: Vec<_> = reader.lines().collect::<Result<_, _>>()?;
    let pb = common::utils::create_progress_bar(lines.len());

    let all_ok = lines.into_par_iter().enumerate().map(|(_, line)| {
        let entry: FileEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(e) => {
                pb.suspend(|| eprintln!("Failed to parse JSON: {}", e));
                pb.inc(1);
                return false;
            }
        };

        let file_path = client_folder.join(&entry.remote_name);
        if !file_path.exists() {
            pb.suspend(|| eprintln!("Missing file: {}", file_path.display()));
            pb.inc(1);
            return false;
        }

        let metadata = match std::fs::metadata(&file_path) {
            Ok(m) => m,
            Err(e) => {
                pb.suspend(|| eprintln!("Failed to get metadata: {} ({})", file_path.display(), e));
                pb.inc(1);
                return false;
            }
        };

        if metadata.len() != entry.file_size {
            pb.suspend(|| eprintln!(
                "File size mismatch: {} (expected {}, got {})",
                file_path.display(), entry.file_size, metadata.len()
            ));
        }

        let hash_result = match common::md5::calculate_md5(&file_path) {
            Ok(h) => h,
            Err(e) => {
                pb.suspend(|| eprintln!("Failed to calculate MD5: {} ({})", file_path.display(), e));
                pb.inc(1);
                return false;
            }
        };

        if !hash_result.eq_ignore_ascii_case(&entry.md5) {
            pb.suspend(|| eprintln!(
                "MD5 mismatch: {} (expected {}, got {})",
                file_path.display(), entry.md5, hash_result
            ));
            pb.inc(1);
            return false;
        }

        pb.inc(1);
        true
    }).reduce(|| true, |a, b| a & b);

    if all_ok {
        println!("✔ All files verified successfully!");
    } else {
        println!("✖ Some files failed verification. Check errors above.");
    }

    Ok(())
}
