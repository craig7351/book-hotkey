# book-hotkey

快速貼上指令的跨平台懸浮小工具。背景常駐，按 **Ctrl+Shift+A** 在游標附近叫出懸浮視窗，點擊按鈕即可把預設好的純文字片段貼到游標處。

## 技術棧

- **Tauri 2**（Rust 後端 + 系統 WebView 前端，檔案小、省資源）
- 前端：原生 HTML/CSS/JS（`withGlobalTauri`，無打包器）
- 全域熱鍵：`tauri-plugin-global-shortcut`（寫死 Ctrl+Shift+A）
- 開機自啟動：`tauri-plugin-autostart`

## 開發

```bash
npm install          # 安裝 Tauri CLI
npm run dev          # 開發模式
npm run build        # 打包安裝檔
```

## 發佈（多平台 CI/CD）

推送 `v*` 標籤即觸發 GitHub Actions，於 Windows / macOS (Intel + Apple Silicon) / Linux
同時打包，並建立一個**草稿** Release 附上各平台安裝檔：

```bash
git tag v0.1.0
git push origin v0.1.0
```

完成後會直接發佈 Release（非草稿）。亦可在 Actions 頁面手動 `workflow_dispatch` 觸發。

> macOS/Linux 產物未做程式碼簽章；macOS 首次開啟需右鍵「打開」繞過 Gatekeeper。

## 里程碑

- [x] **M1 骨架** — 系統匣常駐、全域熱鍵、游標定位懸浮視窗、Esc/失焦隱藏、開機自啟動
- [ ] **M2 貼上** — 剪貼簿暫存 → 寫入 → 模擬 Ctrl/Cmd+V → 還原
- [ ] **M3 資料** — JSON 讀寫、動態渲染按鈕、新增/編輯/刪除
- [ ] **M4 跨平台** — macOS Accessibility 權限與 Cmd/Ctrl 差異、Linux X11 驗證
- [ ] **M5 打磨** — 多螢幕邊界、剪貼簿還原、UI 美化

## 已知平台限制

- **macOS**：模擬貼上需「輔助使用 (Accessibility)」權限。
- **Linux**：支援 X11；Wayland 對全域熱鍵與模擬輸入限制多，列為已知限制。
