use std::{collections::HashSet, fs::{self, File}, io::{self, Read}, path::Path};

pub fn clear_directory(output_dir: &Path, keep_files: HashSet<&&str>) -> std::io::Result<()>{
    for entry in fs::read_dir(&output_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !keep_files.contains(&name) {
                    let _ = fs::remove_file(path);
                }
            }
        } else if path.is_dir() {
            let _ = fs::remove_dir_all(path);
        }
    }
    Ok(())
}

use crc32fast::Hasher;

pub fn block_diff_percent_experimental(path_a: &Path, path_b: &Path) -> io::Result<f64> {
    const BLOCK_SIZE: usize = 64 * 1024; 
    let mut file_a = File::open(path_a)?;
    let mut file_b = File::open(path_b)?;

    let mut buf_a = vec![0u8; BLOCK_SIZE];
    let mut buf_b = vec![0u8; BLOCK_SIZE];

    let mut blocks_a = Vec::new();
    let mut blocks_b = Vec::new();

    loop {
        let read_a = file_a.read(&mut buf_a)?;
        if read_a == 0 { break; }
        let mut hasher = Hasher::new();
        hasher.update(&buf_a[..read_a]);
        blocks_a.push(hasher.finalize());
    }

    loop {
        let read_b = file_b.read(&mut buf_b)?;
        if read_b == 0 { break; }
        let mut hasher = Hasher::new();
        hasher.update(&buf_b[..read_b]);
        blocks_b.push(hasher.finalize());
    }

    let len = blocks_a.len().max(blocks_b.len());
    if len == 0 {
        return Ok(0.0);
    }

    let mut matching = 0usize;
    for (a, b) in blocks_a.iter().zip(blocks_b.iter()) {
        if a == b {
            matching += 1;
        }
    }

    Ok(100.0 - (matching as f64 / len as f64) * 100.0)
}
