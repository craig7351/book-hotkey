// Prevent an extra console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod paste;
mod window;

use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

const HOTKEY_LABEL: &str = "main";

/// 應用程式共享狀態。
#[derive(Default)]
struct AppState {
    /// 為 true 時暫停「失焦自動隱藏」（編輯時用，避免表單被吞掉）。
    pinned: AtomicBool,
    /// 目前已註冊的全域熱鍵，供改鍵時先解除註冊。
    current_hotkey: Mutex<Option<Shortcut>>,
}

/// 前端點擊指令按鈕時呼叫：先隱藏視窗讓焦點回到目標程式，再於背景執行貼上。
#[tauri::command]
fn paste_command(app: AppHandle, text: String) {
    if let Some(win) = app.get_webview_window(HOTKEY_LABEL) {
        let _ = win.hide();
    }
    // 在獨立執行緒處理（含延遲），避免阻塞 UI 執行緒。
    std::thread::spawn(move || {
        // 等待焦點切回目標程式。
        std::thread::sleep(std::time::Duration::from_millis(120));
        if let Err(e) = paste::paste_text(&text) {
            eprintln!("貼上失敗: {e}");
        }
    });
}

/// 讀取已儲存的指令清單。
#[tauri::command]
fn load_commands(app: AppHandle) -> Result<Vec<config::Command>, String> {
    config::load(&app)
}

/// 寫入整份指令清單。
#[tauri::command]
fn save_commands(app: AppHandle, commands: Vec<config::Command>) -> Result<(), String> {
    config::save(&app, commands)
}

/// 設定是否釘住視窗（暫停失焦自動隱藏）。編輯時設為 true。
#[tauri::command]
fn set_pinned(state: State<AppState>, pinned: bool) {
    state.pinned.store(pinned, Ordering::Relaxed);
}

/// 依前端量測的內容高度調整視窗高度（寬度固定）。
#[tauri::command]
fn resize_window(app: AppHandle, height: f64) {
    if let Some(win) = app.get_webview_window(HOTKEY_LABEL) {
        window::set_height_and_fit(&win, height);
    }
}

/// 回傳目前熱鍵（Tauri accelerator 字串）。
#[tauri::command]
fn get_hotkey(app: AppHandle) -> String {
    config::load_hotkey(&app)
}

/// 設定新熱鍵：解除舊的、註冊新的、寫入設定。
#[tauri::command]
fn set_hotkey(app: AppHandle, state: State<AppState>, accelerator: String) -> Result<(), String> {
    let shortcut =
        Shortcut::from_str(&accelerator).map_err(|_| format!("無法解析熱鍵: {accelerator}"))?;

    let gs = app.global_shortcut();

    // 解除舊熱鍵。
    if let Some(old) = state.current_hotkey.lock().unwrap().take() {
        let _ = gs.unregister(old);
    }

    gs.register(shortcut)
        .map_err(|e| format!("註冊熱鍵失敗（可能與其他程式衝突）: {e}"))?;

    *state.current_hotkey.lock().unwrap() = Some(shortcut);
    config::save_hotkey(&app, accelerator)?;
    Ok(())
}

/// 開啟存檔對話框，把指令清單匯出成 JSON。
fn export_commands(app: &AppHandle) {
    let app = app.clone();
    app.dialog()
        .file()
        .add_filter("JSON", &["json"])
        .set_file_name("book-hotkey-commands.json")
        .save_file(move |path| {
            if let Some(p) = path.and_then(|fp| fp.into_path().ok()) {
                if let Err(e) = config::export_to(&app, &p) {
                    eprintln!("匯出失敗: {e}");
                }
            }
        });
}

/// 開啟開檔對話框，匯入 JSON 覆蓋目前指令清單，並通知前端重新載入。
fn import_commands(app: &AppHandle) {
    let app = app.clone();
    app.dialog()
        .file()
        .add_filter("JSON", &["json"])
        .pick_file(move |path| {
            if let Some(p) = path.and_then(|fp| fp.into_path().ok()) {
                match config::import_from(&app, &p) {
                    Ok(()) => {
                        let _ = app.emit("commands-changed", ());
                    }
                    Err(e) => eprintln!("匯入失敗: {e}"),
                }
            }
        });
}

fn main() {
    tauri::Builder::default()
        // 單一實例保護：第二次啟動時叫出既有視窗、不再開新程序。必須最先註冊。
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window(HOTKEY_LABEL) {
                window::show_at_cursor(&win);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            paste_command,
            load_commands,
            save_commands,
            set_pinned,
            resize_window,
            get_hotkey,
            set_hotkey
        ])
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                // 只會註冊一個熱鍵，按下時切換視窗即可。
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        if let Some(win) = app.get_webview_window(HOTKEY_LABEL) {
                            window::toggle(&win);
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            // 開機自啟動（首次啟用）。
            use tauri_plugin_autostart::ManagerExt;
            let _ = app.autolaunch().enable();

            // 讀取設定中的熱鍵並註冊（解析失敗則退回預設）。
            let accelerator = config::load_hotkey(app.handle());
            let shortcut = Shortcut::from_str(&accelerator)
                .or_else(|_| Shortcut::from_str(&config::default_hotkey()))
                .expect("預設熱鍵應可解析");
            app.global_shortcut().register(shortcut)?;
            *app.state::<AppState>().current_hotkey.lock().unwrap() = Some(shortcut);

            // 系統匣圖示 + 選單。
            let show_item = MenuItem::with_id(app, "show", "顯示面板", true, None::<&str>)?;
            let hotkey_item =
                MenuItem::with_id(app, "hotkey", "修改熱鍵…", true, None::<&str>)?;
            let export_item =
                MenuItem::with_id(app, "export", "匯出 JSON…", true, None::<&str>)?;
            let import_item =
                MenuItem::with_id(app, "import", "匯入 JSON…", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "結束", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(
                app,
                &[
                    &show_item,
                    &hotkey_item,
                    &sep1,
                    &export_item,
                    &import_item,
                    &sep2,
                    &quit_item,
                ],
            )?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("book-hotkey")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(win) = app.get_webview_window(HOTKEY_LABEL) {
                            window::show_at_cursor(&win);
                        }
                    }
                    "hotkey" => {
                        // 釘住避免改鍵時失焦隱藏；顯示視窗並通知前端開啟改鍵介面。
                        app.state::<AppState>().pinned.store(true, Ordering::Relaxed);
                        if let Some(win) = app.get_webview_window(HOTKEY_LABEL) {
                            window::show_at_cursor(&win);
                        }
                        let _ = app.emit("edit-hotkey", ());
                    }
                    "export" => export_commands(app),
                    "import" => import_commands(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|win, event| match event {
            // 失焦自動隱藏（編輯/改鍵時釘住則略過）。
            WindowEvent::Focused(false) => {
                let pinned = win.state::<AppState>().pinned.load(Ordering::Relaxed);
                if !pinned {
                    let _ = win.hide();
                }
            }
            // 攔截關閉鍵 → 改為隱藏，保持常駐。
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = win.hide();
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running book-hotkey");
}
