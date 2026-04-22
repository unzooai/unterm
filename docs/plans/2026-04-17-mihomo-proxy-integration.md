# Mihomo 代理集成实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 mock Clash 实现替换为真实 mihomo 进程，通过 REST API 控制，支持订阅导入、节点管理、测速、代理环境变量注入到终端 session。

**Architecture:** Tauri app 层 spawn mihomo 子进程，通过 `127.0.0.1:{随机端口}` REST API 控制。代理 env vars 通过 Tauri IPC → Bridge → JSON-RPC → Core → PTY 注入到每个新建的 shell session。前端按地区分组展示节点。

**Tech Stack:** mihomo (Go binary), reqwest (async HTTP), tokio::process, serde_yaml

---

## 架构概览

```
Frontend (JS)
    │ invoke('create_session', { paneId, shell, cwd, env })
    ▼
main.rs (Tauri command)
    │ bridge.create_session_for_pane(pane_id, shell, cwd, env)
    ▼
bridge.rs (UiCommand::CreateSession { env })
    │ JSON-RPC: session.create { shell, cwd, name, env }
    ▼
session/mod.rs (Core)
    │ PtyConfig { env, proxy_env: None }
    ▼
pty/mod.rs → CommandBuilder.env(k, v)
```

代理流量路径：
```
Shell (curl/git/npm)
    │ HTTP_PROXY=http://127.0.0.1:{mixed_port}
    ▼
mihomo (mixed-port)
    │ proxy-groups → 节点轮换
    ▼
远端代理节点 → 目标服务器
```

---

## Task 1: 环境变量注入链路修复

**问题：** 前端传了 `env` 参数，但 Tauri command、Bridge、JSON-RPC 都没接收/传递。

**Files:**
- Modify: `crates/unterm-app/src/main.rs` (create_session 添加 env 参数)
- Modify: `crates/unterm-app/src/bridge.rs` (UiCommand::CreateSession 添加 env, create_session_for_pane 添加 env)
- Verify: `crates/unterm-core/src/session/mod.rs` (CreateSessionRequest.env 已存在)
- Verify: `crates/unterm-core/src/pty/mod.rs` (PtyConfig.env 已处理)

**Step 1: main.rs — 添加 env 参数**

```rust
#[tauri::command]
async fn create_session(
    state: State<'_, AppState>,
    pane_id: u64,
    shell: Option<String>,
    cwd: Option<String>,
    env: Option<HashMap<String, String>>,  // 新增
) -> Result<(), String> {
    let bridge = state.bridge.lock().await;
    bridge.create_session_for_pane(pane_id, shell, cwd, env);  // 传递 env
    Ok(())
}
```

**Step 2: bridge.rs — UiCommand 添加 env**

```rust
pub enum UiCommand {
    CreateSession {
        pane_id: u64,
        shell: Option<String>,
        cwd: Option<String>,
        env: Option<HashMap<String, String>>,  // 新增
    },
    // ...
}
```

更新 `create_session_for_pane`:
```rust
pub fn create_session_for_pane(
    &self,
    pane_id: u64,
    shell: Option<String>,
    cwd: Option<String>,
    env: Option<HashMap<String, String>>,
) {
    self.send_command(UiCommand::CreateSession { pane_id, shell, cwd, env });
}
```

更新 `background_loop` 中 CreateSession 处理，把 env 加入 JSON-RPC params:
```rust
UiCommand::CreateSession { pane_id, shell, cwd, env } => {
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "session.create",
        "params": {
            "shell": shell,
            "cwd": cwd,
            "name": format!("pane-{}", pane_id),
            "env": env,
        },
        "id": id
    });
    // ...
}
```

**Step 3: 验证 Core 侧已支持**

检查 `unterm-proto/src/session.rs` 中 `CreateSessionRequest` 是否有 `env: Option<HashMap<String, String>>`。
检查 `unterm-core/src/rpc/` 中 JSON-RPC handler 是否解析 `env` 字段。

**Step 4: 编译验证**

```bash
cd E:/code/unterm && cargo build -p unterm-app -p unterm-core
```

---

## Task 2: Mihomo 进程管理

**Files:**
- Modify: `crates/unterm-app/src/proxy.rs` (添加 MihomoProcess 结构体和方法)
- Modify: `crates/unterm-app/Cargo.toml` (添加 rand 依赖)

**Step 1: 添加依赖**

Cargo.toml 添加:
```toml
rand = "0.8"
```

**Step 2: 添加 MihomoProcess 结构体**

在 proxy.rs 中添加:

```rust
use std::process::Child;
use tokio::sync::oneshot;

/// mihomo 子进程管理
struct MihomoProcess {
    child: Option<Child>,
    /// mihomo REST API 端口
    api_port: u16,
    /// mihomo REST API secret
    api_secret: String,
    /// mihomo mixed-port (HTTP+SOCKS)
    mixed_port: u16,
    /// mihomo 配置目录
    config_dir: PathBuf,
    /// mihomo 二进制路径
    binary_path: PathBuf,
}
```

**Step 3: 端口分配**

```rust
impl MihomoProcess {
    fn find_available_port() -> u16 {
        use std::net::TcpListener;
        // 绑定到 0 让 OS 分配端口
        TcpListener::bind("127.0.0.1:0")
            .map(|l| l.local_addr().unwrap().port())
            .unwrap_or(17890)
    }
}
```

**Step 4: 配置生成**

根据 ProxyManager 中的节点列表和策略，生成 mihomo 的 config.yaml:

```yaml
mixed-port: {mixed_port}
external-controller: 127.0.0.1:{api_port}
secret: {api_secret}
mode: rule
log-level: warning

proxies:
  - name: "节点1"
    type: vmess
    ...  # 从订阅 YAML 原始数据保留

proxy-groups:
  - name: "unterm-proxy"
    type: url-test  # 或 fallback/select/load-balance
    proxies:
      - "节点1"  # 只包含 enabled=true 的节点
      - "节点2"
    url: http://www.gstatic.com/generate_204
    interval: 300

rules:
  - MATCH,unterm-proxy
```

**关键设计决策:** 需要保留订阅拉取时的原始 YAML proxy 数据（不仅仅是 name/type），因为 mihomo 需要完整的节点配置（server, port, uuid, password 等）。

修改 `ProxyNode` 添加 `raw_config` 字段:
```rust
pub struct ProxyNode {
    pub name: String,
    pub node_type: String,
    pub latency_ms: Option<u64>,
    pub available: bool,
    pub enabled: bool,
    pub subscription: String,
    /// 原始 Clash YAML 节点配置（mihomo 需要）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_config: Option<serde_yaml::Value>,
}
```

修改 `parse_clash_yaml` 保留原始配置:
```rust
nodes.push(ProxyNode {
    name,
    node_type,
    latency_ms: None,
    available: true,
    enabled: true,
    subscription: String::new(),
    raw_config: Some(proxy.clone()),  // 保留原始 YAML
});
```

修改 `parse_single_uri` — URI 格式的节点需要转换为 Clash YAML 格式。这比较复杂，暂时只支持 YAML 订阅格式的完整代理功能，URI 格式的保留 raw_config: None，不会被加入 mihomo 配置。

**Step 5: 进程启动/停止**

```rust
impl MihomoProcess {
    fn start(&mut self) -> Result<(), String> {
        self.generate_config()?;

        let child = std::process::Command::new(&self.binary_path)
            .arg("-d")
            .arg(&self.config_dir)
            .creation_flags(0x08000000)  // CREATE_NO_WINDOW (Windows)
            .spawn()
            .map_err(|e| format!("启动 mihomo 失败: {}", e))?;

        self.child = Some(child);
        Ok(())
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn is_running(&self) -> bool {
        // check if child process is still alive
    }
}
```

**Step 6: 等待 API 就绪**

启动后轮询 `GET /version` 直到返回 200:

```rust
async fn wait_ready(&self) -> Result<(), String> {
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/version", self.api_port);
    for _ in 0..30 {  // 最多等 3 秒
        if client.get(&url).send().await.is_ok() {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    Err("mihomo API 未就绪".into())
}
```

---

## Task 3: Mihomo REST API 客户端

**Files:**
- Modify: `crates/unterm-app/src/proxy.rs`

替换 mock 实现为真实 REST API 调用。

**Step 1: 测速**

替换 `test_latency()`:
```rust
pub async fn test_latency(&mut self) -> Vec<ProxyNode> {
    if let Some(ref mihomo) = self.mihomo {
        let client = reqwest::Client::new();
        let url = format!(
            "http://127.0.0.1:{}/group/unterm-proxy/delay?url=http://www.gstatic.com/generate_204&timeout=5000",
            mihomo.api_port
        );
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Authorization", format!("Bearer {}", mihomo.api_secret).parse().unwrap());

        match client.get(&url).headers(headers).send().await {
            Ok(resp) => {
                if let Ok(data) = resp.json::<HashMap<String, serde_json::Value>>().await {
                    for node in &mut self.persist.clash.nodes {
                        if let Some(val) = data.get(&node.name) {
                            if let Some(delay) = val.as_u64() {
                                node.latency_ms = Some(delay);
                                node.available = true;
                            } else {
                                node.available = false;
                                node.latency_ms = None;
                            }
                        }
                    }
                }
            }
            Err(e) => warn!("测速失败: {}", e),
        }
    }
    self.save_to_disk();
    self.persist.clash.nodes.clone()
}
```

**Step 2: 切换节点**

`switch_node()` 在 select 模式下通过 REST API 切换:
```rust
// PUT /proxies/unterm-proxy  body: {"name": "node_name"}
```

**Step 3: 配置热更新**

节点启用/禁用或策略切换后，调用 `PUT /configs` 或重新生成配置并 reload:
```rust
// PUT /configs?force=true  body: { path: config_path }
```

---

## Task 4: ProxyManager 集成 MihomoProcess

**Files:**
- Modify: `crates/unterm-app/src/proxy.rs` (ProxyManager 添加 mihomo 字段)
- Modify: `crates/unterm-app/src/main.rs` (set_mode 时启动/停止 mihomo)

**Step 1: ProxyManager 添加 mihomo 字段**

```rust
pub struct ProxyManager {
    persist: ProxyPersist,
    config_path: PathBuf,
    mihomo: Option<MihomoProcess>,
}
```

**Step 2: set_mode 生命周期管理**

```rust
pub async fn set_mode(&mut self, mode: ProxyMode) {
    // 停止旧进程
    if self.persist.mode == ProxyMode::Clash {
        if let Some(ref mut mihomo) = self.mihomo {
            mihomo.stop();
        }
    }

    self.persist.mode = mode.clone();

    // 启动新进程
    if mode == ProxyMode::Clash {
        self.start_mihomo().await;
    }

    self.save_to_disk();
}
```

**Step 3: 更新 env vars 端口**

`proxy_env_vars()` 在 Clash 模式下使用 mihomo 实际分配的端口:
```rust
ProxyMode::Clash => {
    if let Some(ref mihomo) = self.mihomo {
        let http_proxy = format!("http://127.0.0.1:{}", mihomo.mixed_port);
        // ...
    } else {
        None
    }
}
```

**Step 4: 应用退出清理**

main.rs 中在 Tauri app 退出前 kill mihomo:
```rust
// 使用 Drop trait 或 app.on_close_requested
```

---

## Task 5: Mihomo 二进制管理

**Files:**
- Modify: `crates/unterm-app/src/proxy.rs` (添加下载/检测逻辑)

**Step 1: 检测已有二进制**

```rust
fn detect_binary() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let path = home.join(".unterm").join("mihomo").join(mihomo_binary_name());
    if path.exists() { Some(path) } else { None }
}

fn mihomo_binary_name() -> &'static str {
    if cfg!(target_os = "windows") { "mihomo.exe" }
    else if cfg!(target_os = "macos") { "mihomo" }
    else { "mihomo" }
}
```

**Step 2: 下载二进制**

从 GitHub Release 下载:
```
https://github.com/MetaCubeX/mihomo/releases/latest/download/mihomo-windows-amd64-v{version}.zip
```

使用 reqwest 下载 + 解压。提供进度回调给前端。

**Step 3: 手动放置 fallback**

如果下载失败，提示用户手动放置到 `~/.unterm/mihomo/mihomo.exe`。

---

## Task 6: 前端节点地区分组

**Files:**
- Modify: `crates/unterm-app/frontend/js/settings.js` (节点渲染逻辑)

**Step 1: 地区解析函数**

```javascript
function detectRegion(nodeName) {
    const regions = {
        '香港': ['香港', 'HK', 'Hong Kong', 'HongKong', '🇭🇰'],
        '日本': ['日本', 'JP', 'Japan', 'Tokyo', 'Osaka', '🇯🇵'],
        '新加坡': ['新加坡', 'SG', 'Singapore', '🇸🇬'],
        '台湾': ['台湾', 'TW', 'Taiwan', '🇹🇼'],
        '韩国': ['韩国', 'KR', 'Korea', 'Seoul', '🇰🇷'],
        '美国': ['美国', 'US', 'United States', 'America', 'LA', 'SJ', 'SF', 'NY', '🇺🇸'],
        '英国': ['英国', 'UK', 'United Kingdom', 'London', '🇬🇧'],
        '德国': ['德国', 'DE', 'Germany', 'Frankfurt', '🇩🇪'],
        '法国': ['法国', 'FR', 'France', 'Paris', '🇫🇷'],
        '澳大利亚': ['澳大利亚', 'AU', 'Australia', 'Sydney', '🇦🇺'],
        '加拿大': ['加拿大', 'CA', 'Canada', 'Toronto', '🇨🇦'],
        '印度': ['印度', 'IN', 'India', 'Mumbai', '🇮🇳'],
        '泰国': ['泰国', 'TH', 'Thailand', 'Bangkok', '🇹🇭'],
        '越南': ['越南', 'VN', 'Vietnam', '🇻🇳'],
        '马来西亚': ['马来西亚', 'MY', 'Malaysia', '🇲🇾'],
        '俄罗斯': ['俄罗斯', 'RU', 'Russia', 'Moscow', '🇷🇺'],
        '其他': [],
    };
    const upper = nodeName.toUpperCase();
    for (const [region, keywords] of Object.entries(regions)) {
        if (region === '其他') continue;
        if (keywords.some(kw => upper.includes(kw.toUpperCase()))) return region;
    }
    return '其他';
}
```

**Step 2: 分组渲染**

修改 `renderNodeList()`:
- 按地区分组节点
- 每个地区一个折叠区块，header 显示地区名 + 节点数
- 区块默认展开
- 保持现有的勾选/搜索/筛选/测速功能

---

## Task 7: Tauri 命令异步化

**问题:** 当前 `test_latency()` 和 `set_mode()` 是同步的，接入 mihomo REST API 后需要异步。

**Files:**
- Modify: `crates/unterm-app/src/proxy.rs` (方法改 async)
- Modify: `crates/unterm-app/src/main.rs` (Tauri command 适配)

将 `test_latency`、`set_mode`、`switch_node` 等需要 REST 调用的方法改为 async，Tauri command 中 await。

---

## 实施顺序

1. **Task 1** (env 链路) — 最基础，先打通
2. **Task 5** (二进制管理) — 没有二进制其他都跑不了
3. **Task 2** (进程管理) — 核心能力
4. **Task 3** (REST API) — 替换 mock
5. **Task 4** (集成) — 串联起来
6. **Task 7** (异步化) — 适配 async
7. **Task 6** (前端分组) — 体验优化

---

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| mihomo 下载被墙 | 提供手动放置路径，前端提示 |
| 端口冲突 | OS 随机分配 + 重试 |
| 进程残留 | Drop trait + Windows Job Object |
| URI 节点无 raw_config | 仅 YAML 订阅支持完整代理，URI 节点标记为"需要 YAML 订阅" |
| 测速超时阻塞 UI | async + 5秒 timeout |
