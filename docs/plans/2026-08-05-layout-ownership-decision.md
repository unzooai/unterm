# 布局归属决策：分屏排布如何跨 GUI 重启存活

状态：**已决策 —— 采用 B（2026-08-05）**。tab 顺序按 pane id 升序重建、
zoom 不恢复，两项按第 3 节所述处理。C 记为终态方向，留到 M2 的 Store
落地之后；A 不做。  
背景：M1-04c 已让重开 GUI 领养 Core 中的既有会话，但**排布不保真**——
分屏方向一律变成水平、比例一律回到 50%。

## 1. 现状事实（已核对代码）

| 事实 | 位置 |
|---|---|
| `TabRegistry`（tabs + 每 tab 一棵 `Layout` 树）住在 GUI 进程 | `unterm-app/src/window.rs:521` |
| `Layout` 的 `Node` 是私有枚举，携带 `axis` + `first_ratio` | `unterm-engine/src/next_core/layout.rs:45` |
| GUI 分屏走 `create_session` + `tabs.split(...)`，**引擎不知道这是一次分屏** | `window.rs:3895` 一带 |
| MCP 分屏走 `split_session`，引擎会记下 `split_from` | `scheduler.rs:82` |
| `SessionSnapshot` 有 `split_from`，**没有** axis / ratio | `unterm-engine/src/lib.rs:132` |
| 重开时 `sync_tabs` 按 `split_from` 重建，方向写死 Horizontal、比例写死 0.5 | `window.rs:6736` 一带 |
| `mcp_host::SPLITS` 是个进程内 static，专为把 MCP 的分屏方向递给 GUI 而存在 | `mcp_host.rs:29` |

最后一条值得单独说：那个 static 是为了弥合"两条分屏路径"而打的补丁，
**它在 Core 模式下本就是错的**——MCP 与 GUI 若不在同一进程，它递不到。

## 2. 三个方案

### A. 快照托管——Core 替 GUI 保管一个不透明布局 blob

GUI 序列化整个 `TabRegistry`，经 `core.set_layout` / `core.get_layout` 存取；
Core 不解释内容，只保管。

- **成本**：小。两个 IPC 方法 + 几个 `Serialize` derive + 存取时机。
- **得到**：方向、比例、tab 分组、tab 顺序、活动 pane、zoom —— 全部保真。
- **代价**：Core 里存进了一份**它不理解的前端私有状态**。协议边界被弄脏；
  多个客户端时"谁的布局算数"没有答案；blob 一改格式就要版本兼容。
  M2 的 Task Store 落地后，这里会变成第二套持久化。

### B. 引擎记账——分屏是引擎事实，快照带上方向与比例

GUI 的分屏改调引擎已有的 `split_session`；`SessionSnapshot` 增加
`split_axis` / `split_ratio`；`sync_tabs` 用它们重建；拖动分隔条时把新比例
回写引擎。

- **成本**：中。改 GUI 分屏主路径 + 引擎快照字段 + 一条新的比例回写路径。
- **得到**：方向、比例、pane 的分组与排布关系。
- **不覆盖**：两个独立 tab 之间的**先后顺序**、**zoom 状态**。
- **附带修掉**：GUI 与 MCP 从此走同一条分屏路径，`mcp_host::SPLITS`
  那个进程内 static 可以删除——Core 模式下它本就失效。

### C. 布局入 Core 建模——`TabRegistry` 整体搬进 Core

Core 持有 tabs 与 layout，GUI 退化为只读投影 + 发命令。

- **成本**：大。搬 `TabRegistry` + 全套布局 IPC + GUI 重写为投影消费者。
- **得到**：终态正确。多个 GUI 客户端天然看到同一排布。
- **代价**：现在做，等于在 M2 持久化底座落地**之前**重写一遍前端布局层；
  等 M2 有了 Store，这套持久化多半要再改一次。

## 3. 建议：**做 B，把 C 记为终态方向，A 不做**

理由，按分量排序：

1. **A 违反我们刚立的规则。** 核心维护规则写着"禁止绕过 Core 保存状态"，
   其精神是不让状态散落成两套真相。A 反过来做了另一件同样糟的事：把前端
   私有状态塞进 Core，让 Core 保管它无法校验、无法解释、无法迁移的东西。

2. **B 顺手修掉一个真实缺陷。** 现在 GUI 分屏与 MCP 分屏是**两条路**，
   `mcp_host::SPLITS` 是缝合它们的补丁，而这个补丁在 Core 模式下失效。
   走 B 之后只剩一条路，补丁删除，MCP 建的分屏在 GUI 里也就自然正确了。

3. **C 是对的，但时机不对。** 它的收益（多客户端一致）在 M1 还没有消费者——
   现在只有一个 GUI。而它的成本会与 M2 的 Store 撞车。留到 M2 之后做，
   那时布局可以直接落在 Store 上，只写一次。

**B 未覆盖的部分如实处理**：tab 顺序按 pane id 升序重建（确定且可预期）；
zoom 不恢复——zoom 本就是临时视图状态，一次重启把它复位是合理的，
而不是缺陷。若日后需要，这两项加两个 session 元数据字段即可，
不动架构。

## 4. B 的切片拆法（B-1~B-3 已完成，B-4 待真机）

1. **B-1**：`SessionSnapshot` 增 `split_axis` / `split_ratio`；引擎在
   `split_session` 时记录；Core IPC 自动带过去（快照已整体序列化）。
2. **B-2**：GUI 分屏路径从 `create_session` + `tabs.split` 改为
   `split_session`；删除 `mcp_host::SPLITS`。
3. **B-3**：`sync_tabs` 读新字段重建；拖动分隔条时回写比例。
   两半都已落地。回写这半需要先回答"这条分隔条属于谁"：比例记在
   session 上，而调整分隔条的 pane 可能是它的任一侧。规则是**一个 split
   属于被分出来的那个 pane**，即节点第二子树的首叶（`split` 恒把新 pane
   放第二位，其后嵌套不改变首叶）。`Layout` 的两个比例方法因此改为返回
   `SplitRatioChange`，GUI 据此调 `SessionEngine::set_split_ratio`，Core
   模式下经 `session.set_split_ratio` 落到持有会话的进程。
4. **B-4**：真机验收——分屏（含非 50% 比例、垂直方向）→ 杀 GUI → 重开，
   排布逐项比对。
   - **已做**：`right / 25%` 分屏 → 杀 GUI（Core 存活）→ 重开 → 一个 tab、
     右侧仍占 25%。这一轮**发现并修掉了一个真 bug**：首 pane 领养的是
     `is_active` 的会话，而分屏后聚焦的是新 pane，导致父 pane 无处可挂、
     分屏重开后裂成两个 tab。改为沿 `split_from` 上溯领养血缘根。
   - **未做**：键盘调分隔条（Shift+Alt+方向）→ 杀 GUI → 重开这一路。
     需要真实窗口接收按键，合成按键（SendKeys）没能送达。自动化覆盖到
     两头——布局层归属（`a_moved_divider_names_the_pane_its_split_belongs_to`）
     与「比例进了 Core 并留在快照里」（`unterm-core` 的
     `a_dragged_divider_reaches_the_core_and_stays_in_the_snapshot`）——
     中间那 5 行 GUI 回写调用未经真机验证。

## 6. 附带切片：left/up 分屏落错边（已修）

发现于 B-3 的读码：`resolve_split` 把「新 pane 在前」编码进比例
（Left/Up 返回 `share`，Right/Down 返回 `1-share`），但
`Layout::split_node` **恒**把新 pane 放第二位，无人做这个交换。于是
`direction=left, size_percent=30` 的结果是：新 pane 落在右边（方向反了）
且拿到 70%（比例镜像了）。50% 时两个错误互相掩盖，所以一直没被发现。

修法（这是「一位信息缺失」，不是算术错）：

- `resolve_split` 改为返回 `SplitPlan { axis, first_ratio, side }`，
  `side: SplitSide::First | Second` 即新 pane 落在哪一半。
- `Layout::split` 接受 `side`，据此决定 first/second 的落位。
- `SessionSnapshot` 增 `split_side`（`#[serde(default)]`，旧记录读成
  `None` → 退回 Second，与既有行为一致）；`sync_tabs` 重建时带上。
- **归属规则随之改写**：原来的「第二子树的首叶」在新 pane 可能落在任一
  半之后就会指向**源** pane。改为 `Node::Split` 直接持有
  `owner: Option<usize>`，分屏时记下；只有从矩形重建的树
  （`from_positions`，本就无此历史）才退回位置推断。

真机验收：`--direction left --size-percent 30` → 新 pane 在**左**、占
30%（修前在右、占 70%）→ 杀 GUI → 重开 → 仍在左、仍 30%、marker 还在。

## 5. 需要你拍板的

- 选 **B**（建议）、**A**（快而脏）、还是 **C**（一步到位但撞 M2）？
- 若选 B：tab 顺序与 zoom 的处理（按 pane id 排序 / zoom 复位）是否接受？
