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
- [x] B7 search：匹配着色、↑↓ 步进、Ctrl-U、空格已修。Ctrl-R 忽略大小写/精确/正则三态循环已修（引擎 SearchMode 贯通，
      正则非法=零匹配，搜索栏按 0.57.4 命名显示当前模式，默认 ignore case）。（0.57.4 全匹配两色着色+可点击跳转）、无
      ↑↓/翻页/readline 编辑、同步全量搜索。
- [x] B8 拖拽到上下边缘自动滚屏，选区可跨屏。
- [x] B9 侧栏五项全部恢复：滚轮、hover、宽度拖拽、可见滚动条、按住拖拽重排 Tab。：滚轮滚动、可见滚动条、右缘宽度拖拽、拖拽重排 tab、
      行 hover 高亮（sidebar_scroll/sidebar_points 是只初始化的死字段）。
- [x] B10 链接：hover 下划线+单击打开已修。hyperlink_rules 自定义正则已修
      （`[[hyperlink_rules]]` regex+format、$0/$1 捕获替换、用户规则整组替换
      内置集同 0.57.4，schema 校验含逐条正则合法性）。：点击即开→Ctrl+点击；hover 无 Hand 光标无高亮
      （仅按住 Ctrl 时画下划线）。
- [x] B11 拖放文件：路径经引号规则粘贴进焦点 pane。
- [x] B12 会话恢复：关窗写 last_session.json（物理尺寸/最大化/每 tab cwd），裸启动恢复几何+首 tab cwd+其余 tab；已真机验证 2250x1200 精确复原。
- [x] B13 配置：schema check 已接入 load；enable_scroll_bar、window_close_confirmation(=NeverPrompt 可关确认)、audible_bell、default_cwd 已兑现；window.decorations 已兑现（true 换回系统边框）、background_opacity 双键名已统一；
      [keys]/[env] 已真实兑现（[keys] 用户绑定折入内置表、坏 chord/未知
      action=按行号有序警告不崩溃；[env] 注入每个新会话）。tab_bar.* 九键、
      title_button.* 三键、tab_bar_lift/inactive_dim 随顶栏设计变更定性不适用。
- [x] B14 更新轮询已在启动时拉起：update_check start_background_poller 零调用。
- [~] B15 overlay：tab 上下文菜单✓、theme_selector✓(ThemePicker)、Insights 面板✓(Ctrl+Shift+I 只读卡片)；debug overlay+Lua REPL 不回归（Lua 已移除，设计决策）、proxy_settings 走 Web（设计决策）。B15 关闭。

## C 级 — 明显缩水/退化

- [x] C16 copy mode：w/b、V、Ctrl-v、f/F/t/T/;/, 全部实现（带测试）；quick select 14 类+大写=粘贴已修；quick_select_alphabet 已可配
      （重复/过短回退默认并警告）。原：
      f/F/t/T/;/, 无、退出不回滚到底。
- [~] C17 顶栏：双击最大化、关闭确认、Cockpit ⚡/✋ 芯片（点击开收件箱）、滚轮切 tab 已修；▾ 右键等价已修；Snap Layouts 需绕过 winit 命中测试 → 0.61。C17 关闭（余一项 0.61）。
- [x] C18 pane 焦点：点非活动 pane 聚焦（点击吞掉不误选）、滚轮按指针路由到 pane；focus-follows-mouse 作为可选项暂不引入。
- [x] C19 杂项：窗口标题已恢复 `[i/N] 项目 — 标题 — Unterm (实例)`；audible bell 已恢复（可用 audible_bell=Disabled 关闭）；text_blink_rate/text_blink_rate_rapid 已恢复（SGR 5/6 按配置节拍隐显，无闪烁字符时零重绘）；
      visual bell 已可配（visual_bell.fade_in/out_duration_ms + function + target，默认与 0.57.4 一致=不闪）；charselect 13 组已恢复（emoji 9 组+ShortCodes+NerdFonts+Unicode 名表+Recents，Ctrl+R/Ctrl+Shift+R 轮换，nucleo 模糊匹配+U+/hex 码点检索同 0.57.4）；
      launcher workspace 条目（列出/切换/保存，走 ~/.unterm/workspaces 与
      MCP 同源）与键位浏览（组合键+action，选中即执行）已恢复，domains 属
      设计决策不回归；proxy chip 已恢复就地切换语义（左键=探针通过才启停
      env 注入、失败短暂红显、永不碰系统代理，右键=设置页）；inactive_pane
      恢复完整 HSB（hue/saturation/brightness，默认 1.0 同 0.57.4）；
      "+"右键开 shell 选择器已恢复；macOS 右键等价(Ctrl+左键)已恢复（仅
      纯 CTRL，鼠标上报中按 0.57.4 语义先让程序）；default_cwd 已兑现。
      C19 关闭。

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

## 终局处置（2026-07-31 收口，2026-08-01 增强清单清零）

29 项缺口全部处置完毕：除 **3 项设计决策不回归**（debug overlay/Lua REPL、
proxy_settings 专属浮层、launcher 的 mux domains）与 Snap Layouts（Windows
专属，仍列 0.61）外，其余全部修复并有测试：原 0.61 增强清单中的搜索
Ctrl-R 引擎三态、hyperlink_rules 自定义、text_blink_rate/visual_bell、
charselect 13 组、quick_select_alphabet、[keys]/[env]、launcher workspace/
键位、proxy chip 就地切换、inactive_pane HSB、macOS Ctrl+左键均已于
2026-08-01 补齐。tab_bar.*/title_button.* 配置键随顶栏设计变更定性为
不适用，schema 校验会提示。

## macOS 侧验收（2026-08-01）

交接单的 macOS 任务执行结果。构建+全量测试绿（593→596 项，含新增模块）。
过程中发现并修复的 macOS 固有缺口（Windows 真机验收测不到的）：

- chrome 符号字体：▶ ▾ ▸ ✓ 在 macOS 无字体认领、画成空位——fallback 栈
  补 Apple Symbols（几何形状）+ Menlo（✓）+ Noto Sans Symbols 2（Linux 同
  修）。fonts.rs 栈级测试锁定。
- capture.window / capture.region：unterm-services 的实现是 Windows-only
  stub，GUI host 路径在 mac 直接报"未实现"——按 0.57.4 同款移植
  （CGWindowListCopyWindowInfo 找窗口 + `screencapture -l`，region 走全屏
  原生像素裁剪）。真机验证卡在 Screen Recording 权限（0.57.4 在本机同样
  未授权、同样失败，行为已一致；授权是用户一次性动作）。
- selftest styled_scrollback_capture：selftest 的 echo 命令带未加引号的
  `[31;1m`，zsh 当 glob 报 bad pattern，marker 永不打印——POSIX shell 加
  单引号，cmd.exe 保持原样。修后 selftest 13/14（余 capture.window 即上条
  权限项）。
- 窗口标题实例名：用了 `server_info::read()`（全机 active 实例）而非
  `read_current()`（本进程），双窗口时两个标题都自称同一个实例。
- keys `[keys]` 解析警告顺序：配置存储按键名排序遍历，警告乱序且同 chord
  重复定义时"后写的赢"不成立——按行号排序。
- 终端测试在 mac 字形 bearing 下的脆断（block cursor 谓词取最近格）。
