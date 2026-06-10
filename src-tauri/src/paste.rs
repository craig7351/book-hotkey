use std::thread;
use std::time::Duration;

use arboard::Clipboard;
use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};

/// 把文字貼到目前前景程式的游標處。
///
/// 流程：暫存原本剪貼簿 → 寫入指令文字 → 模擬貼上快捷鍵 → 還原剪貼簿。
/// 呼叫端必須**先隱藏懸浮視窗**，讓焦點回到目標程式後再呼叫此函式。
pub fn paste_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| format!("無法存取剪貼簿: {e}"))?;

    // 暫存原本的剪貼簿文字（取不到就略過還原）。
    let previous = clipboard.get_text().ok();

    clipboard
        .set_text(text.to_string())
        .map_err(|e| format!("寫入剪貼簿失敗: {e}"))?;

    // 給系統一點時間讓剪貼簿內容生效。
    thread::sleep(Duration::from_millis(60));

    simulate_paste()?;

    // 等目標程式讀完剪貼簿後再還原，避免競態。
    thread::sleep(Duration::from_millis(180));
    if let Some(prev) = previous {
        let _ = clipboard.set_text(prev);
    }

    Ok(())
}

/// 模擬貼上快捷鍵：macOS 用 Cmd+V，其餘平台用 Ctrl+V。
fn simulate_paste() -> Result<(), String> {
    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("無法初始化輸入模擬: {e}"))?;

    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    enigo
        .key(modifier, Press)
        .map_err(|e| format!("按下修飾鍵失敗: {e}"))?;
    enigo
        .key(Key::Unicode('v'), Click)
        .map_err(|e| format!("按下 V 失敗: {e}"))?;
    enigo
        .key(modifier, Release)
        .map_err(|e| format!("放開修飾鍵失敗: {e}"))?;

    Ok(())
}
