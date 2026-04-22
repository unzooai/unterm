//! unterm-app: Tauri 2 桌面终端应用
//!
//! 通过 IPC 连接 unterm-core daemon，提供 Web 前端渲染的终端界面。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;
mod proxy;
mod screenshot;

use bridge::CoreBridge;
use std::sync::Arc;
use tauri::{Manager, State};
use tokio::sync::Mutex;
use tracing::info;

/// 应用状态，Tauri 管理
struct AppState {
    bridge: Arc<Mutex<CoreBridge>>,
    proxy_manager: Arc<Mutex<proxy::ProxyManager>>,
}

/// 创建新 session（前端调用）
#[tauri::command]
async fn create_session(
    state: State<'_, AppState>,
    pane_id: u64,
    shell: Option<String>,
    cwd: Option<String>,
    env: Option<std::collections::HashMap<String, String>>,
) -> Result<(), String> {
    let bridge = state.bridge.lock().await;
    bridge.create_session_for_pane(pane_id, shell, cwd, env);
    Ok(())
}

/// 发送输入到 pane（前端调用）
#[tauri::command]
async fn send_input(
    state: State<'_, AppState>,
    pane_id: u64,
    input: String,
) -> Result<(), String> {
    let bridge = state.bridge.lock().await;
    bridge.send_input_to_pane(pane_id, input.into_bytes());
    Ok(())
}

/// 调整 session 尺寸（前端调用）
#[tauri::command]
async fn resize_session(
    state: State<'_, AppState>,
    pane_id: u64,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let bridge = state.bridge.lock().await;
    if let Some(session_id) = bridge.get_session_id(pane_id) {
        bridge.send_command(bridge::UiCommand::ResizeSession {
            session_id: session_id.to_string(),
            cols,
            rows,
        });
    }
    Ok(())
}

/// 销毁 session（前端调用）
#[tauri::command]
async fn destroy_session(
    state: State<'_, AppState>,
    pane_id: u64,
) -> Result<(), String> {
    let mut bridge = state.bridge.lock().await;
    bridge.destroy_pane_session(pane_id);
    Ok(())
}

/// 获取 pane 的屏幕内容（前端轮询调用）
#[tauri::command]
async fn get_screen(
    state: State<'_, AppState>,
    pane_id: u64,
) -> Result<Option<String>, String> {
    let bridge = state.bridge.lock().await;
    Ok(bridge.get_pane_content(pane_id).map(|s| s.to_string()))
}

/// 轮询 bridge 事件，返回给前端
#[tauri::command]
async fn poll_events(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut bridge = state.bridge.lock().await;
    let events = bridge.poll_events();
    let json_events: Vec<serde_json::Value> = events.iter().map(|e| {
        match e {
            bridge::CoreEvent::Connected => serde_json::json!({"type": "connected"}),
            bridge::CoreEvent::Disconnected => serde_json::json!({"type": "disconnected"}),
            bridge::CoreEvent::SessionCreated { pane_id, session_id } => {
                serde_json::json!({"type": "session_created", "pane_id": pane_id, "session_id": session_id})
            }
            bridge::CoreEvent::ScreenUpdate { session_id, content } => {
                serde_json::json!({"type": "screen_update", "session_id": session_id, "content": content})
            }
            bridge::CoreEvent::Error(msg) => {
                serde_json::json!({"type": "error", "message": msg})
            }
        }
    }).collect();
    Ok(json_events)
}

/// 截取全屏（返回 base64 编码的 PNG）
#[tauri::command]
async fn capture_screen() -> Result<String, String> {
    // 在阻塞线程中执行，避免阻塞 async runtime
    tokio::task::spawn_blocking(|| screenshot::capture_screen())
        .await
        .map_err(|e| format!("截图任务失败: {}", e))?
}

/// 将 base64 图片数据复制到系统剪贴板
#[tauri::command]
async fn copy_image_to_clipboard(image_data: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || screenshot::copy_image_to_clipboard(&image_data))
        .await
        .map_err(|e| format!("剪贴板任务失败: {}", e))?
}

/// 复制文本到系统剪贴板
#[tauri::command]
async fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        #[cfg(windows)]
        {
            use windows::Win32::System::DataExchange::*;
            use windows::Win32::System::Memory::*;
            use windows::Win32::System::Ole::CF_UNICODETEXT;

            unsafe {
                // UTF-16 编码
                let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
                let byte_len = wide.len() * 2;

                if OpenClipboard(None).is_err() {
                    return Err("无法打开剪贴板".into());
                }
                let _ = EmptyClipboard();

                let hmem = GlobalAlloc(GMEM_MOVEABLE, byte_len)
                    .map_err(|_| "GlobalAlloc 失败".to_string())?;
                let ptr = GlobalLock(hmem) as *mut u16;
                if ptr.is_null() {
                    let _ = CloseClipboard();
                    return Err("GlobalLock 失败".into());
                }
                std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
                GlobalUnlock(hmem);

                SetClipboardData(CF_UNICODETEXT.0 as u32, windows::Win32::Foundation::HANDLE(hmem.0))
                    .map_err(|_| "SetClipboardData 失败".to_string())?;
                let _ = CloseClipboard();
                Ok(())
            }
        }
        #[cfg(not(windows))]
        {
            Err("仅支持 Windows".into())
        }
    })
    .await
    .map_err(|e| format!("任务失败: {}", e))?
}

/// 打开 Windows 自带截图工具 (Snipping Tool)
#[tauri::command]
async fn open_snipping_tool() -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("cmd.exe")
            .args(["/C", "start", "ms-screenclip:"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
            .map_err(|e| format!("启动截图工具失败: {}", e))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("仅支持 Windows".into())
    }
}

/// 检测截图工具是否仍在运行
#[tauri::command]
async fn is_snipping_tool_running() -> Result<bool, String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let output = std::process::Command::new("tasklist")
            .args(["/NH"])
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| format!("检测进程失败: {}", e))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.contains("ScreenClippingHost.exe") || stdout.contains("SnippingTool.exe"))
    }
    #[cfg(not(windows))]
    {
        Ok(false)
    }
}

/// 保存截图到文件，返回文件路径
#[tauri::command]
async fn save_screenshot(image_data: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || screenshot::save_screenshot_to_file(&image_data))
        .await
        .map_err(|e| format!("保存截图失败: {}", e))?
}

/// 从系统剪贴板读取图片，保存到文件并返回路径。
/// 如果剪贴板中没有图片，返回 None。
#[tauri::command]
async fn paste_image_from_clipboard() -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(|| screenshot::read_image_from_clipboard())
        .await
        .map_err(|e| format!("读取剪贴板失败: {}", e))?
}

/// 从剪贴板读取图片，返回 base64 数据（供内联显示和 MCP 存储）。
/// 如果剪贴板中没有图片，返回 None。
#[tauri::command]
async fn paste_image_as_base64() -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(|| screenshot::read_image_as_base64())
        .await
        .map_err(|e| format!("读取剪贴板失败: {}", e))?
}

/// 检测系统可用的 shell profiles
#[tauri::command]
async fn detect_shells() -> Result<Vec<serde_json::Value>, String> {
    tokio::task::spawn_blocking(detect_shells_sync)
        .await
        .map_err(|e| format!("检测 shell 失败: {}", e))
}

fn detect_shells_sync() -> Vec<serde_json::Value> {
    let mut shells = Vec::new();

    if cfg!(target_os = "windows") {
        // PowerShell 7+
        if which_exists("pwsh.exe") {
            shells.push(serde_json::json!({
                "name": "PowerShell",
                "command": "pwsh.exe",
                "icon": "powershell"
            }));
        }
        // Windows PowerShell 5.1
        if which_exists("powershell.exe") {
            shells.push(serde_json::json!({
                "name": "Windows PowerShell",
                "command": "powershell.exe",
                "icon": "powershell"
            }));
        }
        // CMD
        if which_exists("cmd.exe") {
            shells.push(serde_json::json!({
                "name": "命令提示符",
                "command": "cmd.exe",
                "icon": "cmd"
            }));
        }
        // Git Bash
        let git_bash_paths = [
            "C:\\Program Files\\Git\\bin\\bash.exe",
            "C:\\Program Files (x86)\\Git\\bin\\bash.exe",
        ];
        for path in &git_bash_paths {
            if std::path::Path::new(path).exists() {
                shells.push(serde_json::json!({
                    "name": "Git Bash",
                    "command": path,
                    "icon": "git"
                }));
                break;
            }
        }

        // Azure Cloud Shell
        if which_exists("az.cmd") || which_exists("az.exe") {
            shells.push(serde_json::json!({
                "name": "Azure Cloud Shell",
                "command": "powershell.exe -NoExit -Command \"az interactive\"",
                "icon": "azure"
            }));
        }

        // Visual Studio Developer 环境
        detect_vs_shells(&mut shells);

        // WSL 发行版（逐个列出）
        detect_wsl_distros(&mut shells);
    } else {
        // macOS / Linux
        if std::path::Path::new("/bin/zsh").exists() {
            shells.push(serde_json::json!({
                "name": "Zsh",
                "command": "/bin/zsh",
                "icon": "terminal"
            }));
        }
        if std::path::Path::new("/bin/bash").exists() {
            shells.push(serde_json::json!({
                "name": "Bash",
                "command": "/bin/bash",
                "icon": "terminal"
            }));
        }
        if std::path::Path::new("/usr/bin/fish").exists() {
            shells.push(serde_json::json!({
                "name": "Fish",
                "command": "/usr/bin/fish",
                "icon": "terminal"
            }));
        }
    }

    shells
}

/// 检测 Visual Studio Developer Shell/Prompt
fn detect_vs_shells(shells: &mut Vec<serde_json::Value>) {
    let program_files = std::env::var("ProgramFiles").unwrap_or_default();
    let program_files_x86 = std::env::var("ProgramFiles(x86)").unwrap_or_default();

    // VS 版本：年份 + 搜索目录
    // 2022+ 装在 Program Files，2019 及以下装在 Program Files (x86)
    let vs_versions: Vec<(&str, &str)> = vec![
        ("2022", &program_files),
        ("2022", &program_files_x86),
        ("2019", &program_files),
        ("2019", &program_files_x86),
        ("2017", &program_files),
        ("2017", &program_files_x86),
    ];
    let vs_editions = ["Enterprise", "Professional", "Community", "BuildTools", "Preview"];

    let mut found_years: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (year, pf) in &vs_versions {
        if pf.is_empty() || found_years.contains(*year) { continue; }
        for edition in &vs_editions {
            let vcvars = format!(
                "{}\\Microsoft Visual Studio\\{}\\{}\\Common7\\Tools\\VsDevCmd.bat",
                pf, year, edition
            );
            if !std::path::Path::new(&vcvars).exists() {
                continue;
            }

            found_years.insert(year.to_string());

            // Developer Command Prompt
            shells.push(serde_json::json!({
                "name": format!("Developer Command Prompt for VS {}", year),
                "command": format!("cmd.exe /k \"{}\"", vcvars),
                "icon": "vs"
            }));

            // Developer PowerShell
            let devshell_dll = format!(
                "{}\\Microsoft Visual Studio\\{}\\{}\\Common7\\Tools\\Microsoft.VisualStudio.DevShell.dll",
                pf, year, edition
            );
            let vs_install = format!(
                "{}\\Microsoft Visual Studio\\{}\\{}",
                pf, year, edition
            );
            shells.push(serde_json::json!({
                "name": format!("Developer PowerShell for VS {}", year),
                "command": format!(
                    "powershell.exe -NoExit -Command \"& {{ Import-Module '{}'; Enter-VsDevShell -VsInstallPath '{}' }}\"",
                    devshell_dll, vs_install
                ),
                "icon": "vs"
            }));

            break; // 每个年份只取第一个找到的 edition
        }
    }

    // 扫描 Build Tools 独立安装（不在 Visual Studio 目录下）
    for pf in &[&program_files, &program_files_x86] {
        if pf.is_empty() { continue; }
        let bt_base = format!("{}\\Microsoft Visual Studio", pf);
        if let Ok(entries) = std::fs::read_dir(&bt_base) {
            for entry in entries.flatten() {
                let year_name = entry.file_name().to_string_lossy().to_string();
                if found_years.contains(&year_name) { continue; }
                // 检查是否是年份目录
                if !year_name.chars().all(|c| c.is_ascii_digit()) { continue; }
                for edition in &vs_editions {
                    let vcvars = format!(
                        "{}\\{}\\{}\\Common7\\Tools\\VsDevCmd.bat",
                        bt_base, year_name, edition
                    );
                    if std::path::Path::new(&vcvars).exists() {
                        found_years.insert(year_name.clone());

                        shells.push(serde_json::json!({
                            "name": format!("Developer Command Prompt for VS {}", year_name),
                            "command": format!("cmd.exe /k \"{}\"", vcvars),
                            "icon": "vs"
                        }));

                        let devshell_dll = format!(
                            "{}\\{}\\{}\\Common7\\Tools\\Microsoft.VisualStudio.DevShell.dll",
                            bt_base, year_name, edition
                        );
                        let vs_install = format!(
                            "{}\\{}\\{}",
                            bt_base, year_name, edition
                        );
                        shells.push(serde_json::json!({
                            "name": format!("Developer PowerShell for VS {}", year_name),
                            "command": format!(
                                "powershell.exe -NoExit -Command \"& {{ Import-Module '{}'; Enter-VsDevShell -VsInstallPath '{}' }}\"",
                                devshell_dll, vs_install
                            ),
                            "icon": "vs"
                        }));

                        break;
                    }
                }
            }
        }
    }
}

/// 检测 WSL 发行版
fn detect_wsl_distros(shells: &mut Vec<serde_json::Value>) {
    if !which_exists("wsl.exe") {
        return;
    }

    // 尝试用 wsl -l -q 获取发行版列表（带超时，防止 WSL 卡住）
    let child = std::process::Command::new("wsl.exe")
        .args(["-l", "-q"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn();
    let output = match child {
        Ok(mut c) => {
            use std::time::{Duration, Instant};
            let deadline = Instant::now() + Duration::from_secs(3);
            loop {
                match c.try_wait() {
                    Ok(Some(_)) => break c.wait_with_output().map_err(|e| e),
                    Ok(None) => {
                        if Instant::now() >= deadline {
                            let _ = c.kill();
                            break Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "wsl 检测超时"));
                        }
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(e) => break Err(e),
                }
            }
        }
        Err(e) => Err(e),
    };

    match output {
        Ok(out) => {
            // wsl 输出是 UTF-16LE
            let raw = out.stdout;
            let text = String::from_utf16_lossy(
                &raw.chunks(2)
                    .filter_map(|c| {
                        if c.len() == 2 { Some(u16::from_le_bytes([c[0], c[1]])) } else { None }
                    })
                    .collect::<Vec<u16>>()
            );

            for line in text.lines() {
                let name = line.trim().trim_matches('\0');
                if name.is_empty() { continue; }
                shells.push(serde_json::json!({
                    "name": name,
                    "command": format!("wsl.exe -d {}", name),
                    "icon": "linux"
                }));
            }
        }
        Err(_) => {
            // fallback: 只加一个通用 WSL 入口
            shells.push(serde_json::json!({
                "name": "WSL",
                "command": "wsl.exe",
                "icon": "linux"
            }));
        }
    }
}

// ─── 代理相关 Tauri 命令 ───

/// 获取代理状态
#[tauri::command]
async fn proxy_get_status(
    state: State<'_, AppState>,
) -> Result<proxy::ProxyStatus, String> {
    let mut pm = state.proxy_manager.lock().await;
    Ok(pm.get_status())
}

/// 切换代理模式
#[tauri::command]
async fn proxy_set_mode(
    state: State<'_, AppState>,
    mode: String,
) -> Result<proxy::ProxyStatus, String> {
    let proxy_mode = match mode.as_str() {
        "off" => proxy::ProxyMode::Off,
        "manual" => proxy::ProxyMode::Manual,
        "clash" => proxy::ProxyMode::Clash,
        _ => return Err(format!("未知代理模式: {}", mode)),
    };
    let mut pm = state.proxy_manager.lock().await;
    pm.set_mode(proxy_mode)?;
    Ok(pm.get_status())
}

/// 设置手动代理地址
#[tauri::command]
async fn proxy_set_manual(
    state: State<'_, AppState>,
    http: String,
    socks: String,
) -> Result<(), String> {
    let mut pm = state.proxy_manager.lock().await;
    pm.set_manual_config(http, socks);
    Ok(())
}

/// 更新订阅（需要网络请求，在阻塞线程中执行）
#[tauri::command]
async fn proxy_update_subscription(
    state: State<'_, AppState>,
    url: String,
) -> Result<Vec<proxy::ProxyNode>, String> {
    let pm = state.proxy_manager.clone();
    tokio::task::spawn_blocking(move || {
        let mut pm = pm.blocking_lock();
        pm.update_subscription(url)
    })
    .await
    .map_err(|e| format!("订阅更新任务失败: {}", e))
}

/// 删除订阅
#[tauri::command]
async fn proxy_remove_subscription(
    state: State<'_, AppState>,
    url: String,
) -> Result<(), String> {
    let mut pm = state.proxy_manager.lock().await;
    pm.remove_subscription(&url);
    Ok(())
}

/// 获取节点列表
#[tauri::command]
async fn proxy_list_nodes(
    state: State<'_, AppState>,
) -> Result<Vec<proxy::ProxyNode>, String> {
    let pm = state.proxy_manager.lock().await;
    Ok(pm.list_nodes())
}

/// 切换节点
#[tauri::command]
async fn proxy_switch_node(
    state: State<'_, AppState>,
    node_name: String,
) -> Result<(), String> {
    let mut pm = state.proxy_manager.lock().await;
    pm.switch_node(&node_name)
}

/// 设置 Clash 策略
#[tauri::command]
async fn proxy_set_strategy(
    state: State<'_, AppState>,
    strategy: String,
) -> Result<(), String> {
    let clash_strategy = match strategy.as_str() {
        "url-test" => proxy::ClashStrategy::UrlTest,
        "fallback" => proxy::ClashStrategy::Fallback,
        "load-balance" => proxy::ClashStrategy::LoadBalance,
        "select" => proxy::ClashStrategy::Select,
        _ => return Err(format!("未知策略: {}", strategy)),
    };
    let mut pm = state.proxy_manager.lock().await;
    pm.set_strategy(clash_strategy);
    Ok(())
}

/// 测速
#[tauri::command]
async fn proxy_test_latency(
    state: State<'_, AppState>,
) -> Result<Vec<proxy::ProxyNode>, String> {
    let mut pm = state.proxy_manager.lock().await;
    Ok(pm.test_latency())
}

/// 设置节点是否参与轮换
#[tauri::command]
async fn proxy_set_node_enabled(
    state: State<'_, AppState>,
    node_name: String,
    enabled: bool,
) -> Result<(), String> {
    let mut pm = state.proxy_manager.lock().await;
    pm.set_node_enabled(&node_name, enabled);
    Ok(())
}

/// 批量设置所有节点启用/禁用
#[tauri::command]
async fn proxy_set_all_nodes_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    let mut pm = state.proxy_manager.lock().await;
    pm.set_all_nodes_enabled(enabled);
    Ok(())
}

/// 获取代理环境变量
#[tauri::command]
async fn proxy_get_env_vars(
    state: State<'_, AppState>,
) -> Result<Option<std::collections::HashMap<String, String>>, String> {
    let mut pm = state.proxy_manager.lock().await;
    Ok(pm.proxy_env_vars())
}

/// 下载 mihomo 二进制
#[tauri::command]
async fn proxy_download_mihomo(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let pm = state.proxy_manager.clone();
    tokio::task::spawn_blocking(move || {
        let pm = pm.blocking_lock();
        pm.download_mihomo()
    })
    .await
    .map_err(|e| format!("下载任务失败: {}", e))?
}

/// 以管理员身份重启 Unterm（通过 UAC 提权）
#[tauri::command]
async fn open_admin_shell(_shell: Option<String>) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        use windows::core::PCWSTR;

        // 获取当前可执行文件路径
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("获取程序路径失败: {}", e))?;

        let exe_wide: Vec<u16> = exe_path.to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let verb: Vec<u16> = "runas\0".encode_utf16().collect();

        unsafe {
            let result = ShellExecuteW(
                None,
                PCWSTR(verb.as_ptr()),
                PCWSTR(exe_wide.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );

            // ShellExecuteW 返回值 > 32 表示成功
            let code = result.0 as usize;
            if code <= 32 {
                return Err(format!("UAC 提权失败 (code: {})", code));
            }
        }

        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("管理员模式仅支持 Windows".into())
    }
}

fn load_saved_window_size() -> (u32, u32) {
    let default = (1200, 800);
    let path = match dirs::config_dir() {
        Some(d) => d.join("unterm").join("window-state.json"),
        None => return default,
    };
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => return default,
    };
    let json: serde_json::Value = match serde_json::from_str(&data) {
        Ok(j) => j,
        Err(_) => return default,
    };
    let w = json["width"].as_u64().unwrap_or(1200) as u32;
    let h = json["height"].as_u64().unwrap_or(800) as u32;
    (w.max(800), h.max(600))
}

/// 打开文件夹选择对话框，返回选中的路径
#[tauri::command]
async fn pick_folder() -> Result<Option<String>, String> {
    let result = rfd::AsyncFileDialog::new()
        .set_title("选择工作目录")
        .pick_folder()
        .await;

    match result {
        Some(handle) => Ok(Some(handle.path().to_string_lossy().to_string())),
        None => Ok(None),
    }
}

fn which_exists(cmd: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(cmd).exists()))
        .unwrap_or(false)
}

/// 获取启动参数中的 cwd
#[tauri::command]
fn get_launch_cwd(state: State<'_, LaunchArgs>) -> Option<String> {
    state.cwd.clone()
}

/// 保存窗口大小
#[tauri::command]
fn save_window_state(width: u32, height: u32) -> Result<(), String> {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("unterm");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("window-state.json");
    let json = serde_json::json!({ "width": width, "height": height });
    std::fs::write(path, json.to_string()).map_err(|e| e.to_string())
}

/// 读取窗口大小
#[tauri::command]
fn load_window_state() -> Option<serde_json::Value> {
    let path = dirs::config_dir()?.join("unterm").join("window-state.json");
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

/// 启动参数
struct LaunchArgs {
    cwd: Option<String>,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("unterm_app=debug")
        .init();

    info!("unterm-app 启动中...");

    // 解析命令行参数：unterm-app [cwd]
    let args: Vec<String> = std::env::args().collect();
    let cwd = args.get(1).cloned().filter(|s| {
        let p = std::path::Path::new(s);
        p.exists() && p.is_dir()
    });
    if let Some(ref dir) = cwd {
        info!("启动目录: {}", dir);
    }

    // 读取上次窗口大小
    let (win_width, win_height) = load_saved_window_size();

    // 在独立线程中启动内嵌 MCP Server
    let (token_tx, token_rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("无法创建 MCP tokio runtime");
        rt.block_on(async {
            let (handle, token) = unterm_core::spawn_mcp_server();
            let _ = token_tx.send(token);
            handle.await.ok();
        });
    });
    // 等待获取 auth token
    let auth_token = token_rx.recv_timeout(std::time::Duration::from_secs(5))
        .unwrap_or_default();
    // 给 MCP Server 一点启动时间
    std::thread::sleep(std::time::Duration::from_millis(200));

    // 启动 Core 通信桥（连接本进程内的 MCP Server）
    let bridge = CoreBridge::start("127.0.0.1:19876".into(), 50, auth_token);

    // 启动代理管理器
    let mut proxy_manager = proxy::ProxyManager::new();
    proxy_manager.auto_start();

    let state = AppState {
        bridge: Arc::new(Mutex::new(bridge)),
        proxy_manager: Arc::new(Mutex::new(proxy_manager)),
    };

    let launch_args = LaunchArgs { cwd };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .manage(launch_args)
        .setup(move |app| {
            // 恢复窗口大小
            if let Some(window) = app.get_webview_window("main") {
                use tauri::LogicalSize;
                let _ = window.set_size(tauri::Size::Logical(LogicalSize::new(
                    win_width as f64,
                    win_height as f64,
                )));
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_session,
            send_input,
            resize_session,
            destroy_session,
            get_screen,
            poll_events,
            detect_shells,
            capture_screen,
            copy_image_to_clipboard,
            proxy_get_status,
            proxy_set_mode,
            proxy_set_manual,
            proxy_update_subscription,
            proxy_remove_subscription,
            proxy_list_nodes,
            proxy_switch_node,
            proxy_set_strategy,
            proxy_test_latency,
            proxy_set_node_enabled,
            proxy_set_all_nodes_enabled,
            proxy_get_env_vars,
            proxy_download_mihomo,
            open_admin_shell,
            save_screenshot,
            paste_image_from_clipboard,
            paste_image_as_base64,
            copy_text_to_clipboard,
            open_snipping_tool,
            is_snipping_tool_running,
            get_launch_cwd,
            save_window_state,
            load_window_state,
            pick_folder,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri 应用启动失败");
}
