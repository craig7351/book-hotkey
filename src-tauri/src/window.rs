use tauri::{PhysicalPosition, WebviewWindow};

/// 視窗固定寬度（邏輯像素）；高度由內容動態決定。
pub const WINDOW_WIDTH: f64 = 440.0;

/// 切換顯示／隱藏懸浮視窗。
pub fn toggle(win: &WebviewWindow) {
    match win.is_visible() {
        Ok(true) => {
            let _ = win.hide();
        }
        _ => show_at_cursor(win),
    }
}

/// 在游標附近顯示視窗，並處理多螢幕邊界，避免超出畫面。
pub fn show_at_cursor(win: &WebviewWindow) {
    if let Some(pos) = cursor_position(win) {
        let _ = win.set_position(pos);
    }
    let _ = win.show();
    let _ = win.set_focus();
}

/// 依游標位置算出視窗左上角座標（夾在游標所在螢幕的工作區內）。
fn cursor_position(win: &WebviewWindow) -> Option<PhysicalPosition<i32>> {
    let cursor = win.cursor_position().ok()?; // PhysicalPosition<f64>
    let cursor = PhysicalPosition::new(cursor.x as i32, cursor.y as i32);

    let win_size = win.outer_size().ok()?; // PhysicalSize<u32>
    let (w, h) = (win_size.width as i32, win_size.height as i32);

    // 預設出現在游標右下方一點點的位置。
    let mut x = cursor.x + 8;
    let mut y = cursor.y + 8;

    // 找出游標所在的螢幕，夾住邊界。
    if let Ok(monitors) = win.available_monitors() {
        let monitor = monitors
            .iter()
            .find(|m| {
                let mp = m.position();
                let ms = m.size();
                cursor.x >= mp.x
                    && cursor.x < mp.x + ms.width as i32
                    && cursor.y >= mp.y
                    && cursor.y < mp.y + ms.height as i32
            })
            .or_else(|| monitors.first());

        if let Some(m) = monitor {
            let mp = m.position();
            let ms = m.size();
            let max_x = mp.x + ms.width as i32 - w;
            let max_y = mp.y + ms.height as i32 - h;
            x = x.clamp(mp.x, max_x.max(mp.x));
            y = y.clamp(mp.y, max_y.max(mp.y));
        }
    }

    Some(PhysicalPosition::new(x, y))
}
