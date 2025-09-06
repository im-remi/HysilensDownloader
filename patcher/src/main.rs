#![feature(once_cell_try)]

use std::path::{Path, PathBuf};

use crate::options::{hdiff::{handle_hdiff, HdiffHandler}, ldiff::{handle_ldiff, handler::LdiffHandler}};

mod options;
mod utils;

fn main() {
    println!("HysilensDownloader by Remi made with love <3");
    println!("Options:");
    println!("0 - Patch game via hdiff");
    println!("1 - Patch game via ldiff");
    println!("2 - Patch game via Sophon (not yet supported)");
    println!("3 - Verify file integrity");
    println!("4 - Delete leftover files");
    
    let input = common::input::read_input("Please select action: ");
    
    match input.as_str() {
        "0" => {
            let game_folder = common::input::read_input("Please enter game folder: ");
            handle_hdiff(&game_folder);
        },
        "1" => {
            let game_folder = common::input::read_input("Please enter game folder: ");
            handle_ldiff(&game_folder);
        },
        "3" => {
            let game_folder = common::input::read_input("Please enter game folder: ");
            if let Err(e) = options::verify::verify_files(&Path::new(&game_folder)) {
                eprintln!("An error occurred while verifying file integrity: {}", e);
            };
        }
        "4" => {
            let game_folder = common::input::read_input("Please enter game folder: ");
            if utils::manifest_exists(&PathBuf::from(&game_folder)) {
                let ldiff_handler = LdiffHandler::new(Path::new(&game_folder));
                let Some(manifest_proto) = ldiff_handler.get_manifest_proto() else {
                    return;
                };
                let _ = ldiff_handler.handle_delete_files(&manifest_proto);
                
            } else {
                HdiffHandler::new(Path::new(&game_folder)).remove_deleted_files();
            }
        }
        _ => println!("Option is not supported")
    }
}