# Unterm 2.0 — 基于 WezTerm 引擎的原生终端

## 为什么换方案

Tauri WebView + xterm.js 做终端渲染存在**不可修复**的结构性问题：

- WebView 内的 Canvas/WebGL 渲染经过 3 层抽象（JS → Canvas → Chromium 合成），导致亚像素偏移和时序竞态
- CSS 布局引擎参与终端尺寸计算，resize 触发 reflow + redraw 导致内容晃动
- 历经 5+ 次修复尝试（CSS GPU 合成层、scrollbar 补丁、ResizeObserver 阈值、DSR 过滤、直连 PTY），晃动问题始终无法彻底消除
- 所有成熟终端（Windows Terminal、Alacritty、WezTerm、Kitty、iTerm2）均使用原生 GPU 渲染，无一例外

**结论：WebView 不适合做终端渲染。**

---

## 新架构

### 核心思路

**复用 WezTerm 的终端引擎，不重新造轮子。** WezTerm 是 Rust 编写的 MIT 许可终端模拟器，已解决所有 VTE 兼容性和 GPU 渲染问题。Unterm 2.0 在其基础上叠加 AI 功能和自定义 UI。

### 架构图

```
┌──────────────────────────────────────────────────┐
│                   Unterm 2.0                      │
│                                                   │
│  ┌─────────────────────┐  ┌───────────────────┐  │
│  │   Terminal Engine    │  │    AI Layer        │  │
│  │   (from WezTerm)    │  │   (Unterm 原创)     │  │
│  │                     │  │                    │  │
│  │  termwiz (VTE解析)   │  │  AI 面板 (wgpu UI) │  │
│  │  wezterm-term (状态) │  │  Ghost Text 补全   │  │
│  │  wezterm-font (字体) │  │  错误检测/修复      │  │
│  │  wgpu (GPU渲染)     │  │  模型选择器         │  │
│  │  portable-pty       │  │  命令流程推荐        │  │
│  └─────────────────────┘  └───────────────────┘  │
│                                                   │
│  ┌─────────────────────────────────────────────┐  │
│  │              unterm-core (保留)               │  │
│  │  MCP Server · Session API · 代理管理          │  │
│  └─────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────┘
```

### 技术选型

| 组件 | 方案 | 来源 |
|------|------|------|
| VTE 解析 | `termwiz` | WezTerm (crates.io) |
| 终端状态机 | `wezterm-term` | WezTerm fork (未发布到 crates.io) |
| 字体渲染 | `wezterm-font` | WezTerm fork (FreeType + HarfBuzz) |
| GPU 渲染 | wgpu | WezTerm 渲染管线 |
| 窗口管理 | winit / WezTerm window | 跨平台 |
| PTY 管理 | `portable-pty` | WezTerm (crates.io) |
| AI 功能 | 自研 | Unterm 原创 |
| MCP Server | `unterm-core` | 保留现有 |

### 集成方式

**Fork WezTerm 仓库**，在其基础上添加 Unterm 功能：

1. Fork `https://github.com/wezterm/wezterm` → `github.com/user/unterm-wezterm`
2. 裁剪不需要的模块（SSH multiplexer、serial port、Lua config 等）
3. 替换 UI 为 Unterm 设计（标题栏、Tab 栏、AI 面板）
4. 集成 `unterm-core` MCP Server
5. 添加 AI 功能层

---

## 保留 vs 废弃

### 保留的模块

| 模块 | 说明 |
|------|------|
| `unterm-core` | MCP Server、Session API — AI agent / CLI 通过它访问终端 |
| `unterm-proto` | 共享协议定义 |
| `unterm-cli` | 命令行客户端 |
| 代理管理 | mihomo 集成（可后续移植到新 GUI） |

### 废弃的模块

| 模块 | 原因 |
|------|------|
| `unterm-app` (Tauri 2) | WebView 渲染不可靠，整体废弃 |
| `unterm-ui` (wgpu 半成品) | 直接用 WezTerm 的完整渲染管线替代 |
| 前端 JS/CSS/HTML | 全部废弃，不再使用 WebView |
| xterm.js 及所有 addon | 废弃 |

---

## 实施阶段

### 阶段零：Fork 与裁剪（1-2 天）

1. Fork WezTerm 仓库
2. 本地 clone，确认编译通过
3. 裁剪不需要的功能：
   - 移除 SSH multiplexer (`mux-*` crate)
   - 移除 serial port 支持
   - 移除 Lua 配置系统（改用简单的 TOML/JSON）
   - 移除 domains（本地 PTY only）
4. 确认裁剪后仍可编译运行
5. 验证：打开终端，运行 TUI 程序，**零晃动**

### 阶段一：Unterm 品牌化（2-3 天）

1. 替换窗口标题、图标、应用名
2. 自定义标题栏样式（匹配 Stitch 设计稿）
3. 修改默认主题（Catppuccin Mocha 或自定义深色主题）
4. 默认窗口 800x600
5. Tab 栏样式调整
6. 状态栏调整

### 阶段二：集成 unterm-core（2-3 天）

1. 在 WezTerm 进程内嵌入 `unterm-core` MCP Server
2. PTY session 与 unterm-core session 映射
3. 前台终端输出同步到 unterm-core 的 Grid（供 AI 读屏）
4. 验证：`unterm-cli` 仍可正常连接和操作

### 阶段三：AI 功能层（3-5 天）

#### 3.1 AI Insights 右侧面板
- 在 wgpu 渲染管线中添加右侧面板区域
- 文本渲染（使用 WezTerm 的字体引擎）
- AI 卡片组件（标题 + 内容 + Execute 按钮）
- Ctrl+Shift+I 切换面板
- 面板开关时终端区域自动 resize

#### 3.2 Ghost Text 补全
- 在光标位置渲染淡灰色的预测文本
- Tab 键接受补全
- 连接 AI API 获取建议

#### 3.3 错误检测与一键修复
- 监听 PTY 输出，匹配错误模式
- 在 AI 面板中显示修复建议
- "Execute in Terminal" 按钮直接执行

#### 3.4 模型选择器
- 状态栏或面板内的下拉选择
- 支持 Claude / Gemini / GPT
- API Key 配置持久化

#### 3.5 命令流程推荐
- 基于上下文的后续操作建议
- `git push` → 推荐创建 PR
- 命令失败 → 推荐修复方案

### 阶段四：代理与高级功能（2-3 天）

1. 移植代理管理（mihomo 集成）
2. Shell Profile 检测（PowerShell、CMD、WSL、Git Bash）
3. 截图功能
4. 剪贴板图片粘贴
5. 窗口状态持久化

---

## 产品设计参考

### Stitch 设计稿要点

- **PowerShell Core Integration (800x600)**：原生 shell 输出 + AI overlay
- **AI Overlay**：命令行上直接显示 ghost text 补全，不改变 shell 行为
- **Contextual AI Insights**：右侧面板主动分析终端输出
- **AI Chat**：右侧面板支持与 AI 对话，生成代码块带 "Execute in Terminal" 按钮
- **Model Selector**：Claude / Gemini / GPT 自由切换
- **错误修复**：AI 识别拼写错误（如 `gti`）→ 一键 Apply 修正
- **流程优化**：命令成功后推荐下一步操作
- **UI 与 Kernel 分离**：纯 UI 层增强，不影响底层 shell 功能

---

## 风险与对策

| 风险 | 对策 |
|------|------|
| WezTerm 代码量大（10万+ 行），理解成本高 | 先不改渲染管线，只改配置/UI 层 |
| WezTerm 依赖复杂（vendored FreeType/HarfBuzz/Cairo） | Windows 编译验证优先 |
| AI 面板需要自定义 wgpu 渲染 | 先用简单文本渲染，逐步完善 |
| WezTerm 更新与上游同步 | 定期 rebase，只 fork 稳定版 |

---

## 文件结构（目标）

```
unterm/
├── crates/
│   ├── unterm-core/       # 保留：MCP Server + Session API
│   ├── unterm-proto/      # 保留：共享协议
│   ├── unterm-cli/        # 保留：CLI 客户端
│   ├── unterm-app/        # 废弃：Tauri 前端 (归档)
│   └── unterm-ui/         # 废弃：wgpu 半成品 (归档)
├── wezterm/               # 新增：WezTerm fork (submodule 或 vendored)
│   ├── termwiz/
│   ├── term/
│   ├── font/
│   ├── gui/               # 主要修改区域
│   └── ...
├── docs/
│   ├── plans/             # 当前方案
│   └── archive/           # 历史方案
└── Cargo.toml             # workspace 根
```
