# Block Terminal 实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 Unterm 从传统流式终端升级为 Block 式终端，实现底部固定输入区、命令 Block 展示、全屏模式切换、主题 YAML 化、# 自然语言命令。

**Architecture:** 前端采用双模式设计：Block 模式（默认）将终端输出按命令分割为可折叠 Block，输入固定底部；全屏模式（TUI 程序自动触发）使用 xterm.js 全屏渲染。通过 OSC 133 Shell Integration 协议检测命令边界。后端 MCP/CLI 完全不动。

**Tech Stack:** Vanilla JavaScript, xterm.js (全屏模式), CSS3, Tauri IPC, Rust bridge

---

### Task 0: 修复 exec.run 换行符

**Files:**
- Modify: `crates/unterm-core/src/mcp/router.rs:223`

**Step 1: 修改 exec.run 中的换行符**

将 `\n` 改为 `\r`，兼容 PowerShell：

```rust
// router.rs:223 — 改 \n 为 \r
.send_input(&session_id, &format!("{}\r", command))
```

**Step 2: 验证编译**

Run: `cargo build -p unterm-core 2>&1 | grep -E "^error|Finished"`
Expected: `Finished`

**Step 3: 提交**

```bash
git add crates/unterm-core/src/mcp/router.rs
git commit -m "fix: exec.run 用 \\r 代替 \\n 兼容 PowerShell"
```

---

### Task 1: ANSI → HTML 渲染器

**Files:**
- Create: `crates/unterm-app/frontend/js/ansi-renderer.js`

**Step 1: 创建 ANSI 渲染器**

解析 ANSI SGR 序列，输出带 CSS class 的 HTML。支持：
- 16 色 + 256 色 + 真彩色（前景/背景）
- 粗体/斜体/下划线/删除线/反转/暗淡
- OSC 8 超链接
- 转义 HTML 特殊字符（防 XSS）

```javascript
// ansi-renderer.js
const AnsiRenderer = {
  // 256色调色板（索引 0-255）
  palette256: null,

  _initPalette() {
    if (this.palette256) return;
    this.palette256 = [];
    // 0-15: 标准色（由主题 CSS 变量控制，用 class）
    for (let i = 0; i < 16; i++) this.palette256.push(null); // null = use class
    // 16-231: 6x6x6 color cube
    for (let r = 0; r < 6; r++)
      for (let g = 0; g < 6; g++)
        for (let b = 0; b < 6; b++)
          this.palette256.push(`rgb(${r?r*40+55:0},${g?g*40+55:0},${b?b*40+55:0})`);
    // 232-255: grayscale
    for (let i = 0; i < 24; i++)
      this.palette256.push(`rgb(${i*10+8},${i*10+8},${i*10+8})`);
  },

  render(ansiText) {
    this._initPalette();
    const result = [];
    let i = 0;
    let currentAttrs = {};
    let openSpan = false;

    while (i < ansiText.length) {
      // ESC sequence
      if (ansiText[i] === '\x1b') {
        // CSI: ESC [
        if (ansiText[i + 1] === '[') {
          const end = ansiText.indexOf('m', i + 2);
          if (end !== -1 && ansiText.substring(i + 2, end).match(/^[\d;]*$/)) {
            // SGR sequence
            const params = ansiText.substring(i + 2, end).split(';').map(Number);
            this._applySgr(params, currentAttrs);
            i = end + 1;
            // 关闭旧 span，开新的
            if (openSpan) { result.push('</span>'); openSpan = false; }
            const style = this._attrsToStyle(currentAttrs);
            if (style) { result.push(`<span ${style}>`); openSpan = true; }
            continue;
          }
          // 跳过其他 CSI 序列
          let j = i + 2;
          while (j < ansiText.length && ansiText[j] >= '\x20' && ansiText[j] <= '\x3f') j++;
          if (j < ansiText.length) j++; // skip final byte
          i = j;
          continue;
        }
        // OSC: ESC ]
        if (ansiText[i + 1] === ']') {
          // OSC 8 超链接: ESC ] 8 ; params ; uri ST
          if (ansiText.substring(i + 2, i + 4) === '8;') {
            const stIdx = ansiText.indexOf('\x1b\\', i + 4);
            const belIdx = ansiText.indexOf('\x07', i + 4);
            const termIdx = stIdx !== -1 && (belIdx === -1 || stIdx < belIdx) ? stIdx : belIdx;
            if (termIdx !== -1) {
              const content = ansiText.substring(i + 4, termIdx);
              const semiIdx = content.indexOf(';');
              if (semiIdx !== -1) {
                const uri = content.substring(semiIdx + 1);
                if (uri) {
                  if (openSpan) { result.push('</span>'); openSpan = false; }
                  result.push(`<a href="${this._escHtml(uri)}" target="_blank" class="ansi-link">`);
                  // 找到关闭 OSC 8
                  // 关闭标签在后面的空 URI OSC 8 处理
                }
              }
              i = termIdx + (ansiText[termIdx] === '\x07' ? 1 : 2);
              continue;
            }
          }
          // 跳过其他 OSC
          const st = ansiText.indexOf('\x07', i + 2);
          const st2 = ansiText.indexOf('\x1b\\', i + 2);
          const end = st !== -1 && (st2 === -1 || st < st2) ? st + 1 : (st2 !== -1 ? st2 + 2 : ansiText.length);
          i = end;
          continue;
        }
        // 跳过其他 ESC 序列
        i += 2;
        continue;
      }

      // 普通字符
      if (ansiText[i] === '\n') {
        result.push('\n');
      } else if (ansiText[i] === '\r') {
        // 忽略 CR
      } else if (ansiText.charCodeAt(i) < 32) {
        // 忽略其他控制字符
      } else {
        result.push(this._escHtml(ansiText[i]));
      }
      i++;
    }

    if (openSpan) result.push('</span>');
    return result.join('');
  },

  _applySgr(params, attrs) {
    if (params.length === 0 || (params.length === 1 && params[0] === 0)) {
      // Reset
      for (const k of Object.keys(attrs)) delete attrs[k];
      return;
    }
    let i = 0;
    while (i < params.length) {
      const p = params[i];
      switch (p) {
        case 0: for (const k of Object.keys(attrs)) delete attrs[k]; break;
        case 1: attrs.bold = true; break;
        case 2: attrs.dim = true; break;
        case 3: attrs.italic = true; break;
        case 4: attrs.underline = true; break;
        case 7: attrs.inverse = true; break;
        case 9: attrs.strikethrough = true; break;
        case 22: delete attrs.bold; delete attrs.dim; break;
        case 23: delete attrs.italic; break;
        case 24: delete attrs.underline; break;
        case 27: delete attrs.inverse; break;
        case 29: delete attrs.strikethrough; break;
        case 30: case 31: case 32: case 33: case 34: case 35: case 36: case 37:
          attrs.fg = p - 30; break;
        case 38:
          if (params[i+1] === 5 && i+2 < params.length) { attrs.fg256 = params[i+2]; delete attrs.fg; delete attrs.fgRgb; i += 2; }
          else if (params[i+1] === 2 && i+4 < params.length) { attrs.fgRgb = `rgb(${params[i+2]},${params[i+3]},${params[i+4]})`; delete attrs.fg; delete attrs.fg256; i += 4; }
          break;
        case 39: delete attrs.fg; delete attrs.fg256; delete attrs.fgRgb; break;
        case 40: case 41: case 42: case 43: case 44: case 45: case 46: case 47:
          attrs.bg = p - 40; break;
        case 48:
          if (params[i+1] === 5 && i+2 < params.length) { attrs.bg256 = params[i+2]; delete attrs.bg; delete attrs.bgRgb; i += 2; }
          else if (params[i+1] === 2 && i+4 < params.length) { attrs.bgRgb = `rgb(${params[i+2]},${params[i+3]},${params[i+4]})`; delete attrs.bg; delete attrs.bg256; i += 4; }
          break;
        case 49: delete attrs.bg; delete attrs.bg256; delete attrs.bgRgb; break;
        case 90: case 91: case 92: case 93: case 94: case 95: case 96: case 97:
          attrs.fg = p - 90 + 8; break;
        case 100: case 101: case 102: case 103: case 104: case 105: case 106: case 107:
          attrs.bg = p - 100 + 8; break;
      }
      i++;
    }
  },

  _attrsToStyle(attrs) {
    const classes = [];
    const styles = [];

    if (attrs.bold) classes.push('ansi-bold');
    if (attrs.dim) classes.push('ansi-dim');
    if (attrs.italic) classes.push('ansi-italic');
    if (attrs.underline) classes.push('ansi-underline');
    if (attrs.strikethrough) classes.push('ansi-strike');
    if (attrs.inverse) classes.push('ansi-inverse');

    // 前景色
    if (attrs.fg !== undefined) classes.push(`ansi-fg-${attrs.fg}`);
    else if (attrs.fg256 !== undefined) {
      if (attrs.fg256 < 16) classes.push(`ansi-fg-${attrs.fg256}`);
      else styles.push(`color:${this.palette256[attrs.fg256]}`);
    }
    else if (attrs.fgRgb) styles.push(`color:${attrs.fgRgb}`);

    // 背景色
    if (attrs.bg !== undefined) classes.push(`ansi-bg-${attrs.bg}`);
    else if (attrs.bg256 !== undefined) {
      if (attrs.bg256 < 16) classes.push(`ansi-bg-${attrs.bg256}`);
      else styles.push(`background:${this.palette256[attrs.bg256]}`);
    }
    else if (attrs.bgRgb) styles.push(`background:${attrs.bgRgb}`);

    if (classes.length === 0 && styles.length === 0) return '';
    let result = '';
    if (classes.length) result += `class="${classes.join(' ')}"`;
    if (styles.length) result += ` style="${styles.join(';')}"`;
    return result;
  },

  _escHtml(str) {
    return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }
};
```

**Step 2: 验证**

在浏览器 DevTools console 手动测试：
```javascript
AnsiRenderer.render('\x1b[1;31mERROR\x1b[0m: something failed')
// 期望: '<span class="ansi-bold ansi-fg-1">ERROR</span>: something failed'
```

**Step 3: 提交**

```bash
git add crates/unterm-app/frontend/js/ansi-renderer.js
git commit -m "feat: 新增 ANSI → HTML 渲染器"
```

---

### Task 2: Block 数据模型与渲染器

**Files:**
- Create: `crates/unterm-app/frontend/js/block-renderer.js`
- Create: `crates/unterm-app/frontend/css/blocks.css`

**Step 1: 创建 Block 渲染器**

```javascript
// block-renderer.js
const BlockRenderer = {
  nextBlockId: 1,

  // 创建新 Block（命令开始执行时调用）
  createBlock(paneId, command, cwd) {
    const block = {
      id: this.nextBlockId++,
      paneId,
      command: command || '',
      cwd: cwd || '',
      outputChunks: [],    // 原始 ANSI 输出片段
      renderedHtml: '',     // 缓存的渲染 HTML
      exitCode: null,
      startTime: Date.now(),
      duration: null,
      collapsed: false,
      state: 'running',     // running | completed | error
    };
    return block;
  },

  // 向 Block 追加输出
  appendOutput(block, data) {
    block.outputChunks.push(data);
    block.renderedHtml = ''; // 清除缓存
  },

  // 完成 Block
  completeBlock(block, exitCode) {
    block.exitCode = exitCode;
    block.duration = Date.now() - block.startTime;
    block.state = exitCode === 0 ? 'completed' : 'error';
    block.renderedHtml = ''; // 清除缓存
  },

  // 获取渲染后的 HTML
  getRenderedOutput(block) {
    if (!block.renderedHtml) {
      const raw = block.outputChunks.join('');
      block.renderedHtml = AnsiRenderer.render(raw);
    }
    return block.renderedHtml;
  },

  // 渲染单个 Block 为 DOM 元素
  renderBlock(block) {
    const el = document.createElement('div');
    el.className = `block block-${block.state}`;
    el.dataset.blockId = block.id;

    // Header
    const header = document.createElement('div');
    header.className = 'block-header';
    header.innerHTML = `
      <span class="block-chevron">${block.collapsed ? '▸' : '▾'}</span>
      <span class="block-prompt">${this._escHtml(block.cwd || '$')}</span>
      <span class="block-command">${this._escHtml(block.command)}</span>
      <span class="block-meta">
        ${block.state === 'running'
          ? '<span class="block-spinner"></span>'
          : `<span class="block-duration">${this._formatDuration(block.duration)}</span>
             <span class="block-exit exit-${block.exitCode === 0 ? 'ok' : 'fail'}">${block.exitCode === 0 ? '✓' : '✗'} ${block.exitCode}</span>`
        }
      </span>
    `;

    // 点击 header 折叠/展开
    header.addEventListener('click', () => {
      block.collapsed = !block.collapsed;
      const chevron = header.querySelector('.block-chevron');
      const body = el.querySelector('.block-body');
      chevron.textContent = block.collapsed ? '▸' : '▾';
      body.style.display = block.collapsed ? 'none' : '';
    });

    el.appendChild(header);

    // Body
    const body = document.createElement('div');
    body.className = 'block-body';
    body.style.display = block.collapsed ? 'none' : '';

    const output = document.createElement('pre');
    output.className = 'block-output';
    output.innerHTML = this.getRenderedOutput(block);
    body.appendChild(output);

    el.appendChild(body);

    // 右键复制
    el.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      this._showBlockMenu(e, block);
    });

    return el;
  },

  // 更新运行中 Block 的输出（增量追加）
  updateRunningBlock(blockEl, block) {
    const output = blockEl.querySelector('.block-output');
    if (output) {
      output.innerHTML = this.getRenderedOutput(block);
      // 自动滚动到底部
      const container = blockEl.closest('.block-list');
      if (container) container.scrollTop = container.scrollHeight;
    }
  },

  // 更新 Block 完成状态
  updateBlockComplete(blockEl, block) {
    blockEl.className = `block block-${block.state}`;
    const meta = blockEl.querySelector('.block-meta');
    if (meta) {
      meta.innerHTML = `
        <span class="block-duration">${this._formatDuration(block.duration)}</span>
        <span class="block-exit exit-${block.exitCode === 0 ? 'ok' : 'fail'}">${block.exitCode === 0 ? '✓' : '✗'} ${block.exitCode}</span>
      `;
    }
  },

  _formatDuration(ms) {
    if (ms === null || ms === undefined) return '';
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    const min = Math.floor(ms / 60000);
    const sec = Math.floor((ms % 60000) / 1000);
    return `${min}m${sec}s`;
  },

  _escHtml(str) {
    if (!str) return '';
    return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  },

  _showBlockMenu(e, block) {
    // 复用现有的 context-menu 或创建 Block 专用菜单
    const output = block.outputChunks.join('');
    const items = [
      { label: '复制命令', action: () => navigator.clipboard.writeText(block.command) },
      { label: '复制输出', action: () => navigator.clipboard.writeText(output.replace(/\x1b\[[0-9;]*m/g, '')) },
      { label: block.collapsed ? '展开' : '折叠', action: () => {
        block.collapsed = !block.collapsed;
        const el = document.querySelector(`[data-block-id="${block.id}"]`);
        if (el) {
          el.querySelector('.block-chevron').textContent = block.collapsed ? '▸' : '▾';
          el.querySelector('.block-body').style.display = block.collapsed ? 'none' : '';
        }
      }},
    ];
    // 显示菜单（复用 terminal.js 的右键菜单模式）
    const menu = document.getElementById('context-menu');
    menu.innerHTML = '';
    items.forEach(item => {
      const div = document.createElement('div');
      div.className = 'ctx-item';
      div.textContent = item.label;
      div.addEventListener('click', () => { item.action(); menu.classList.add('hidden'); });
      menu.appendChild(div);
    });
    menu.style.left = e.clientX + 'px';
    menu.style.top = e.clientY + 'px';
    menu.classList.remove('hidden');
    setTimeout(() => document.addEventListener('click', () => menu.classList.add('hidden'), { once: true }), 0);
  }
};
```

**Step 2: 创建 Block CSS 样式**

```css
/* blocks.css */

/* Block 列表容器 */
.block-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px;
  scroll-behavior: smooth;
}

/* 单个 Block */
.block {
  margin-bottom: 8px;
  border: 1px solid var(--surface0, #313244);
  border-radius: 6px;
  background: var(--mantle, #181825);
  overflow: hidden;
  transition: border-color 0.15s;
}

.block:hover {
  border-color: var(--overlay0, #6c7086);
}

.block-error {
  border-left: 3px solid var(--red, #f38ba8);
}

.block-running {
  border-left: 3px solid var(--blue, #89b4fa);
}

.block-completed {
  border-left: 3px solid var(--green, #a6e3a1);
}

/* Block Header */
.block-header {
  display: flex;
  align-items: center;
  padding: 6px 10px;
  background: var(--surface0, #313244);
  cursor: pointer;
  user-select: none;
  gap: 8px;
  font-family: var(--font-mono, 'Cascadia Mono', 'Consolas', monospace);
  font-size: 13px;
}

.block-header:hover {
  background: var(--surface1, #45475a);
}

.block-chevron {
  color: var(--subtext0, #a6adc8);
  width: 12px;
  flex-shrink: 0;
}

.block-prompt {
  color: var(--subtext0, #a6adc8);
  font-size: 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 200px;
  flex-shrink: 0;
}

.block-command {
  color: var(--text, #cdd6f4);
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}

.block-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  font-size: 12px;
}

.block-duration {
  color: var(--subtext1, #bac2de);
}

.block-exit {
  padding: 1px 6px;
  border-radius: 3px;
  font-size: 11px;
  font-weight: 600;
}

.exit-ok {
  color: var(--green, #a6e3a1);
  background: rgba(166, 227, 161, 0.1);
}

.exit-fail {
  color: var(--red, #f38ba8);
  background: rgba(243, 139, 168, 0.1);
}

/* Block Body */
.block-body {
  border-top: 1px solid var(--surface0, #313244);
}

.block-output {
  margin: 0;
  padding: 8px 12px;
  font-family: var(--font-mono, 'Cascadia Mono', 'Consolas', monospace);
  font-size: 13px;
  line-height: 1.4;
  color: var(--text, #cdd6f4);
  white-space: pre-wrap;
  word-break: break-all;
  overflow-x: auto;
  max-height: 600px;
  overflow-y: auto;
}

/* Block Spinner (running) */
.block-spinner {
  display: inline-block;
  width: 12px;
  height: 12px;
  border: 2px solid var(--surface1, #45475a);
  border-top-color: var(--blue, #89b4fa);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* ANSI 颜色 class — 16 色由主题 CSS 变量控制 */
.ansi-bold { font-weight: bold; }
.ansi-dim { opacity: 0.7; }
.ansi-italic { font-style: italic; }
.ansi-underline { text-decoration: underline; }
.ansi-strike { text-decoration: line-through; }
.ansi-inverse { filter: invert(1); }

/* 标准 16 色 — 前景 */
.ansi-fg-0 { color: var(--ansi-black, #45475a); }
.ansi-fg-1 { color: var(--ansi-red, #f38ba8); }
.ansi-fg-2 { color: var(--ansi-green, #a6e3a1); }
.ansi-fg-3 { color: var(--ansi-yellow, #f9e2af); }
.ansi-fg-4 { color: var(--ansi-blue, #89b4fa); }
.ansi-fg-5 { color: var(--ansi-magenta, #cba6f7); }
.ansi-fg-6 { color: var(--ansi-cyan, #94e2d5); }
.ansi-fg-7 { color: var(--ansi-white, #bac2de); }
.ansi-fg-8 { color: var(--ansi-bright-black, #585b70); }
.ansi-fg-9 { color: var(--ansi-bright-red, #f38ba8); }
.ansi-fg-10 { color: var(--ansi-bright-green, #a6e3a1); }
.ansi-fg-11 { color: var(--ansi-bright-yellow, #f9e2af); }
.ansi-fg-12 { color: var(--ansi-bright-blue, #89b4fa); }
.ansi-fg-13 { color: var(--ansi-bright-magenta, #cba6f7); }
.ansi-fg-14 { color: var(--ansi-bright-cyan, #94e2d5); }
.ansi-fg-15 { color: var(--ansi-bright-white, #a6adc8); }

/* 标准 16 色 — 背景 */
.ansi-bg-0 { background: var(--ansi-black, #45475a); }
.ansi-bg-1 { background: var(--ansi-red, #f38ba8); }
.ansi-bg-2 { background: var(--ansi-green, #a6e3a1); }
.ansi-bg-3 { background: var(--ansi-yellow, #f9e2af); }
.ansi-bg-4 { background: var(--ansi-blue, #89b4fa); }
.ansi-bg-5 { background: var(--ansi-magenta, #cba6f7); }
.ansi-bg-6 { background: var(--ansi-cyan, #94e2d5); }
.ansi-bg-7 { background: var(--ansi-white, #bac2de); }
.ansi-bg-8 { background: var(--ansi-bright-black, #585b70); }
.ansi-bg-9 { background: var(--ansi-bright-red, #f38ba8); }
.ansi-bg-10 { background: var(--ansi-bright-green, #a6e3a1); }
.ansi-bg-11 { background: var(--ansi-bright-yellow, #f9e2af); }
.ansi-bg-12 { background: var(--ansi-bright-blue, #89b4fa); }
.ansi-bg-13 { background: var(--ansi-bright-magenta, #cba6f7); }
.ansi-bg-14 { background: var(--ansi-bright-cyan, #94e2d5); }
.ansi-bg-15 { background: var(--ansi-bright-white, #a6adc8); }

/* 超链接 */
.ansi-link {
  color: var(--blue, #89b4fa);
  text-decoration: underline;
  cursor: pointer;
}
.ansi-link:hover {
  color: var(--mauve, #cba6f7);
}
```

**Step 3: 提交**

```bash
git add crates/unterm-app/frontend/js/block-renderer.js crates/unterm-app/frontend/css/blocks.css
git commit -m "feat: Block 数据模型、渲染器和样式"
```

---

### Task 3: Shell Integration Hook

**Files:**
- Create: `crates/unterm-app/frontend/js/shell-hook.js`

**Step 1: 创建 Shell Hook 管理器**

向 PTY 注入 precmd/preexec hook 脚本，发送 OSC 133 标记。

```javascript
// shell-hook.js
const ShellHook = {
  // 获取 hook 注入脚本（在 session 创建后立即发送到 PTY）
  getHookScript(shellName) {
    const shell = (shellName || '').toLowerCase();

    if (shell.includes('powershell') || shell.includes('pwsh')) {
      return this._powershellHook();
    }
    if (shell.includes('bash')) {
      return this._bashHook();
    }
    if (shell.includes('zsh')) {
      return this._zshHook();
    }
    if (shell.includes('cmd')) {
      // CMD 不支持 hook，返回 null
      return null;
    }
    if (shell.includes('fish')) {
      return this._fishHook();
    }
    // 默认尝试 PowerShell hook（Windows 默认 shell）
    return this._powershellHook();
  },

  _powershellHook() {
    // PowerShell prompt 函数注入
    // 用 iex + base64 编码避免引号转义问题
    return [
      // 定义新的 prompt 函数
      'function global:prompt {',
      '  $exitCode = if ($?) { 0 } else { if ($LASTEXITCODE) { $LASTEXITCODE } else { 1 } }',
      '  $cwd = $executionContext.SessionState.Path.CurrentLocation.Path',
      // D: 上一条命令结束
      '  [Console]::Write("`e]133;D;$exitCode`a")',
      // A: 新 prompt 开始
      '  [Console]::Write("`e]133;A`a")',
      '  $out = "PS $cwd> "',
      '  [Console]::Write($out)',
      // B: 命令输入区开始（用户即将输入）
      '  [Console]::Write("`e]133;B`a")',
      '  return " "',  // 返回空格，实际提示已经 Write 了
      '}',
      '' // 换行确保执行
    ].join('\r');
  },

  _bashHook() {
    return [
      // preexec — 命令即将执行
      '__unterm_preexec() { printf "\\e]133;C\\a"; }',
      'trap \'__unterm_preexec\' DEBUG',
      // precmd — 命令执行完毕
      '__unterm_precmd() { printf "\\e]133;D;$?\\a\\e]133;A\\a"; }',
      'PROMPT_COMMAND="__unterm_precmd${PROMPT_COMMAND:+;$PROMPT_COMMAND}"',
      // PS1 尾部加 OSC 133;B
      'PS1="${PS1}\\[\\e]133;B\\a\\]"',
      '' // 换行
    ].join('\n');
  },

  _zshHook() {
    return [
      'autoload -Uz add-zsh-hook',
      '__unterm_precmd() { print -Pn "\\e]133;D;$?\\a\\e]133;A\\a" }',
      '__unterm_preexec() { print -Pn "\\e]133;C\\a" }',
      'add-zsh-hook precmd __unterm_precmd',
      'add-zsh-hook preexec __unterm_preexec',
      'PS1="${PS1}%{\\e]133;B\\a%}"',
      ''
    ].join('\n');
  },

  _fishHook() {
    return [
      'function __unterm_postexec --on-event fish_postexec',
      '  printf "\\e]133;D;%d\\a" $status',
      'end',
      'function __unterm_prompt --on-event fish_prompt',
      '  printf "\\e]133;A\\a"',
      'end',
      'function __unterm_preexec --on-event fish_preexec',
      '  printf "\\e]133;C\\a"',
      'end',
      ''
    ].join('\n');
  },

  // 解析 OSC 133 标记，返回事件类型
  // 返回: { type: 'prompt_start'|'command_start'|'command_executed'|'command_finished', exitCode?: number }
  // 或 null（不是 OSC 133）
  parseOsc133(data) {
    // OSC 133;X 的格式: \x1b]133;X\x07 或 \x1b]133;X;params\x07
    const matches = [];
    const regex = /\x1b\]133;([ABCD])(?:;([^\x07\x1b]*))?\x07/g;
    let m;
    while ((m = regex.exec(data)) !== null) {
      const type = m[1];
      const params = m[2] || '';
      switch (type) {
        case 'A': matches.push({ type: 'prompt_start', index: m.index, length: m[0].length }); break;
        case 'B': matches.push({ type: 'command_start', index: m.index, length: m[0].length }); break;
        case 'C': matches.push({ type: 'command_executed', index: m.index, length: m[0].length }); break;
        case 'D':
          const exitCode = parseInt(params) || 0;
          matches.push({ type: 'command_finished', exitCode, index: m.index, length: m[0].length });
          break;
      }
    }
    return matches;
  },

  // 从数据中去除 OSC 133 标记
  stripOsc133(data) {
    return data.replace(/\x1b\]133;[ABCD](?:;[^\x07\x1b]*)?\x07/g, '');
  }
};
```

**Step 2: 提交**

```bash
git add crates/unterm-app/frontend/js/shell-hook.js
git commit -m "feat: Shell Integration hook (OSC 133) 管理器"
```

---

### Task 4: 底部输入编辑器

**Files:**
- Create: `crates/unterm-app/frontend/js/input-editor.js`
- Create: `crates/unterm-app/frontend/css/input-editor.css`

**Step 1: 创建输入编辑器**

```javascript
// input-editor.js
const InputEditor = {
  history: [],
  historyIndex: -1,
  currentInput: '',
  paneId: null,  // 当前绑定的 pane

  init(containerEl) {
    this.container = containerEl;
    this._render();
  },

  _render() {
    this.container.innerHTML = `
      <div class="input-editor">
        <div class="input-prompt">
          <span class="input-cwd" id="input-cwd">~</span>
          <span class="input-arrow">❯</span>
        </div>
        <div class="input-field-wrap">
          <textarea id="input-field" class="input-field" rows="1"
            placeholder="输入命令... (#自然语言)"
            spellcheck="false" autocomplete="off"></textarea>
        </div>
        <button class="input-send-btn" id="input-send-btn" title="执行 (Enter)">⏎</button>
      </div>
    `;

    this.field = document.getElementById('input-field');
    this.cwdEl = document.getElementById('input-cwd');
    this.sendBtn = document.getElementById('input-send-btn');

    // 自动增高
    this.field.addEventListener('input', () => this._autoResize());

    // 键盘事件
    this.field.addEventListener('keydown', (e) => this._handleKey(e));

    // 发送按钮
    this.sendBtn.addEventListener('click', () => this._submit());

    // 自动聚焦
    this.field.focus();
  },

  _handleKey(e) {
    // Enter: 执行命令（Shift+Enter: 换行）
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      this._submit();
      return;
    }

    // 上箭头: 历史上一条
    if (e.key === 'ArrowUp' && this.field.selectionStart === 0) {
      e.preventDefault();
      if (this.historyIndex < this.history.length - 1) {
        if (this.historyIndex === -1) this.currentInput = this.field.value;
        this.historyIndex++;
        this.field.value = this.history[this.history.length - 1 - this.historyIndex];
        this._autoResize();
      }
      return;
    }

    // 下箭头: 历史下一条
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (this.historyIndex > 0) {
        this.historyIndex--;
        this.field.value = this.history[this.history.length - 1 - this.historyIndex];
      } else if (this.historyIndex === 0) {
        this.historyIndex = -1;
        this.field.value = this.currentInput;
      }
      this._autoResize();
      return;
    }

    // Ctrl+C: 清空输入或发 SIGINT
    if (e.key === 'c' && e.ctrlKey && !e.shiftKey) {
      if (this.field.value.trim() === '') {
        // 输入为空，发 SIGINT 到 shell
        if (this.paneId !== null) {
          TerminalManager._sendInput(this.paneId, '\x03');
        }
      } else {
        // 有内容，清空
        this.field.value = '';
        this._autoResize();
      }
      e.preventDefault();
      return;
    }

    // Ctrl+L: 清屏（清空 Block 列表）
    if (e.key === 'l' && e.ctrlKey) {
      e.preventDefault();
      this.onClearBlocks?.();
      return;
    }

    // Tab: 补全（TODO: 后续接入 shell 补全）
    if (e.key === 'Tab') {
      e.preventDefault();
      // 暂时不实现
      return;
    }

    // Escape: 清空输入
    if (e.key === 'Escape') {
      this.field.value = '';
      this.historyIndex = -1;
      this._autoResize();
      return;
    }
  },

  _submit() {
    const cmd = this.field.value.trim();
    if (!cmd) return;

    // # 自然语言模式
    if (cmd.startsWith('#')) {
      this.onAiCommand?.(cmd.substring(1).trim());
      this.field.value = '';
      this._autoResize();
      return;
    }

    // 记录历史
    if (this.history[this.history.length - 1] !== cmd) {
      this.history.push(cmd);
    }
    this.historyIndex = -1;
    this.currentInput = '';

    // 发送命令
    this.onSubmit?.(cmd);

    // 清空输入
    this.field.value = '';
    this._autoResize();
  },

  _autoResize() {
    this.field.style.height = 'auto';
    this.field.style.height = Math.min(this.field.scrollHeight, 120) + 'px';
  },

  // 更新 CWD 显示
  setCwd(cwd) {
    if (this.cwdEl) {
      // 缩短路径显示
      let display = cwd || '~';
      const home = display.match(/^[A-Z]:\\Users\\[^\\]+/i);
      if (home) display = display.replace(home[0], '~');
      this.cwdEl.textContent = display;
      this.cwdEl.title = cwd;
    }
  },

  // 聚焦输入框
  focus() {
    if (this.field) this.field.focus();
  },

  // 设置 AI 建议命令（# 自然语言结果）
  setSuggestion(command) {
    this.field.value = command;
    this.field.classList.add('input-suggestion');
    this._autoResize();
    this.field.focus();
    this.field.select();
  },

  clearSuggestion() {
    this.field.classList.remove('input-suggestion');
  },

  // 回调接口
  onSubmit: null,     // (command: string) => void
  onAiCommand: null,  // (query: string) => void
  onClearBlocks: null, // () => void
};
```

**Step 2: 创建输入编辑器样式**

```css
/* input-editor.css */

/* 输入区容器 */
.input-editor-container {
  flex-shrink: 0;
  border-top: 1px solid var(--surface0, #313244);
  background: var(--base, #1e1e2e);
}

.input-editor {
  display: flex;
  align-items: flex-end;
  padding: 8px 12px;
  gap: 8px;
}

/* 提示符 */
.input-prompt {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
  padding-bottom: 4px;
  font-family: var(--font-mono, 'Cascadia Mono', monospace);
  font-size: 13px;
}

.input-cwd {
  color: var(--blue, #89b4fa);
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.input-arrow {
  color: var(--green, #a6e3a1);
  font-weight: bold;
}

/* 输入框 */
.input-field-wrap {
  flex: 1;
  min-width: 0;
}

.input-field {
  width: 100%;
  padding: 6px 10px;
  background: var(--surface0, #313244);
  border: 1px solid var(--surface1, #45475a);
  border-radius: 6px;
  color: var(--text, #cdd6f4);
  font-family: var(--font-mono, 'Cascadia Mono', monospace);
  font-size: 14px;
  line-height: 1.4;
  resize: none;
  outline: none;
  overflow: hidden;
}

.input-field:focus {
  border-color: var(--blue, #89b4fa);
  box-shadow: 0 0 0 1px var(--blue, #89b4fa);
}

.input-field::placeholder {
  color: var(--subtext0, #a6adc8);
}

/* AI 建议状态 */
.input-field.input-suggestion {
  color: var(--mauve, #cba6f7);
  border-color: var(--mauve, #cba6f7);
}

/* 发送按钮 */
.input-send-btn {
  flex-shrink: 0;
  width: 32px;
  height: 32px;
  border: none;
  border-radius: 6px;
  background: var(--blue, #89b4fa);
  color: var(--base, #1e1e2e);
  font-size: 16px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s;
}

.input-send-btn:hover {
  background: var(--mauve, #cba6f7);
}
```

**Step 3: 提交**

```bash
git add crates/unterm-app/frontend/js/input-editor.js crates/unterm-app/frontend/css/input-editor.css
git commit -m "feat: 底部固定输入编辑器"
```

---

### Task 5: 重构终端区域 — Block 模式集成

**Files:**
- Modify: `crates/unterm-app/frontend/index.html`
- Modify: `crates/unterm-app/frontend/js/main.js`
- Modify: `crates/unterm-app/frontend/js/terminal.js`
- Modify: `crates/unterm-app/frontend/js/tabs.js`

**Step 1: 修改 HTML — 新增 Block 容器和输入区**

在 `index.html` 中：
- `#terminal-area` 内部改为包含 `.block-list`（Block 模式）和 `.fullscreen-terminal`（全屏模式）
- 底部新增 `#input-editor-container`
- 引入新的 JS/CSS 文件

**Step 2: 修改 tabs.js — Tab 数据模型加 blocks**

每个 tab 增加 `blocks` 数组和 `blockMode` 状态。

**Step 3: 修改 terminal.js — 接管输出流**

- `writeToPane()` 不再直接写入 xterm.js
- 解析 OSC 133 标记，分割为 Block
- Block 模式下：追加输出到当前 Block
- 全屏模式下：直接写入 xterm.js

**Step 4: 修改 main.js — 绑定 InputEditor 和 BlockRenderer**

- `handleCoreEvent` 中 `screen_update` 事件走新的 Block 处理流程
- 初始化 InputEditor，绑定命令提交
- session 创建后注入 Shell Hook

**Step 5: 提交**

```bash
git add crates/unterm-app/frontend/
git commit -m "feat: Block 模式集成 — 替换流式输出为 Block UI"
```

---

### Task 6: 全屏模式自动切换

**Files:**
- Modify: `crates/unterm-app/frontend/js/terminal.js`
- Modify: `crates/unterm-app/frontend/css/terminal.css`

**Step 1: 检测 alternate screen 序列**

在输出流中检测 `\x1b[?1049h`（进入全屏）和 `\x1b[?1049l`（退出全屏），切换 DOM 可见性。

**Step 2: 全屏模式下复用 xterm.js**

每个 pane 始终保持一个 xterm.js 实例（隐藏），全屏模式下显示它并直接写入。

**Step 3: 提交**

```bash
git add crates/unterm-app/frontend/
git commit -m "feat: TUI 程序全屏模式自动切换"
```

---

### Task 7: Block 交互增强

**Files:**
- Modify: `crates/unterm-app/frontend/js/block-renderer.js`
- Modify: `crates/unterm-app/frontend/css/blocks.css`

**Step 1: 添加 Block 搜索**

Ctrl+F 在所有 Block 中搜索文本，高亮匹配。

**Step 2: 添加全部折叠/展开**

Ctrl+Shift+[ 折叠所有 Block，Ctrl+Shift+] 展开所有。

**Step 3: 提交**

```bash
git add crates/unterm-app/frontend/
git commit -m "feat: Block 搜索和批量折叠"
```

---

### Task 8: 主题 YAML 化

**Files:**
- Modify: `crates/unterm-app/frontend/js/themes.js`
- Modify: `crates/unterm-app/frontend/js/settings.js`
- Modify: `crates/unterm-app/src/main.rs` (新增 Tauri command 读写 YAML)

**Step 1: 添加 YAML 主题加载**

- 内置主题保留在 JS 中作为默认
- 支持从 `~/.unterm/themes/*.yaml` 加载自定义主题
- 通过 Tauri IPC 读取 YAML 文件
- 设置面板新增"导入主题"按钮

**Step 2: 提交**

```bash
git add crates/unterm-app/
git commit -m "feat: 支持 YAML 自定义主题"
```

---

### Task 9: # 自然语言命令

**Files:**
- Create: `crates/unterm-app/frontend/js/ai-command.js`
- Modify: `crates/unterm-app/frontend/js/settings.js` (AI Key 设置)
- Modify: `crates/unterm-app/frontend/js/input-editor.js` (接入 AI)

**Step 1: 创建 AI 命令模块**

- 用户在输入框输入 `# 自然语言描述`
- 调用 AI API（Claude/OpenAI）将描述转为命令
- 返回结果显示在输入框中，高亮为建议状态
- Enter 确认执行，Esc 取消

**Step 2: 设置面板新增 AI 配置**

- API Provider 选择（Claude / OpenAI）
- API Key 输入
- 模型选择

**Step 3: 提交**

```bash
git add crates/unterm-app/frontend/
git commit -m "feat: # 自然语言转命令"
```

---

### Task 10: 集成测试与打包

**Step 1: 编译验证**

Run: `cargo build 2>&1 | grep -E "^error|Finished"`
Expected: `Finished`

**Step 2: 启动应用手动测试**

验证清单：
- [ ] Block 模式：命令输入 → 输出显示为 Block
- [ ] Block 元数据：耗时、退出码正确
- [ ] Block 折叠/展开
- [ ] Block 右键复制
- [ ] 底部输入区：Enter 发送、Shift+Enter 换行
- [ ] 历史记录：上下箭头
- [ ] Ctrl+C / Escape 清空
- [ ] 全屏模式：vim / less 自动切换到 xterm.js
- [ ] 退出 vim 后回到 Block 模式
- [ ] 主题切换正常
- [ ] 分屏功能正常
- [ ] MCP 19 项测试全部通过

**Step 3: 打包安装包**

Run: `cargo tauri build`

**Step 4: 提交**

```bash
git add -A
git commit -m "chore: Block Terminal 集成测试通过"
```
