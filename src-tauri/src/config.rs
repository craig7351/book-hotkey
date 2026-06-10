use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Manager};

/// 單筆快速指令。
#[derive(Serialize, Deserialize, Clone)]
pub struct Command {
    pub id: String,
    pub label: String,
    pub content: String,
}

/// 預設熱鍵（跨平台：Windows/Linux=Ctrl、macOS=Cmd）。
pub fn default_hotkey() -> String {
    "CmdOrControl+Shift+A".to_string()
}

/// 設定檔結構。
#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub commands: Vec<Command>,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
            hotkey: default_hotkey(),
        }
    }
}

/// 設定檔路徑：<app_config_dir>/commands.json。
fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("取得設定目錄失敗: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("建立設定目錄失敗: {e}"))?;
    Ok(dir.join("commands.json"))
}

/// 讀取完整設定；檔案不存在時回傳預設值。
pub fn load_config(app: &AppHandle) -> Result<Config, String> {
    let path = config_path(app)?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let data = fs::read_to_string(&path).map_err(|e| format!("讀取設定失敗: {e}"))?;
    serde_json::from_str(&data).map_err(|e| format!("解析設定失敗: {e}"))
}

/// 寫入完整設定。
pub fn save_config(app: &AppHandle, config: &Config) -> Result<(), String> {
    let path = config_path(app)?;
    let data =
        serde_json::to_string_pretty(config).map_err(|e| format!("序列化設定失敗: {e}"))?;
    fs::write(&path, data).map_err(|e| format!("寫入設定失敗: {e}"))?;
    Ok(())
}

// ---- 指令清單 ----

pub fn load(app: &AppHandle) -> Result<Vec<Command>, String> {
    Ok(load_config(app)?.commands)
}

/// 只更新指令清單，保留熱鍵等其他設定。
pub fn save(app: &AppHandle, commands: Vec<Command>) -> Result<(), String> {
    let mut config = load_config(app)?;
    config.commands = commands;
    save_config(app, &config)
}

// ---- 熱鍵 ----

pub fn load_hotkey(app: &AppHandle) -> String {
    load_config(app)
        .map(|c| c.hotkey)
        .unwrap_or_else(|_| default_hotkey())
}

/// 只更新熱鍵，保留指令清單。
pub fn save_hotkey(app: &AppHandle, hotkey: String) -> Result<(), String> {
    let mut config = load_config(app)?;
    config.hotkey = hotkey;
    save_config(app, &config)
}

// ---- 匯入 / 匯出（僅指令，不動熱鍵）----

/// 把目前的指令清單匯出到指定檔案（格式：{ "commands": [...] }）。
pub fn export_to(app: &AppHandle, dest: &Path) -> Result<(), String> {
    let commands = load(app)?;
    let data = serde_json::to_string_pretty(&json!({ "commands": commands }))
        .map_err(|e| format!("序列化失敗: {e}"))?;
    fs::write(dest, data).map_err(|e| format!("寫入匯出檔失敗: {e}"))?;
    Ok(())
}

/// 從指定檔案匯入指令清單並覆蓋目前指令（熱鍵不變）。
/// 接受兩種格式：`{ "commands": [...] }` 或直接是 `[...]` 陣列。
pub fn import_from(app: &AppHandle, src: &Path) -> Result<(), String> {
    let data = fs::read_to_string(src).map_err(|e| format!("讀取匯入檔失敗: {e}"))?;
    let commands = parse_commands(&data)?;
    save(app, commands)
}

fn parse_commands(data: &str) -> Result<Vec<Command>, String> {
    #[derive(Deserialize)]
    struct CommandsOnly {
        commands: Vec<Command>,
    }
    if let Ok(parsed) = serde_json::from_str::<CommandsOnly>(data) {
        return Ok(parsed.commands);
    }
    serde_json::from_str::<Vec<Command>>(data)
        .map_err(|e| format!("JSON 格式不符（需含 id/label/content）: {e}"))
}
