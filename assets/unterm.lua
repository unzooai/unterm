-- Unterm 默认配置
-- Native Super Terminal

local wezterm = require 'wezterm'
local config = wezterm.config_builder()
local act = wezterm.action

-- 一体顶栏:让整合标题栏 / tab 栏与终端内容同色,随主题走(Warp 式不割裂)。
-- 复刻 Rust 默认解析:读 ~/.unterm/theme.json 的 color_scheme,取其 bg/fg;
-- 取不到就回退中性高对比 Unterm Dark(开箱默认,接近 Warp 的纯中性深色)。
local function scheme_colors()
  local name = 'Unterm Dark'
  local home = os.getenv('HOME')
  if home then
    local fh = io.open(home .. '/.unterm/theme.json', 'r')
    if fh then
      local txt = fh:read('*a')
      fh:close()
      name = txt:match('"color_scheme"%s*:%s*"([^"]+)"') or name
    end
  end
  local bg, fg = '#191919', '#d4d4d4'
  local ok, schemes = pcall(function() return wezterm.color.get_builtin_schemes() end)
  if ok and schemes[name] then
    bg = schemes[name].background or bg
    fg = schemes[name].foreground or fg
  end
  return bg, fg
end
local theme_bg, theme_fg = scheme_colors()

-------------------------------------------------
-- 基础设置
-------------------------------------------------
config.check_for_updates = false
-- 配色由主题系统(~/.unterm/theme.json,见状态栏 theme: / `unterm-cli theme`)决定,
-- 不在此硬设 color_scheme,否则会覆盖「选主题」。需要固定配色时取消下一行注释:
-- config.color_scheme = 'Catppuccin Mocha'
-- 默认字体:JetBrains Mono(随 Unterm 打包,任何机器都有,不依赖系统已装),
-- 开 ligatures;图标回退到同样打包的 Nerd Font Symbols。比裸 Cascadia Code
-- 更精致、更稳定(系统没装 Cascadia 时不会回退到难看的默认字体)。
config.font = wezterm.font_with_fallback({
  { family = 'JetBrains Mono', weight = 'Medium', harfbuzz_features = { 'calt=1', 'liga=1', 'clig=1' } },
  'PingFang SC',
  'Microsoft YaHei UI',
  'Noto Sans CJK SC',
  'Noto Sans Mono CJK SC',
  'Symbols Nerd Font Mono',
})
config.font_size = 13
config.line_height = 1.15
config.enable_scroll_bar = true
config.scrollback_lines = 100000
config.window_close_confirmation = 'NeverPrompt'
config.window_background_opacity = 1.0
config.win32_system_backdrop = 'Disable'
config.show_unterm_status_bar = true

-------------------------------------------------
-- 窗口（Windows Terminal 风格单栏）
-------------------------------------------------
config.window_decorations = 'INTEGRATED_BUTTONS|RESIZE'
-- macOS 使用自绘三色点:AppKit 原生按钮固定锚在窗口顶部,在 Unterm 的
-- 一体化顶栏里无法真正上下居中。自绘按钮走同一套 chrome 布局,与
-- 右侧动作图标、侧栏顶部边界保持一致。
if wezterm.target_triple:find('darwin') then
  config.integrated_title_button_style = 'MacOsCustom'
  config.integrated_title_buttons = { 'Close', 'Hide', 'Maximize' }
elseif wezterm.target_triple:find('windows') then
  config.integrated_title_button_style = 'Windows'
  config.integrated_title_button_alignment = 'Right'
else
  config.integrated_title_button_style = 'Gnome'
end
config.window_padding = { left = 16, right = 16, top = 10, bottom = 8 }
config.initial_cols = 120
config.initial_rows = 30

-- 左侧垂直 tab 栏(Warp 式):每行是一个 tab,标题下显示
-- "驱动它的 AI agent · 项目目录",拖右缘改宽,空间足够时比横向 tab 更
-- 适合"一窗多 agent 各开一摊"的用法。INTEGRATED_BUTTONS 仍把交通灯叠进
-- 顶栏,所以顶部依旧一行(交通灯 + 标题 + 菜单 + 快捷动作)。
config.tab_bar_position = 'Left'

-- 顶栏 = 主题底轻提亮一档(~5%),活动 tab 仍与内容同色 → 层次分明但不割裂
-- (用户反馈 2026-06-10:完全同色太平,需要一定色差)。
local bar_bg = theme_bg
do
  local ok, c = pcall(function() return wezterm.color.parse(theme_bg) end)
  if ok and c then bar_bg = tostring(c:lighten(0.05)) end
end
config.window_frame = {
  inactive_titlebar_bg = bar_bg,
  active_titlebar_bg = bar_bg,
  inactive_titlebar_fg = theme_fg,
  active_titlebar_fg = theme_fg,
  inactive_titlebar_border_bottom = bar_bg,
  active_titlebar_border_bottom = bar_bg,
  button_fg = theme_fg,
  button_bg = bar_bg,
  button_hover_fg = theme_fg,
  button_hover_bg = bar_bg,
  -- ① 顶栏加高:标题字号 10 → 12,整条栏随之增高,不再紧凑
  font_size = 12.0,
}

-------------------------------------------------
-- Tab 栏（简洁风格）
-------------------------------------------------
config.use_fancy_tab_bar = true
config.tab_max_width = 32
config.show_tab_index_in_tab_bar = false
config.show_new_tab_button_in_tab_bar = true
config.hide_tab_bar_if_only_one_tab = false

-- Tab 栏整条 = 内容底(theme_bg),活动 tab 无缝、文字加粗;非活动 tab 同底、
-- 用 Half 亮度自动变暗(随主题,不需算色)。整条顶栏与内容一体。
-- 非活动 tab 用「主题前景调暗到 65%」的显式灰,而不是 Half 强度(Half 直接
-- 砍半,暗主题下几乎看不清——对比度反馈 2026-06-10)。
local dim_fg = theme_fg
do
  local ok, c = pcall(function() return wezterm.color.parse(theme_fg) end)
  if ok and c then dim_fg = tostring(c:darken(0.35)) end
end
config.colors = {
  tab_bar = {
    background = bar_bg,
    active_tab = { bg_color = theme_bg, fg_color = theme_fg, intensity = 'Bold' },
    inactive_tab = { bg_color = bar_bg, fg_color = dim_fg },
    inactive_tab_hover = { bg_color = bar_bg, fg_color = theme_fg },
    new_tab = { bg_color = bar_bg, fg_color = dim_fg },
    new_tab_hover = { bg_color = bar_bg, fg_color = theme_fg },
  },
}

-------------------------------------------------
-- 分屏焦点区分
-- 不给非活动分屏变暗：变暗会在分屏交界处留下一条可见的亮度台阶,
-- 一直延伸到底部状态栏文字上沿,观感很差。焦点改由活动分屏的实心
-- 光标 vs 非活动分屏的空心光标来区分(多数专业终端的做法)。
-- identity 变换(全 1.0)= 不做任何明暗/饱和度改动。
-------------------------------------------------
config.inactive_pane_hsb = { brightness = 1.0, saturation = 1.0, hue = 1.0 }

-------------------------------------------------
-- Tab 标题：只显示 Shell 名称
-------------------------------------------------
wezterm.on('format-tab-title', function(tab, tabs, panes, cfg, hover, max_width)
  local pane = tab.active_pane
  local title = pane.title or ''

  -- 尝试从进程名获取
  if title == '' or title == 'default' then
    local proc = pane.foreground_process_name or ''
    title = proc:match('([^/\\]+)$') or 'Terminal'
  end

  -- 清理 .exe 后缀，首字母大写
  title = title:gsub('%.exe$', '')
  if #title > 0 then
    title = title:sub(1, 1):upper() .. title:sub(2)
  end
  if title == '' then title = 'Terminal' end

  return '  ' .. title .. '  '
end)

-------------------------------------------------
-- Windows Terminal / PowerShell 默认不显示额外右侧状态文本
-------------------------------------------------
config.status_update_interval = 2000

wezterm.on('update-status', function(window, pane)
  window:set_right_status('')
end)

-------------------------------------------------
-- Windows PATH 扩展（Node/Bun/Perl 等常用工具）
-------------------------------------------------
if wezterm.target_triple == 'x86_64-pc-windows-msvc' then
  local path = os.getenv('PATH') or os.getenv('Path') or ''
  local extra_paths = {
    'C:\\Program Files\\nodejs',
    os.getenv('APPDATA') and (os.getenv('APPDATA') .. '\\npm') or nil,
    'C:\\Strawberry\\perl\\bin',
    os.getenv('USERPROFILE') and (os.getenv('USERPROFILE') .. '\\.bun\\bin') or nil,
  }

  for _, dir in ipairs(extra_paths) do
    if dir and not path:find(dir, 1, true) then
      path = dir .. ';' .. path
    end
  end

  config.set_environment_variables = {
    PATH = path,
    Path = path,
  }
end

-------------------------------------------------
-- 默认 Shell
-------------------------------------------------
if wezterm.target_triple == 'x86_64-pc-windows-msvc' then
  -- 优先 pwsh，回退 powershell
  local pwsh = 'C:\\Program Files\\PowerShell\\7\\pwsh.exe'
  local f = io.open(pwsh, 'r')
  if f then
    f:close()
    config.default_prog = { pwsh, '-NoLogo' }
  else
    config.default_prog = { 'powershell.exe', '-NoLogo' }
  end
end

-------------------------------------------------
-- 快捷键（匹配原 Unterm 设计）
-------------------------------------------------
config.keys = {
  -- Tab
  { key = 'T', mods = 'CTRL|SHIFT', action = act.SpawnTab('CurrentPaneDomain') },
  { key = 'W', mods = 'CTRL|SHIFT', action = act.CloseCurrentTab({ confirm = false }) },
  -- 分屏（D=垂直分屏，E=水平分屏）
  { key = 'D', mods = 'CTRL|SHIFT', action = act.SplitVertical({ domain = 'CurrentPaneDomain' }) },
  { key = 'E', mods = 'CTRL|SHIFT', action = act.SplitHorizontal({ domain = 'CurrentPaneDomain' }) },
  { key = 'X', mods = 'CTRL|SHIFT', action = act.CloseCurrentPane({ confirm = false }) },
  -- 分屏焦点切换
  { key = 'LeftArrow', mods = 'ALT', action = act.ActivatePaneDirection('Left') },
  { key = 'RightArrow', mods = 'ALT', action = act.ActivatePaneDirection('Right') },
  { key = 'UpArrow', mods = 'ALT', action = act.ActivatePaneDirection('Up') },
  { key = 'DownArrow', mods = 'ALT', action = act.ActivatePaneDirection('Down') },
  -- Tab 切换
  { key = 'Tab', mods = 'CTRL', action = act.ActivateTabRelative(1) },
  { key = 'Tab', mods = 'CTRL|SHIFT', action = act.ActivateTabRelative(-1) },
  -- 复制粘贴
  { key = 'C', mods = 'CTRL|SHIFT', action = act.CopyTo('Clipboard') },
  { key = 'V', mods = 'CTRL|SHIFT', action = act.PasteFrom('Clipboard') },
  -- 搜索
  { key = 'F', mods = 'CTRL|SHIFT', action = act.Search({ CaseSensitiveString = '' }) },
  -- 目录跳转面板
  { key = 'O', mods = 'CTRL|SHIFT', action = act.ShowDirJump },
  -- 目录树侧栏
  { key = 'B', mods = 'CTRL|SHIFT', action = act.ToggleTreeSidebar },
  -- 字号
  { key = '=', mods = 'CTRL', action = act.IncreaseFontSize },
  { key = '-', mods = 'CTRL', action = act.DecreaseFontSize },
  { key = '0', mods = 'CTRL', action = act.ResetFontSize },
  -- 全屏
  { key = 'F11', action = act.ToggleFullScreen },
  -- 命令面板
  { key = 'P', mods = 'CTRL|SHIFT', action = act.ActivateCommandPalette },
  -- Shell 选择器
  { key = 'N', mods = 'CTRL|SHIFT', action = act.ShowShellSelector },
}

-------------------------------------------------
-- 鼠标
-------------------------------------------------
config.mouse_bindings = {
  -- 右键 = 快速动作:有选中就复制 + 清空选中,无选中就粘贴(无菜单)
  {
    event = { Down = { streak = 1, button = 'Right' } },
    mods = 'NONE',
    action = act.ShowContextMenu,
  },
  -- 选中后左键释放自动复制
  {
    event = { Up = { streak = 1, button = 'Left' } },
    mods = 'NONE',
    action = act.CompleteSelectionOrOpenLinkAtMouseCursor('Clipboard'),
  },
}

return config
