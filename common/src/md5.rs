use std::{fs::File, io::Read, path::PathBuf};

#[derive(thiserror::Error, Debug)]
pub enum Md5Error {
    #[error("Failed to open file for MD5 calculation: {0}")]
    FileOpenError(String, #[source] std::io::Error),

    #[error("Failed to read data from file: {0}")]
    FileReadError(String, #[source] std::io::Error),
}

pub fn calculate_md5(path: &PathBuf) -> Result<String, Md5Error> {
    let mut file = File::open(path)
        .map_err(|e| Md5Error::FileOpenError(path.display().to_string(), e))?;

    let mut context = md5::Context::new();
    let mut buffer = [0u8; 8192];

    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| Md5Error::FileReadError(path.display().to_string(), e))?;

        if count == 0 {
            break;
        }

        context.consume(&buffer[..count]);
    }

    let digest = context.finalize();
    Ok(format!("{:x}", digest))
}