use std::path::Path;

use indicatif::{ProgressBar, ProgressStyle};


pub fn get_temp_files_path() -> Result<String, std::io::Error> {
    let temp_dir = dirs::cache_dir().unwrap_or(std::env::temp_dir()).join("hysilensdownloader");
    if !temp_dir.exists() {
        std::fs::create_dir_all(&temp_dir)?;
    }
    Ok(temp_dir.to_string_lossy().to_string())
}

pub fn run_command_with_nixos_wrapper(executable: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
    use std::process::Command;
    let is_nixos = std::fs::read_to_string("/etc/os-release")
        .map(|s| s.contains("ID=nixos"))
        .unwrap_or(false);

    if is_nixos {
        Command::new("steam-run")
            .arg(executable)
            .args(args)
            .output()
    } else {
        Command::new(executable)
            .args(args)
            .output()
    }
}

static PROGRESS_TEMPLATE: &str = "{spinner:.green} [{elapsed}] [{bar:35.cyan/blue}] {pos}/{len}";
static PROGRESS_CHARS: &str = "#>-";

pub fn create_progress_bar(len: usize) -> ProgressBar {
    let pb = ProgressBar::new(len as u64);
    pb.set_style(
        ProgressStyle::with_template(PROGRESS_TEMPLATE)
            .unwrap()
            .progress_chars(PROGRESS_CHARS),
    );
    pb
}

pub fn read_input(prompt: &str) -> String {
    use std::io::Write;
    print!("{}", prompt);
    std::io::stdout().flush().expect("Failed to flush stdout");

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");
    input.trim().to_string()
}