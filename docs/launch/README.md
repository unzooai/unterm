# Unterm 推广作战手册(v0.40 起)

现状基线(2026-06-11):5 stars,冷启动。打法 = 一次有节奏的 launch,不是日常运营。

**2026-07-25 更新**:v0.57.4 已发(9 stars / 571 downloads)。第 1 波(目录)基本收官:
官方 Registry 0.57.4、PulseMCP 自动同步、**Glama 已上线**(glama.ai/mcp/servers/zhitongblog/unterm)、
awesome-mcp-servers PR #7166 已加徽章+rebase(MERGEABLE,等维护者)。mcp.so 待浏览器核对。
第 2 波起(Show HN/X/Reddit/V2EX)改由 Claude 经 Unzoo 浏览器 lixd220 profile 代发(alex 2026-07-25 授权)。
新钩子:Agent Cockpit(fleet + 状态信号 + inbox + checkpoint/rollback),演示动图见 docs/。
数字口径:99 MCP 方法 / 21 命名空间(v0.57.4)。

## 核心叙事(所有渠道统一)

> 所有终端都在往里塞 AI。我们反着做:终端里零 AI,终端本身是 MCP server,
> 让你已经在用的 agent(Claude Code/Codex/Gemini)像人一样驱动它。
> "终端不需要 AI,终端需要被 AI 控制。"

钩子素材:滚屏长截图(可视化强)、安装即自动注册进所有 AI CLI(零配置)、Warp 对照。

## 渠道优先级与节奏

| 周次 | 动作 | 文件 |
|---|---|---|
| 第 0 步(发帖前) | README 加 hero 演示图;GitHub 上传社交预览图(assets/social-preview.png,Settings→Social preview) | — |
| 第 1 波 | MCP 目录全量提交(awesome-mcp-servers PR、mcp.so、Glama claim、PulseMCP、mcpservers.org) | mcp-directories.md |
| 第 2 波(同一天) | Show HN(美东早上)+ X 英文线程互导 | show-hn.md / x-thread.md |
| 第 2 波+1 天 | V2EX 分享创造 + 即刻 | zh-channels.md |
| 第 3 波(HN 后 3–7 天) | r/ClaudeAI → r/mcp → r/commandline(隔天发) | reddit.md |
| 第 4 波 | Product Hunt(需 gallery 图 ≥3 + maker comment) | product-hunt.md |
| 持续 | 掘金/少数派长文、知乎问题蹲点、每个 minor 版本在 r/mcp 发 changelog 贴 | zh-channels.md |

## 分工

- **Claude 可直接执行**:目录 PR(用你的 gh 身份,需你点头)、表单类提交(Unzoo 浏览器)、
  所有文案维护、演示素材制作(hero 截图摆拍、滚屏长截图 demo GIF)。
- **必须你亲手**:HN / PH / Reddit / V2EX / X 发帖(账号信誉),社交预览图上传
  (GitHub Settings 页),发帖后头 2 小时的评论互动(文案包里备了高频问答弹药)。

## 红线

- 不买量、不互赞群、不在 HN/Reddit 用小号自顶(封号毁渠道)。
- 每个 sub 间隔 ≥1 天;同一链接不同社区文案必须本地化重写(已做)。
- 永远先回答技术质疑,再谈产品;被喷"又一个 wrapper"时用事实回(67 方法/无云端/MIT)。
