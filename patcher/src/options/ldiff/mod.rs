use std::path::PathBuf;
use sophon::SevenZip;

pub mod handler;

use crate::options::{ldiff::handler::LdiffHandler};
use crate::utils;

pub fn handle_ldiff(game_path: &str) {
    let game_path = PathBuf::from(game_path);
    if !game_path.exists() {
        eprintln!("Could not find folder {}", game_path.display());
        return;
    }
    
    if !utils::ldiff_is_unpacked(&game_path) {
        let ldiff_path = common::input::read_input("Please enter ldiff archive location: ");
        let ldiff_path = PathBuf::from(ldiff_path);
        if !ldiff_path.exists() {
            eprintln!("Could not find file {}", ldiff_path.display());
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
        if let Err(e) = sevenzip.extract_to(&ldiff_path, &game_path) {
            eprintln!("Failed to extract ldiff: {}", e);
            return;
        }
        if !utils::ldiff_is_unpacked(&game_path) {
            eprintln!("Ldiff is damaged, redownload and unzip manually and try again.");
            return;
        }
    }
    
    LdiffHandler::new(&game_path).apply();
}