use std::{collections::HashSet, error::Error, fs::{remove_file, File}, io::{Read, Seek, SeekFrom, Write}, path::{Path, PathBuf}};

use common::{embedded::{HPatchz, SevenZip}, utils};
use indicatif::ProgressBar;
use sophon::{modules::{Manifest, SophonParser}, sophon_patch::{SophonPatchAssetChunk, SophonPatchAssetInfo, SophonPatchAssetProperty, SophonPatchProto}};


pub struct LdiffHandler<'a> {
    pub game_path: &'a Path
}

impl<'a> LdiffHandler<'a> {
    pub fn new(game_path: &'a Path) -> Self {
        Self { game_path }
    }
    
    pub fn apply(&self) {
        let Some(manifest_proto) = self.get_manifest_proto() else {
            eprintln!("Failed to parse manifest proto");
            return;
        };
        
        println!("Item Count: {}", manifest_proto.patch_assets.len());
        self.process_patch_assets(&manifest_proto);
        self.clean();
    }
    
    pub fn process_patch_assets(&self, manifest_proto: &SophonPatchProto) {
        let complex_assets = manifest_proto
            .patch_assets
            .iter()
            .filter(|a| !a.asset_infos.is_empty())
            .collect::<Vec<&SophonPatchAssetProperty>>();
        
        let complex_assets_len = complex_assets.len();
        println!("Patching {} files...", complex_assets_len);
        let pb = utils::create_progress_bar(complex_assets_len);
        
        for asset in &complex_assets {
            self.process_complex_asset(asset, &pb);
        }
        
        if let Err(e) = self.handle_delete_files(manifest_proto) {
            eprintln!("An error occured while deleting files: {}", e);
        }
        println!("Done!")
    }
    
    pub fn handle_delete_files(&self, manifest_proto: &SophonPatchProto) -> Result<(), Box<dyn Error>> {
        let ldiff_asset_set: HashSet<PathBuf> = manifest_proto
            .patch_assets
            .iter()
            .map(|asset| PathBuf::from(&asset.asset_name))
            .collect();
    
        let star_rail_data_path = self.game_path.join("StarRail_Data");
        let all_game_files = self.walk_dir_excluding(&star_rail_data_path, "Persistent")?;

        let files_to_delete: Vec<_> = all_game_files
            .into_iter()
            .filter(|file_path| {
                file_path
                    .strip_prefix(self.game_path)
                    .map(|relative_path| !ldiff_asset_set.contains(&relative_path.to_path_buf()))
                    .unwrap_or(true)
            })
            .collect();
        
        let files_to_delete_len = files_to_delete.len();
        println!("Deleting {} files...", files_to_delete_len);
        let pb = utils::create_progress_bar(files_to_delete_len);
        
        for file in files_to_delete {
            std::fs::remove_file(&file).map_err(|e| format!("Failed to remove file {:?}: {}", file, e))?;
            pb.inc(1);
        }

        Ok(())
    }
    
    fn clean(&self) {
        let ldiff_path = self.game_path.join("ldiff");
        
        if ldiff_path.exists() {
            let _ = std::fs::remove_dir_all(&ldiff_path);
        }
        
        if let Some(manifest_path) = self.locate_manifest_file(){
            let _= std::fs::remove_file(&manifest_path);
        }
    }
    
    fn walk_dir_excluding(
        &self,
        dir: &Path,
        exclude_dir: &str,
    ) -> Result<Vec<PathBuf>, std::io::Error> {
        let mut files = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
    
        while let Some(current_dir) = stack.pop() {
            for entry in std::fs::read_dir(current_dir)? {
                let entry = entry?;
                let path = entry.path();
    
                if path.is_dir() {
                    if path.file_name().and_then(|n| n.to_str()) != Some(exclude_dir) {
                        stack.push(path);
                    }
                } else {
                    files.push(path);
                }
            }
        }
        Ok(files)
    }
    
    fn process_complex_asset(&self, asset: &SophonPatchAssetProperty, pb: &ProgressBar) {
        let chosen_info = self.choose_asset_info(asset);
        if let Some(chunk) = &chosen_info.chunk {
            if let Err(e) = self.apply_chunk(asset, chunk) {
                pb.suspend(|| eprintln!("Failed to apply chunk for {}: {}", asset.asset_name, e));
            }
            pb.inc(1);
        }
    }
    
    fn choose_asset_info(&self, asset: &SophonPatchAssetProperty) -> SophonPatchAssetInfo {
        if asset.asset_infos.len() == 1 {
            asset.asset_infos[0].clone()
        } else {
            let version_tags: HashSet<_> = asset.asset_infos.iter().map(|c| c.version_tag.clone()).collect();
            println!("Available version tags:");
            for tag in &version_tags { println!("- {}", tag); }
            let version_tag = common::input::read_input("Select version tag: ");
            asset.asset_infos.iter()
                .find(|info| info.version_tag == version_tag)
                .unwrap_or(&asset.asset_infos[0]).clone()
        }
    }
    
    fn apply_chunk(&self, asset: &SophonPatchAssetProperty, chunk: &SophonPatchAssetChunk) -> Result<(), String> {
        let ldiff_path = self.game_path.join("ldiff");
        let source_file = if chunk.original_file_name.is_empty() { PathBuf::new() } else { self.game_path.join(&chunk.original_file_name) };
        let temp_patch = ldiff_path.join("temp_patch_file");
        let temp_target = ldiff_path.join("temp_target_file");
    
        write_patch_slice(&ldiff_path.join(&chunk.patch_name), chunk.patch_offset as u64, chunk.patch_length as u64, &temp_patch)
            .map_err(|e| format!("Failed to write patch slice: {}", e))?;
    
        HPatchz::instance().unwrap().patch(&source_file, &temp_patch, &temp_target)
            .map_err(|e| format!("Patch failed: {}", e))?;
    
        let md5 = common::md5::calculate_md5(&temp_target).map_err(|e| format!("Failed to read patched file: {}", e))?;
    
        let asset_path = self.game_path.join(&asset.asset_name);
        if md5 == asset.asset_hash_md5 {
            std::fs::rename(&temp_target, &asset_path).map_err(|e| format!("Failed to replace asset: {}", e))?;
        } else {
            eprintln!("MD5 mismatch after patching {} (expected {}, got {})", asset.asset_name, asset.asset_hash_md5, md5);
        }
    
        if asset_path != source_file {
            let _ = remove_file(&source_file);
        }
        
        let _ = remove_file(&temp_patch);
        if temp_target.exists() {
            let _ = remove_file(&temp_target);
        }
        Ok(())
    }
    
    pub fn get_manifest_proto(&self) -> Option<SophonPatchProto> {
        let Some(manifest) = self.locate_manifest_file() else {
            eprintln!("Failed to find the manifest file");
            return None;
        };
        
        let manifest_to_read = if !manifest.to_string_lossy().contains('~') {
                let extract_dir = manifest.parent().unwrap();
                if let Err(err) = SevenZip::instance().unwrap().extract_to(&manifest, extract_dir) {
                    eprintln!("Failed to extract archive: {}", err);
                    return None;
                }
                let _ = std::fs::remove_file(&manifest);
                extract_dir.join(format!("{}~", manifest.file_name().unwrap().display()))
            } else {
                manifest.clone()
            };
        
        let manifest_vec = match std::fs::read(&manifest_to_read) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to extract path {}: {}", manifest_to_read.display(), e);
                return None;
            }
        };
        
        let Ok(manifest_proto) = SophonParser::new().parse_manifest_file(manifest_vec) else {
            return None;
        };
        
        let Manifest::Diff(manifest_proto) = manifest_proto else {
            return None;
        };
        
        Some(manifest_proto)
    }
    
    fn locate_manifest_file(&self) -> Option<PathBuf> {
        match std::fs::read_dir(&self.game_path) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .find(|path| {
                    path.is_file() && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.contains("manifest"))
                        .unwrap_or(false)
                }),
            Err(_) => None,
        }
    } 
}

fn write_patch_slice(patch_file: &std::path::Path, offset: u64, length: u64, output_file: &std::path::Path) -> std::io::Result<()> {
    let mut file = File::open(patch_file)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut out = File::create(output_file)?;
    let mut remaining = length;
    let mut buffer = [0u8; 8192];

    while remaining > 0 {
        let read_len = buffer.len().min(remaining as usize);
        let n = file.read(&mut buffer[..read_len])?;
        if n == 0 { break; }
        out.write_all(&buffer[..n])?;
        remaining -= n as u64;
    }

    Ok(())
}
