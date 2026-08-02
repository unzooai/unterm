# 研究 — Ghostty / Kitty 对照 Unterm:tab 栏信息结构与观感

> 2026-08-02。素材:/tmp/ghostty-src(MIT,可参考实现)、/tmp/kitty-src
> (**GPLv3,只学思路,严禁抄码**)、/tmp/alacritty-src(Apache-2.0,无 tab,
> 仅作反例)。结论先行:**左侧栏的结构不换,但两家的克制和信息语法必须学。**

---

## 1. 两家怎么做 tab

### Ghostty:原生到底

- macOS 上**完全不画 tab 栏**:每个 tab 是真 NSWindow tab group,AppKit 的
  NSTabBar,Ghostty 只做"事后装饰"(macos/Sources/.../TerminalWindow.swift)。
  拖出/合并/概览/全屏/无障碍全部白得。
- 每个 tab 里只注入三样东西(accessoryView):**6px 色点**(右键 9 色可选)、
  **实时快捷键提示**(relabelTabs() 把用户真实绑定的 ⌘1-9 画进 tab,重排/
  关闭后重算)、分屏 zoom 时的复位钮。仅此而已。
- 铃 = 标题前缀 "🔔"+Dock 角标;进度(OSC 9;4)画在 **pane 顶部**的细进度条
  (15s 过期),不进 tab。注意力标记**只标非活动 tab、聚焦即自动清除**。
- 哲学(README):"让每个平台的用户觉得 Ghostty 是先为他们的平台做的"。
  代价:为了在原生 tab 里塞一个色点,靠类名嗅探+私有 API,自己注释里写着
  "This is fragile"、"huge hack"。
- 弱点:横向 tab 8-10 个就挤没了;**零层级**——没有项目分组、没有 agent
  状态语义,想加就得跟 AppKit 打私有 API 游击战。

### Kitty:终端网格里画 tab,信息模板化

- tab 栏本身是一块**终端屏幕**(tab_bar.py 持有一个 Screen,按 cell 画),
  样式=不同的 draw 函数(fade/separator/powerline/slant/hidden/custom)。
  这版已支持 `tab_bar_edge left/right` 垂直栏:每 tab 最多 2 行,挤了降 1 行,
  再挤画红色 `…` 溢出行;栏宽自动=标题上限+8,封顶窗宽 1/3。
- **杀手锏是 `tab_title_template`**:Python f-string 模板,变量包括
  `{index} {title} {layout_name} {num_windows} {bell_symbol}
  {activity_symbol} {tab.progress_percent} {tab.active_wd} {tab.active_exe}
  {fmt.fg.red} {sup.index}`…活动/铃/进度是**固定状态语法**;用户模板漏写
  状态符号时自动前置(状态永不静默丢失)。
- 密度算法:每 tab 均分列宽 → 量一遍理想宽 → 富余先补活动 tab;
  `tab_bar_min_tabs 2`(单 tab 不显示栏);`tab_bar_filter` 搜索表达式
  同时约束显示与 next/prev 导航;`goto_tab -1` 走 **MRU 栈**;
  `tab_switch_strategy` 定关 tab 后焦点去向。
- 状态通道分层:铃/活动是标题里的红色字形;pane 级用**边框颜色**说话
  (active/inactive/bell 三色边框)+ 失焦文字降 alpha。

### Alacritty:没有 tab(README 原话:交给 tmux/窗管)。唯一启示是克制。

---

## 2. 我们现状的诚实批判(2026-08-02 截图)

结构是对的,**观感确实糙**。逐条:

1. **计数徽章太重**:分组行右侧的大黑药丸 "2"/"1" 是全栏视觉最重的元素,
   但它承载的信息价值最低。Ghostty 一个 tab 只有 标题+点+键位。
2. **"claude zsh" 双名并置**:agent 图标 + "claude" + "zsh" 三个身份并排,
   冗余且参差。
3. **图标语义混乱**:socal 的文件夹实心 teal、Home 的是暗色——teal 到底表示
   "活动项目"还是装饰?终端图标带描边方框,平添噪音。
4. **排版节奏散**:分组标题、tab 行、footer 三种行高/缩进/字重各自为政;
   小三角与文本基线不齐;组与组之间的间距和组内行距区分不明。
5. **footer 动作行(+ ▾ 🔍)**:目标小、无分隔、悬浮在空白里。
6. 活动行的 jade 竖条+亮底本身成立,是全栏少数说得通的元素。

## 3. 判词:结构不学,纪律要学

- **不换成 Ghostty 式原生横 tab**:我们的信息模型(项目分组、agent 状态、
  10+ 并发会话、舱位徽标)在 NSTabBar 里表达不出来——Ghostty 自己的代码就是
  证据:塞个色点都要 hack。左栏是 Agent Cockpit 的结构性优势。
- **要学的是两家的纪律**:Ghostty 的"每行只许安静的三样",Kitty 的
  "状态是固定语法、信息是模板变量、密度有降级算法"。

## 4. 改造设计(完整交付,无 MVP 切片)

### A. 行语法(每行只有四个槽,从左到右)

`[活动竖条] [序号/键位] [单一身份图标] [标题] …… [状态点]`

- **序号槽升级为键位提示**(学 Ghostty relabelTabs):显示真实绑定
  (⌘1-9/Ctrl+1-9),第 10 个起显示序号灰字;重排/关闭后重算。
- **身份图标只留一个**:agent 行显 agent 图标(teal 只在"正在工作"时呼吸),
  普通 shell 行显无框、单色、降一档灰度的终端字形。方框描边删除。
- **标题单一来源**:agent 行 = `claude · unterm`(agent 名 + 目录尾级,
  2026-06-12 已拍板的格式),shell 行 = 自动标题。不再并置 shell 名。
- **状态点(右端,单点,自清除)**:铃/需要确认=琥珀点;活动(失焦后有输出,
  学 Kitty activity)=灰点;进度(OSC 9;4)=细环。**聚焦行永不显示状态点,
  聚焦即清除**(Ghostty 生命周期)。进度条本体画在 pane 顶部,15s 过期。

### B. 分组行

- 标题小一号、全小写、降灰度;计数徽章**删除**,折叠时才在标题后跟
  暗色数字 `home 2`;文件夹图标统一暗色,teal 只给"含活动 tab"的组。
- 组间距 = 2×行内距,建立节奏;三角与文本基线对齐。

### C. 密度与导航(学 Kitty)

- 行降级:>N 行时 tab 行从两行(标题+副标题)降为单行;再溢出显示
  `… +4` 溢出行,点击开 Tab Navigator。
- `goto_tab -1` = MRU 上一个 tab(真实 MRU 栈);关 tab 焦点策略=MRU。
- 侧栏搜索(已有 🔍)升级为 filter:过滤同时约束 next/prev 循环。

### D. footer

- 与列表间加 1px hairline;三个动作等分整行宽、hover 亮底;+ 号行高与
  tab 行一致。

### E. 显式不做

- kitty 的自定义 draw 函数插件(它成立是因为其 tab 栏是终端屏;我们用
  声明式模板即可,列入远期)。
- Ghostty 式原生 NSTabBar;tab 撕出成窗(等真实需求)。

## 5. 待拍板

1. 键位提示常显 vs 仅按住 ⌘ 时显示(Ghostty 常显;常显更教学,略吵)。
2. 状态点色板(琥珀=等人 / teal=工作中 / 灰=有输出)是否与 cockpit
   徽标统一成一套语义色。
3. A-D 是否一次性合入(工量 ≈ 2-3 天,含真机逐项截图验收)。
