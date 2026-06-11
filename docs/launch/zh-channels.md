# 中文渠道发帖包

## V2EX — /go/create(分享创造)⭐ 中文首发主阵地

**时机**:工作日上午 10 点或下午 3 点。发完守评论,V2EX 用户毒舌但转化真实。

**标题**:`做了一个"反 Warp"的终端:终端里没有 AI,但任何 AI 都能驱动它(MIT 开源)`

**正文**:

```
我是重度 Claude Code 用户。一直有个别扭的点:agent 能改文件、能跑命令,
但它看不见我真正的终端——我的会话、分屏、环境变量、输出历史都在那里。

市面上的答案是把 AI 塞进终端(Warp:内嵌 AI + 云端 + 订阅)。我觉得方向反了:
终端是用三十年的工具,AI 模型是六个月一换的组件;而且你已经有最强的 agent 了,
它缺的不是大脑,是手。

所以做了 Unterm:跨平台终端(Rust,基于深度定制的 WezTerm 引擎),
终端本身就是一个 MCP server。Claude Code / Codex / Gemini CLI / 你自己的脚本,
都能通过本地 JSON-RPC:

- 开 tab / 分屏、跑命令拿结构化结果(exit code + 输出,不用爬屏)
- 读任意 pane 的屏幕和完整回滚区(纯文本,不用 OCR)
- 截图,包括把整个回滚区无头渲染成一张长图(窗口被挡住也能出图)
- 会话录制成 markdown(自动打码 token/密钥)、身份 profile(一窗一套
  git/SSH/API 凭据)、代理管理

终端内部刻意一点 AI 都没有:没有聊天框、没有 copilot、没有补全订阅。
全部 127.0.0.1 + token,无账号无云端无遥测。MIT,macOS / Linux / Windows 全平台。

装好后会自动把自己注册进本机 Claude Code / Codex / Gemini 等的 MCP 配置,
agent 零配置就能发现它。

官网:https://unterm.app
GitHub:https://github.com/zhitongblog/unterm

被喷也欢迎,想听真话:你们到底想要 AI 在终端"里面",还是想让 AI "握住"终端?
```

**预备问答**:
- "和 tmux send-keys 比?" → 结构化输出 / 读屏 / 截图 / 身份隔离 / agent 自动发现,tmux 都要自己拼。
- "fork WezTerm 算什么本事" → 上游全额致谢;MCP server 进 GUI 进程、截图管线、身份系统,插件 API 给不了。
- "安全?" → 全本地、token 门控、审计日志、exec 有策略层可设黑名单,可只读运行。

---

## 即刻(AI 工具圈)

短文+图(用社交预览图或长截图演示):

```
做了个有点反共识的东西:一个"里面没有 AI"的终端。

所有终端都在往里塞 AI(Warp 们),我反着来——终端做成 MCP server,
让你已经在用的 Claude Code / Codex 从外面驱动它:开分屏、跑命令、
读屏、滚屏长截图、录会话。AI 进化多快都无所谓,终端只负责"被驱动"。

MIT 开源,全平台,零云端。unterm.app
```

---

## 掘金 / 少数派(投稿长文)

写一篇 2000–3000 字技术文,标题候选:
1. 《终端不需要 AI,终端需要被 AI 控制——Unterm 的一个反共识设计》
2. 《我把终端做成了 MCP Server:让 Claude Code 像人一样用终端》

结构:痛点(agent 看不见终端)→ 为什么不学 Warp(寿命错配/你已有 agent/缺手不缺脑)
→ 架构(MCP in GUI 进程、67 方法 11 命名空间、自动发现)→ 演示(滚屏长截图、
会话录制)→ 开源信息。官网宣言区(unterm.app 的 #why)可直接扩写。

---

## 知乎

问题蹲点回答(搜这些问题,认真答+文末带链接):
- "Warp 终端怎么样?"
- "有什么好用的终端推荐?"
- "Claude Code 怎么用效率最高?"
- "MCP 有什么实用的 server?"
