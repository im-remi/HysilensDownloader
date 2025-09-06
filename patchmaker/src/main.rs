#![feature(once_cell_try)]
use std::{
    collections::HashSet, fs, path::PathBuf, time::Instant
};

use walkdir::WalkDir;

use crate::{block::BlockPatchEntry, patch::process_regular_files};

mod block;
mod patch;
mod scan;
mod utils;

#[derive(serde::Serialize)]
struct HdiffMap {
    diff_map: Vec<BlockPatchEntry>,
}

fn main() -> std::io::Result<()> {
    println!("Hysilens-Download Patchmaker made by Remi with love <3");

    let old_client_path = PathBuf::from(common::input::read_input("Please enter old client path: "));
    let new_client_path = PathBuf::from(common::input::read_input("Please enter new client path: "));
    let output_dir = PathBuf::from(common::input::read_input("Please enter hdiff output path: "));
    let hdiff_every_file = common::input::confirm("Apply HDiff to every file?");
    let use_faster_check = common::input::confirm("Use faster block check?");
    let start = Instant::now(); 
    fs::create_dir_all(&output_dir)?;
    
    let keep_files: HashSet<_> = ["old_files.json", "new_files.json"].iter().collect();
    utils::clear_directory(&output_dir, keep_files)?;
    let old_files = scan::load_or_scan(&old_client_path, &output_dir.join("old_files.json"))?;
    let new_files = scan::load_or_scan(&new_client_path, &output_dir.join("new_files.json"))?;

    let mut delete_list = Vec::new();
    let mut hdiff_entries = Vec::new();
    
    process_regular_files(&old_files, &new_files, &output_dir, &mut delete_list, &mut hdiff_entries, hdiff_every_file)?;

    let block_entries = block::generate_block_map(&old_files, &new_files, &output_dir, use_faster_check)?;
    hdiff_entries.extend(block_entries);
    
    let map_path = output_dir.join("hdiffmap.json");
    let json_data = serde_json::to_string_pretty(&HdiffMap { diff_map: hdiff_entries })?;
    fs::write(map_path, json_data)?;

    fs::write(output_dir.join("deletefiles.txt"), delete_list.join("\n"))?;
    
    let folder_size: u64 = WalkDir::new(&output_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|entry| fs::metadata(entry.path()).ok())
        .map(|meta| meta.len())
        .sum();
    
    let elapsed = start.elapsed();
    println!("Patch folder prepared successfully!");
    println!("Total patch folder size: {:.1} MiB", folder_size as f64 / 1024.0 / 1024.0);
    println!("Total processing time: {:.2?}", elapsed);
    
    Ok(())
}
