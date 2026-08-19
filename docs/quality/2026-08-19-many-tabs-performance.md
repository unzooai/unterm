# 几十个 tab / 多实例下的性能分析与调优

2026-08-19。起因:担心开几十个 tab 或多个实例时卡死。

## 结论先讲

**卡死的根源只有一处**,而且不在渲染、不在 IPC 带宽、不在多实例 —— 是 Agent Cockpit 的轮询:
它每 400ms 对**每一个 pane** 拉一次完整 styled 屏幕(4800 个带颜色和属性的单元格),
**然后只取出字符,把其余全部丢掉**。

40 个 tab 实测:

| 每次轮询 | 修复前 | 修复后 |
|---|---|---|
| `session.styled_screen` × 40(旧做法) | **8941 ms** | — |
| `session.pane_pulse` × 40(revision 门控) | — | **67 ms** |
| 占 400ms 预算 | **2235%** | **16.9%** |

**快 133 倍。** 主线程原本每 400ms 要干 8.9 秒的活 —— 永远追不上,于是窗口再也画不完一帧。

## 量法

不量猜的东西。压测走 **Core 自己的 IPC socket**(`feed_cockpit` 真正走的那条),
不走 MCP —— 那会量成另一件事。每项取 5 轮中位数,单次 IPC 调用的一个样本是噪声。

`scratchpad/perf/bench*.py`,可复现:起一个 headless Core,建 40 个 pane,对比三种读法。

## 各条轴的实测

### 1. Cockpit 轮询(问题所在)

```
before  styled_screen every pane : 8941.5 ms  (223.54 ms/pane)
after   pane_pulse, revision-gated:  67.4 ms  (  1.69 ms/pane)
```

对比中还量了另外两种可能的写法:

| 读法 | 40 pane 耗时 | 说明 |
|---|---|---|
| `styled_screen`(旧) | 3829–8941 ms | 随屏幕内容增长 —— pane 里输出越多越慢 |
| `visible_text` | 12.2 ms | 只有文本,拿不到 revision 与通知计数 |
| `styled_frame` | 54.3 ms | 有 revision 门,但变化时仍传整屏 |
| **`pane_pulse`(新)** | **67 ms** | 8 行文本 + revision + 计数;变化时也只传 8 行 |

`pane_pulse` 比 `visible_text` 略慢是因为它**多带了 cockpit 真正需要的两个计数**,
省掉了额外的 IPC 往返。

### 2. 最坏情况:所有 pane 同时输出

```
pulse, all idle              : 14.17 ms
pulse, every pane just wrote : 14.00 ms
```

**最坏情况和空闲一样便宜。** 因为传的是 8 行文本而不是 4800 个单元格,
revision 门失效时代价也没有塌方 —— 这是这个设计比单纯加缓存更重要的性质。

### 3. `session.list`

1.68 ms(空闲)/ 1.72 ms(满载)。每次轮询一次,不是瓶颈。

### 4. 多实例

`list_live_instances` 扫描:

```
 1 instances: 0.04 ms
 8 instances: 0.18 ms
32 instances: 0.65 ms
```

线性且极廉价,**而且早已在主线程之外跑**(`cockpit-peer-snapshot` 线程 + 去重旗标)。
这根轴当初就处理对了,不需要动。

### 5. 内存

40 个 pane、每个约 200 行输出:Core RSS **12.7 MB**,合 **0.32 MB/pane**。
不是问题。

## 修法与两个设计决定

新增 `PanePulse` 与 `session.pane_pulse`:

**一、传调用方真正要的东西。** Cockpit 要三样:最后 8 行文本、revision、通知计数。
以前为了拿这三样传了整屏带属性单元格,**313 倍的白付**。

**二、revision 门控,并且计数照送。** 没人敲过的 pane 只回一个空信封。
但**计数仍然回** —— 响铃不是屏幕变化,一个在屏幕静止时到达的通知,
如果被 revision 门吞掉,Cockpit 就永远看不到它。这条有专门的测试。

trait 方法给了**默认实现**(从整屏快照构造),因为本地引擎的单元格本来就在内存里,
省不出什么;只有**要跨 socket 序列化**的那个引擎重写它。一个必须每个引擎各实现一遍的
优化,是迟早会有引擎忘记实现的优化。

## 守卫

4 项回归测试钉住契约,并做了**变异验证**:把 revision 门改成永不命中,
`a_pane_nobody_typed_in_sends_no_tail_at_all` 立刻红。
一个不会失败的守卫比没有守卫更糟。

## 没做的

- **GUI 真实帧率**仍需真机目视 —— 这次量的是「主线程每次轮询要付多少毫秒」,
  那是卡死的直接成因,但不等同于帧率数字。
- **release 构建的数字**:本次为与 M1 基线同口径用 debug 构建;release 只会更快。
- 100+ pane 未测。40 是「几十个 tab」的上界,已经从 2235% 降到 17%,
  按这个斜率 100 个 pane 约 42%,仍在预算内。
