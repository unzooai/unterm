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
- 2026-07-31 当日新增（详见 `docs/parity-gap-audit-2026-07-31.md` 与台账
  "Remediation (same day)" 一节）：
  - 外观对齐补完：真实行高度量、主题全窗一致、chrome 12pt/等宽 facts、
    侧栏小写标题+单指示符+呼吸、∨ 菜单恢复 0.57.4 全清单、exe 图标与
    版本信息、ScaleFactorChanged 处理。
  - 交互级缺口修复：A 级 4/4 清零（关窗确认、tab 右键菜单、状态栏点击
    接线、选区体系）；B 级大部清零（alt-screen 滚轮、拖拽自动滚屏、侧栏
    五项、链接单击打开、拖放粘贴路径、会话恢复、更新轮询；通知/search/
    配置兑现为部分完成）；C 级过半（copy mode 词移动与 V/Ctrl-v、顶栏
    双击最大化与滚轮切 tab、pane 点击聚焦与滚轮按指针路由）。
  - CI 三平台（Linux/macOS/Windows）首次全绿；门禁计数体系已校准
    （`fc79c8ad`、`9b869c22`），门禁保持强制而非跳过。
  - master 已合并至 `540a84df`。

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
   - 把 `docs/parity-gap-audit-2026-07-31.md` 中标 `[x]` 的交互项在 macOS
     上逐项过一遍（关窗确认、tab 右键菜单、状态栏点击、选区体系含中键
     粘贴、alt-screen 滚轮、链接单击、拖放、会话恢复、侧栏五项等）——
     这些修复的验收都在 Windows 真机做的，mac 行为需独立确认。
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
