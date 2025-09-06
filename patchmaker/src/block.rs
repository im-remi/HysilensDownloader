use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::io::Write;
use std::sync::{Arc, Mutex};

use common::embedded::HDiff;

#[derive(Debug, Serialize)]
pub struct BlockPatchEntry {
    pub source_file_name: String,
    pub source_file_md5: String,
    pub source_file_size: u64,
    pub target_file_name: String,
    pub target_file_md5: String,
    pub target_file_size: u64,
    pub patch_file_name: String,
    pub patch_file_md5: String,
    pub patch_file_size: u64,
}

pub fn generate_block_map(
    old_files: &HashMap<String, crate::scan::FileMeta>,
    new_files: &HashMap<String, crate::scan::FileMeta>,
    output_dir: &Path,
    use_faster_check: bool
) -> std::io::Result<Vec<BlockPatchEntry>> {
    let mut used_targets = HashSet::new();
    let mut result = Vec::new();

    let filtered_new: Vec<_> = new_files
        .iter()
        .filter(|(p, _)| p.ends_with(".block") 
            && p.contains("StreamingAssets/Asb")
            && !old_files.contains_key(*p)) 
        .collect();

    
    let old_blocks: Vec<_> = old_files
        .iter()
        .filter(|(rel, old_meta)| {
            rel.ends_with(".block")
                && rel.contains("StreamingAssets/Asb")
                && new_files
                    .get(*rel)
                    .map(|new_meta| old_meta.md5 != new_meta.md5)
                    .unwrap_or(true)
        })
        .collect();

    println!("Patching .block files...");
    let pb = common::utils::create_progress_bar(old_blocks.len());
    let delete_path = output_dir.join("deletefiles.txt");
    let mut delete_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(delete_path)?;

    for (rel, old_meta) in old_blocks
    {
        if let Some(new_meta) = new_files.get(rel) {
            if old_meta.md5 == new_meta.md5 {
                used_targets.insert(rel.clone());
                pb.inc(1);
                continue;
            }
        }
        let mut hdiff_to_delete = Vec::new();
        match find_best_patch_candidate(old_meta, &filtered_new, &used_targets, output_dir, &mut hdiff_to_delete, use_faster_check) {
            Some((new_rel, new_meta, patch_rel, patch_file_size)) => {
                if patch_file_size > new_meta.size {
                    let _ = std::fs::remove_file(output_dir.join(&rel));
                    for bad_patch in hdiff_to_delete {
                        let _ = std::fs::remove_file(bad_patch);
                    }
            
                    std::fs::copy(&new_meta.full_path, output_dir.join(&new_rel))?;
                    used_targets.insert(new_rel);
                } else {
                    let patch_md5 = common::md5::calculate_md5(&output_dir.join(&patch_rel))
                        .map_err(|e| std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("MD5 error: {}", e)
                        ))?;
                
                    result.push(BlockPatchEntry {
                        source_file_name: rel.clone(),
                        source_file_md5: old_meta.md5.clone(),
                        source_file_size: old_meta.size,
                        target_file_name: new_rel.clone(),
                        target_file_md5: new_meta.md5.clone(),
                        target_file_size: new_meta.size,
                        patch_file_name: patch_rel.clone(),
                        patch_file_md5: patch_md5,
                        patch_file_size,
                    });
            
                    for bad_patch in hdiff_to_delete.iter().filter(|p| *p != &output_dir.join(&patch_rel)) {
                        let _ = std::fs::remove_file(bad_patch);
                    }
                    used_targets.insert(new_rel);
                }
            }
            None => {
                writeln!(delete_file, "{}", rel)?;
            }
        }
        pb.inc(1);
    }
    
    pb.finish();

    for (new_rel, new_meta) in &filtered_new {
        if used_targets.contains(*new_rel) {
            continue;
        }

        let patch_path = output_dir.join(&new_rel);
        if let Some(parent) = patch_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&new_meta.full_path, &patch_path)?;
    }

    Ok(result)
}

fn find_best_patch_candidate<'a>(
    old_meta: &'a crate::scan::FileMeta,
    filtered_new: &[(&'a String, &'a crate::scan::FileMeta)],
    used_targets: &HashSet<String>,
    output_dir: &Path,
    hdiff_to_delete: &mut Vec<PathBuf>,
    use_faster_check: bool,
) -> Option<(String, &'a crate::scan::FileMeta, String, u64)> {
    let best_candidate;

    let mut candidates: Vec<_> = filtered_new
        .iter()
        .filter(|(new_rel, _)| !used_targets.contains(*new_rel))
        .collect();

    candidates.sort_by_key(|(_, new_meta)| (new_meta.size as i64 - old_meta.size as i64).abs());

    let (take_top_n, take_for_hdiff) = if use_faster_check { (15, 2) } else { (50, 5) };

    let top_candidates: Vec<_> = candidates.iter().take(take_top_n).collect();
    
    use rayon::prelude::*;

    let mut candidates_with_similarity: Vec<_> = top_candidates
        .par_iter()
        .map(|(new_rel, new_meta)| {
            let similarity = crate::utils::block_diff_percent_experimental(
                &old_meta.full_path,
                &new_meta.full_path,
            )
            .unwrap_or(f64::MAX);
            (*new_rel, *new_meta, similarity)
        })
        .collect();

    candidates_with_similarity.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

    let hdiff_collector = Arc::new(Mutex::new(Vec::new()));
    let results: Vec<_> = candidates_with_similarity
        .into_par_iter()
        .take(take_for_hdiff)
        .filter_map(|(new_rel, new_meta, _)| {
            let size_diff = (new_meta.size as i64 - old_meta.size as i64).abs();
            if size_diff > 5_000_000 {
                return None;
            }
    
            let patch_rel = format!("{new_rel}.hdiff");
            let patch_path = output_dir.join(&patch_rel);
    
            let hdiff = HDiff::instance().ok()?;
            if hdiff.diff(&old_meta.full_path, &new_meta.full_path, &patch_path).is_err() {
                return None;
            }
            hdiff_collector.lock().unwrap().push(patch_path.clone());
    
            let patch_size = std::fs::metadata(&patch_path).ok()?.len();
    
            Some(((*new_rel).clone(), new_meta, patch_rel, patch_size))
        })
        .collect();
    
    hdiff_to_delete.extend(hdiff_collector.lock().unwrap().drain(..));
    best_candidate = results.into_iter().min_by_key(|(_, _, _, size)| *size);

    best_candidate
}
