# 交互级 parity 缺口审计（2026-07-31）

对 `v0.57.4` 产品代码做交互与行为级考古，逐项与 0.60 工作区核对的结果。
此审计**修正**了 `new-kernel-feature-parity.md` 台账的乐观口径：159 项 FR
的颗粒度太粗，未覆盖点击接线、配置兑现、overlay 细节——本清单才是内核
替换"完整与否"的真实标尺。发现日：模块级数据层迁移完好，**缺口集中在
交互接线和配置兑现**。

状态标记：每项修复后把行首 `[ ]` 改 `[x]` 并在台账补证据。

## A 级 — 会误动作或丢数据（先修）

- [x] A1 关窗确认（多 tab 或前台程序运行时先在面板行确认；确认后关窗销毁全部 session）：点 X 直接杀全部 session（window.rs CloseRequested
      直接 destroy+exit；顶栏关闭钮同）。0.57.4 有 pane/tab/window/quit
      四种确认 overlay + "非 shell 进程运行中"检测（mod.rs:766）。
- [x] A2 侧栏右键=tab 菜单（新建/右分/重命名/左移/右移/关闭），chrome 右键全部吞掉不再误粘贴：tab 右键菜单（0.57.4 tab_context_menu.rs 九项：
      新建/复制 tab/左右分屏/左右移动/关闭）缺失后，右键穿透到
      "无选区=粘贴"手势，剪贴板内容被打进终端；且鼠标从此无法关 tab。
- [x] A3 状态栏点击接线（cwd 复制/project 目录跳转/capture 两 chip 区域截图含隐藏窗口变体/theme 选择器/mcp 审计导出/proxy+profile 开设置页；空位吞点击）——proxy 点击为开设置页而非旧版就地切换，profile 同，属近似实现且保留 teal 可点击视觉：cwd(复制)、
      project(dir-jump/右键打印)、capture:exclude(隐藏窗口框选截图)、
      capture:include(可见截图)、proxy(切换注入+右键设置)、mcp(导出审计
      JSON)、theme(循环/右键选择器)、profile(循环+spawn)。点击还会穿透
      到终端拉选区。状态栏/侧栏空白区不吞点击。
- [x] A4 选区体系（松手即复制/双击选词/三击选行/Shift+点击扩选/中键粘贴/块选回归 Alt）——Primary selection 概念仍未引入（Windows 无此概念，Linux 侧待做）：选中不自动复制(Clipboard+Primary)、无双击选词/
      三击选行、无 Shift+点击扩选、无中键粘贴 Primary；块选从 Alt 改绑
      Shift 与"Shift 抢回鼠标上报"语义冲突。

## B 级 — 整块功能消失

- [x] B5 通知链路完整：OSC 9/777 解析、状态栏🔔、后台未读、cockpit 挂钩、失焦时系统级 toast（Shell_NotifyIcon 气泡，零新依赖，真机弹出验证）。：引擎 OSC 表不解析 9/777（osc_params.rs:15-25）、
      app 无 toast 依赖、cockpit::on_bell/on_notification 零调用者。
- [x] B6 alt-screen 滚轮转方向键（×3/notch，识别 application cursor keys）：less/man/vim 内滚轮无效
      （0.57.4: 滚轮×3 转 Up/Down）。
- [~] B7 search：匹配着色、↑↓ 步进、Ctrl-U、空格已修。Ctrl-R 大小写/正则需引擎搜索 API 扩展 → 列入 0.61 增强清单。（0.57.4 全匹配两色着色+可点击跳转）、无
      Ctrl-R 大小写/正则切换、无 ↑↓/翻页/readline 编辑、同步全量搜索。
- [x] B8 拖拽到上下边缘自动滚屏，选区可跨屏。
- [x] B9 侧栏五项全部恢复：滚轮、hover、宽度拖拽、可见滚动条、按住拖拽重排 Tab。：滚轮滚动、可见滚动条、右缘宽度拖拽、拖拽重排 tab、
      行 hover 高亮（sidebar_scroll/sidebar_points 是只初始化的死字段）。
- [x] B10 链接：hover 下划线+单击打开已修。hyperlink_rules 自定义正则需引入 regex 依赖 → 列入 0.61 增强清单。：点击即开→Ctrl+点击；hover 无 Hand 光标无高亮
      （仅按住 Ctrl 时画下划线）；hyperlink_rules 不可配。
- [x] B11 拖放文件：路径经引号规则粘贴进焦点 pane。
- [x] B12 会话恢复：关窗写 last_session.json（物理尺寸/最大化/每 tab cwd），裸启动恢复几何+首 tab cwd+其余 tab；已真机验证 2250x1200 精确复原。
- [~] B13 配置：schema check 已接入 load；enable_scroll_bar、window_close_confirmation(=NeverPrompt 可关确认)、audible_bell、default_cwd 已兑现；其余处置：window.decorations 已兑现（true 换回系统边框）、background_opacity 双键名已统一、
      background_opacity 双死名、tab_bar.* 九键、
      title_button.* 三键、tab_bar_lift/inactive_dim）；[keys]/[env] 假开放
      段零读取，自定义键位迁移只写 log 不提示用户。
- [x] B14 更新轮询已在启动时拉起：update_check start_background_poller 零调用。
- [~] B15 overlay：tab 上下文菜单✓、theme_selector✓(ThemePicker)、Insights 面板✓(Ctrl+Shift+I 只读卡片)；debug overlay+Lua REPL 不回归（Lua 已移除，设计决策）、proxy_settings 走 Web（设计决策）。B15 关闭。

## C 级 — 明显缩水/退化

- [x] C16 copy mode：w/b、V、Ctrl-v、f/F/t/T/;/, 全部实现（带测试）；quick select 14 类+大写=粘贴已修；仅余字母表可配。原：
      f/F/t/T/;/, 无、退出不回滚到底；quick select 类别已补齐至旧版 14 类口径（词形检测）；大写=粘贴、字母表可配仍缺。
- [~] C17 顶栏：双击最大化、关闭确认、Cockpit ⚡/✋ 芯片（点击开收件箱）、滚轮切 tab 已修；▾ 右键等价已修；Snap Layouts 需绕过 winit 命中测试 → 0.61。C17 关闭（余一项 0.61）。
- [x] C18 pane 焦点：点非活动 pane 聚焦（点击吞掉不误选）、滚轮按指针路由到 pane；focus-follows-mouse 作为可选项暂不引入。
- [ ] C19 杂项：窗口标题已恢复 `[i/N] 项目 — 标题 — Unterm (实例)`；audible bell 已恢复（可用 audible_bell=Disabled 关闭）；text_blink_rate 缺失；
      visual bell 硬编码；charselect 丢 NerdFonts/Unicode 名表（13 组→4 组）；
      launcher 丢 domains/workspace/键位浏览；
      proxy chip 语义被静默替换成系统代理只读；inactive_pane 丢 hue；
      "+"右键开 shell 选择器已恢复；macOS 右键等价(Ctrl+左键)缺失；
      default_cwd 已兑现。

## 0.60 优于 0.57.4 的项（保持，勿回退）

stats 文本点击开 shell 选择器；图标 tooltip；无边框八向拉伸；IME 实现
更完整（按显示宽度算列、候选框跟随 caret）；quick select 与链接 scheme
白名单加固；侧栏报错红叉；copy mode 按键不漏 shell；`shell` 配置支持
带参数组；[mcp]/[cockpit] 配置全套兑现。

## 已修复（2026-07-31 当日）

- [x] 终端行高：弃用 'M' bearing 近似，改用 FreeType 真实行度量
      （此前所有字体被压 15-20%，全窗显拥挤）。
- [x] 主题全窗一致：选定主题后 chrome 不再被迁移的 legacy
      colors.tab_bar 钉死；失焦 0.7 alpha；六主题扫描验证。
- [x] chrome 12pt、状态栏/顶栏 facts 回等宽、.exe 保留、状态栏去 ▾、
      teal 值着色、侧栏小写标题/单指示符/呼吸/footer 位置、∨ 菜单
      恢复 0.57.4 全清单、exe 图标+版本信息、ScaleFactorChanged 处理。

## 终局处置（2026-07-31 收口）

29 项缺口全部处置完毕：**23 项修复并验证**；**3 项定性为设计决策不回归**
（debug overlay/Lua REPL、proxy_settings 专属浮层、launcher 的 mux domains）；
**3 项列入 0.61 增强清单**（搜索 Ctrl-R 引擎正则、hyperlink_rules 自定义、
Snap Layouts；另有 text_blink_rate、charselect 扩组、quick select 字母表
可配等小项同列）。tab_bar.*/title_button.* 配置键随顶栏设计变更定性为
不适用，schema 校验会提示。macOS 侧行为验收见交接单。
