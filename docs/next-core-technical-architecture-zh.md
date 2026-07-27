# next-core 技术架构方案

Status: working architecture draft  
Last updated: 2026-07-27  
Scope: 用自研 next-core 逐步取代 WezTerm 内核

## 1. 结论

next-core 可行，但不能做成“重写一个更大的 WezTerm”。

正确方案是：

- 自研：调度模型、pane/session 模型、屏幕/scrollback 数据结构、dirty snapshot、输入/粘贴管线、MCP/产品边界、性能基准。
- 复用：PTY、VT parser、Unicode width、字体 shaping/raster、GPU 抽象、平台窗口基础能力。
- 暂缓：Lua 兼容、SSH/mux、图片协议、插件运行时、完整 copy mode、复杂字体特性。

目标不是“纯自研”，而是把 Unterm 最影响体验的路径握在自己手里：输入、输出、滚动、粘贴、agent 大量输出、MCP 读取、窗口/实例稳定性。

## 2. 借鉴的开源技术方向

| 项目/技术 | 吸收点 | 不照搬的点 |
|---|---|---|
| Ghostty | 核心库和 UI 分层、多线程读写渲染分离、终端核心可被不同 UI 消费 | 不引入完整 Ghostty 栈，不把渲染/字体/平台能力重新耦合进产品层 |
| Alacritty / vte | parser/perform 分离，VT parser 只负责状态机，语义由 terminal core 实现 | 不直接继承 Alacritty 的产品形态和配置模型 |
| Rio | WebGPU/wgpu 路线、渲染器只消费终端快照、面向高帧率输出 | 不把视觉特效和插件体系放进 alpha core |
| COSMIC Text / swash / fontdb | 字体发现、fallback、shaping、emoji/CJK 的成熟处理 | alpha 不追求所有高级排版特性，先保证宽度、fallback、性能 |
| Windows Terminal / ConPTY 生态 | Windows shell 兼容、PTY 边界、IME/剪贴板/窗口生命周期经验 | 不采用大型 XAML/平台 UI 架构 |

## 3. 模块边界

```text
unterm-core
  pty
  vt parser adapter
  terminal semantics
  screen grid
  scrollback ring
  dirty tracking
  input/paste translator
  render snapshot API

unterm-render
  font discovery/cache
  shaping/glyph atlas
  GPU command generation
  frame pacing
  headless capture renderer

unterm-app
  native windows
  tabs/splits
  selection/copy/paste UI
  IME
  keybinding dispatch
  profile/settings/window lifecycle

unterm-product
  MCP
  CLI
  Agent Cockpit
  Fleet/Review/Recording/Profile/Proxy/Workspace
```

硬约束：`unterm-core` 不知道窗口、菜单、Agent Cockpit、Fleet、Review、Web Settings；产品层不能直接改屏幕网格，只能通过 engine traits。

## 4. 推荐技术栈

| 层 | 首选方案 | 原因 |
|---|---|---|
| 语言 | Rust | 当前代码基础一致，性能和内存安全适合终端内核 |
| Windows PTY | 先沿用 `portable-pty` / ConPTY 路径 | 已在项目中存在，先用 benchmark 证明瓶颈再替换 |
| Unix PTY | 窄平台 wrapper 或成熟 crate | alpha 后补齐，避免 Windows spike 被跨平台拖慢 |
| VT parser | `vte` crate 风格的 parser/perform 边界 | 避免手写完整状态机，保持 chunk split-safe |
| 屏幕模型 | 自研 sparse/row-major grid + scrollback ring | 这是性能、MCP、capture、dirty diff 的核心控制点 |
| 渲染 | `wgpu` 优先 | 跨平台 GPU 抽象，适合独立 renderer 消费 dirty snapshots |
| 字体 | `cosmic-text` / `swash` / `fontdb` 评估后选择 | 避免自研 shaping/fallback，控制 CJK/emoji 风险 |
| UI 窗口 | 先选最小可控 event loop | IME、剪贴板、输入延迟、窗口生命周期比 UI 框架华丽度更重要 |

## 5. 关键实现路径

### 5.1 输入路径

目标：按键进入 PTY writer 不经过 UI 重计算、agent 扫描、磁盘读取。

```text
OS key event
  -> app keybinding dispatch
  -> input translator
  -> bounded writer queue
  -> PTY writer thread
```

要求：

- key event 只做内存操作。
- 补全接受、右箭头、End、Tab 等都走同一套输入状态机。
- 粘贴按 UTF-8 边界分块，bracketed paste marker 保留。
- writer queue 有背压和遥测，不能卡住 UI thread。

### 5.2 输出路径

目标：Codex/Claude 启动或大量输出时，解析和渲染不抢 UI 响应。

```text
PTY read thread
  -> byte chunks
  -> parser state
  -> screen mutations
  -> dirty rows/cells
  -> render snapshot channel
  -> renderer frame
```

要求：

- parser state 必须 split-safe，不能假设一个 read 包含完整 escape sequence。
- DSR/DA/DECRQM 等 query response 保持输入顺序写回 PTY。
- output flood 合并 dirty rows，限制 render wakeup 频率。
- MCP screen read 读取稳定快照，不等待下一帧渲染。

### 5.3 滚动路径

目标：PageUp/PageDown 和滚轮只改 logical viewport，不扫描全量 scrollback，不触发 agent 状态刷新。

```text
scroll input
  -> viewport offset
  -> visible range snapshot
  -> renderer dirty viewport
```

要求：

- scrollback ring 裁剪时保持 viewport 稳定。
- live-tail 只在用户回到底部后恢复。
- 搜索命中跳转和 MCP `screen.scroll(goto)` 使用同一 viewport 模型。

### 5.4 渲染路径

目标：renderer 是消费者，不拥有终端语义。

```text
render frame snapshot
  -> visible styled cells
  -> shape/cache glyph runs
  -> atlas update
  -> GPU draw
```

要求：

- full-frame fallback 用于首帧/resize。
- 常规帧只传 dirty rows/cells。
- 主题、粗体/斜体、反色、下划线、超链接样式在 snapshot 层表达。
- headless renderer 复用同一 styled snapshot，用于 scrollback PNG 和 CI。

## 6. 为什么不会比 WezTerm 更大

Size budget 用功能边界控制，不靠口头保证。

Alpha 禁止进入 core 的内容：

- Lua 配置兼容层
- SSH/mux server clone
- 图片协议
- 复杂插件运行时
- Web Settings
- Agent Cockpit UI
- Fleet/Review UI
- 外部窗口长截图
- 全量 legacy keybinding 兼容

Alpha 必须进入 core 的内容：

- PTY lifecycle
- VT parser adapter
- screen/scrollback
- input/paste
- dirty snapshot
- 基础样式/颜色
- 查询响应
- renderer contract

衡量方式：

- `unterm-engine/verify-next-core-size-budget.ps1` 作为可执行 size gate，默认检查 next-core 源码行数、probe/benchmark 二进制源码行数、直接依赖数量和 debug 二进制大小。
- `cargo bloat` / binary size 每个 milestone 记录。
- `cargo tree -e features` 每个 milestone 记录。
- 每个新增依赖必须写清楚“替代了哪段自研复杂度”。
- 任何功能只要不影响终端正确性、延迟或 renderer contract，就不能放进 core。

## 7. 近期落地顺序

1. 继续扩 next-core VT 兼容测试，覆盖常见 shell/TUI 真实序列。
2. 继续收紧 `TerminalParser` 边界，后续可替换为 `vte`。
3. 继续把 screen model 拆成小模块，优先收紧 live viewport、selection 边界。
4. 继续收紧 benchmark：paste、output flood、PageUp/PageDown、MCP screen read、真实 GUI key-to-paint。
5. 做 wgpu renderer spike，只消费已有 render-frame snapshot。
6. 做字体方案 spike：ASCII/CJK/emoji 宽度、fallback、缓存开销。
7. 对比 current-core 与 next-core，达不到指标就不扩大范围。

## 8. 当前代码进展

next-core 已经在 screen/parser 方向具备基础能力，包括：

- split-safe CSI/OSC/DCS/APC/PM/SOS control string 消费
- 基础 VT 光标、滚动区、模式报告、DA/DSR/DECRQM query response
- SGR 样式、扩展色、OSC 8 hyperlink
- scrollback ring、logical viewport、styled render-frame full/delta snapshot
- render-frame dirty rows 跨 PTY chunk 累计；如果请求 revision 早于当前 dirty baseline，则回退 full frame，避免未来 GUI renderer 漏 repaint
- render-frame full snapshot 稳定返回完整 `rows x cols` viewport grid，dirty snapshot 稳定返回 dirty range 内每一行的 `cols` 个 cell；缺失内容以 styled blank cell 表达，避免 renderer 自行推断
- render-frame cursor-only movement 也会返回 dirty row 和新 cursor snapshot，保证 renderer 能重画旧/新光标位置
- `ScreenEngine::read_render_draw_plan` 将 styled cell grid 合并成 glyph runs、cell style runs 和 cursor draw state，作为未来 wgpu renderer 的轻量 CPU 输入层；dirty frame 生成的 draw run row 必须保留 viewport row，避免局部重绘写错屏幕行
- `RenderDrawPlan::to_geometry_plan` 用显式 cell metrics 将 glyph/cell/cursor runs 映射为像素矩形，作为真正接入 wgpu 前的轻量布局契约，暂不引入 GPU/font shaping 依赖
- `RenderGeometryPlan::to_submission_plan` 将像素几何转换成 damage rects、background quads、text runs 和 cursor quad，让未来 wgpu renderer 只消费提交计划，不反向拥有终端语义
- `RenderConsumerState::prepare_commit` 记录 renderer 已提交 revision，首帧/resize 强制 full repaint，重复 revision 跳过，revision gap 可观测，避免真实 renderer 自己推断增量协议
- `ScreenEngine::read_render_commit_plan` 将 frame/draw/geometry/submission/commit 链收敛为 engine 级读取接口，GUI renderer 只需要持有 consumer state 和 cell metrics
- `wezterm-gui/src/engine/mod.rs` 的 `CurrentTerminalEngine` 已 re-export render contract 类型，并显式转发 `read_render_frame`、`read_render_draw_plan`、`read_render_commit_plan`，未来真实 GUI renderer 可以经 engine-neutral facade 消费 next-core commit plan
- `wezterm-gui/src/engine/render_consumer.rs` 已提供 `EngineRenderConsumer`，把 pane id、cell metrics、submitted revision state 和 commit batch 读取封装为 renderer-side 对象；真实 wgpu backend 后续只替换提交到 GPU 的部分
- `EngineRenderConsumer::read_buffer_plan` 已把 engine-neutral commit batch、command-list backend 和 `EngineRenderBufferPlan` 串成单个 renderer-side frame preparation 调用；未来 pane draw branch 不需要自己组装 commit/backend/buffer 三段流程
- `EngineRenderConsumerSet` 已按 pane id 缓存 renderer consumer，并在 cell metrics 变化时更新 consumer 而不丢 submitted revision；真实 WebGPU pane draw 分支后续可以跨 paint 复用增量状态，只在 viewport metrics 变化时强制 full repaint
- `TermWindow` 已持有持久化 next-core render consumer cache，并在直接 pane/window 清理路径同步移除对应状态；后续真实 WebGPU pane 分支可以从窗口长期状态读取增量 renderer consumer，而不是在 draw loop 局部重建
- `TermWindow::prepare_next_core_render_buffer_plan` 已把当前 engine、pane id、当前 cell metrics 和持久 renderer consumer cache 收敛为单个 frame preparation 入口；下一步 WebGPU pane draw branch 只需要拿该 buffer plan 交给 `WebGpuState::encode_next_core_buffer_plan`
- WebGPU draw loop 已提供 `UNTERM_NEXT_CORE_WEBGPU_PANE` 实验分支：默认不启用；`1/true/on/append` 在 legacy pass 后追加 next-core buffer plan pass；`replace` 会跳过 legacy pane quad ranges，由 next-core 绘制 pane，同时保留 legacy chrome/UI，用于验证 pane-only 替代路径
- `wezterm-gui/src/engine/render_backend.rs` 已提供 GPU-free `CommandListRenderBackend`，将 damage/background/text/cursor submission 展开为稳定顺序的 backend command list，为后续 wgpu command encoder 接入固定输入契约
- `EngineRenderBufferPlan` 已将 backend command 转为 damage rects、quad vertices 和 indices，并保留原始 `RenderTextRun` 的 row/col/cell-span/text/style/rect 元数据；下一步 glyph atlas 可以消费真实文本与单元格跨度信息，而不是只能看到匿名纯色 text quad
- `EngineRenderTextAtlasPlan` 已把 submitted text runs 准备成 GPU-free atlas/shaping 输入，保留 foreground color、cell span、text、style 和 pixel rects；真实字体 atlas 后续只需要替换该 preparation 层的消费端
- `EngineRenderShapedGlyphPlan` 已固定真实 GUI shaper 的下一层输入 ABI，并可从 `wezterm_font::GlyphInfo` runs 构建；shaped glyph 可以携带 text、rect、style、foreground、cells、`font_idx` 和 `glyph_pos`，再进入共享 atlas/cache/upload 路径
- `EngineRenderGlyphAtlasPlan` 已把 text-atlas runs 转成稳定 glyph cache key 和 cell-aligned glyph instance；glyph key 已能携带可选 shaped `(font_idx, glyph_pos)` raster identity；`EngineRenderFontGlyphRasterSource` 已把迁移期 `LoadedFont::rasterize_glyph` 桥接隔离在 next-core raster-source trait 后面，避免新渲染管线直接依赖旧 `GlyphCache`
- GUI WebGPU 的 next-core pane render path 已能用默认 `LoadedFont` 对 text-atlas runs 做 shaping，并通过 `EngineRenderFontGlyphRasterSource` 上传真实 raster bytes；字体查找或 shaping 失败时仍回退到 deterministic placeholder raster path
- GUI WebGPU 的 next-core shaped glyph atlas preparation 已按 pane / revision / font id / text-atlas fingerprint 缓存，pane 内容未变化的 repaint 不再重复执行 `LoadedFont::shape`
- GUI glyph texture update 已携带 raster source 的原始 bitmap 尺寸和 bearing metrics；下一步可以从 cell-aligned quad 逐步迁移到真实 glyph bearing/advance placement
- GUI textured glyph upload 已把 raster metrics 持久化进 pane glyph atlas cache，并使用 source bitmap 尺寸与 bearing metrics 生成真实 glyph quad 和 UV；没有真实 raster metrics 的路径继续保持 deterministic cell-aligned fallback
- `EngineRenderGlyphAtlasCache` 已提供确定性的 shelf placement，记录已插入和 overflow 的 glyph key；未来 WebGPU glyph texture 可以按 cache update 更新 atlas 区域，而不是每帧重建 placement state
- `WebGpuState` 已持有按 pane 划分的 next-core glyph atlas state，并在 pane render consumer 清理时同步释放；glyph placement 复用现在具备跨 paint 生命周期，不再停留在单帧局部计划
- `EngineRenderGlyphAtlasTextureUpdatePlan` 已通过 `EngineRenderGlyphRasterSource` 边界把新插入的 glyph key 转成 texture update region；默认 deterministic source 保持测试稳定，后续 GUI font raster/cache 可以提供真实 RGBA bytes，而不改变 `queue.write_texture` 上传契约
- `NextCoreGlyphTexture` 已持有独立 WebGPU glyph texture atlas，并对 next-core glyph texture region 做尺寸/bytes 校验后用 `queue.write_texture` 上传
- `EngineWgpuRenderBackend` 已持有 textured glyph pipeline/pass ABI，`WebGpuState` 会把 next-core glyph atlas texture 绑定到 sampler 后，在 solid next-core pass 之后追加 textured glyph pass
- `EngineRenderTexturedGlyphUploadPlan` 已把 glyph atlas placement 转成带 clip-space position 和 atlas UV 的 textured glyph vertices；真实 font raster/cache 接入前，texture draw ABI 已先固定
- `EngineWgpuRenderBackend::prepare_frame_for_viewport` 已把 clip-space upload buffers、text-atlas input 与 glyph-atlas instances 合并为同一帧 preparation；WebGPU pane encoder 现在会先生成 combined frame plan 再绘制
- `EngineWgpuRenderBackend` 已提供最小 wgpu upload skeleton：把 buffer plan 转成 POD GPU vertex ABI，并创建 vertex/index buffers；该层复用 GUI 现有 `wgpu`，不把 GPU 依赖塞进 `unterm-engine`
- `EngineWgpuRenderPassPlan` 已固定最小 indexed draw-pass 契约，`EngineWgpuRenderBackend::encode_pass` 可以把已上传 buffer 写入真实 `wgpu::CommandEncoder`，重复 revision/空帧不会产生 draw
- `EngineWgpuPipelineConfig`、next-core GPU vertex layout 和最小 WGSL shader 已固定 solid-color quad pipeline ABI；背景/文本/光标顶点携带 RGBA，窗口接入时通过 viewport 尺寸转换为 clip-space，字体 atlas 仍留给后续独立步骤
- `WebGpuState` 已在设备初始化时缓存 next-core solid-quad backend/pipeline，与现有 legacy pipeline 并存；后续 pane 绘制只需提交 commit buffers，不需要每帧创建 shader/pipeline
- `WebGpuState::encode_next_core_upload` 已把 next-core GPU upload plan、缓存 pipeline 和 `wgpu::CommandEncoder` 串成一个 GUI 侧调用点；当前 legacy draw loop 仍保持不变
- `WebGpuState::encode_next_core_buffer_plan` 已把 render buffer plan、当前 viewport 尺寸、viewport-to-clip 转换和缓存 pipeline 串成更高层 GUI 入口；下一步接具体 pane 分支时不需要在 draw loop 里散落 upload/pass 细节
- JSON probe smoke 已输出并校验 render draw/geometry/submission/commit plan 的 revision、run/quad counts、viewport、damage rects、cursor state 和首帧 full-repaint state，让 renderer 输入契约进入 CI 可见面
- benchmark 已覆盖 input write、key-to-screen、input burst under output、echo、paste、output flood、scrollback paging、viewport scroll、screen-read under flood、render-frame empty/dirty/cursor-move delta、render draw plan、render geometry plan、render submission plan、render commit plan API、focus/session lifecycle
- zero-width combining marks attach to preceding visible cells without advancing cursor position
- DECFRA、DECERA、DECCARA、DECRARA 矩形操作
- DECSCA protected/erasable cell 属性
- DECSED/DECSEL selective display/line erase
- DECSERA selective rectangular erase
- UTF-8 安全 paste chunk 和 bracketed paste marker 保留
- `next_core/cell.rs` 已拆出 cell/attribute/style 转换边界，避免 screen、parser、renderer 继续堆在单个巨型文件里
- `next_core/history.rs` 已拆出 scrollback ring 和 viewport pin 状态，降低翻页、截图、renderer 对 screen 内部字段的耦合
- `next_core/render_state.rs` 已拆出 revision 和 dirty range 状态，为后续 GPU renderer 的增量帧消费提供稳定边界
- `next_core/screen_state.rs` 已拆出 alternate-screen snapshot 和 mouse/mode tracking 状态，为后续 screen model 独立模块化铺路
- `TerminalParser` 已成为 screen 的显式 parser 边界，`next_core/parser_state.rs` 存放 parser 状态枚举，为后续替换成 `vte` parser/perform 边界降低耦合

## 9. 开源参考

- Ghostty: core library / UI boundary, multi-threaded terminal architecture
- Alacritty `vte`: parser/perform separation
- Rio: WebGPU renderer direction
- COSMIC Text: shaping, font fallback, raster abstraction
