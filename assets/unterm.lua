-- Unterm 默认配置
-- Native Super Terminal

local wezterm = require 'wezterm'
local config = wezterm.config_builder()
local act = wezterm.action

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
  { family = 'JetBrains Mono', harfbuzz_features = { 'calt=1', 'liga=1', 'clig=1' } },
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
config.integrated_title_button_alignment = 'Right'
config.integrated_title_button_style = 'Windows'
config.window_padding = { left = 16, right = 16, top = 10, bottom = 8 }
config.initial_cols = 120
config.initial_rows = 30

-- Windows Terminal / PowerShell 标题栏
-- 暖黑标题栏,跟 Notion Dark 内容底(#191919)统一,避免冷灰 chrome 撞暖黑正文。
config.window_frame = {
  inactive_titlebar_bg = '#1c1c1c',
  active_titlebar_bg = '#191919',
  inactive_titlebar_fg = '#8b8b8b',
  active_titlebar_fg = '#ffffff',
  inactive_titlebar_border_bottom = '#141414',
  active_titlebar_border_bottom = '#2a2a2a',
  button_fg = '#cccccc',
  button_bg = '#191919',
  button_hover_fg = '#ffffff',
  button_hover_bg = '#2f2f2f',
}

-------------------------------------------------
-- Tab 栏（简洁风格）
-------------------------------------------------
config.use_fancy_tab_bar = true
config.tab_max_width = 32
config.show_tab_index_in_tab_bar = false
config.show_new_tab_button_in_tab_bar = true
config.hide_tab_bar_if_only_one_tab = false

-- Tab 栏暖化到 Notion Dark:活动 tab 与内容底同色(#191919)做无缝过渡,
-- 非活动 tab 稍亮、文字降为柔和灰,hover 再亮一档。
config.colors = {
  tab_bar = {
    background = '#191919',
    active_tab = {
      bg_color = '#191919',
      fg_color = '#ffffff',
      intensity = 'Bold',
    },
    inactive_tab = {
      bg_color = '#202020',
      fg_color = '#8b8b8b',
    },
    inactive_tab_hover = {
      bg_color = '#262626',
      fg_color = '#ffffff',
    },
    new_tab = {
      bg_color = '#191919',
      fg_color = '#8b8b8b',
    },
    new_tab_hover = {
      bg_color = '#262626',
      fg_color = '#ffffff',
    },
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

  return ' ' .. title .. ' '
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
