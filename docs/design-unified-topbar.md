# 设计 & 工量 — 统一暗顶栏 + 窗内菜单(Warp 式 chrome)

## 目标
顶部一条**与内容同色的暗顶栏**,取代现在"系统菜单栏 + 灰原生标题栏 + 暗 tab 栏"三段割裂:
1. 无灰色原生标题栏;交通灯叠到顶栏左侧
2. tab 在左/中
3. 右侧一排**常用动作按钮 + 一个菜单**,高频操作不必走 macOS 原生菜单栏
4. 顶栏颜色随主题走,任何配色都不割裂

## 现状(已查证)
- 代码**已有 INTEGRATED_BUTTONS 全套**:`window/src/os/macos/window.rs` 设 `titlebarAppearsTransparent` + `NSFullSizeContentViewWindowMask`;`render/window_buttons.rs` 把交通灯画进 tab 行;`render/fancy_tab_bar.rs` 是 box-model 元素系统,已含 `TabBarItem::MenuButton` + `right_eles` 右侧区 + `UIItemType` 点击派发。
- 0.38 那张截图仍显灰原生标题栏 = window_frame 颜色没随主题套上 / 该实例没加载到配置,**不是机制缺失**。

## Part A — 一体暗顶栏(色彩统一)· 小
1. 确认 `INTEGRATED_BUTTONS|RESIZE` 实际生效(已在 unterm.lua,需实测 0.38 显灰的原因)。
2. **window_frame.titlebar_bg + tab_bar.background 随当前主题背景**(不再写死 #191919):在 Rust 配置解析处,color_scheme 解析后,若用户未显式覆盖 window_frame,就注入 `scheme.background`。顶栏永远 = 内容色。
3. 交通灯 auto 配色已按背景明暗自适应(`auto_button_color`),无需改。
- **工量 ≈ 0.5 天**(Rust 配置注入 + 实测)

## Part B — 窗内动作栏 + 菜单 · 中
在 `fancy_tab_bar.rs` 的 `right_eles` 加一排图标按钮 + 一个 ▾ 菜单,复用现有 box-model 元素 + `UIItemType` 点击路由 + 已有 action(`ActivateCommandPalette` 等):
- **动作按钮(建议 6 个)**:命令面板 ⌘ · 新建 tab · 分屏 · AI Agents · 主题切换 · 设置
- **▾ 菜单**:把原生菜单的常用项搬进窗内(New Window/Tab、Split、Copy/Paste、Find、Shell 选择、Settings、关于…)
- 窄窗口:超出宽度的按钮收进溢出菜单(`…`)
- 跨平台:tab bar 是共用渲染,win/linux 同步受益
- **工量 ≈ 2–3 天**(图标 + 菜单弹层 + 点击路由 + i18n 标签 + 自测)

## 需你拍板
1. **macOS 原生菜单栏**:系统强制存在、无法真删 → 保留,但常用项也放窗内(建议)。接受?
2. **动作栏按钮集**:上面 6 个够吗?增/减?
3. **平台范围**:只 macOS,还是 win/linux 一起出(tab bar 跨平台,顺带就有)?
4. **发版**:做完出 v0.39,还是攒着跟后续 chrome 细节一起发?

## 风险 / 减法守则
- INTEGRATED_BUTTONS 在个别 macOS 版本/全屏态有边角行为,需实测
- 动作栏只放**高频**项,别堆成 IDE 工具条 —— 守住"薄层/减法"定位
- 顶栏占宽,窄窗必须有溢出收纳

## 总工量 ≈ 3–4 天(Part A + Part B + 跨平台实测 + 自测)
