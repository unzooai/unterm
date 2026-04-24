# Cross-Platform Super Terminal 重构计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 Unterm 从卡顿、晃动的终端重构为流畅的 "Super Terminal"，先解决基础终端体验，再叠加 AI 层。

**Architecture:** 核心改变是**消除 TCP 轮询瓶颈**——PTY I/O 直接在 Tauri 进程内完成，通过 Tauri Event 推送到前端，延迟从 50-100ms 降到 <1ms。unterm-core 保留为 MCP Server（供 AI/外部工具使用），但不再参与渲染关键路径。前端 xterm.js 是唯一的 VTE 解析器和渲染器。

**Tech Stack:** Tauri 2 + Rust (portable-pty) + xterm.js 5.5 + WebGL renderer

---

## 阶段一：基础终端体验（最高优先级）

> 目标：做一个比 Windows Terminal 还流畅的基础终端，彻底消除晃动和卡顿。

### Task 1: 直连 PTY 管理器（绕过 TCP/JSON-RPC）

**问题：** 当前架构 `PTY → unterm-core (TCP:19876) → bridge.rs (50ms poll) → 前端 (50ms setInterval)` 引入 50-100ms 延迟。

**方案：** 在 Tauri 进程内直接创建 PTY，用 Tauri Event 推送输出。

**Files:**
- Create: `crates/unterm-app/src/pty_manager.rs`
- Modify: `crates/unterm-app/src/main.rs`
- Modify: `crates/unterm-app/Cargo.toml`

**Step 1: 添加 portable-pty 依赖**

在 `crates/unterm-app/Cargo.toml` 的 `[dependencies]` 中添加：
```toml
portable-pty = "0.8"
base64 = "0.22"
```

**Step 2: 创建 PtyManager 模块**

创建 `crates/unterm-app/src/pty_manager.rs`：

```rust
use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

/// 直连 PTY 会话
struct PtySession {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    /// 用于通知 reader 线程退出
    alive: Arc<std::sync::atomic::AtomicBool>,
}

/// Tauri 进程内 PTY 管理器 — 不经过 TCP/JSON-RPC
pub struct DirectPtyManager {
    sessions: HashMap<u64, PtySession>,
}

impl DirectPtyManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// 创建 PTY 并启动 reader 线程，输出通过 Tauri Event 推送
    pub fn create_session(
        &mut self,
        pane_id: u64,
        shell: Option<String>,
        cwd: Option<String>,
        env: Option<HashMap<String, String>>,
        cols: u16,
        rows: u16,
        app_handle: AppHandle,
    ) -> Result<(), String> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("openpty failed: {}", e))?;

        let shell_cmd = shell.unwrap_or_else(|| {
            if cfg!(windows) {
                "pwsh.exe".to_string()
            } else {
                std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
            }
        });

        let mut cmd = CommandBuilder::new(&shell_cmd);
        if let Some(dir) = &cwd {
            cmd.cwd(dir);
        }
        if let Some(env_map) = &env {
            for (k, v) in env_map {
                cmd.env(k, v);
            }
        }

        let _child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("spawn failed: {}", e))?;

        // writer 用于接收前端输入
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(pair.master.take_writer().map_err(|e| e.to_string())?));

        // reader 线程：PTY 输出 → Tauri Event（零延迟）
        let alive = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let alive_clone = alive.clone();
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;

        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            while alive_clone.load(std::sync::atomic::Ordering::Relaxed) {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        // 直接发送原始字节（lossy UTF-8），不做任何 VTE 解析
                        let content = String::from_utf8_lossy(&buf[..n]).to_string();
                        let _ = app_handle.emit(
                            &format!("pty-output-{}", pane_id),
                            content,
                        );
                    }
                    Err(_) => break,
                }
            }
            // PTY 关闭事件
            let _ = app_handle.emit(&format!("pty-exit-{}", pane_id), ());
        });

        self.sessions.insert(
            pane_id,
            PtySession { writer, alive },
        );
        Ok(())
    }

    /// 向 PTY 写入数据（前端键盘输入）
    pub fn write_input(&self, pane_id: u64, data: &[u8]) -> Result<(), String> {
        let session = self.sessions.get(&pane_id).ok_or("session not found")?;
        let mut w = session.writer.lock().map_err(|e| e.to_string())?;
        w.write_all(data).map_err(|e| e.to_string())?;
        w.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 调整 PTY 尺寸
    pub fn resize(&self, pane_id: u64, cols: u16, rows: u16) -> Result<(), String> {
        // portable-pty resize 需要 master fd，这里先保留接口
        // 实际实现需要保存 PtyPair::master
        Ok(())
    }

    /// 销毁会话
    pub fn destroy_session(&mut self, pane_id: u64) {
        if let Some(session) = self.sessions.remove(&pane_id) {
            session.alive.store(false, std::sync::atomic::Ordering::Relaxed);
        }
    }
}
```

**Step 3: 在 main.rs 中注册新的 Tauri 命令**

在 `crates/unterm-app/src/main.rs` 中添加新的 Tauri 命令，替代现有的 bridge 通信：

```rust
mod pty_manager;
use pty_manager::DirectPtyManager;

struct DirectPtyState(Mutex<DirectPtyManager>);

#[tauri::command]
async fn pty_create(
    pane_id: u64,
    shell: Option<String>,
    cwd: Option<String>,
    env: Option<HashMap<String, String>>,
    cols: u16,
    rows: u16,
    state: tauri::State<'_, DirectPtyState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    state.0.lock().unwrap().create_session(
        pane_id, shell, cwd, env, cols, rows, app_handle,
    )
}

#[tauri::command]
async fn pty_write(
    pane_id: u64,
    data: String,
    state: tauri::State<'_, DirectPtyState>,
) -> Result<(), String> {
    state.0.lock().unwrap().write_input(pane_id, data.as_bytes())
}

#[tauri::command]
async fn pty_resize(
    pane_id: u64,
    cols: u16,
    rows: u16,
    state: tauri::State<'_, DirectPtyState>,
) -> Result<(), String> {
    state.0.lock().unwrap().resize(pane_id, cols, rows)
}

#[tauri::command]
async fn pty_destroy(
    pane_id: u64,
    state: tauri::State<'_, DirectPtyState>,
) -> Result<(), ()> {
    state.0.lock().unwrap().destroy_session(pane_id);
    Ok(())
}
```

在 `main()` 的 Builder 中注册：
```rust
.manage(DirectPtyState(Mutex::new(DirectPtyManager::new())))
.invoke_handler(tauri::generate_handler![
    pty_create, pty_write, pty_resize, pty_destroy,
    // ... 保留原有命令
])
```

**Step 4: 编译验证**

```bash
cd E:\code\unterm
cargo check -p unterm-app
```
Expected: 编译通过，无错误。

**Step 5: 提交**

```bash
git add crates/unterm-app/src/pty_manager.rs crates/unterm-app/src/main.rs crates/unterm-app/Cargo.toml
git commit -m "feat: 新增直连 PTY 管理器，绕过 TCP/JSON-RPC 瓶颈"
```

---

### Task 2: 前端事件驱动替代轮询

**问题：** 前端 50ms `setInterval` 轮询 + `requestAnimationFrame` 批处理引入额外延迟。

**方案：** 使用 Tauri 2 Event 监听，PTY 输出即时写入 xterm.js。

**Files:**
- Modify: `crates/unterm-app/frontend/js/main.js`
- Modify: `crates/unterm-app/frontend/js/terminal.js`

**Step 1: 修改 terminal.js — 使用 Tauri Event 接收输出**

在 `createPane()` 方法中，创建 session 后，注册 Tauri Event 监听器：

```javascript
// 替换原有的 poll_events 机制
// 直接监听 Tauri Event，PTY 输出零延迟到达
const { listen } = window.__TAURI__.event;

const unlisten = await listen(`pty-output-${paneId}`, (event) => {
  terminal.write(event.payload);
});

const unlistenExit = await listen(`pty-exit-${paneId}`, () => {
  terminal.writeln('\r\n\x1b[90m[进程已退出]\x1b[0m');
});

// 保存 unlisten 函数用于清理
pane._unlisten = unlisten;
pane._unlistenExit = unlistenExit;
```

修改 `createPane()` 中的 IPC 调用，使用新的 `pty_create` 命令：

```javascript
// 替换原来的 create_session 调用
const envVars = typeof ProxyManager !== 'undefined' ? await ProxyManager.getEnvVars() : null;
await window.__TAURI__.core.invoke('pty_create', {
  paneId,
  shell: shell || null,
  cwd: cwd || null,
  env: envVars,
  cols: terminal.cols || 80,
  rows: terminal.rows || 24,
});
```

修改 `_sendInput()` 使用新命令：
```javascript
async _sendInput(paneId, data) {
  try {
    await window.__TAURI__.core.invoke('pty_write', { paneId, data });
  } catch (e) {
    console.error('[Unterm] pty_write failed:', e);
  }
},
```

修改 `_resize()` 使用新命令：
```javascript
async _resize(paneId, cols, rows) {
  try {
    await window.__TAURI__.core.invoke('pty_resize', { paneId, cols, rows });
  } catch (e) {}
},
```

修改 `destroyPane()` 清理监听器：
```javascript
destroyPane(paneId) {
  const pane = this.panes.get(paneId);
  if (!pane) return;
  if (pane._unlisten) pane._unlisten();
  if (pane._unlistenExit) pane._unlistenExit();
  if (pane._resizeObserver) pane._resizeObserver.disconnect();
  pane.terminal.dispose();
  this.panes.delete(paneId);
  window.__TAURI__.core.invoke('pty_destroy', { paneId }).catch(() => {});
},
```

**Step 2: 修改 main.js — 移除 50ms 轮询循环**

删除 `setInterval` 轮询和 `_pendingScreen`/`_flushScreen` 相关代码。

替换为简单的连接状态指示：
```javascript
// 不再需要 poll_events 和 setInterval
// 移除: setInterval(async () => { ... poll_events ... }, 50);
// 移除: _pendingScreen, _rafScheduled, _flushScreen

// 连接状态直接设为已连接（PTY 现在是进程内直连）
document.getElementById('connection-status').textContent = '● 已连接';
document.getElementById('connection-status').className = 'connected';
```

**Step 3: 编译运行验证**

```bash
touch crates/unterm-app/build.rs
cargo tauri build 2>&1 | tail -5
```

启动应用，验证：
1. 终端可以输入输出
2. 无延迟感
3. 无左右晃动

**Step 4: 提交**

```bash
git add crates/unterm-app/frontend/js/main.js crates/unterm-app/frontend/js/terminal.js
git commit -m "feat: 事件驱动替代轮询，PTY 输出零延迟推送到前端"
```

---

### Task 3: 启用 WebGL 渲染器

**问题：** Canvas2D 渲染器慢且有亚像素抖动。

**方案：** 重新启用 WebGL renderer，之前因双重 VTE 问题禁用，现在单一 VTE 后应该正常。

**Files:**
- Modify: `crates/unterm-app/frontend/js/terminal.js`

**Step 1: 取消 WebGL Addon 的注释**

在 `terminal.js` 的 `createPane()` 中，恢复 WebGL 加载：

```javascript
// WebGL 渲染器 — 单一 VTE 架构下可安全启用
if (typeof WebglAddon !== 'undefined') {
  try {
    const webglAddon = new WebglAddon.WebglAddon();
    terminal.loadAddon(webglAddon);
    webglAddon.onContextLoss(() => {
      webglAddon.dispose();
    });
  } catch (e) {
    console.warn('[Unterm] WebGL 渲染器加载失败，回退 Canvas:', e);
  }
}
```

**Step 2: 编译运行验证**

```bash
touch crates/unterm-app/build.rs && cargo tauri build
```

验证 TUI 应用（如 htop、vim）渲染流畅，无闪烁。

**Step 3: 提交**

```bash
git add crates/unterm-app/frontend/js/terminal.js
git commit -m "feat: 重新启用 WebGL 渲染器，提升渲染性能"
```

---

### Task 4: 清理残留的 CSS hack 和 scrollbar 补丁

**问题：** 之前为修复晃动做的各种 CSS hack 和 xterm.js 补丁，在新架构下不再需要。

**Files:**
- Modify: `crates/unterm-app/frontend/css/terminal.css`
- Modify: `crates/unterm-app/frontend/lib/addon-fit.js` — 恢复原始 scrollBarWidth 逻辑
- Modify: `crates/unterm-app/frontend/lib/xterm.js` — 恢复原始 scrollBarWidth fallback

**Step 1: 清理 terminal.css**

移除 GPU 合成层 hack（translateZ(0)），保持干净的 CSS：

```css
/* 移除这些 hack */
/* .pane .xterm .xterm-screen { transform: translateZ(0); } */
/* .pane .xterm .xterm-screen canvas { transform: translateZ(0); } */
```

**Step 2: 恢复 addon-fit.js 原始逻辑**

还原 scrollBarWidth 计算：
```javascript
// 恢复原始行: const r=0===this._terminal.options.scrollback?0:e.viewport.scrollBarWidth,
// 之前被改为: const r=0,
```

**Step 3: 恢复 xterm.js scrollBarWidth fallback**

还原 fallback 值：
```javascript
// 恢复原始行: this.scrollBarWidth=...||15
// 之前被改为: this.scrollBarWidth=...||0
```

**Step 4: 编译运行验证**

```bash
touch crates/unterm-app/build.rs && cargo tauri build
```

**Step 5: 提交**

```bash
git add crates/unterm-app/frontend/css/terminal.css crates/unterm-app/frontend/lib/addon-fit.js crates/unterm-app/frontend/lib/xterm.js
git commit -m "refactor: 移除旧架构遗留的 CSS hack 和 scrollbar 补丁"
```

---

### Task 5: 保留 unterm-core MCP Server（供 AI 使用）

**说明：** unterm-core 保持原样运行，供 MCP 工具 / AI agent / CLI 使用。但它不再在渲染关键路径上。

**保留的 bridge.rs 和原有命令暂时不删除**（避免破坏 MCP 功能），只是前端不再调用 `poll_events` / `get_screen`。

后续 AI 功能需要 `screen.read` 等 MCP 工具时，可以通过 unterm-core 提供。

**无代码变更，仅记录决策。**

---

### Task 6: PTY resize 完整实现

**问题：** Task 1 中 `resize()` 只有接口，需要保存 master PTY 引用。

**Files:**
- Modify: `crates/unterm-app/src/pty_manager.rs`

**Step 1: 修改 PtySession 保存 master**

```rust
struct PtySession {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    master: Box<dyn portable_pty::MasterPty + Send>,
    alive: Arc<std::sync::atomic::AtomicBool>,
}
```

**Step 2: 在 create_session 中保存 master**

```rust
// pair.master 需要在 take_writer/try_clone_reader 之后保存
self.sessions.insert(pane_id, PtySession {
    writer,
    master: pair.master,
    alive,
});
```

**Step 3: 实现 resize**

```rust
pub fn resize(&self, pane_id: u64, cols: u16, rows: u16) -> Result<(), String> {
    let session = self.sessions.get(&pane_id).ok_or("session not found")?;
    session.master.resize(PtySize {
        rows, cols,
        pixel_width: 0,
        pixel_height: 0,
    }).map_err(|e| e.to_string())
}
```

**Step 4: 编译验证**

```bash
cargo check -p unterm-app
```

**Step 5: 提交**

```bash
git add crates/unterm-app/src/pty_manager.rs
git commit -m "feat: 完整实现 PTY resize"
```

---

### Task 7: 窗口默认 800x600 + 自定义标题栏

**Files:**
- Modify: `crates/unterm-app/tauri.conf.json`

**Step 1: 修改默认窗口尺寸**

```json
{
  "app": {
    "windows": [{
      "width": 800,
      "height": 600
    }]
  }
}
```

**Step 2: 提交**

```bash
git add crates/unterm-app/tauri.conf.json
git commit -m "feat: 默认窗口尺寸调整为 800x600"
```

---

## 阶段二：AI 功能层（阶段一完成后）

> 前提：基础终端体验已流畅无卡顿后再开始。

### Task 8: AI Insights 右侧面板 — 布局

**Files:**
- Modify: `crates/unterm-app/frontend/index.html`
- Create: `crates/unterm-app/frontend/css/ai-panel.css`
- Create: `crates/unterm-app/frontend/js/ai-panel.js`

**Step 1: 修改 index.html 添加 AI 面板 DOM**

在 `#terminal-area` 旁边添加可折叠的 AI 面板：

```html
<div id="main-content">
  <div id="terminal-area"></div>
  <div id="ai-panel" class="ai-panel hidden">
    <div class="ai-panel-header">
      <span class="ai-panel-title">AI INSIGHTS</span>
      <button class="ai-panel-close" onclick="AiPanel.toggle()">&times;</button>
    </div>
    <div class="ai-panel-body">
      <div id="ai-insights-content"></div>
    </div>
    <div class="ai-panel-input">
      <input type="text" id="ai-chat-input" placeholder="Ask AI..." />
      <button id="ai-chat-send">&#9654;</button>
    </div>
  </div>
</div>
```

**Step 2: 创建 ai-panel.css**

```css
.ai-panel {
  width: 280px;
  min-width: 200px;
  max-width: 400px;
  border-left: 1px solid var(--surface0);
  background: var(--mantle);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.ai-panel.hidden {
  display: none;
}

.ai-panel-header {
  height: var(--tab-height);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  border-bottom: 1px solid var(--surface0);
}

.ai-panel-title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--subtext0);
}

.ai-panel-body {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
  font-size: 13px;
  color: var(--text);
  line-height: 1.5;
}

.ai-panel-input {
  display: flex;
  padding: 8px;
  border-top: 1px solid var(--surface0);
  gap: 4px;
}

#ai-chat-input {
  flex: 1;
  background: var(--surface0);
  border: 1px solid var(--surface1);
  color: var(--text);
  padding: 6px 10px;
  border-radius: 4px;
  font-size: 13px;
  outline: none;
}

#ai-chat-send {
  background: var(--blue);
  border: none;
  color: #fff;
  padding: 6px 10px;
  border-radius: 4px;
  cursor: pointer;
}

/* AI 建议卡片 */
.ai-card {
  background: var(--surface0);
  border-radius: 6px;
  padding: 10px 12px;
  margin-bottom: 8px;
}

.ai-card-title {
  font-size: 11px;
  font-weight: 600;
  color: var(--blue);
  text-transform: uppercase;
  margin-bottom: 4px;
}

/* Execute 按钮 */
.ai-execute-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: var(--green);
  color: var(--base);
  border: none;
  padding: 4px 10px;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
  margin-top: 6px;
}

.ai-execute-btn:hover {
  opacity: 0.9;
}
```

**Step 3: 创建 ai-panel.js**

```javascript
const AiPanel = {
  visible: false,

  toggle() {
    this.visible = !this.visible;
    const panel = document.getElementById('ai-panel');
    panel.classList.toggle('hidden', !this.visible);
    // 重新 fit 终端（面板开关改变了终端区域大小）
    setTimeout(() => TerminalManager.handleResize(), 100);
  },

  show() {
    if (!this.visible) this.toggle();
  },

  hide() {
    if (this.visible) this.toggle();
  },

  // 显示 AI 洞察内容
  setInsights(html) {
    document.getElementById('ai-insights-content').innerHTML = html;
  },

  // 添加一条 AI 建议卡片
  addCard(title, content, command) {
    const container = document.getElementById('ai-insights-content');
    const card = document.createElement('div');
    card.className = 'ai-card';
    card.innerHTML = `
      <div class="ai-card-title">${title}</div>
      <div class="ai-card-body">${content}</div>
      ${command ? `<button class="ai-execute-btn" onclick="AiPanel.executeCommand('${command.replace(/'/g, "\\'")}')">&#9654; Execute in Terminal</button>` : ''}
    `;
    container.appendChild(card);
    container.scrollTop = container.scrollHeight;
  },

  // 一键执行命令到终端
  executeCommand(cmd) {
    const tab = typeof Tabs !== 'undefined' ? Tabs.getActiveTab() : null;
    if (tab) {
      TerminalManager._sendInput(tab.activePaneId, cmd + '\r');
    }
  },

  // 初始化聊天输入
  init() {
    const input = document.getElementById('ai-chat-input');
    const send = document.getElementById('ai-chat-send');
    if (!input || !send) return;

    const doSend = () => {
      const text = input.value.trim();
      if (!text) return;
      input.value = '';
      // TODO: 发送到 AI API
      this.addCard('You', text);
    };

    send.addEventListener('click', doSend);
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') doSend();
    });
  }
};
```

**Step 4: 修改 main-content 布局**

在 `css/style.css` 中确保 `#main-content` 使用 flex 布局：
```css
#main-content {
  display: flex;
  flex: 1;
  overflow: hidden;
}

#terminal-area {
  flex: 1;
  overflow: hidden;
}
```

**Step 5: 在 index.html 中引入新文件**

```html
<link rel="stylesheet" href="css/ai-panel.css" />
<script src="js/ai-panel.js"></script>
```

**Step 6: 添加快捷键 Ctrl+Shift+I 切换 AI 面板**

在 `main.js` 的快捷键注册中添加：
```javascript
if (e.ctrlKey && e.shiftKey && e.key === 'I') {
  e.preventDefault();
  AiPanel.toggle();
}
```

**Step 7: 编译运行验证**

```bash
touch crates/unterm-app/build.rs && cargo tauri build
```

**Step 8: 提交**

```bash
git add crates/unterm-app/frontend/
git commit -m "feat: 添加 AI Insights 右侧面板布局"
```

---

### Task 9: AI Ghost Text 补全（Overlay 层）

**概念：** 命令行上显示淡灰色的 "ghost text" 预测补全，按 Tab 接受。

**Files:**
- Create: `crates/unterm-app/frontend/js/ai-suggest.js`
- Create: `crates/unterm-app/frontend/css/ai-suggest.css`

**Step 1: 创建 ai-suggest.js**

```javascript
const AiSuggest = {
  _overlay: null,
  _currentSuggestion: '',
  _enabled: true,

  init() {
    // 创建 ghost text overlay 元素
    this._overlay = document.createElement('div');
    this._overlay.className = 'ai-ghost-overlay';
    this._overlay.style.display = 'none';
    document.body.appendChild(this._overlay);
  },

  // 显示 ghost text 建议
  show(paneId, suggestion) {
    if (!this._enabled || !suggestion) return;
    this._currentSuggestion = suggestion;

    const pane = TerminalManager.panes.get(paneId);
    if (!pane || !pane.terminal) return;

    // 获取光标位置对应的 DOM 坐标
    const term = pane.terminal;
    const cursorEl = pane.element?.querySelector('.xterm-cursor-layer');
    if (!cursorEl) return;

    // ghost text 定位到光标后方
    const rect = cursorEl.getBoundingClientRect();
    this._overlay.textContent = suggestion;
    this._overlay.style.display = 'block';
    this._overlay.style.left = `${rect.right}px`;
    this._overlay.style.top = `${rect.top}px`;
    this._overlay.style.fontSize = `${term.options.fontSize}px`;
    this._overlay.style.fontFamily = term.options.fontFamily;
    this._overlay.style.lineHeight = `${term.options.lineHeight}`;
  },

  // 隐藏 ghost text
  hide() {
    if (this._overlay) {
      this._overlay.style.display = 'none';
    }
    this._currentSuggestion = '';
  },

  // 接受当前建议（Tab 键触发）
  accept(paneId) {
    if (!this._currentSuggestion) return false;
    TerminalManager._sendInput(paneId, this._currentSuggestion);
    this.hide();
    return true;
  },
};
```

**Step 2: 创建 ai-suggest.css**

```css
.ai-ghost-overlay {
  position: fixed;
  color: var(--overlay0);
  opacity: 0.5;
  pointer-events: none;
  z-index: 100;
  white-space: pre;
}
```

**Step 3: 提交**

```bash
git add crates/unterm-app/frontend/js/ai-suggest.js crates/unterm-app/frontend/css/ai-suggest.css
git commit -m "feat: AI ghost text 补全 overlay 基础框架"
```

---

### Task 10: AI 错误检测与一键修复

**概念：** 检测到命令错误（如拼写错误 `gti` → `git`），显示修复建议 + Apply 按钮。

**方案：** 在 PTY 输出事件中匹配常见错误模式，显示 AI 卡片。

**Files:**
- Create: `crates/unterm-app/frontend/js/ai-error-detect.js`

**Step 1: 创建 ai-error-detect.js**

```javascript
const AiErrorDetect = {
  // 常见命令拼写修正
  _typoMap: {
    'gti': 'git', 'gít': 'git', 'gi': 'git',
    'dcoker': 'docker', 'dokcer': 'docker',
    'pnpn': 'pnpm', 'npn': 'npm',
    'cagro': 'cargo', 'crago': 'cargo',
  },

  // 检查 PTY 输出中是否有错误信号
  check(output) {
    // 检测 "command not found" 类错误
    const notFoundMatch = output.match(
      /['"]?(\w+)['"]?\s*(?:is not recognized|not found|无法识别|command not found)/i
    );
    if (notFoundMatch) {
      const typo = notFoundMatch[1].toLowerCase();
      const fix = this._typoMap[typo];
      if (fix) {
        AiPanel.show();
        AiPanel.addCard(
          'Fix Available',
          `<code>${typo}</code> → <code>${fix}</code>`,
          fix
        );
      }
    }
  }
};
```

**Step 2: 在 terminal.js 的 Tauri Event 回调中挂载检测**

```javascript
// 在 pty-output 监听回调中追加
AiErrorDetect.check(event.payload);
```

**Step 3: 提交**

```bash
git add crates/unterm-app/frontend/js/ai-error-detect.js
git commit -m "feat: AI 命令错误检测与一键修复"
```

---

### Task 11: 模型选择器 UI

**概念：** 状态栏或 AI 面板中的下拉菜单，选择 Claude / Gemini / GPT。

**Files:**
- Create: `crates/unterm-app/frontend/js/ai-models.js`

**Step 1: 创建 ai-models.js**

```javascript
const AiModels = {
  models: [
    { id: 'claude-sonnet-4-6', name: 'Claude Sonnet 4.6', provider: 'anthropic' },
    { id: 'claude-opus-4-6', name: 'Claude Opus 4.6', provider: 'anthropic' },
    { id: 'gemini-2.5-flash', name: 'Gemini 2.5 Flash', provider: 'google' },
    { id: 'gpt-4o', name: 'GPT-4o', provider: 'openai' },
  ],

  current: 'claude-sonnet-4-6',

  getCurrent() {
    return this.models.find(m => m.id === this.current);
  },

  setCurrent(modelId) {
    this.current = modelId;
    localStorage.setItem('ai-model', modelId);
    this._updateUI();
  },

  init() {
    const saved = localStorage.getItem('ai-model');
    if (saved) this.current = saved;
    this._updateUI();
  },

  _updateUI() {
    const el = document.getElementById('ai-model-name');
    if (el) {
      const model = this.getCurrent();
      el.textContent = model ? model.name : this.current;
    }
  }
};
```

**Step 2: 提交**

```bash
git add crates/unterm-app/frontend/js/ai-models.js
git commit -m "feat: AI 模型选择器基础框架"
```

---

### Task 12: AI Flow Optimization — 后续操作推荐

**概念：** 命令成功执行后（如 `git push`），AI 主动推荐下一步（如 "Create PR"）。

**Files:**
- Modify: `crates/unterm-app/frontend/js/ai-error-detect.js` → 重命名为 `ai-context.js`

**Step 1: 扩展为上下文感知系统**

```javascript
const AiContext = {
  // 合并错误检测 + 流程推荐
  _flowSuggestions: {
    'git push': { title: 'Next Step', text: 'Create a pull request?', cmd: 'gh pr create' },
    'npm test': { title: 'Tests Passed', text: 'Ready to commit?', cmd: 'git add -A && git commit' },
    'cargo build': { title: 'Build Complete', text: 'Run tests?', cmd: 'cargo test' },
    'docker build': { title: 'Image Built', text: 'Run the container?', cmd: null },
  },

  checkOutput(output, lastCommand) {
    // 错误检测（同 AiErrorDetect.check）
    // ...

    // 流程推荐
    if (lastCommand) {
      for (const [pattern, suggestion] of Object.entries(this._flowSuggestions)) {
        if (lastCommand.includes(pattern)) {
          AiPanel.addCard(suggestion.title, suggestion.text, suggestion.cmd);
          break;
        }
      }
    }
  }
};
```

**Step 2: 提交**

```bash
git add crates/unterm-app/frontend/js/ai-context.js
git commit -m "feat: AI 上下文感知 — 错误检测 + 后续操作推荐"
```

---

## 阶段三：完善与集成

### Task 13: Shell Integration Hook (OSC 133)

复用 `docs/plans/2026-04-21-block-terminal-design.md` 中已设计的 OSC 133 方案。
在 PTY 创建后注入 shell hook 脚本，实现：
- 命令边界检测
- 退出码获取
- CWD 追踪

这为 AI 上下文分析提供精确的命令/输出边界。

### Task 14: AI API 集成

在设置面板中添加 API Key 配置，支持：
- Anthropic API (Claude)
- Google AI (Gemini)
- OpenAI (GPT)

通过 Tauri 后端代理 API 请求（避免前端暴露 Key）。

### Task 15: 完整的 AI Chat 集成

右侧面板升级为完整对话界面：
- 多轮对话
- 代码块高亮 + "Execute in Terminal" 按钮
- 上下文自动注入（当前终端输出）
- 流式响应

---

## 文件变更总结

### 新建文件
| 文件 | 用途 |
|------|------|
| `src/pty_manager.rs` | 直连 PTY 管理器 |
| `frontend/js/ai-panel.js` | AI 面板控制 |
| `frontend/js/ai-suggest.js` | Ghost text 补全 |
| `frontend/js/ai-context.js` | 错误检测 + 流程推荐 |
| `frontend/js/ai-models.js` | 模型选择器 |
| `frontend/css/ai-panel.css` | AI 面板样式 |
| `frontend/css/ai-suggest.css` | Ghost text 样式 |

### 修改文件
| 文件 | 变更 |
|------|------|
| `src/main.rs` | 注册新 PTY 命令 |
| `Cargo.toml` | 添加 portable-pty 依赖 |
| `frontend/js/main.js` | 移除 50ms 轮询 |
| `frontend/js/terminal.js` | Tauri Event 接收 + WebGL |
| `frontend/css/terminal.css` | 移除 CSS hack |
| `frontend/lib/addon-fit.js` | 恢复原始逻辑 |
| `frontend/lib/xterm.js` | 恢复原始逻辑 |
| `frontend/index.html` | AI 面板 DOM |
| `tauri.conf.json` | 默认 800x600 |

### 不变文件
| 模块 | 说明 |
|------|------|
| `unterm-core/*` | MCP Server 完整保留，供 AI/CLI 使用 |
| `unterm-cli/*` | CLI 不变 |
| `unterm-proto/*` | 协议不变 |
