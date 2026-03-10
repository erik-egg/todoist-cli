use anyhow::{Context, Result};
use directories::ProjectDirs;
use keyring::Entry;
use std::fs;
use std::path::PathBuf;

const QUALIFIER: &str = "com";
const ORGANIZATION: &str = "todo-cli";
const APPLICATION: &str = "todo";
const KEYCHAIN_SERVICE: &str = "todo-cli";
const KEYCHAIN_ACCOUNT: &str = "todoist-api-token";
const TASK_IDS_FILE: &str = "task_ids.txt";

fn app_cache_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
        .context("failed to determine platform-specific app directories")?;

    Ok(dirs.cache_dir().to_path_buf())
}

fn keychain_entry() -> Result<Entry> {
    Ok(Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)?)
}

pub fn save_token(token: &str) -> Result<()> {
    let entry = keychain_entry()?;
    entry
        .set_password(token)
        .context("failed to save token in OS keychain")?;

    Ok(())
}

pub fn get_token() -> Result<String> {
    let entry = keychain_entry()?;
    let token = entry
        .get_password()
        .context("failed to read token from OS keychain")?;

    Ok(token.trim().to_owned())
}

pub fn save_task_ids(task_ids: &[String]) -> Result<()> {
    let parent = app_cache_dir()?;
    let task_ids_path = parent.join(TASK_IDS_FILE);

    fs::create_dir_all(&parent)
        .with_context(|| format!("failed to create directory {}", parent.display()))?;
    fs::write(&task_ids_path, task_ids.join("\n"))
        .with_context(|| format!("failed to write task list file {}", task_ids_path.display()))?;

    Ok(())
}

pub fn get_task_ids() -> Result<Vec<String>> {
    let task_ids_path = app_cache_dir()?.join(TASK_IDS_FILE);
    let content = fs::read_to_string(&task_ids_path)
        .with_context(|| format!("failed to read task list file {}", task_ids_path.display()))?;

    Ok(content.lines().map(str::to_owned).collect::<Vec<String>>())
}
