// Tauri 全域注入（withGlobalTauri: true），無需打包器。
const { getCurrentWindow } = window.__TAURI__.window;
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const appWindow = getCurrentWindow();

// 匯入 JSON 後後端會發出此事件，重新載入按鈕。
listen("commands-changed", () => loadAndRender());

const panel = document.getElementById("panel");
const grid = document.getElementById("grid");
const editor = document.getElementById("editor");
const editorTitle = document.getElementById("editorTitle");
const labelInput = document.getElementById("labelInput");
const contentInput = document.getElementById("contentInput");
const saveBtn = document.getElementById("saveBtn");
const cancelBtn = document.getElementById("cancelBtn");
const deleteBtn = document.getElementById("deleteBtn");

let commands = []; // [{ id, label, content }]
let editingId = null; // null = 新增模式

// ---- 載入與渲染 ----
async function loadAndRender() {
  try {
    commands = (await invoke("load_commands")) ?? [];
  } catch (e) {
    console.error("載入指令失敗:", e);
    commands = [];
  }
  renderGrid();
}

function renderGrid() {
  grid.replaceChildren();

  if (commands.length === 0) {
    const ph = document.createElement("button");
    ph.className = "cmd placeholder";
    ph.disabled = true;
    ph.textContent = "（尚無指令，點＋新增）";
    grid.appendChild(ph);
  }

  for (const cmd of commands) {
    const btn = document.createElement("button");
    btn.className = "cmd";
    btn.textContent = cmd.label;
    btn.title = cmd.content;
    btn.dataset.id = cmd.id;
    grid.appendChild(btn);
  }

  const addBtn = document.createElement("button");
  addBtn.className = "cmd add";
  addBtn.id = "addBtn";
  addBtn.textContent = "＋ 新增";
  grid.appendChild(addBtn);
}

// ---- 點擊：左鍵貼上、右鍵編輯 ----
grid.addEventListener("click", (e) => {
  const btn = e.target.closest(".cmd");
  if (!btn) return;
  if (btn.id === "addBtn") {
    openEditor(null);
    return;
  }
  if (btn.disabled) return;
  const cmd = commands.find((c) => c.id === btn.dataset.id);
  if (cmd) invoke("paste_command", { text: cmd.content });
});

grid.addEventListener("contextmenu", (e) => {
  const btn = e.target.closest(".cmd");
  if (!btn || btn.id === "addBtn" || btn.disabled) return;
  e.preventDefault();
  const cmd = commands.find((c) => c.id === btn.dataset.id);
  if (cmd) openEditor(cmd);
});

// ---- 編輯表單 ----
function openEditor(cmd) {
  editingId = cmd ? cmd.id : null;
  editorTitle.textContent = cmd ? "編輯指令" : "新增指令";
  labelInput.value = cmd ? cmd.label : "";
  contentInput.value = cmd ? cmd.content : "";
  deleteBtn.classList.toggle("hidden", !cmd);
  grid.classList.add("hidden");
  editor.classList.remove("hidden");
  // 釘住視窗，避免編輯時點到輸入框以外導致失焦隱藏。
  invoke("set_pinned", { pinned: true });
  labelInput.focus();
}

function closeEditor() {
  editor.classList.add("hidden");
  grid.classList.remove("hidden");
  editingId = null;
  invoke("set_pinned", { pinned: false });
}

async function persist() {
  try {
    await invoke("save_commands", { commands });
  } catch (e) {
    console.error("儲存失敗:", e);
  }
}

saveBtn.addEventListener("click", async () => {
  const label = labelInput.value.trim();
  const content = contentInput.value;
  if (!label) {
    labelInput.focus();
    return;
  }
  if (!content) {
    contentInput.focus();
    return;
  }

  if (editingId) {
    const cmd = commands.find((c) => c.id === editingId);
    if (cmd) {
      cmd.label = label;
      cmd.content = content;
    }
  } else {
    commands.push({ id: crypto.randomUUID(), label, content });
  }

  await persist();
  closeEditor();
  renderGrid();
});

deleteBtn.addEventListener("click", async () => {
  if (!editingId) return;
  commands = commands.filter((c) => c.id !== editingId);
  await persist();
  closeEditor();
  renderGrid();
});

cancelBtn.addEventListener("click", () => closeEditor());

// ---- Esc：表單開啟時先關表單，否則隱藏視窗 ----
window.addEventListener("keydown", (e) => {
  if (e.key !== "Escape") return;
  if (!editor.classList.contains("hidden")) {
    closeEditor();
  } else {
    appWindow.hide();
  }
});

// ---- 視窗高度自適應：依面板內容高度調整視窗 ----
let lastHeight = 0;
function fitWindow() {
  const h = Math.ceil(panel.getBoundingClientRect().height);
  if (h > 0 && h !== lastHeight) {
    lastHeight = h;
    invoke("resize_window", { height: h });
  }
}
// 內容（按鈕數量、表單開關、換行）改變時自動貼合。
new ResizeObserver(() => fitWindow()).observe(panel);

loadAndRender();
