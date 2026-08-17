# M5 Provider Registry 与 Unzoo Binding — 验收与决策记录

2026-08-17。对应计划第 24–28 条。

## 门禁四条

| 门禁 | 证据 | 结果 |
|---|---|---|
| 不依赖固定端口 | `unterm-providers/src/discovery.rs::this_crate_never_writes_down_a_port` 扫自己的非注释源码；真机验证读 Unzoo 的 `config/rest-port` 得到 9399 | 通过 |
| 离线进入 waiting_provider | `contract.rs::a_closed_browser_is_something_to_wait_for`；`registry::Acquire::Waiting{reason: "waiting_provider"}` | 通过 |
| 取消传播 | `contract.rs::cancelling_reaches_the_provider` — 真起线程发在途调用，`cancel_task` 让 provider 侧返回 `Cancelled`，记录落 cancelled，租约一并撤销 | 通过 |
| 动作可反查完整授权链 | 真机跑通：agent 要 profile → 审批 → 用户在设置页答"本任务允许" → 再要拿到租约 → 真调 `profile_list` → `provider chain` 打出 lease → grant(task) → approval(the user, in settings) → 该租约下的调用与响应哈希 | 通过 |

## 真机端到端（本机 Unzoo Browser 2.5.16）

```
discovered at Http { url: "http://127.0.0.1:9399/mcp" } via unzoo:rest-port
bound to unzoo-service 2.5.16 over 2024-11-05
ok   handshake    unzoo-service 2.5.16 over 2024-11-05
ok   lease        issued lse_452a…
ok   evidence     request 44136fa3… response 9048c1d7…
ok   idempotency  the second call returned the first one's record
ok   replay       a repeated sequence number was refused
ok   record       2 calls recorded under the lease
```

三条路都验过：`cargo test --test unzoo_live -- --ignored`（库）、`unterm-cli provider …`（CLI 打到 headless Core）、设置页 Providers 面板（浏览器里点按钮，由 Unzoo 自己驱动）。

## 锁定的决策

**端点是读来的，不是写死的。** 每个 `Endpoint` 都带着自己需要的东西，没有默认端口。这不只是为了"换端口能找到"——更是为了一个**没在跑**的 provider 不会被误认成恰好占了它旧端口的另一个进程。发现顺序：环境变量 > Unterm 的描述符目录 > provider 自己的广告文件；先环境变量，因为操作员改了东西自有其道理，一个悄悄推翻他的发现机制是他没法调试的。

**身份是钉住的（TOFU）。** loopback 只能证明"这台机器上有个进程回话了",不能证明"是我绑的那个"。首次握手记下 name+version,之后每次比对；变了就 `Degraded`,要人来处理。这条要说实话:**它能发现变更,不能发现一开始就是错的**。

**租约带 epoch 和 seq。** epoch 每次续租 +1 —— 谁续了谁才持有当前租约,录下旧的那份不再是同一份。seq 必须严格递增,重放在**执行之前**就被拒 —— 事后发现的重放,已经把它要重放的事做完了。整个检查和记录在同一个事务里,两个并发的使用不会都读到旧值都放行。

**幂等落库,不落内存。** 进程内的备忘录恰恰在进程崩在半路时忘记 —— 而那正是调用方要重试的时刻。`provider_calls` 表 UNIQUE 在幂等键上,重复的调用返回第一次的记录,`replayed_from_record: true` 让审计线看得见。

**证据是哈希,不是载荷。** 请求和响应各存一个 sha256(canonical JSON,键排序),16KB 以内的响应才留原文。一次 provider 调用可能带着别人的一页邮件,数据库不是它该待的地方。

**能力分三家:browser / profile / computer。** 粗粒度是故意的 —— 用户在回答的是"要不要让 agent 开我的浏览器",不是四百个方法。而**开浏览器不等于读我的登录态**:browser 租约打 `cookie_get_all` 会被拒(真机验过)。前缀没被声明过的工具映射到 nothing,任何租约都盖不住它。

**三种能力三个问题,风险不同。** `capability.browser` 算 mutation(用户看得见在发生);`capability.profile` 和 `capability.computer` 算 destructive —— 读过的读了、敲过的敲了,事后收回也回不去。所以后两者必须有人点头。

**审批必须有人能答,而那个人不能是 agent。** M3 起网关就会问,但在此之前**没有任何地方能回答** —— destructive 的请求就那么挂着直到过期,那是"延迟五分钟的拒绝"披了审批的皮。现在:`approval.list` 谁都能读(自己的问题在排队,agent 有权知道,能看见队列不等于能清空它),`approval.decide` **拒绝一切来自网络的调用**,只有进程内(设置页)能答。

这条边界要说清楚:**它不是安全边界**。有 shell 的 agent 能读实例 token、能跑 CLI,能干的事和坐在这台机器前的人一样多。它是**审议边界** —— 让请求权限的 agent 必须停下、让人注意到。能自己回答自己问题的 agent 永远不会停。真机验过:已认证的 agent 走 TCP 调 `approval.decide` 拿到的是拒绝,同一时刻设置页点按钮就能答。

## 新增的面

- MCP:`provider.list/bind/pause/resume/unbind/diagnose/leases/acquire/call/revoke_lease/chain` + `approval.list/decide`
- CLI:`unterm-cli provider list|bind|pause|resume|unbind|diagnose|leases|acquire|call|approvals|revoke|chain`
- 设置页:Providers 面板 —— 发现/绑定/暂停/诊断/解绑、活跃租约与撤销、"Waiting for you" 审批卡(允许一次 / 本任务允许 / 始终允许 / 拒绝)
- 迁移 v3(`capability_leases`)与 v4(`provider_calls`)

新加了一条合约测试:**当前发布的每个方法都必须有风险分类**(不只是冻结的 103 个)。做过变异验证 —— 拿掉 `provider.list` 的分类,它红。原来的测试只查冻结表,新方法一上线就不在覆盖范围里。

## 还没做的

- **诊断只跑只读那半套。** 关掉别人的浏览器看会发生什么,不是一个诊断按钮该干的事;`offline_is_a_wait` 留给能被关掉的 provider(fake)。
- **`provider.call` 还不是强制路由。** agent 仍可以自己去连 Unzoo 的 MCP 或 CDP shim 绕开租约与审计 —— 堵死这条路是 M6-05(强制意图路由)。
- **第三方 provider 靠描述符**,`families` 表必须自己声明;没声明的工具任何租约都盖不住。目前只有 Unzoo 有内置表。
- 审批的 TTL 是 300 秒,过期即失效;桌面通知/托盘提醒留给 M7 Supervisor。
