# Unterm 2.0 — Block Terminal 设计文档

## 目标

将 Unterm 从传统流式终端升级为 Block 式终端，借鉴 Warp 的核心交互创新，同时保持 MCP Server / CLI 完全不受影响。

## 架构

### 双模式终端

- **Block 模式（默认）**：每条命令+输出是独立 Block，输入框固定底部
- **全屏模式（自动）**：TUI 程序（vim/htop/less）激活 alternate screen 时自动切换到 xterm.js 全屏

### 不变的部分

- `unterm-core`：MCP Server、Session 管理、PTY、Grid — 完全不动
- `bridge.rs`：Tauri IPC 桥接 — 仅新增事件类型
- CLI（`unterm-cli`）— 完全不动

### 改动范围

- `unterm-app/frontend/`：JS/CSS/HTML 前端 — 主要改动区域
- `unterm-app/src/bridge.rs`：新增 shell hook 标记解析、Block 事件
- `unterm-app/src/main.rs`：新增 AI 命令转换 Tauri command

## 核心模块设计

### 1. Shell Integration Hook

使用 OSC 133 协议（VS Code Terminal / iTerm2 标准）在命令边界注入标记：

```
OSC 133;A ST  — 命令提示符开始（prompt start）
OSC 133;B ST  — 命令输入开始（command start）
OSC 133;C ST  — 命令执行开始（command executed）
OSC 133;D;exitcode ST  — 命令执行结束
```

**PowerShell 注入**：
```powershell
function prompt {
  $exitCode = $LASTEXITCODE
  if ($exitCode -eq $null) { $exitCode = 0 }
  "`e]133;D;$exitCode`a`e]133;A`aPS $($executionContext.SessionState.Path.CurrentLocation)> `e]133;B`a"
}
```

**Bash 注入**：
```bash
__unterm_preexec() { printf '\e]133;C\a'; }
__unterm_precmd() { printf "\e]133;D;$?\a\e]133;A\a"; }
PS1="$PS1\[\e]133;B\a\]"
trap '__unterm_preexec' DEBUG
PROMPT_COMMAND="__unterm_precmd;$PROMPT_COMMAND"
```

**注入方式**：创建 session 时通过 PTY 发送 hook 脚本。

### 2. ANSI → HTML 渲染器

`ansi-renderer.js` — 将 ANSI 转义序列转换为带 CSS class 的 HTML：

- 解析 SGR（颜色/粗体/斜体/下划线）
- 解析超链接 OSC 8
- 支持 256 色 + 真彩色
- 输出 `<span class="fg-red bold">text</span>` 格式
- 性能目标：10万行输出在 100ms 内完成转换

### 3. Block 数据模型

```javascript
class Block {
  id: number
  command: string        // 用户输入的命令
  output: string         // 原始 ANSI 输出
  renderedHtml: string   // 缓存的 HTML
  exitCode: number       // 退出码
  startTime: Date        // 开始时间
  duration: number       // 耗时 ms
  cwd: string            // 执行时的工作目录
  collapsed: boolean     // 是否折叠
  state: 'running' | 'completed' | 'error'
}
```

### 4. Block UI 渲染

`block-renderer.js` — 渲染 Block 列表：

```html
<div class="block completed">
  <div class="block-header">
    <span class="block-chevron">▸</span>
    <span class="block-command">$ git status</span>
    <span class="block-meta">
      <span class="block-duration">2.1s</span>
      <span class="block-exit exit-0">✓ 0</span>
    </span>
  </div>
  <div class="block-body">
    <pre class="block-output"><!-- ANSI rendered HTML --></pre>
  </div>
</div>
```

- 滚动虚拟化：只渲染可见 Block（IntersectionObserver）
- 大输出截断：超过 5000 行显示 "展开更多"
- 运行中 Block：底部实时追加输出

### 5. 底部输入区

`input-editor.js` — 固定底部的命令编辑器：

- 单行默认，Shift+Enter 多行
- 上下箭头：历史记录导航
- Tab：触发补全（通过 MCP 调 shell 补全）
- `#` 前缀：AI 自然语言模式
- Ctrl+C：取消当前输入 / 发送 SIGINT
- Enter：发送命令
- 显示当前 CWD 作为提示符

### 6. 全屏模式切换

检测 VT 序列自动切换：

- `\x1b[?1049h`（DECSET alternate screen）→ 切到 xterm.js 全屏
- `\x1b[?1049l`（DECRST alternate screen）→ 回到 Block 模式
- 在 bridge.rs 中解析 PTY 输出流，检测这两个序列
- 全屏模式复用现有的 xterm.js 逻辑

### 7. 主题 YAML 化

```yaml
# ~/.unterm/themes/catppuccin.yaml
name: Catppuccin Mocha
author: catppuccin
colors:
  background: "#1e1e2e"
  foreground: "#cdd6f4"
  cursor: "#f5e0dc"
  selection: "#585b70"
  black: "#45475a"
  red: "#f38ba8"
  green: "#a6e3a1"
  # ... 16 colors
ui:
  base: "#1e1e2e"
  surface: "#313244"
  overlay: "#585b70"
  text: "#cdd6f4"
  accent: "#89b4fa"
  border: "#45475a"
  block-header: "#313244"
  block-border: "#45475a"
  input-bg: "#313244"
```

### 8. # 自然语言命令

- 输入 `# 找到最大的10个文件` → 调用 AI API → 返回 `find . -type f -exec du -h {} + | sort -rh | head -10`
- 显示在输入框中，高亮为"建议"状态
- Enter 确认执行，Esc 取消
- 支持 Claude API / OpenAI API，用户在设置中配置 Key

## 文件变更清单

### 新增文件

- `frontend/js/ansi-renderer.js` — ANSI → HTML 转换器
- `frontend/js/block-renderer.js` — Block UI 渲染 + 虚拟滚动
- `frontend/js/input-editor.js` — 底部输入编辑器
- `frontend/js/shell-hook.js` — Shell Integration hook 管理
- `frontend/js/ai-command.js` — # 自然语言命令
- `frontend/css/blocks.css` — Block 样式
- `frontend/css/input-editor.css` — 输入区样式

### 修改文件

- `frontend/index.html` — 新增 Block 容器 + 输入区 DOM
- `frontend/js/main.js` — 新增 Block 事件处理
- `frontend/js/terminal.js` — 全屏模式切换逻辑
- `frontend/js/themes.js` — YAML 主题加载
- `frontend/js/settings.js` — AI Key 配置
- `frontend/js/tabs.js` — Tab 数据模型加 blocks 数组
- `src/bridge.rs` — OSC 133 解析、Block 事件
- `src/main.rs` — AI 命令 Tauri command

### 不变文件

- `unterm-core/*` — 全部不动
- `unterm-cli/*` — 全部不动
- `unterm-proto/*` — 全部不动
