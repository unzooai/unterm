// session_id -> pane_id 映射
const sessionToPaneMap = new Map();

// Unterm 主入口
(async function main() {

  // 初始化主题
  ThemeManager.load();

  // 初始化菜单
  initAppMenu();

  // 初始化 Tab 管理
  Tabs.init();

  // 检测可用 Shell
  await Profiles.detect();

  // 获取启动 cwd（从命令行参数传入，如资源管理器右键）
  let launchCwd = null;
  try {
    launchCwd = await window.__TAURI__.core.invoke('get_launch_cwd');
  } catch (e) {}

  // 创建第一个 Tab（使用默认 shell）
  const defaultShell = Profiles.getDefault();
  Tabs.createTab(defaultShell.name, defaultShell.command, launchCwd);

  // 初始化截图功能
  ScreenshotManager.init();

  // 保存窗口大小（关闭和调整时）
  initWindowStateSaver();

  // 启动事件轮询循环
  startEventPolling();

  // 注册全局快捷键
  registerShortcuts();

  // 窗口控制按钮
  initWindowControls();

  // 截图按钮绑定
  initScreenshotBindings();
})();

// 事件轮询
function startEventPolling() {
  setInterval(async () => {
    try {
      if (!window.__TAURI__ || !window.__TAURI__.core) return;
      const events = await window.__TAURI__.core.invoke('poll_events');
      for (const event of events) {
        handleCoreEvent(event);
      }
    } catch (e) {}
  }, 50);
}

// 处理后端事件
function handleCoreEvent(event) {
  const statusEl = document.getElementById('connection-status');

  switch (event.type) {
    case 'connected':
      statusEl.className = 'connected';
      statusEl.textContent = '\u25CF \u5DF2\u8FDE\u63A5';
      break;

    case 'disconnected':
      statusEl.className = 'disconnected';
      statusEl.textContent = '\u25CF \u672A\u8FDE\u63A5';
      break;

    case 'session_created':
      sessionToPaneMap.set(event.session_id, event.pane_id);
      break;

    case 'screen_update':
      {
        const paneId = sessionToPaneMap.get(event.session_id);
        if (paneId !== undefined && event.content) {
          TerminalManager.writeToPane(paneId, event.content);
        }
      }
      break;

    case 'error':
      console.error('Core \u9519\u8BEF:', event.message);
      break;
  }
}

// 全局快捷键
function registerShortcuts() {
  document.addEventListener('keydown', (e) => {
    const ctrl = e.ctrlKey || e.metaKey;
    const shift = e.shiftKey;

    if (ctrl && shift) {
      switch (e.key) {
        case 'T': // 新建 Tab
          e.preventDefault();
          const shell = Profiles.getDefault();
          Tabs.createTab(shell.name, shell.command);
          break;
        case 'W': // 关闭 Tab
          e.preventDefault();
          Tabs.closeTab(Tabs.activeTabId);
          break;
        case 'D': // 左右分屏
          e.preventDefault();
          SplitManager.splitHorizontal();
          break;
        case 'R': // 上下分屏
          e.preventDefault();
          SplitManager.splitVertical();
          break;
        case 'X': // 关闭 Pane
          e.preventDefault();
          SplitManager.closeActivePane();
          break;
        case 'O': // 打开目录
          e.preventDefault();
          openFolderAndCd();
          break;
        case 'S': // 截图
          e.preventDefault();
          ScreenshotManager.capture(false);
          break;
      }
    }

    // Ctrl+Shift+1~9 快捷键打开对应 shell profile
    if (ctrl && shift && e.key >= '1' && e.key <= '9') {
      e.preventDefault();
      const idx = parseInt(e.key) - 1;
      const shell = Profiles.getByIndex(idx);
      if (shell) {
        Tabs.createTab(shell.name, shell.command);
      }
    }

    // Ctrl+Tab / Ctrl+Shift+Tab
    if (ctrl && e.key === 'Tab') {
      e.preventDefault();
      if (shift) {
        Tabs.prevTab();
      } else {
        Tabs.nextTab();
      }
    }
  });
}

// 应用菜单
function initAppMenu() {
  const menuBtn = document.getElementById('menu-btn');
  const appMenu = document.getElementById('app-menu');
  const themeList = document.getElementById('theme-list');

  // 加载设置和代理
  SettingsManager.load();
  ProxyManager.load();

  // 渲染主题列表
  function renderThemeList() {
    themeList.innerHTML = '';
    for (const { id, name } of ThemeManager.list()) {
      const baseColor = ThemeManager.themes[id].css['--base'];
      const item = document.createElement('div');
      item.className = 'theme-item';
      item.innerHTML =
        `<span class="theme-swatch" style="background:${baseColor}"></span>` +
        `<span class="theme-name">${name}</span>` +
        (id === ThemeManager.current ? '<span class="theme-check">✓</span>' : '');
      item.addEventListener('click', () => {
        ThemeManager.apply(id);
        renderThemeList();
      });
      themeList.appendChild(item);
    }
  }

  renderThemeList();

  // 切换菜单
  menuBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    appMenu.classList.toggle('hidden');
  });

  // 点击外部关闭菜单
  document.addEventListener('click', (e) => {
    if (!appMenu.contains(e.target) && e.target !== menuBtn) {
      appMenu.classList.add('hidden');
    }
  });

  // 初始化各面板
  initSettingsPanel();
  initProxyPanel();
  initAboutPanel();
  initAdminAction();
  initOverlayInteractions();

  // 分屏菜单项
  document.getElementById('menu-split-v').addEventListener('click', () => {
    appMenu.classList.add('hidden');
    SplitManager.splitVertical();
  });
  document.getElementById('menu-split-h').addEventListener('click', () => {
    appMenu.classList.add('hidden');
    SplitManager.splitHorizontal();
  });
  document.getElementById('menu-close-pane').addEventListener('click', () => {
    appMenu.classList.add('hidden');
    SplitManager.closeActivePane();
  });

  // 打开目录
  document.getElementById('menu-open-folder').addEventListener('click', () => {
    appMenu.classList.add('hidden');
    openFolderAndCd();
  });

  // Ctrl+, 快捷键打开设置
  document.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === ',') {
      e.preventDefault();
      // 填充并打开设置面板
      document.getElementById('menu-settings').click();
    }
  });
}

// 截图按钮绑定
function initScreenshotBindings() {
  // 状态栏截图按钮（普通截图）
  document.getElementById('status-screenshot-btn').addEventListener('click', () => {
    ScreenshotManager.capture(false);
  });

  // 菜单截图（不隐藏窗口）
  document.getElementById('menu-screenshot').addEventListener('click', () => {
    document.getElementById('app-menu').classList.add('hidden');
    ScreenshotManager.capture(false);
  });

  // 菜单截图选区（隐藏窗口）
  document.getElementById('menu-screenshot-hide').addEventListener('click', () => {
    document.getElementById('app-menu').classList.add('hidden');
    ScreenshotManager.capture(true);
  });
}

// 窗口控制按钮 + 拖拽
function initWindowControls() {
  const { getCurrentWindow } = window.__TAURI__.window;
  const appWindow = getCurrentWindow();

  // 拖拽区域 — mousedown 立即拖拽，dblclick 最大化/还原
  const dragRegion = document.getElementById('drag-region');

  dragRegion.addEventListener('mousedown', (e) => {
    if (e.button !== 0) return;
    // 必须在 mousedown 事件中同步调用，否则 Tauri 不认用户手势
    appWindow.startDragging();
  });

  dragRegion.addEventListener('dblclick', (e) => {
    e.preventDefault();
    toggleMaximize();
  });

  async function toggleMaximize() {
    if (await appWindow.isMaximized()) {
      await appWindow.unmaximize();
    } else {
      await appWindow.maximize();
    }
  }

  document.getElementById('btn-minimize').addEventListener('click', () => {
    appWindow.minimize();
  });
  document.getElementById('btn-maximize').addEventListener('click', () => {
    toggleMaximize();
  });
  document.getElementById('btn-close').addEventListener('click', () => {
    appWindow.close();
  });
}

// 打开文件夹选择对话框，cd 到选中目录
async function openFolderAndCd() {
  try {
    const folder = await window.__TAURI__.core.invoke('pick_folder');
    if (!folder) return;

    const tab = Tabs.getActiveTab();
    if (!tab) return;
    const paneId = tab.activePaneId;
    const pane = TerminalManager.panes.get(paneId);
    if (!pane) return;

    // 根据 shell 类型构建 cd 命令
    const shell = (tab.shell || '').toLowerCase();
    let cmd;
    if (shell.includes('cmd') || shell.includes('cmd.exe')) {
      // CMD: cd /d 支持跨盘符
      cmd = `cd /d "${folder}"`;
    } else if (shell.includes('bash') || shell.includes('zsh') || shell.includes('fish') || shell.includes('wsl')) {
      // Unix shells
      const unixPath = folder.replace(/\\/g, '/');
      cmd = `cd "${unixPath}"`;
    } else {
      // PowerShell (default)
      const escaped = folder.replace(/'/g, "''");
      cmd = `cd '${escaped}'`;
    }

    // 先发 Escape 清除当前输入行，再延迟发 cd 命令
    TerminalManager._sendInput(paneId, '\x1b');
    setTimeout(() => {
      TerminalManager._sendInput(paneId, cmd + '\r');
    }, 150);
  } catch (e) {
    console.error('[Unterm] 打开目录失败:', e);
  }
}

// 窗口大小持久化
function initWindowStateSaver() {
  let saveTimer = null;
  const save = async () => {
    try {
      const { getCurrentWindow } = window.__TAURI__.window;
      const appWindow = getCurrentWindow();
      if (await appWindow.isMaximized()) return; // 最大化时不保存
      const size = await appWindow.innerSize();
      const scaleFactor = await appWindow.scaleFactor();
      const logicalW = Math.round(size.width / scaleFactor);
      const logicalH = Math.round(size.height / scaleFactor);
      await window.__TAURI__.core.invoke('save_window_state', {
        width: logicalW,
        height: logicalH,
      });
    } catch (e) {}
  };

  window.addEventListener('resize', () => {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(save, 500);
  });
}
