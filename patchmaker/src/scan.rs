use std::{collections::HashMap, fs, path::Path};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMeta {
    pub full_path: std::path::PathBuf,
    pub md5: String,
    pub size: u64,
}

pub fn scan_files(root: &Path) -> std::io::Result<HashMap<String, FileMeta>> {
    let entries: Vec<_> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();

    let pb = common::utils::create_progress_bar(entries.len());

    let map: HashMap<_, _> = entries.into_par_iter().filter_map(|entry| {
        let path = entry.path().to_path_buf();
        let rel_path = path.strip_prefix(root).ok()?.to_string_lossy().replace("\\", "/");
        let md5 = common::md5::calculate_md5(&path).ok()?;
        let size = fs::metadata(&path).ok()?.len();
        pb.inc(1);
        Some((rel_path, FileMeta { full_path: path, md5, size }))
    }).collect();

    pb.finish();
    Ok(map)
}

pub fn save_cache(path: &Path, map: &HashMap<String, FileMeta>) -> std::io::Result<()> {
    fs::write(path, serde_json::to_string_pretty(map).unwrap())
}

pub fn load_cache(path: &Path) -> Option<HashMap<String, FileMeta>> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

pub fn load_or_scan(root: &Path, cache_path: &Path) -> std::io::Result<HashMap<String, FileMeta>> {
    if let Some(map) = load_cache(cache_path) {
        Ok(map)
    } else {
        let map = scan_files(root)?;
        save_cache(cache_path, &map)?;
        Ok(map)
    }
}
