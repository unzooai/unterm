# 研究 — Warp(已开源)对照 Unterm:字体精致度 & 左侧 Tab 栏

> 2026-06-12。素材:Warp 客户端已于 2026-04-28 开源(github.com/warpdotdev/warp,
> 本地克隆在 `/tmp/warp-src`,5530 个文件,Rust)。**许可红线:主体代码 AGPL v3,
> 不能抄进 Unterm(MIT 系);`crates/ui_components`/`warpui` 等 UI 框架 crate 是 MIT,
> 实现思路可参考;设计参数(尺寸/字号/颜色策略)是事实,随便用。**

---

## 1. 为什么我们的字"不精致" — 三个根因

### 1.1 UI chrome 字体:我们在 macOS 上用的是 Roboto(安卓字体)

- Unterm:`wezterm-font/src/lib.rs:568-574` — tab 栏/标题字体 macOS 落到
  **Roboto**(wezterm 上游遗产),Windows 用 Segoe UI Variable。
  Roboto 的字形宽度、x-height、字重曲线都是为 Android 设计的,放在 macOS
  原生窗口里"说不出哪里不对但就是糙"。
- Warp:**全平台系统字体** — macOS `.AppleSystemUIFont`(= SF Pro)、Windows
  `Segoe UI`、Linux `Noto Sans`(`crates/ui_components/examples/library.rs:19-26`,
  加载于 `app/src/appearance.rs`)。
- **结论:一行根因。UI 字体改成平台系统字体,是性价比最高的一刀。**

### 1.2 macOS 渲染管线:子像素 LCD 抗锯齿是逆系统潮流的

- Unterm:FreeType 光栅 + HarfBuzz 整形(全平台),macOS 默认
  `freetype_load_target = HorizontalLcd`(`config/src/config.rs:2220-2233`,
  当时为了"更接近 CoreText"特意调的)。
  但 **macOS 自 Mojave 起系统级移除了子像素 AA**,全系统都是灰度渲染;
  我们的子像素渲染在 Retina 上产生彩色毛边(fringing),与系统其它文字
  并排时观感"脏"。
- Warp:`crates/warpui/src/fonts/font_kit.rs:54-56` —
  **`GrayscaleAa` + `HintingOptions::None`**,加上:
  - 子像素**定位**(不是子像素 AA):水平偏移量化到 1/3 像素
    (`warpui_core/src/fonts.rs:160-192`),消除字距抖动;
  - **ThinStrokes 默认 OnHighDpiDisplays**(`warpui_core/src/rendering/mod.rs:19-43`):
    Retina 上自动减细笔画,复刻 CoreText 的细腻感;
  - 基线公式用真实 ascent/descent 垂直居中
    (`warpui_core/src/text_layout.rs:514-529`),不是拍脑袋 padding。
  - macOS 上排版直接走 **Core Text**(`warpui/src/platform/mac/text_layout.rs`)。
- **结论:macOS 默认改灰度渲染(HorizontalLcd → Normal),再补子像素定位/
  笔画变细的等价调校,放大截图 A/B 验收。**

### 1.3 排版 token 系统缺失

- Warp 有一套完整字阶:overline 10 / UI 12 / palette 14 / header 18,
  UI line-height **1.2**,圆角 **4px**,行 padding **8px**,贯穿所有组件
  (`warp_core/src/ui/appearance.rs:8-12`)。
- Unterm 的 tab 栏字号 12(`wezterm-font/src/lib.rs:605`,"match Windows
  Terminal")没问题,但 padding/行高/圆角是 `fancy_tab_bar.rs` 里逐处手拼的
  magic number,新 UI(状态栏、弹层、菜单)各写各的。
- **结论:抽一个 `ui_tokens` 常量模块,所有窗内 UI 统一引用。**

### 1.4 终端等宽字体本身不是问题

我们捆 JetBrains Mono,Warp 捆 Hack(`app/src/settings/font.rs:11`,13px)。
JBM 的质量、连字、CJK 回退都不输 Hack——**别动它**,糙的是渲染和 chrome,
不是字体选型。(Warp 的 Noto CJK/emoji 回退链是按需下载的,
`app/src/font_fallback.rs`,这点我们的 vendored 方案反而更省心。)

---

## 2. Warp 左侧 Tab 栏拆解(实测参数)

结构:最左是一条**图标轨(toolbelt)**切换多个面板(垂直 tabs / 文件树 /
全局搜索 / Warp Drive / Agent 会话),同一时刻只显示一个面板。
垂直 tabs 是其中的主面板(`app/src/workspace/view/left_panel.rs`,
`vertical_tabs.rs`)。

| 项 | 值 |
|---|---|
| 面板宽 | 默认 248px,min 200px,max 50% 窗宽,右缘可拖拽 |
| 行 | 图标 24px + 标题 12pt + 副标题 12pt(灰),padding 8px,圆角 4px |
| 标题来源 | 自动:prompt/cwd/最后命令;双击可改名 |
| 选中态 | 1px 边框 + 亮一档底色;hover 亮底 |
| 分组 | tab 按 session 分组,组可折叠(chevron),组内缩进 12px |
| 排序 | 行可拖拽重排,可拖出/拖入窗口 |
| 溢出 | 垂直滚动,4px 细滚动条 |
| 顶栏 | 开垂直 tabs 后顶部 tab 条隐藏(hover 顶缘 12px 可唤出);顶栏高 34px |
| 交通灯 | macOS 内联占 64px 固定宽 |
| 面板底色 | `fg_overlay_1`(前景按 ~15% 混入背景),全部主题驱动,无写死色 |

**为什么左侧比顶部好**:tab 多了以后顶栏挤成省略号,左列天然可滚动、
能放两行信息(标题+正在跑什么),宽度可调;对"一窗多 agent 各开一摊"
的用法,左列就是会话总览。

---

## 3. Unterm 现状盘点(地基比想象的好)

- `fancy_tab_bar.rs` 已是 **box-model 元素系统**(Element 树、padding/border/
  corners、hover 态、`UIItemType` 点击路由)——Warp 那套行渲染我们的元素系统
  都表达得出来,**不需要新 UI 框架**。
- `INTEGRATED_BUTTONS` 全套已就位(交通灯入 tab 行,
  `window/src/os/macos/window.rs`),`docs/design-unified-topbar.md` 的
  Part A/B(顶栏同色 + 窗内动作区)与本研究完全兼容,左 tab 落地后顶栏
  正好只剩 交通灯+标题+右侧动作区。
- 自动标题已有(多实例设计时做的 auto-title),左列副标题直接复用。
- 底部状态栏(proxy/mcp/theme/profile)保持不动,与左列不冲突。

---

## 4. 改进设计(完整交付,无 MVP 切片)

### A. 字体精致化 — 三刀

1. **UI 字体换系统字体**:`compute_title_font` 里 macOS 用
   `.AppleSystemUIFont`(CoreText locator 特判;直接按 family 名查不到,
   需要走 `CTFontCreateUIFontForLanguage` 或别名),Windows 维持
   Segoe UI Variable,Linux 改 `Noto Sans` → 回退 `DejaVu Sans`。
   Roboto 降为最后回退(免得 Linux 无 Noto 时开天窗)。
2. **macOS 渲染默认改灰度**:`default_freetype_load_target()` macOS 分支
   HorizontalLcd → Normal;同时实测 stem-darkening / gamma
   (`freetype_load_flags`、合成 blending)找到最接近 CoreText 的组合。
   **验收:同字号同字符串,Unterm/Warp/原生 TextEdit 三方放大截图并排比。**
3. **`ui_tokens` 模块**:`UI_FONT_SIZE=12`、`LINE_HEIGHT=1.2`、`RADIUS=4`、
   `ROW_PADDING=8`、`OVERLINE=10`/`PALETTE=14`/`HEADER=18`,
   fancy_tab_bar/状态栏/今后所有弹层统一引用,杜绝再长 magic number。

工量 ≈ 1.5–2 天(含三方对比验收 + Win/Linux 回归截图)。

### B. 左侧垂直 Tab 栏

**范围减法(守住"薄层"定位):只做垂直 tab 列表。**
toolbelt 图标轨、文件树、Drive、Agent 会话面板 —— 全部 OUT。
一个配置:`tab_bar_position = "top" | "left"`。

- 布局照 Warp 实测参数:248 默认 / 200 min / 50% max,右缘拖拽;
  行 = 24px 图标 + 标题(12pt)+ 副标题(12pt 灰),8px padding,4px 圆角,
  选中 1px 边框 + 亮底,hover 亮底;垂直滚动 + 4px 滚动条;拖拽重排;
  hover 出 ✕;双击改名(改名已有 action)。
- **副标题 = 我们的差异化**(2026-06-12 已拍板):格式固定
  **`Agent名 · 项目目录最后一级`**,如 `claude · unterm`、`codex · notebook`;
  无 agent 占用时只显目录最后一级。数据源:MCP 侧 agent_identify/会话归属 +
  session cwd。Warp 没有这个;会话总览 = agent 总览,
  这是"AI agent 能开的终端"该长出来的样子。
- 折叠:⌘B(可配)显/隐整列;不做 48px 图标轨(减法,真要等用户喊)。
- 顶栏联动:left 模式下顶部 tab 条不渲染,顶栏只剩
  交通灯 + 窗口标题 + 右侧动作区(与 design-unified-topbar.md Part B 合并)。
- 实现路径:新 `left_tab_bar.rs`,完全复用 Element/box-model/UIItemType;
  termwindow 内容区原点 x 偏移 sidebar 宽;鼠标命中 + resize 拖拽热区;
  Win/Linux 同步生效(渲染层共用)。
- 默认值:**默认仍 top**,左列做成一键切换;等自己 dogfood 两周再定默认。
  ("现在没人用"=可以激进,但默认翻转应该由用着爽不爽决定,不是由研究决定。)

工量 ≈ 4–6 天(渲染 + 交互 + 拖拽重排 + 跨平台实测 + MCP 自测脚本)。

### C. 显式不做(OUT)

toolbelt 多面板、Warp Drive 类云对象、blocks/分离输入框、
按 session 分组折叠(我们 tab 模型没有"组",硬造概念违反减法)。

---

## 5. 发版建议

- 研究期间不发版(2026-06-12 用户指示:研究透彻再做新版本)。
- A + B 合并为一个"精致化"版本(v0.44 候选),做完整体自测后再提发版。
- v0.43 现状:GitHub release 已有 Win/Linux 8 个产物;macOS 构建+签名已完成,
  公证上传超时未传(产物在仓库根:`Unterm-macos-v0.43/`、`*.notary.zip`),
  按指示暂停,要补随时可续(notarytool submit 重试即可)。

## 6. 待拍板

1. ~~左列副标题信息密度~~ → **已拍板(2026-06-12):`Agent名 · 项目目录最后一级`**,
   如 `claude · unterm`;无 agent 时只显目录。
2. ⌘B 这个键位(和未来命令面板的键位表一起定)。
3. A 先行单独自测合入 master,还是 A+B 一起合?(都不发版,只谈合码)

## 附:素材索引

- Warp 源码:`/tmp/warp-src`(注意 AGPL,只看不抄;`crates/warpui*`/
  `ui_components` 为 MIT)
- 字体/渲染:`crates/warpui/src/fonts/font_kit.rs`、
  `crates/warpui_core/src/{fonts.rs,text_layout.rs,rendering/mod.rs}`、
  `app/src/{appearance.rs,settings/font.rs,font_fallback.rs}`
- 侧栏:`app/src/workspace/view/{left_panel.rs,vertical_tabs.rs}`、
  `app/src/workspace/view.rs`(tab bar 常量 523-545)
- Unterm 对应:`wezterm-font/src/lib.rs:559-614`(title font)、
  `config/src/config.rs:2220`(load target)、
  `wezterm-gui/src/termwindow/render/fancy_tab_bar.rs`
