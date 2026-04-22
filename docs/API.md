# Unterm API 文档

Unterm 提供三种接口供外部应用调��：

| 接口类型 | 协议 | 适用场景 |
|---------|------|---------|
| **MCP JSON-RPC** | TCP `127.0.0.1:19876` | AI Agent、自动化脚本、IDE 插件 |
| **CLI** | 命令行 `unterm.exe` | Shell 脚本、CI/CD、快速调试 |
| **MCP Stdio Server** | MCP 协议 (stdin/stdout) | Claude Code、Cursor 等 AI 工具 |

---

## 目录

- [快速开始](#快速开始)
- [认证](#认证)
- [MCP JSON-RPC 接口](#mcp-json-rpc-接口)
  - [会话管理 (session.*)](#会话管理)
  - [命令执行 (exec.*)](#命令执行)
  - [屏幕读取 (screen.*)](#屏幕读取)
  - [信号处理 (signal.*)](#信号处理)
  - [图片管理 (image.*)](#图片管理)
  - [编排调度 (orchestrate.*)](#编排调度)
  - [工作区 (workspace.*)](#工作区)
  - [截图与剪贴板 (capture.*)](#截图与剪贴板)
  - [代理管理 (proxy.*)](#代理管理)
  - [系统与策略 (system.*, policy.*)](#系统与策略)
- [CLI 命令](#cli-命令)
- [MCP Stdio Server](#mcp-stdio-server)
- [错误码](#错误码)
- [示例](#示例)

---

## 快速开始

### 1. 确认 Unterm 正在运行

```bash
# CLI 方式
unterm system

# 或直接检测端口
netstat -an | grep 19876
```

### 2. 创建会话并执行命令

```bash
# 创建会话
unterm session create --shell powershell.exe

# 列出会话，获取 session_id
unterm session list

# 执行命令并等待结果
unterm run <session_id> "echo hello"

# 读取屏幕内容
unterm screen text <session_id>
```

### 3. 用 Node.js 调用 JSON-RPC

```javascript
const net = require('net');
const fs = require('fs');
const path = require('path');

// 读取认证 token
const token = fs.readFileSync(
  path.join(process.env.USERPROFILE, '.unterm', 'auth_token'), 'utf-8'
).trim();

function callRpc(method, params = {}) {
  return new Promise((resolve, reject) => {
    const client = new net.Socket();
    let buf = '';
    let authed = false;

    client.connect(19876, '127.0.0.1', () => {
      // 先认证
      client.write(JSON.stringify({
        jsonrpc: '2.0', method: 'auth.login',
        params: { token }, id: 0
      }) + '\n');
    });

    client.on('data', (data) => {
      buf += data.toString();
      const lines = buf.split('\n');
      buf = lines.pop();
      for (const line of lines) {
        if (!line.trim()) continue;
        const resp = JSON.parse(line);
        if (!authed) {
          authed = true;
          client.write(JSON.stringify({
            jsonrpc: '2.0', method, params, id: 1
          }) + '\n');
        } else {
          client.destroy();
          resp.error ? reject(resp.error) : resolve(resp.result);
        }
      }
    });

    client.on('error', reject);
    setTimeout(() => { client.destroy(); reject(new Error('timeout')); }, 10000);
  });
}

// 使用示例
(async () => {
  const sessions = await callRpc('session.list');
  console.log(sessions);
})();
```

---

## 认证

Unterm 启动时自动生成 auth token，保存在：

```
~/.unterm/auth_token
```

**JSON-RPC 认证**：每个 TCP 连接的第一个请求必须是 `auth.login`。

```json
{"jsonrpc":"2.0","method":"auth.login","params":{"token":"<token>"},"id":1}
```

响应：

```json
{"jsonrpc":"2.0","result":{"authenticated":true},"id":1}
```

**CLI 认证**：自动读取 `~/.unterm/auth_token`，无需手动指定。

---

## MCP JSON-RPC 接口

协议：JSON-RPC 2.0 over TCP，每行一个 JSON 对象（以 `\n` 分隔）。

地址：`127.0.0.1:19876`

### 会话管理

#### session.create

创建新终端会话。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| shell | string | 否 | Shell 程序路径，如 `powershell.exe`、`cmd.exe`、`bash` |
| name | string | 否 | 会话名称 |
| cwd | string | 否 | 初始工作目录 |
| cols | number | 否 | 列数，默认 80 |
| rows | number | 否 | 行数，默认 24 |
| env | object | 否 | 额外环境变量 `{"KEY": "VALUE"}` |

**返回：**

```json
{
  "id": "7b8942a1-1ff6-4903-8aa4-6e4eaa02e87a",
  "name": null,
  "shell": "powershell.exe",
  "cwd": "C:\\Users\\Alex",
  "cols": 80,
  "rows": 24,
  "status": "running",
  "policy": "full",
  "created_at": "2026-04-22T12:32:17.998Z",
  "last_activity": "2026-04-22T12:32:17.998Z"
}
```

#### session.list

列出所有活跃会话。

**参数：** 无

**返回：** `SessionInfo[]`（与 `session.create` 返回结构相同的数组）

#### session.status

查看指定会话状态。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |

**返回：** `SessionInfo` 对象

#### session.destroy

销毁会话。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |

**返回：** `{"destroyed": true}`

#### session.resize

调整终端尺寸。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |
| cols | number | 否 | 列数，默认 80 |
| rows | number | 否 | 行数，默认 24 |

**返回：** `{"resized": true}`

#### session.cwd

获取当前工作目录。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |

**返回：**

```json
{
  "cwd": "E:\\code\\unterm",
  "source": "prompt"
}
```

#### session.idle

检查会话是否空闲（等待输入）。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |

**返回：**

```json
{
  "idle": false,
  "idle_by_timing": false,
  "prompt_detected": true,
  "shell_type": "powershell",
  "cursor_line": "PS E:\\code\\unterm>",
  "since_last_output_ms": 5000
}
```

#### session.history

获取会话 I/O 历史记录。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |
| since | string | 否 | ISO 时间戳，只返回此时间之后的记录 |
| limit | number | 否 | 最大返回条数 |

**返回：**

```json
[
  {
    "direction": "output",
    "content": "PS E:\\code\\unterm> ",
    "timestamp": "2026-04-22T12:26:27.634Z"
  },
  {
    "direction": "input",
    "content": "echo hello",
    "timestamp": "2026-04-22T12:26:30.123Z"
  }
]
```

#### session.env

读取会话中的环境变量。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |
| name | string | 是 | 环境变量名 |

**返回：** `{"name": "PATH", "value": "..."}`

#### session.set_env

设置会话中的环境变量。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |
| name | string | 是 | 环境变量名 |
| value | string | 是 | 环境变量值 |

**返回：** `{"set": true}`

#### session.audit_log

获取审计日志。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 否 | 过滤指定会话 |
| limit | number | 否 | 最大条数，默认 50 |

**返回：** 审计日志数组

---

### 命令执行

#### exec.run

在会话中执行命令（异步，不等待完成）。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |
| command | string | 是 | 要执行的命令 |

**返回：** `{"sent": true}`

#### exec.run_wait

执行命令并等待输出（同步模式，适合 AI Agent）。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |
| command | string | 是 | 要执行的命令 |
| timeout_ms | number | 否 | 超时毫秒数，默认 30000 |

**返回：**

```json
{
  "command": "echo hello",
  "output": "hello\n",
  "completed": true,
  "elapsed_ms": 150
}
```

#### exec.send

向会话发送原始输入（包括控制字符）。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |
| input | string | 是 | 原始输入内容，如 `"ls\r"` 或 `"\x03"` (Ctrl+C) |

**返回：** `{"sent": true}`

#### exec.status

查看当前命令执行状态。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |

**返回：** 命令执行状态对象

#### exec.cancel

取消正在执行的命令（发送 Ctrl+C）。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |

**返回：** `{"cancelled": true, "prompt_restored": true}`

---

### 屏幕读取

#### screen.read

读取终端屏幕完整内容（含样式属性）。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |

**返回：** 包含 `cells` 二维数组的屏幕对象，每个 cell 含 `ch`（字符）、`attrs`（前景色、背景色、粗体等属性）。

#### screen.read_raw

读取屏幕原始 ANSI 序列（base64 编码）。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |

**返回：**

```json
{
  "content": "<base64 编码的 ANSI 文本>",
  "encoding": "base64",
  "length": 4096
}
```

#### screen.text

读取屏幕纯文本（无样式，适合 AI 分析）。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |

**返回：**

```json
{
  "text": "Windows PowerShell\n...\nPS E:\\code\\unterm> ",
  "cols": 120,
  "rows": 50
}
```

#### screen.cursor

获取光标位置。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |

**返回：**

```json
{
  "col": 22,
  "row": 6,
  "visible": true
}
```

#### screen.search

搜索屏幕内容。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |
| pattern | string | 是 | 搜索文本 |
| max_results | number | 否 | 最大匹配数，默认 50 |

**返回：**

```json
{
  "matches": [
    {"row": 0, "col": 8, "text": "PowerShell"}
  ],
  "total": 1
}
```

#### screen.scroll

读取滚动缓冲区历史内容。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |
| offset | number | 否 | 起始偏移行，默认 0 |
| count | number | 否 | 读取行数，默认 100 |

**返回：**

```json
{
  "lines": ["line1", "line2", ...],
  "offset": 0,
  "total": 150
}
```

---

### 信号处理

#### signal.send

向会话发送系统信号。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |
| signal | string | 是 | 信号名称：`SIGINT`、`SIGTERM`、`SIGKILL` |

**返回：** `{"sent": true, "signal": "SIGINT"}`

---

### 图片管理

用于 AI multimodal 场景，将图片关联到会话。

#### image.store

存储图片到会话。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |
| data | string | 是 | base64 编码的图片数据 |
| mime_type | string | 否 | MIME 类型，默认 `image/png` |

**返回：** `{"image_id": "uuid"}`

#### image.list

列出会话中的图片。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |

**返回：**

```json
{
  "images": [
    {
      "id": "uuid",
      "mime_type": "image/png",
      "size": 1024,
      "timestamp": "2026-04-22T12:38:22.716Z"
    }
  ]
}
```

#### image.get

获取指定图片数据。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |
| image_id | string | 是 | 图片 ID |

**返回：**

```json
{
  "image_id": "uuid",
  "data": "<base64>",
  "mime_type": "image/png",
  "timestamp": "2026-04-22T12:38:22.716Z"
}
```

---

### 编排调度

用于 AI Agent 并行操控多个终端会话。

#### orchestrate.launch

启动新的 AI Agent 会话并执行命令。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| command | string | 是 | 要执行的命令 |
| name | string | 否 | 会话名称 |
| cwd | string | 否 | 工作目录 |

**返回：**

```json
{
  "session_id": "uuid",
  "command": "echo test",
  "status": "launched"
}
```

#### orchestrate.broadcast

向多个会话广播执行同一命令。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| sessions | string[] | 是 | 会话 ID 数组 |
| command | string | 是 | 要执行的命令 |

**返回：**

```json
{
  "command": "echo hello",
  "results": [
    {"session_id": "uuid-1", "sent": true},
    {"session_id": "uuid-2", "sent": true}
  ]
}
```

#### orchestrate.wait

等待会话输出匹配指定模式。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| session_id | string | 是 | 会话 ID |
| pattern | string | 是 | 匹配文本 |
| timeout_ms | number | 否 | 超时毫秒数，默认 10000 |

**返回：**

```json
{
  "matched": true,
  "pattern": "done",
  "timeout": false
}
```

---

### 工作区

保存和恢复多会话工作区快照。

#### workspace.save

保存当前所有会话为工作区快照。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| name | string | 是 | 工作区名称 |

**返回：** `{"name": "my_workspace", "saved": true}`

#### workspace.list

列出所有已保存的工作区。

**参数：** 无

**返回：**

```json
[
  {
    "name": "my_workspace",
    "session_count": 3,
    "created_at": "2026-04-22T12:34:26.847Z"
  }
]
```

#### workspace.restore

恢复工作区快照。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| name | string | 是 | 工作区名称 |

**返回：**

```json
{
  "name": "my_workspace",
  "restored": true,
  "sessions": ["uuid-1", "uuid-2", "uuid-3"]
}
```

---

### 截图与剪贴板

#### capture.screen

截取所有终端会话的文本快照。

**参数：** 无

**返回：**

```json
{
  "type": "text",
  "message": "终端文本快照（非图像截图）",
  "captures": [
    {
      "session_id": "uuid",
      "name": "pane-1",
      "screen": { "cells": [[...]] }
    }
  ]
}
```

#### capture.window

截取指定会话的文本快照。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| title | string | 否 | 按会话名称匹配 |

**返回：**

```json
{
  "session_id": "uuid",
  "name": "pane-1",
  "screen": { "cells": [[...]] },
  "type": "text"
}
```

#### capture.clipboard

读取系统剪贴板文本内容。

**参数：** 无

**返回：** `{"type": "text", "content": "剪贴板文本"}`

#### capture.select

交互式框选截图（需要 GUI 环境）。

**参数：** 无

**返回：** 截图数据或错误

---

### 代理管理

> 注意：代理功能由 App 层（Tauri GUI）管理，MCP 接口仅返回只读状态。

#### proxy.status

查看代理状态。

**参数：** 无

**返回：** `{"enabled": false, "message": "代理功能由 App 层管理"}`

#### proxy.nodes

列出代理节点。

**参数：** 无

**返回：** `{"nodes": [], "message": "代理功能由 App 层管理"}`

---

### 系统与策略

#### system.info

获取系统信息。

**参数：** 无

**返回：**

```json
{
  "os": "windows",
  "arch": "x86_64",
  "family": "windows",
  "hostname": "MY-PC",
  "user": "Alex",
  "home": "C:\\Users\\Alex",
  "pid": 12345
}
```

#### policy.set

���置命令执行策略（控制哪些命令可以执行）。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| enabled | boolean | 是 | 是否启用策略 |
| blocked_patterns | string[] | 是 | 屏蔽命令模式列表 |
| allowed_patterns | string[] | 是 | 允许命令模式列表 |

**返回：** `{"set": true}`

#### policy.check

检查命令是否被策略允许。

**参数：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| command | string | 是 | 待检查的命令 |

**返回：** `{"command": "echo test", "allowed": true}`

---

## CLI 命令

CLI 自动读取 `~/.unterm/auth_token` 进行认证，无需手动配置���

### 总览

```
unterm <command> [subcommand] [options] [args]
```

### 会���管理

```bash
# 列出所有会话
unterm session list

# 创建会话
unterm session create [--name <名称>] [--shell <shell路径>] [--cwd <工作目录>]

# 查看会话状态
unterm session status <session_id>

# 获取工作目录
unterm session cwd <session_id>

# 检查是否空闲
unterm session idle <session_id>

# 查看 I/O 历史
unterm session history <session_id> [--since <ISO时间>] [--limit <条数>]

# 调整终端尺寸
unterm session resize <session_id> --cols <列数> --rows <行数>

# 读取环境变量
unterm session env <session_id> <变量名>

# 设置环境变量
unterm session set-env <session_id> <变量名> <值>

# 销毁会话
unterm session destroy <session_id>
```

### 命令执行

```bash
# 异步执行命令
unterm exec <session_id> "<command>"

# 执行并等待结果（推荐 AI 使用）
unterm run <session_id> "<command>" [--timeout <毫秒>]

# 发送原始输入
unterm send <session_id> "<input>"

# 取消正在执行的命令
unterm cancel <session_id>

# 发送信号
unterm signal <session_id> <SIGINT|SIGTERM|SIGKILL>
```

### 屏幕读取

```bash
# 读取屏幕（含样式）
unterm screen read <session_id>

# 读取纯文本（推荐 AI 使用）
unterm screen text <session_id>

# 获取光标位置
unterm screen cursor <session_id>

# 搜索屏幕内容
unterm screen search <session_id> "<pattern>" [--max-results <数量>]

# 读取滚动缓冲区
unterm screen scroll <session_id> --offset <起始行> --count <行数>
```

### 编排调度

```bash
# 启动新 Agent 会话
unterm orchestrate launch "<command>" [--name <名称>] [--cwd <目录>]

# 向多个会话广播命令
unterm orchestrate broadcast --sessions <id1,id2,...> "<command>"

# 等待输出匹配
unterm orchestrate wait <session_id> "<pattern>" [--timeout <毫秒>]
```

### 工作区

```bash
# 保存工作区
unterm workspace save <名称>

# 列出已保存的工作区
unterm workspace list

# 恢复工作区
unterm workspace restore <名称>
```

### 截图与剪贴板

```bash
# 截取所有终端文本
unterm capture screen

# 截取指定会话
unterm capture window [--title <名称>] [--pid <进程ID>]

# 读取剪贴板
unterm capture clipboard

# 交互式截图
unterm capture select
```

### 系统与策略

```bash
# 系统信息
unterm system

# 检查命令策略
unterm policy check "<command>"

# 设置策略
unterm policy set '{"enabled":true,"blocked_patterns":["rm -rf"],"allowed_patterns":["*"]}'

# 查看审计日志
unterm audit [--session-id <id>] [--limit <条数>]
```

---

## MCP Stdio Server

供 Claude Code、Cursor 等 AI 工具通过 MCP 协议调用。

### 配置

在 Claude Code 的 MCP 配置中添加：

```json
{
  "mcpServers": {
    "unterm": {
      "command": "node",
      "args": ["<path-to-unterm>/tools/unterm-mcp-server.js"]
    }
  }
}
```

### 提供的工具

#### screenshot

截取当前屏幕截图，返回 PNG 图片（base64）。

```json
{"name": "screenshot", "arguments": {}}
```

返回 `image` 类型的内容块。

#### terminal_read

读取所有终端会话的文本内容。

```json
{"name": "terminal_read", "arguments": {}}
```

返回 `text` 类型的内容块，包含所有会话的屏幕文本 JSON。

---

## 错误码

| 错误码 | 含义 | 说明 |
|-------|------|------|
| -32600 | Invalid Request | 请求格式错误 |
| -32601 | Method not found | 方法不存在 |
| -32602 | Invalid params | 参数缺失或格式错误 |
| -32603 | Internal error | 内部错误 |

错误响应格式：

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32602,
    "message": "missing session_id"
  },
  "id": 1
}
```

---

## 示例

### Python 调用

```python
import socket
import json
import os

TOKEN_PATH = os.path.expanduser("~/.unterm/auth_token")

def call_unterm(method, params=None):
    token = open(TOKEN_PATH).read().strip()
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(("127.0.0.1", 19876))

    # 认证
    auth = json.dumps({"jsonrpc":"2.0","method":"auth.login","params":{"token":token},"id":0})
    sock.sendall((auth + "\n").encode())
    sock.recv(4096)  # 读取认证响应

    # 发送请求
    req = json.dumps({"jsonrpc":"2.0","method":method,"params":params or {},"id":1})
    sock.sendall((req + "\n").encode())

    # 读取响应
    data = b""
    while b"\n" not in data:
        data += sock.recv(4096)
    sock.close()

    resp = json.loads(data.decode().strip())
    if "error" in resp and resp["error"]:
        raise Exception(resp["error"]["message"])
    return resp.get("result")

# 示例：列出会话并读取屏幕
sessions = call_unterm("session.list")
for s in sessions:
    print(f"Session {s['id']}: {s['shell']} ({s['status']})")
    text = call_unterm("screen.text", {"session_id": s["id"]})
    print(text["text"][:200])
```

### AI Agent 典型工作流

```bash
# 1. 创建专用会话
SESSION=$(unterm session create --shell bash --name "ai-agent" | jq -r '.id')

# 2. 执行命令并获取结果
unterm run $SESSION "ls -la"

# 3. 检查命令是否完成
unterm session idle $SESSION

# 4. 读取屏幕输出
unterm screen text $SESSION

# 5. 并行操控：广播到多个会话
unterm orchestrate broadcast --sessions "$S1,$S2,$S3" "git pull"

# 6. 等待特定输出
unterm orchestrate wait $SESSION "Build succeeded" --timeout 60000

# 7. 完成后销毁
unterm session destroy $SESSION
```

### 保存和恢复工作环境

```bash
# 保存当前所有会话
unterm workspace save "my-dev-env"

# 下次恢复
unterm workspace restore "my-dev-env"

# 查看已保存的工作区
unterm workspace list
```
