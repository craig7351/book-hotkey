// Prevent an extra console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod paste;
mod window;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, State, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const HOTKEY_LABEL: &str = "main";

/// 應用程式共享狀態。
#[derive(Default)]
struct AppState {
    /// 為 true 時暫停「失焦自動隱藏」（編輯指令時用，避免表單被吞掉）。
    pinned: AtomicBool,
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

/// 設定是否釘住視窗（暫停失焦自動隱藏）。編輯指令時設為 true。
#[tauri::command]
fn set_pinned(state: State<AppState>, pinned: bool) {
    state.pinned.store(pinned, Ordering::Relaxed);
}

/// 依前端量測的內容高度調整視窗高度（寬度固定）。
#[tauri::command]
fn resize_window(app: AppHandle, height: f64) {
    if let Some(win) = app.get_webview_window(HOTKEY_LABEL) {
        let _ = win.set_size(tauri::LogicalSize::new(window::WINDOW_WIDTH, height));
    }
}

fn main() {
    // Ctrl+Shift+A — 寫死的全域熱鍵。
    let toggle_hotkey = Shortcut::new(
        Some(Modifiers::CONTROL | Modifiers::SHIFT),
        Code::KeyA,
    );

    tauri::Builder::default()
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
            resize_window
        ])
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if shortcut == &toggle_hotkey && event.state() == ShortcutState::Pressed {
                        if let Some(win) = app.get_webview_window(HOTKEY_LABEL) {
                            window::toggle(&win);
                        }
                    }
                })
                .build(),
        )
        .setup(move |app| {
            // 開機自啟動（首次啟用）。
            use tauri_plugin_autostart::ManagerExt;
            let autostart = app.autolaunch();
            let _ = autostart.enable();

            // 註冊全域熱鍵。
            app.global_shortcut().register(toggle_hotkey)?;

            // 系統匣圖示 + 選單。
            let show_item = MenuItem::with_id(app, "show", "顯示 (Ctrl+Shift+A)", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "結束", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("book-hotkey — 按 Ctrl+Shift+A 叫出")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(win) = app.get_webview_window(HOTKEY_LABEL) {
                            window::show_at_cursor(&win);
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|win, event| match event {
            // 失焦自動隱藏（編輯指令時釘住則略過）。
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
