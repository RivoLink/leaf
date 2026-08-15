use std::{
    collections::BTreeMap,
    io::{self, Write as _},
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{Deserialize, Deserializer};

use crate::picker_width::{parse_picker_width_spec, PickerWidthSpec};
use crate::theme::{resolve_theme_selection, CustomThemeConfig};

const DEFAULT_CONFIG: &str = include_str!("../config.toml");

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct LeafConfig {
    pub(crate) theme: Option<String>,
    pub(crate) editor: Option<String>,
    pub(crate) watch: Option<bool>,
    pub(crate) width: Option<usize>,
    pub(crate) extras: Vec<String>,
    #[serde(rename = "code-line-numbers")]
    pub(crate) code_line_numbers: Option<bool>,
    #[serde(
        rename = "tab-title-length",
        deserialize_with = "deserialize_lenient_i32"
    )]
    pub(crate) tab_title_length: Option<i32>,
    #[serde(
        rename = "file-picker-width",
        deserialize_with = "deserialize_lenient_picker_width"
    )]
    pub(crate) file_picker_width: Option<PickerWidthSpec>,
    #[serde(
        rename = "file-history-length",
        deserialize_with = "deserialize_lenient_i32"
    )]
    pub(crate) file_history_length: Option<i32>,
    pub(crate) themes: BTreeMap<String, CustomThemeConfig>,
    #[serde(skip)]
    pub(crate) config_dir: Option<PathBuf>,
}

pub(crate) const FILE_HISTORY_LENGTH_MAX: i32 = 50;

fn deserialize_lenient_i32<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = toml::Value::deserialize(deserializer)?;
    Ok(value.as_integer().and_then(|n| i32::try_from(n).ok()))
}

fn deserialize_lenient_picker_width<'de, D>(
    deserializer: D,
) -> Result<Option<PickerWidthSpec>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = toml::Value::deserialize(deserializer)?;
    Ok(value.as_str().and_then(parse_picker_width_spec))
}

#[derive(Default)]
pub(crate) struct CliOverrides {
    pub(crate) width: Option<usize>,
    pub(crate) theme: Option<String>,
}

pub(crate) fn load_config(overrides: &CliOverrides) -> (LeafConfig, Option<String>) {
    let Some(path) = config_path() else {
        return (LeafConfig::default(), None);
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return (LeafConfig::default(), None),
    };
    let mut config = match toml::from_str::<LeafConfig>(&content) {
        Ok(c) => c,
        Err(_) => {
            return (
                LeafConfig::default(),
                Some("Could not parse config.toml, using defaults".to_string()),
            );
        }
    };
    config.config_dir = path.parent().map(Path::to_path_buf);

    let mut warnings: Vec<String> = Vec::new();

    let leaf_theme_overrides = std::env::var("LEAF_THEME").is_ok_and(|s| !s.is_empty());
    if overrides.theme.is_none() && !leaf_theme_overrides {
        if let Some(ref name) = config.theme {
            if let Err(message) =
                resolve_theme_selection(name, &config.themes, config.config_dir.as_deref())
            {
                warnings.push(format!("{message} in config, using default"));
            }
        }
    }

    let leaf_width_overrides =
        std::env::var("LEAF_WIDTH").is_ok_and(|v| v.parse::<usize>().is_ok_and(|w| w >= 20));
    if overrides.width.is_none() && !leaf_width_overrides {
        if let Some(w) = config.width.filter(|&w| w < 20) {
            warnings.push(format!(
                "width={w} in config is below minimum (20), will use 20"
            ));
        }
    }

    let leaf_tab_title_env_overrides = std::env::var("LEAF_TAB_TITLE_LENGTH")
        .is_ok_and(|v| v.parse::<i32>().is_ok_and(crate::is_valid_tab_title_length));
    if !leaf_tab_title_env_overrides {
        if let Some(n) = config.tab_title_length {
            if (0..20).contains(&n) {
                warnings.push("tab-title-length is invalid, no truncation applied".to_string());
            }
        }
    }

    let warning = if warnings.is_empty() {
        None
    } else {
        Some(warnings.join("; "))
    };
    (config, warning)
}

pub(crate) fn config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .ok()
            .map(|dir| PathBuf::from(dir).join("leaf").join("config.toml"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let base = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| std::env::var("HOME").ok().map(|h| format!("{h}/.config")))?;
        Some(PathBuf::from(base).join("leaf").join("config.toml"))
    }
}

pub(crate) fn history_path() -> Option<PathBuf> {
    config_path().and_then(|p| p.parent().map(|d| d.join("history.toml")))
}

pub(crate) fn run_config() -> anyhow::Result<()> {
    let path = config_path().context("Cannot determine config directory")?;

    if !path.exists() {
        println!("Creating default config.toml...");
        write_default_config(&path)?;
    }

    println!("Configuration file: {}", path.display());
    open_config_in_editor(&path)
}

pub(crate) fn reset_config() -> anyhow::Result<()> {
    let path = config_path().context("Cannot determine config directory")?;

    let (old_config, _) = load_config(&CliOverrides::default());
    let editor = crate::editor::resolve_editor(None, old_config.editor.as_deref());

    if !confirm("Reset leaf configuration to defaults?")? {
        println!("Reset cancelled.");
        return Ok(());
    }

    write_default_config(&path)?;
    println!("Configuration reset: {}", path.display());
    launch_editor(&editor, &path);
    Ok(())
}

pub(crate) fn remove_config() -> anyhow::Result<()> {
    let dir = config_path()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .context("Cannot determine config directory")?;

    if !dir.exists() {
        println!("No configuration to remove: {}", dir.display());
        return Ok(());
    }

    if !confirm("Remove leaf configuration?")? {
        println!("Remove cancelled.");
        return Ok(());
    }

    std::fs::remove_dir_all(&dir)
        .with_context(|| format!("Cannot remove config directory: {}", dir.display()))?;
    println!("Configuration removed: {}", dir.display());
    Ok(())
}

pub(crate) fn edit_history() -> anyhow::Result<()> {
    let path = history_path().context("Cannot determine history file path")?;

    if !path.exists() {
        println!("No file history to edit: {}", path.display());
        return Ok(());
    }

    println!("File history: {}", path.display());
    open_config_in_editor(&path)
}

pub(crate) fn remove_history() -> anyhow::Result<()> {
    let path = history_path().context("Cannot determine history file path")?;

    if !path.exists() {
        println!("No file history to remove: {}", path.display());
        return Ok(());
    }

    if !confirm("Remove file history?")? {
        println!("Remove cancelled.");
        return Ok(());
    }

    std::fs::remove_file(&path)
        .with_context(|| format!("Cannot remove file history: {}", path.display()))?;
    println!("File history removed: {}", path.display());
    Ok(())
}

pub(crate) fn confirm(prompt: &str) -> anyhow::Result<bool> {
    print!("{prompt} (y/N): ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(answer.trim().eq_ignore_ascii_case("y"))
}

fn write_default_config(dest: &Path) -> anyhow::Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Cannot create config directory: {}", parent.display()))?;
    }
    std::fs::write(dest, DEFAULT_CONFIG)
        .with_context(|| format!("Cannot write config file: {}", dest.display()))
}

pub(crate) fn open_config_in_editor(path: &Path) -> anyhow::Result<()> {
    let (config, _) = load_config(&CliOverrides::default());
    let editor = crate::editor::resolve_editor(None, config.editor.as_deref());
    launch_editor(&editor, path);
    Ok(())
}

fn launch_editor(editor: &str, path: &Path) {
    let expanded = crate::editor::expand_editor_placeholders(editor, 1, path);
    if try_launch_editor(&expanded, path) {
        return;
    }
    if let Some(fallback) = crate::editor::resolve_fallback_editor(&expanded) {
        try_launch_editor(fallback, path);
    }
}

fn try_launch_editor(editor: &str, path: &Path) -> bool {
    let (bin, args) = crate::editor::split_editor_cmd(editor);
    std::process::Command::new(bin)
        .args(args)
        .arg(path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
