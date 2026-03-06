use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

fn todoist_file_path() -> Result<PathBuf> {
    let base_dir = if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(path)
    } else {
        let home = env::var_os("HOME").context("HOME is not set")?;
        PathBuf::from(home).join(".config")
    };

    Ok(base_dir.join("todoist"))
}

pub fn save_token(token: &str) -> Result<()> {
    let parent = todoist_file_path()?;
    let token_path = &parent.join("auth.txt");

    fs::create_dir_all(&parent)
        .with_context(|| format!("failed to create directory {}", parent.display()))?;
    fs::write(&token_path, token)
        .with_context(|| format!("failed to write token file {}", token_path.display()))?;

    Ok(())
}

pub fn get_token() -> Result<String> {
    let token_path = todoist_file_path()?.join("auth.txt");
    let token = fs::read_to_string(&token_path)
        .with_context(|| format!("failed to read token file {}", token_path.display()))?;

    Ok(token.trim().to_owned())
}

pub fn save_list(list: &Vec<String>, file_name: &str) -> Result<()> {
    let parent = todoist_file_path()?;
    let list_path = &parent.join(file_name);

    let to_write = list
        .iter()
        .enumerate()
        .map(|(_, task_id)| task_id.as_str())
        .collect::<Vec<&str>>()
        .join("\n");
    fs::create_dir_all(&parent)
        .with_context(|| format!("failed to create directory {}", parent.display()))?;
    fs::write(&list_path, to_write)
        .with_context(|| format!("failed to write list file {}", list_path.display()))?;

    Ok(())
}

pub fn get_list(file_name: &str) -> Result<Vec<String>> {
    let list_path = todoist_file_path()?.join(file_name);
    let content = fs::read_to_string(&list_path)
        .with_context(|| format!("failed to read list file {}", list_path.display()))?;

    let list = content
        .lines()
        .filter_map(|line| line.to_owned().into())
        .collect::<Vec<String>>();

    Ok(list)
}
