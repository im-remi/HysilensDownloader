use std::io::{self, Write};


pub fn read_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().expect("Failed to flush stdout");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");
    input.trim().to_string()
}

pub fn confirm(prompt: &str) -> bool {
    let input = read_input(&format!("{} [y/N]: ", prompt));
    matches!(input.to_lowercase().as_str(), "y" | "yes")
}