use std::path::PathBuf;

#[derive(PartialEq)]
pub enum HdiffUpdateMode {
    Hdiffmap,
    Hdifffiles,
    None
}

pub fn detect_hdiff_update_type(game_path: &PathBuf) -> HdiffUpdateMode {
    let deletefiles_path = game_path.join("deletefiles.txt");
    let hdiffmap_path = game_path.join("hdiffmap.json");
    let hdifffiles_path = game_path.join("hdifffiles.txt");
    
    let has_hdiff_files = hdiffmap_path.exists();
    let has_custom_hdiff = hdifffiles_path.exists() && deletefiles_path.exists();
    
    if has_hdiff_files {
        HdiffUpdateMode::Hdiffmap
    } else if has_custom_hdiff {
        HdiffUpdateMode::Hdifffiles
    } else {
        HdiffUpdateMode::None
    }
}

pub fn ldiff_is_unpacked(game_path: &PathBuf) -> bool {
    let manifest_exists = manifest_exists(game_path);
    
    let ldiff_exists = game_path.join("ldiff").is_dir();
    
    manifest_exists && ldiff_exists
}

pub fn manifest_exists(game_path: &PathBuf) -> bool {
    match std::fs::read_dir(&game_path) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .any(|entry| {
                let path = entry.path();
                path.is_file() && path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.contains("manifest"))
                    .unwrap_or(false)
            }),
        Err(_) => false,
    }
}