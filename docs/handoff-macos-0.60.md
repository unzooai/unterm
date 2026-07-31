# 0.60 macOS 交接单

Windows 侧的 0.60（自研 next-core 内核，替代 WezTerm fork）已完成功能/性能/外观
对 0.57.4 的逐项对齐、本地 MSI 安装与全面自测，代码已合入 master。本单据是
macOS 机器上继续工作的唯一入口：按顺序执行，逐项打勾，证据补进
`docs/new-kernel-feature-parity.md` 对应 FR 行。

## 当前状态（Windows 侧已完成）

- 分支：`agent/new-kernel-feature-parity`（已合入 `master`）。
- 关键提交：`9f843a06`（parity 总收口）、`8af8c082`（托管 CI 运行器测试守卫）。
- 台账：`docs/new-kernel-feature-parity.md` — 159 项 FR，146 verified，
  13 项 runtime pending（其中大部分是 macOS/Linux 侧动作，见下）。
- Windows 已验证：MSI 安装（`dist/Unterm-0.60.0-x64.msi`）、内置 selftest
  14/14、I/O・分屏・多 Tab・主题切换・截屏、0.57.4 外观逐面对齐、exe 图标与
  版本信息、启动 761ms vs 0.57.4 1349ms。

## macOS 侧任务（按序）

1. **构建与测试**
   - `git pull` 后 `cargo build --release -p unterm-app -p unterm-cli`。
   - `cargo test --release --workspace -- --test-threads=1` 全绿
     （桌面机不受 `GITHUB_ACTIONS` 守卫影响，字体/控制台探针会真实运行）。
2. **真窗口验收（FR-TERM-001 / FR-UI-001 的 macOS 半边）**
   - 启动应用：GPU 渲染无抖动、输入输出、分屏、Tab 切换、IME 中文输入。
   - 无边框窗口的 traffic-light 区域、拖拽、缩放、全屏切换。
   - 对照 0.57.4 DMG（GitHub Releases `Unterm-macos-v0.57.4.dmg`）做
     与 Windows 同口径的外观对比（顶栏/侧栏/状态栏/∨菜单/命令面板），
     Windows 侧的对齐结论见台账 "Chrome alignment" 一节。
3. **macOS 专属 FR（台账中 runtime pending 的 mac 项）**
   - FR-CAP-006：`capture.window_scroll` 滚动拼接另一个 App 的窗口。
   - FR-PROF-003：Keychain 后端 set-secret round-trip（写入→解析→删除）。
   - FR-MCP-004：Unix 0600 权限测试在原生环境跑通（`cargo test` 已含）。
   - FR-TERM-005：剪贴板 worker 在 macOS pasteboard 上的真机验证。
4. **打包与签名（FR-REL-001/002）**
   - `ci/sign-macos.sh`：构建 DMG → 签名 → notarytool 公证 → staple。
   - 干净机器（或新用户账户）安装验证：Gatekeeper 放行、首启动正常。
5. **收尾**
   - 台账对应 FR 行从 "Implemented, runtime pending" 改为 "Verified" 并附证据。
   - 全部通过后在 master 上打 `v0.60.0` tag 并出四平台产物（Windows MSI 已有）。

## 环境备忘

- 仓库：`zhitongblog/unterm`；推送需 `gh auth switch -u guangtoutong`
  （unzooai 账号只读）。
- MCP 端口 19876 起、HTTP 19877 起；实例注册表 `~/.unterm/instances/`。
- CLI 全量面：`unterm-cli reference`；内置自测：`unterm-cli server selftest`。
- Windows 侧遗留的三处已知样式差（快捷菜单为居中面板而非锚定下拉卡片、
  shell 选择器与目录跳转复用面板壳、顶栏 hover/高度非像素级），macOS 复核时
  同样适用，暂不阻塞发版。
