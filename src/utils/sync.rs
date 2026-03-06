use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

fn token_file_path() -> Result<PathBuf> {
    let base_dir = if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(path)
    } else {
        let home = env::var_os("HOME").context("HOME is not set")?;
        PathBuf::from(home).join(".config")
    };

    Ok(base_dir.join("todoist"))
}

pub fn save_token(token: &str) -> Result<()> {
    let token_path = token_file_path()?;
    let parent = token_path
        .parent()
        .context("failed to determine token directory")?;

    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create directory {}", parent.display()))?;
    fs::write(&token_path, token)
        .with_context(|| format!("failed to write token file {}", token_path.display()))?;

    Ok(())
}

pub fn get_token() -> Result<String> {
    let token_path = token_file_path()?;
    let token = fs::read_to_string(&token_path)
        .with_context(|| format!("failed to read token file {}", token_path.display()))?;

    Ok(token.trim().to_owned())
}
