use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

/// 單筆快速指令。
#[derive(Serialize, Deserialize, Clone)]
pub struct Command {
    pub id: String,
    pub label: String,
    pub content: String,
}

/// 設定檔結構（目前只有指令清單，未來可擴充）。
#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub commands: Vec<Command>,
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

/// 讀取指令清單；檔案不存在時回傳空清單。
pub fn load(app: &AppHandle) -> Result<Vec<Command>, String> {
    let path = config_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(&path).map_err(|e| format!("讀取設定失敗: {e}"))?;
    let config: Config = serde_json::from_str(&data).map_err(|e| format!("解析設定失敗: {e}"))?;
    Ok(config.commands)
}

/// 寫入整份指令清單。
pub fn save(app: &AppHandle, commands: Vec<Command>) -> Result<(), String> {
    let path = config_path(app)?;
    let config = Config { commands };
    let data =
        serde_json::to_string_pretty(&config).map_err(|e| format!("序列化設定失敗: {e}"))?;
    fs::write(&path, data).map_err(|e| format!("寫入設定失敗: {e}"))?;
    Ok(())
}

/// 把目前的指令清單匯出到指定檔案。
pub fn export_to(app: &AppHandle, dest: &Path) -> Result<(), String> {
    let commands = load(app)?;
    let config = Config { commands };
    let data =
        serde_json::to_string_pretty(&config).map_err(|e| format!("序列化失敗: {e}"))?;
    fs::write(dest, data).map_err(|e| format!("寫入匯出檔失敗: {e}"))?;
    Ok(())
}

/// 從指定檔案匯入指令清單並覆蓋目前設定。
/// 接受兩種格式：`{ "commands": [...] }` 或直接是 `[...]` 陣列。
pub fn import_from(app: &AppHandle, src: &Path) -> Result<(), String> {
    let data = fs::read_to_string(src).map_err(|e| format!("讀取匯入檔失敗: {e}"))?;
    let commands = parse_commands(&data)?;
    save(app, commands)
}

fn parse_commands(data: &str) -> Result<Vec<Command>, String> {
    if let Ok(config) = serde_json::from_str::<Config>(data) {
        return Ok(config.commands);
    }
    serde_json::from_str::<Vec<Command>>(data)
        .map_err(|e| format!("JSON 格式不符（需含 id/label/content）: {e}"))
}
