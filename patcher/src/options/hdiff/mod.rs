use std::path::PathBuf;
use serde::Deserialize;
use common::embedded::SevenZip;

mod handler;
pub use handler::HdiffHandler;

use crate::utils::HdiffUpdateMode;


#[derive(Deserialize)]
pub struct HdiffFilesEntry {
    #[serde(rename="remoteName")]
    pub remote_name: String
}

#[derive(Deserialize)]
pub struct HdiffMap {
    pub diff_map: Vec<HdiffMapEntry>
}

#[derive(Deserialize)]
pub struct HdiffMapEntry {
    pub source_file_name: String,
    pub source_file_md5: String,
    pub source_file_size: u64,
    pub target_file_name: String,
    pub target_file_md5: String,
    pub target_file_size: u64,
    pub patch_file_name: String,
    pub patch_file_md5: String,
    pub patch_file_size: u64
}

pub fn handle_hdiff(game_path: &str) {
    let game_path = PathBuf::from(game_path);
    if !game_path.exists() {
        eprintln!("Could not find folder {}", game_path.display());
        return;
    }

    let mut hdiff_type = crate::utils::detect_hdiff_update_type(&game_path);
    if hdiff_type == HdiffUpdateMode::None {
        let hdiff_path = common::input::read_input("Please enter hdiff archive location: ");
        let hdiff_path = PathBuf::from(hdiff_path);
        if !hdiff_path.exists() {
            eprintln!("Could not find file {}", hdiff_path.display());
            return;
        }

        let sevenzip = match SevenZip::instance() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to fetch SevenZip: {}", e);
                return;
            }
        };

        println!("Extracting patch...");
        if let Err(e) = sevenzip.extract_to(&hdiff_path, &game_path) {
            eprintln!("Failed to extract hdiff: {}", e);
            return;
        }

        hdiff_type = crate::utils::detect_hdiff_update_type(&game_path);
        if hdiff_type == HdiffUpdateMode::None {
            eprintln!("Hdiff package is wrongly built; please redownload and unpack it manually.");
            return;
        }
    }

    HdiffHandler::new(&game_path).apply();
}
