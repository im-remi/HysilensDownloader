use std::{collections::HashMap, fs, path::Path};
use crate::{block::BlockPatchEntry, scan::FileMeta};
use common::embedded::HDiff;

pub fn process_regular_files(
    old_files: &HashMap<String, FileMeta>,
    new_files: &HashMap<String, FileMeta>,
    output_dir: &Path,
    delete_list: &mut Vec<String>,
    hdiff_entries: &mut Vec<crate::block::BlockPatchEntry>,
    hdiff_every_file: bool
) -> std::io::Result<()> {
    let filtered_old: Vec<_> = old_files
        .iter()
        .filter(|(rel_path, _)| {
            !rel_path.ends_with(".block")
                && !rel_path.contains("Persistent/")
                && !rel_path.contains("SDKCaches/")
        })
        .collect();

    let pb = common::utils::create_progress_bar(filtered_old.len());
    println!("Processing old files...");
    for (rel_path, old_meta) in filtered_old {
        match new_files.get(rel_path) {
            Some(new_meta) => {
                if old_meta.md5 == new_meta.md5 {
                    pb.inc(1);
                    continue;
                }
                
                if !hdiff_every_file 
                    && !rel_path.ends_with(".pck") 
                    && !(rel_path.contains("Plugins/x86_64") && rel_path.ends_with(".dll")) 
                {
                    let target_path = output_dir.join(rel_path); 
                    if let Some(parent) = target_path.parent() { 
                        fs::create_dir_all(parent)?; 
                    } 
                    fs::copy(&new_meta.full_path, &target_path)?;
                    pb.inc(1);
                    continue;
                }
                
                let patch_path = output_dir.join(format!("{}.hdiff", rel_path));
                if let Some(parent) = patch_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                
                if let Ok(hdiff) = HDiff::instance() {
                    if let Err(e) = hdiff.diff(&old_meta.full_path, &new_meta.full_path, &patch_path) {
                        eprintln!("hdiff failed for {}: {}", rel_path, e);
                    }
                } else {
                    eprintln!("Failed to initialize HDiff for {}", rel_path);
                }
                
                let patch_file_md5 = common::md5::calculate_md5(&patch_path)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("MD5 error: {}", e)))?;
                
                hdiff_entries.push(BlockPatchEntry{
                    source_file_name: rel_path.clone(),
                    source_file_md5: old_meta.md5.clone(),
                    source_file_size: old_meta.size,
                    target_file_name: rel_path.clone(),
                    target_file_md5: new_meta.md5.clone(),
                    target_file_size: new_meta.size,
                    patch_file_name: patch_path.strip_prefix(&output_dir).unwrap_or(&patch_path).to_string_lossy().to_string(),
                    patch_file_md5,
                    patch_file_size: std::fs::metadata(&patch_path)?.len() 
                });
                pb.inc(1); 
            },
            None => {
                delete_list.push(rel_path.clone());
                pb.inc(1);
            }
        }
    }
    pb.finish();
    
    let filtered_new: Vec<_> = old_files
        .iter()
        .filter(|(rel_path, _)| {
            !rel_path.ends_with(".block")
                && !rel_path.contains("Persistent/")
                && !rel_path.contains("SDKCaches/")
                && !old_files.contains_key(*rel_path)
        })
        .collect();
    let pb = common::utils::create_progress_bar(filtered_new.len());
    println!("Processing new files...");
    for (rel_path, new_meta) in filtered_new {
        let target_path = output_dir.join(rel_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&new_meta.full_path, &target_path)?;
        pb.inc(1);
    }
    
    pb.finish();
    Ok(())
}