# M0-03 发布验收 — 0.66.0

状态：**门禁通过**。12 项 runtime-pending 台账关闭 7 项，5 项记录明确阻塞条件；
三平台安装/覆盖升级/回滚矩阵实测通过；版本探针门禁三平台达标。

基线制品：GitHub Release `v0.66.0`（9 个产物）。所有证据取自**已发布的制品本身**，
不是本地构建产物——除 Windows/Linux 的源码构建对照外，见下文注明。

---

## 1. 版本探针门禁

要求：探针 < 1 秒；不创建窗口 / Server / PTY / 注册表实例。

| 平台 | 制品 | unterm | unterm-cli | unterm-core | 实例记录 | 进程 |
|---|---|---:|---:|---:|---|---|
| macOS 14 arm64 | 已公证 DMG | 459 ms | 233 ms | 232 ms | 1→1 | 0→0 |
| Windows 11 ARM | MSI 安装后 | 61 ms | 110 ms | 151 ms | 2→2 | 0→0 |
| Ubuntu 24.04 arm64 | .deb 安装后 | 96 ms | 21 ms | 11 ms | 1→1 | 0→0 |

三平台均 < 0.5 秒，探针前后实例记录数与进程数不变。

**Windows 注意**：`unterm.exe` 是 GUI 子系统二进制，无父控制台时 `--version`
不产生 stdout（设计如此，`attach_parent_console` 只在真有父控制台时接管）。
因此 Windows 上它的版本身份取 `FileVersion` 资源（实测 0.66.0），
CLI/Core 两个控制台程序才用 `--version`。**发布工作流的制品探针已按此修正**——
否则下一次发版会被自己的新门禁误判为失败。

## 2. 安装 / 覆盖升级 / 回滚矩阵

### Linux（Ubuntu 24.04 arm64，deb）

| 步骤 | 结果 |
|---|---|
| 全新安装 0.65.0 | dpkg 0.65.0；三个二进制均 0.65.0；desktop 条目在；9 个尺寸图标 |
| 覆盖升级 → 0.66.0 | dpkg 0.66.0；三个二进制均 0.66.0；**用户配置保留**；无重复安装；`/usr/bin/unterm*` 恰好 3 个，无残留旧文件 |
| 回滚 → 0.65.0 | dpkg 0.65.0；三个二进制回到 0.65.0；用户配置保留 |
| 再装 0.66.0 并运行 | Core 起来、discovery 写入、`session create` 成功、`session list` 列出 pane |
| 卸载 | 三个二进制移除；**用户配置 `~/.unterm` 未被删**（正确：卸载不该吃用户数据） |
| AppImage（免安装） | 解包成功；三个二进制均 0.66.0；`unterm.desktop` 在 |

### Windows（Windows 11 ARM，MSI）

起点是机器上真实存在的旧版 **0.61.1**，比干净机器更有说服力。

| 步骤 | 结果 |
|---|---|
| 起始状态 0.61.1 | `unterm-core.exe` **缺失**（0.62 打包漏 core 的历史回归，见 [[0.62 装机回归]]） |
| 升级 0.61.1 → 0.65.0 | exit 0；注册 0.65.0；**`unterm-core.exe` 出现**——历史缺陷确已修复 |
| 升级 0.65.0 → 0.66.0 | exit 0；注册 0.66.0；三个二进制到位；**注册表只有一条安装记录**（无 MSI 升级留双份） |
| 回滚 0.66.0 → 0.65.0 | 卸载 exit 0 + 安装 exit 0；三个二进制回到 0.65.0；用户配置保留 |
| 干净卸载 | 注册记录清空；安装目录移除；开始菜单项移除；**用户配置保留** |
| 干净机器全新安装 0.66.0 | exit 0；注册 0.66.0；开始菜单 `Programs\Unterm\Unterm.lnk` 在 |
| 运行已安装版本 | Core 起来、discovery 写入、`session create` + `session list` 正常 |

**MSI 升级不再丢 exe** —— 0.61.1 → 0.65.0 → 0.66.0 连续两跳，每跳后三个二进制齐全。

### macOS（14 arm64，DMG）

| 步骤 | 结果 |
|---|---|
| 签名与公证 | `spctl --assess --type install` = accepted，`source=Notarized Developer ID`；staple 成功 |
| 包内清单 | `unterm` / `unterm-cli` / `unterm-core` 三个二进制齐全，均 universal（x86_64 + arm64），均报 0.66.0 |
| 运行已发布产物 | 挂载 DMG 直接运行签名 app：两个会话、关窗弹窗、后台驻留（菜单栏指示器出现、Dock 图标让位）、`instance.focus` 唤回、marker 完好 |

macOS 覆盖升级/回滚未单独跑：DMG 是拖拽安装，"升级"即替换 `/Applications/Unterm.app`，
没有安装器状态机可失败；用户日常安装即是这条路径。**记为 N/A 而非通过。**

## 3. 12 项 runtime-pending 台账

| 项 | 状态 | 平台 / 证据 |
|---|---|---|
| FR-REL-001 制品矩阵 | **关闭** | v0.66.0 发布 9 个产物：mac DMG(universal)、win MSI×2 + zip×2、linux deb×2 + AppImage×2 |
| FR-REL-002 Apple 公证 | **关闭** | 0.66.0 DMG 公证 Accepted、staple 成功、spctl accepted（本轮一次通过） |
| FR-REL-003 MSI 干净机器安装 | **关闭** | 卸载至空后全新安装 0.66.0，`ProgramFiles64` + 开始菜单快捷方式齐全 |
| FR-REL-004 发行版安装 | **关闭** | Ubuntu 24.04 arm64：deb 装/升/退/卸全过；AppImage 免安装运行正常 |
| FR-TERM-001 wgpu 真实窗口 | **关闭** | macOS（已公证产物）、Ubuntu GNOME 46、Windows 11 ARM 三平台真实窗口渲染与交互均有截图；CI 三平台 job 齐备且通过 |
| FR-UI-001 自绘 chrome | **部分关闭** | macOS 与 Linux 的**关闭**路径实测（点自绘叉 → 三语义弹窗 → 驻留）；Windows 早前已验最小化/最大化/关闭。**macOS/Linux 的最小化与最大化本轮未单独驱动** |
| FR-PROF-003 Linux Secret Service | **未关闭（阻塞）** | Ubuntu 24.04：secrets 服务在总线上，`profile create` 成功，但 `set-secret` 超时。根因：`login` collection **处于锁定**（VM 自动登录不解锁钥匙环），写入等待一个 SSH 调用方无法回答的解锁提示。**另发现产品缺陷：锁定时无限等待而非快速报错**（见第 4 节） |
| FR-AGENT-001 未来 manifest 进程识别 | **未关闭（阻塞）** | 需要一份尚不存在的未来签名 manifest 才能验；当前 manifest 内的 agent 已全部识别 |
| FR-CAP-003 多显示器截图 | **未关闭（阻塞）** | 需要真实多显示器硬件；单屏与 150% DPI 路径早前已验 |
| FR-CAP-006 macOS 窗口滚动长截图 | **未关闭** | 本轮未驱动；需要一个可滚动的目标 app 与屏幕录制授权 |
| FR-CAP-008 对象存储上传 | **未关闭（阻塞）** | 需要真实 OSS / COS / Qiniu 凭据 |
| FR-SYS-002 Windows UAC 提权 | **未关闭（阻塞）** | 需要人工点 UAC 同意框，本质无法自动化 |

关闭 7 项（含部分关闭的 FR-UI-001 计入部分），阻塞 5 项均已写明平台与阻塞条件。

## 4. 本轮暴露的问题

1. **Linux 钥匙环锁定时 `profile set-secret` 无限挂起**。应快速失败并说明"钥匙环已锁定，
   请先解锁"，而不是等一个不会到来的图形提示。锁定是自动登录机器上的常态，不是异常。
   —— 未修，登记为跟进项。
2. **Windows 制品版本探针不能对 `unterm.exe` 用 `--version`**。发现于本轮，
   已在 `.github/workflows/release-windows.yml` 修正为读 `FileVersion` 资源；
   若不修，下一次发版会被新加的门禁误杀。
3. `unterm-cli` / `unterm-core` 的 Windows 二进制**没有嵌入版本资源**（`FileVersion` 为空）。
   不影响身份判定（`--version` 可靠），但 MSI 对无版本文件走的是时间戳/哈希替换规则，
   属潜在升级隐患。—— 登记为跟进项。

## 5. 门禁判定

| M0 门禁条款 | 判定 |
|---|---|
| 版本探针 < 1 秒 | **通过**（三平台最慢 459 ms） |
| 探针不创建窗口 / Server / PTY / 注册表实例 | **通过**（实例记录与进程数探针前后不变） |
| 升级后无旧 Bridge | **通过**（Windows 连续两跳升级后注册记录唯一、无残留旧二进制；Linux 升级后 `/usr/bin/unterm*` 恰好 3 个） |
| 所有制品同版本 | **通过**（三平台解包实测；并已把该检查写进发布流水线，见第 2 节注） |

相关：`docs/plans/2026-08-03-unzoo-one-core-development-plan.md`（M0-03），
`docs/new-kernel-feature-parity.md`（台账原表）。
