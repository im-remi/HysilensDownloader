use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use serde_json::from_str;
use common::embedded::HPatchz;

use crate::options::hdiff::{HdiffFilesEntry, HdiffMap, HdiffUpdateMode};
use crate::utils;

pub struct HdiffHandler<'a> {
    pub game_path: &'a Path,
}

impl<'a> HdiffHandler<'a> {
    pub fn new(game_path: &'a Path) -> Self {
        Self { game_path }
    }

    pub fn apply(&self) {
        match utils::detect_hdiff_update_type(&self.game_path.to_path_buf()) {
            HdiffUpdateMode::Hdifffiles => self.apply_hdifffiles(),
            HdiffUpdateMode::Hdiffmap => self.apply_hdiffmap(),
            HdiffUpdateMode::None => eprintln!("No hdiff update found at {}", self.game_path.display()),
        }
        self.remove_hdiff_files();
    }
    
    fn remove_hdiff_files(&self) {
        let hpatchz = match HPatchz::instance() {
            Ok(h) => h,
            Err(e) => {
                eprintln!("Failed to get HPatchz instance: {}", e);
                return;
            }
        };
        hpatchz.remove_file(&self.game_path.join("deletefiles.txt"));
    }

    fn apply_hdifffiles(&self) {
        let entries = match self.read_hdifffiles() {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Failed to read hdifffiles.txt: {}", e);
                return;
            }
        };

        let hpatchz = match HPatchz::instance() {
            Ok(h) => h,
            Err(e) => {
                eprintln!("Failed to get HPatchz instance: {}", e);
                return;
            }
        };
        
        println!("Patching files via hdifffiles.txt method...");
        let pb = common::utils::create_progress_bar(entries.len());

        for entry in entries {
            let source = self.game_path.join(&entry.remote_name);
            let patch_file = source.with_file_name(format!(
                "{}.hdiff",
                source.file_name().unwrap().to_string_lossy()
            ));
            if let Err(e) = hpatchz.patch(&source, &patch_file, &source) {
                pb.suspend(|| eprintln!("Failed to patch {}: {}", source.display(), e));
            }
            pb.inc(1);
        }
        pb.finish();
        self.remove_deleted_files();
        hpatchz.remove_file(&self.game_path.join("hdifffiles.txt"));
        hpatchz.remove_file(&self.game_path.join("README.txt"));
    }

    fn read_hdifffiles(&self) -> Result<Vec<HdiffFilesEntry>, Box<dyn std::error::Error>> {
        let path = self.game_path.join("hdifffiles.txt");
        let file = fs::File::open(path)?;
        let reader = io::BufReader::new(file);
    
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            let entry: HdiffFilesEntry = serde_json::from_str(&line)?;
            entries.push(entry);
        }
    
        Ok(entries)
    }

    fn apply_hdiffmap(&self) {
        let map = match self.read_hdiffmap() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Failed to read or parse hdiffmap.json: {}", e);
                return;
            }
        };

        let hpatchz = match HPatchz::instance() {
            Ok(h) => h,
            Err(e) => {
                eprintln!("Failed to get HPatchz instance: {}", e);
                return;
            }
        };
        
        println!("Patching files via hdiffmap.json method");
        let pb = common::utils::create_progress_bar(map.diff_map.len());

        for entry in map.diff_map {
            let source = self.game_path.join(entry.source_file_name);
            let patch = self.game_path.join(entry.patch_file_name);
            let target = self.game_path.join(entry.target_file_name);
            
            let source_md5 = match common::md5::calculate_md5(&source) {
                Ok(md5) => md5,
                Err(e) => {
                    pb.suspend(|| eprintln!("Source file error {}: {}", source.display(), e));
                    pb.inc(1);
                    continue;
                }
            };
            
            let source_size = source.metadata().map(|m| m.len()).unwrap_or(0);
            if source_md5 != entry.source_file_md5 || source_size != entry.source_file_size {
                pb.suspend(|| eprintln!("Source file invalid: {}", source.display()));
                pb.inc(1);
                continue;
            }
            let patch_md5 = match common::md5::calculate_md5(&patch) {
                Ok(md5) => md5,
                Err(e) => {
                    pb.suspend(|| eprintln!("Patch file error {}: {}", patch.display(), e));
                    pb.inc(1);
                    continue;
                }
            };
            let patch_size = patch.metadata().map(|m| m.len()).unwrap_or(0);
            if patch_md5 != entry.patch_file_md5 || patch_size != entry.patch_file_size {
                pb.suspend(|| eprintln!("Patch file invalid: {}", patch.display()));
                pb.inc(1);
                continue;
            }    

            if let Err(e) = hpatchz.patch(&source, &patch, &target) {
                pb.suspend(|| eprintln!("Failed to patch {}: {}", source.display(), e));
                pb.inc(1);
                continue;
            }
            
            let target_md5 = match common::md5::calculate_md5(&target) {
                Ok(md5) => md5,
                Err(e) => {
                    pb.suspend(|| eprintln!("Target file error {}: {}", target.display(), e));
                    pb.inc(1);
                    continue;
                }
            };
            let target_size = target.metadata().map(|m| m.len()).unwrap_or(0);
            if target_md5 != entry.target_file_md5 || target_size != entry.target_file_size {
                pb.suspend(|| eprintln!("Patched file does not match expected MD5/size: {}", target.display()));
            }
            pb.inc(1);
        }

        self.remove_deleted_files();
        hpatchz.remove_file(&self.game_path.join("hdiffmap.json"));
    }

    fn read_hdiffmap(&self) -> Result<HdiffMap, Box<dyn std::error::Error>> {
        let path = self.game_path.join("hdiffmap.json");
        let json_data = fs::read_to_string(path)?;
        let map: HdiffMap = from_str(&json_data)?;
        Ok(map)
    }

    pub fn remove_deleted_files(&self) {
        println!("Deleting files...");
        let deletefiles_path = self.game_path.join("deletefiles.txt");
        let file = match std::fs::File::open(&deletefiles_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Failed to open deletefiles.txt: {}", e);
                return;
            }
        };
    
        let reader = std::io::BufReader::new(file);
        let lines: Vec<_> = reader.lines().flatten().collect();
        let pb = common::utils::create_progress_bar(lines.len());
    
        for line in lines {
            let clean_line = line.trim().replace("\\", "/");
            let file_path = self.game_path.join(&clean_line);
            if file_path.exists() {
                if let Err(e) = std::fs::remove_file(&file_path) {
                    pb.suspend(|| eprintln!("Failed to delete {}: {}", file_path.display(), e));
                }
            } else {
                pb.suspend(|| println!("Already gone: {}", file_path.display()));
            }
            pb.inc(1);
        }
    
        pb.finish();
        println!("Done!");
    }
}
