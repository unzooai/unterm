# M6 Workspace、Artifact、Audit 与路由 — 验收与决策记录

2026-08-17。对应计划第 29–33 条。

## 门禁三条

| 门禁 | 证据 | 结果 |
|---|---|---|
| Workspace 互相隔离 | `workspace_scope.rs::two_workspaces_cannot_see_each_other`(读写两个方向)、`a_workspace_that_appears_later_is_denied_too`、`an_archived_workspace_is_still_off_limits`、`a_symlink_out_of_a_workspace_does_not_leave_it` | 通过 |
| Artifact 可追溯 | 内容寻址 + `verify` 重算哈希;`evidence.rs::a_bundle_holds_the_whole_story_and_verifies`、改记录/换文件都会被发现 | 通过 |
| Unzoo 离线不回退到其他浏览器栈 | `routing.rs::an_offline_browser_makes_work_wait_and_closes_the_other_stacks` —— 一个测试里同时验:provider 离线→`waiting_provider`,四条替代路线(playwright / chromium --headless / selenium / 裸 CDP)全被拒 | 通过 |

## 锁定的决策

**隔离是集合的性质,不是单次检查的性质。** 拿一条路径去比一个根,永远证明不了"两个 workspace 互相看不见"。所以 scope 每次都从**全表**构造:自己的根可读可写,其它每一个根都进 deny —— 包括已归档的。归档只是"没人在那儿干活了",文件还是别人的。

**嵌套在创建时就拒绝,并且说清是哪一个。** 一个 workspace 套在另一个里面根本无法互相隔离(外层的 allow 和内层的 deny 描述同一批文件)。诚实的时机是创建那一刻,不是等到某条路径让人意外。

**大小写要探测,不能按平台猜。** APFS 可以是敏感也可以是不敏感,两种猜错都糟:猜敏感→`/work/SECRET` 溜过 `/work/secret` 的 deny(fail-open,正是要命的方向);猜不敏感→一个真正不同的目录被当成同一个。所以用 dev+ino 探真实卷,只读不写。Windows 按不敏感处理(NTFS 默认如此,WSL 的每目录敏感开关默认关)。

**`\\?\` 与 UNC 前缀要归一。** `canonicalize` 返回 `\\?\C:\work`,调用方写 `C:\work`,是同一个地方;而 `\\?\UNC\server\share` 归一成 `\\server\share` —— 丢掉前缀会把网络路径变成看起来像本地的路径。

**大文件不进 SQLite。** 一次任务的产出可能是录屏。装着视频的 SQLite 是没人能拷贝、备份、打开的 SQLite,而症状要几个月后才以"终端变慢"的形式出现。所以字节按 sha256 落 `<state>/artifacts/sha256/ab/…`,库里只留索引。

**同内容存一份,行分开记。** 两个任务下载到同一个文件共享一个 blob,但各留各的行 —— **出处不是内容**。删除按引用计数:删掉最后一行才收字节,否则一个任务的清理会悄悄掏空另一个任务的证据(有专门的测试)。

**先写 `.partial` 再 rename。** 读到一个以哈希命名的文件,必须能相信内容是完整的;崩溃留下的半截写入会是一个"名字在说谎"的 blob。

**配额够不到时要说出来。** 正在跑的任务的产物永远不删 —— 那不是保留策略,那是数据丢失。于是配额可能达不到,`Sweep` 里就有 `still_over` 和 `held_by_live_work`:一个用户能据此行动的限额,和一个悄悄不对的数字,区别就在这里。

**审计链只保证"改动看得见",不保证改不了。** 那是个文件,能读它的人就能编辑它。每条带 `seq`、前一条的哈希和自己的哈希 —— 改一行,它的哈希变了,后一行还记着本该是多少。**从改动处往后整个重写的人留下的链是能验证的**,唯一的防御是别处有副本,文档里写明了。链前的旧日志(没有哈希字段)**不算被篡改** —— 否则升级后第一次运行就会把用户自己的历史报成攻击证据。

**证据包是给不在场的人看的。** 所以是普通文件 + 一份哈希清单,`verify` 重算而不是相信。包里只放这个任务的东西:别的任务的行在这种场合是泄漏,不是好奇心。已被保留策略扫掉的 artifact 记 `present: false` —— 导出照做并说明,比"拒绝生成"更诚实。两次导出同一个未变任务,**record 哈希相同**(时间戳只在 manifest 里),否则"这份被改过吗"就无从回答。

**绕过 provider 的命令直接拒,不走审批。** 让用户批准"运行这条 shell 命令"是在问错的问题:他以为在同意一条命令,实际发生的是一个没有租约、没有痕迹的浏览器。拒绝里必须写清替代路径(`provider.acquire` + `provider.call`),**不说怎么办的拒绝,模型会绕开而不是遵守**。

匹配用词边界而不是裸 `contains`:`playwright-notes` 仓库不是自动化企图,分不清的检查最后会被关掉。裸 CDP 要求**两个条件同时成立** —— 回环地址带端口 **且** devtools 路径 —— 因为单独一个都是正常工作(`localhost:8080` 是别人的开发服务器,`example.com/json/version-history` 是个网页)。这两条都是我自己的测试逼出来的:第一版把 `curl https://example.com/json/version-history` 误杀,同时漏掉 `localhost:9222/json/list`。

**这不是沙箱**,文档里写死了这句。足够坚决的模型可以把命令混淆到模式匹配之外。它做到的是:让受支持的路径成为容易的那条,让绕行成为一个**在审计线里留下拒绝记录的刻意行为**。不可绕过的强制需要操作系统层面,那是另一件工作。

## 新增的面

- MCP:`scope.list/create/check/archive`、`artifact.list/usage/verify/forget`、`audit.verify`、`task.export_evidence/verify_evidence`(11 个,总数 116 → **127**)
- CLI:`unterm-cli scope|artifact|evidence`
- 迁移 v5:`workspaces` 与 `artifacts`

命名冲突说明:本仓库的 `workspace.*` 从很早就指**保存的分屏布局**,为了一个词去改它会打断线上所有 agent。所以文件系统作用域走 `scope.*`,老命名保持原义。

## 真机验证

同一个 headless Core 上跑通:

```
$ unterm-cli scope create alpha …/alpha        → wsp_5433ae…
$ unterm-cli scope create bravo …/bravo        → wsp_b86eab…
$ unterm-cli scope check wsp_5433ae… …/bravo/secret.txt
  Denied: path is explicitly denied            exit=1
$ unterm-cli scope create inner …/alpha/inner
  …/alpha/inner is inside the workspace "alpha", which cannot be isolated from it
$ unterm-cli evidence audit
  Entries: 7   Chain: intact
```

审计链是这台机器上**真实写下的 7 条**(`scope.create` ×3、`provider.bind`/`acquire` ×4),`seq` 与 `prev_sha256` 逐条相扣。把其中一条的 `provider.bind` 改成 `session.destroy` 之后:

```
Entries: 7   First break at entry: 3
Error: an entry no longer matches its own hash    exit=1
```

改回去即恢复 intact。

**一个计划外但有价值的观察**:验证过程中 Unzoo 服务恰好停了,而 `rest-port` 文件仍写着 9399。系统的表现正是设计要的 —— provider 仍被**发现**(文件说它在哪),但**不可达**(bind 报 offline),`provider.acquire` 返回 `waiting_provider` 并带上原因,没有任何回退。陈旧的端口文件不会被误认成"能用"。

## 还没做的

- 审计脱敏仍在调用方(写入前 redact),这一版没有把脱敏规则收进 `audit_store`;链和关联 ID 是这次补的。
- 证据包是目录,不是单文件压缩包 —— 打包/加密留给需要它的场景。
- 路由检查只看命令文本;真正不可绕过的强制要靠 OS 级隔离。
- `scope.*` 尚未强制接进 brain 的文件工具路径(`brain.read/write` 目前只判风险,不判 workspace 归属)。那是把两者接起来的一步,留在 M7 与 Supervisor 一起做。
