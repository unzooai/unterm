// 终端管理器
const TerminalManager = {
  panes: new Map(), // paneId -> { terminal, fitAddon, element }

  async createPane(paneId, shell, cwd) {
    try {
      if (typeof Terminal === 'undefined') {
        console.error('[Unterm] xterm.js not loaded');
        return;
      }

      // 从 SettingsManager 读取配置（若已加载）
      const fontName = typeof SettingsManager !== 'undefined' ? SettingsManager.get('font') : 'Cascadia Mono';
      const fontSize = typeof SettingsManager !== 'undefined' ? SettingsManager.get('fontSize') : 16;
      const cursorStyle = typeof SettingsManager !== 'undefined' ? SettingsManager.get('cursorStyle') : 'bar';
      const cursorBlink = typeof SettingsManager !== 'undefined' ? SettingsManager.get('cursorBlink') : true;

      const xtermTheme = ThemeManager.getXtermTheme();

      const terminal = new Terminal({
        fontFamily: `'${fontName}', 'Cascadia Mono', 'Consolas', 'Courier New', monospace`,
        fontSize,
        lineHeight: 1.0,
        fontWeight: 'normal',
        fontWeightBold: 'bold',
        drawBoldTextInBrightColors: true,
        minimumContrastRatio: 1,
        cursorBlink,
        cursorStyle,
        scrollback: 100000,
        scrollOnUserInput: true,
        theme: xtermTheme,
        allowProposedApi: true,
      });

      const fitAddon = new FitAddon.FitAddon();
      terminal.loadAddon(fitAddon);

      if (typeof WebLinksAddon !== 'undefined') {
        const webLinksAddon = new WebLinksAddon.WebLinksAddon();
        terminal.loadAddon(webLinksAddon);
      }

      // WebGL 渲染器（色彩还原更准确）
      if (typeof WebglAddon !== 'undefined') {
        try {
          const webglAddon = new WebglAddon.WebglAddon();
          terminal.loadAddon(webglAddon);
          webglAddon.onContextLoss(() => {
            webglAddon.dispose();
          });
        } catch (e) {
          console.warn('[Unterm] WebGL 渲染器加载失败，回退 Canvas:', e);
        }
      }

      // 加载图片协议支持（Sixel / iTerm2 inline images）
      if (typeof ImageAddon !== 'undefined') {
        try {
          const imageAddon = new ImageAddon.ImageAddon();
          terminal.loadAddon(imageAddon);
        } catch (e) {
          console.warn('[Unterm] ImageAddon 加载失败:', e);
        }
      }

      this.panes.set(paneId, { terminal, fitAddon, element: null });

      // 输入回调（过滤 DSR 响应，后端已处理，避免双重回复）
      terminal.onData((data) => {
        // xterm.js 收到 ESC[6n 后会自动回复 ESC[row;colR，
        // 但后端 VTE 解析器已经回复了，多余的响应会被 shell 当成输入吃掉字符
        if (/^\x1b\[\d+;\d+R$/.test(data)) return;
        this._sendInput(paneId, data);
      });

      // 选中文本自动复制到剪贴板（Windows Terminal 风格）
      terminal.onSelectionChange(() => {
        const sel = terminal.getSelection();
        if (sel && sel.trim().length > 0) {
          this._copyText(sel);
        }
      });

      await this._mountWithRetry(paneId, 5);

      // 通知后端创建 session
      try {
        if (window.__TAURI__ && window.__TAURI__.core) {
          const envVars = typeof ProxyManager !== 'undefined' ? await ProxyManager.getEnvVars() : null;
          await window.__TAURI__.core.invoke('create_session', {
            paneId,
            shell: shell || null,
            cwd: cwd || null,
            env: envVars,
          });
        } else {
          terminal.writeln('\x1b[31m连接失败: Tauri IPC 未就绪\x1b[0m');
        }
      } catch (e) {
        terminal.writeln(`\r\n\x1b[31m连接失败: ${e}\x1b[0m`);
      }
    } catch (e) {
      console.error('[Unterm] createPane failed:', e);
    }
  },

  // 带重试的 DOM 挂载
  async _mountWithRetry(paneId, maxRetries) {
    for (let i = 0; i < maxRetries; i++) {
      const el = document.getElementById(`pane-${paneId}`);
      if (el && el.offsetWidth > 0 && el.offsetHeight > 0) {
        this.attachToPane(paneId, el);
        return;
      }
      await new Promise(r => requestAnimationFrame(r));
      await new Promise(r => setTimeout(r, 50));
    }
    const el = document.getElementById(`pane-${paneId}`);
    if (el) this.attachToPane(paneId, el);
  },

  attachToPane(paneId, element) {
    const pane = this.panes.get(paneId);
    if (!pane) return;
    if (pane.element === element) return;

    // 清理旧的 ResizeObserver
    if (pane._resizeObserver) {
      pane._resizeObserver.disconnect();
      pane._resizeObserver = null;
    }

    pane.element = element;

    if (pane.terminal.element) {
      element.insertBefore(pane.terminal.element, element.firstChild);
    } else {
      pane.terminal.open(element);
    }

    // 自动聚焦
    setTimeout(() => pane.terminal.focus(), 50);

    // 右键上下文菜单（避免重复绑定）
    if (!element._ctxBound) {
      element._ctxBound = true;
      element.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        e.stopPropagation();
        this._showContextMenu(e, paneId);
      });
    }

    // 用 ResizeObserver 监听容器尺寸变化，自动 fit
    // 防抖 + cols/rows 去重，避免滚动条引起的循环抖动
    let resizeTimer = null;
    let lastCols = 0, lastRows = 0;
    const ro = new ResizeObserver(() => {
      if (resizeTimer) clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        try {
          pane.fitAddon.fit();
          const cols = pane.terminal.cols;
          const rows = pane.terminal.rows;
          if (cols !== lastCols || rows !== lastRows) {
            lastCols = cols;
            lastRows = rows;
            this._resize(paneId, cols, rows);
          }
        } catch (e) {}
      }, 100);
    });
    ro.observe(element);
    pane._resizeObserver = ro;
  },

  destroyPane(paneId) {
    const pane = this.panes.get(paneId);
    if (!pane) return;
    if (pane._resizeObserver) {
      pane._resizeObserver.disconnect();
    }
    pane.terminal.dispose();
    this.panes.delete(paneId);
    if (window.__TAURI__ && window.__TAURI__.core) {
      window.__TAURI__.core.invoke('destroy_session', { paneId }).catch(() => {});
    }
  },

  async _sendInput(paneId, data) {
    try {
      if (window.__TAURI__ && window.__TAURI__.core) {
        await window.__TAURI__.core.invoke('send_input', { paneId, input: data });
      }
    } catch (e) {
      console.error('[Unterm] send_input failed:', e);
    }
  },

  async _resize(paneId, cols, rows) {
    try {
      if (window.__TAURI__ && window.__TAURI__.core) {
        await window.__TAURI__.core.invoke('resize_session', { paneId, cols, rows });
      }
    } catch (e) {
      console.error('[Unterm] resize failed:', e);
    }
  },

  // 右键粘贴：先检查图片（保存+路径），再检查文本
  async _rightClickPaste(paneId) {
    if (!window.__TAURI__ || !window.__TAURI__.core) return;

    // 先检查剪贴板是否有图片
    try {
      const imgPath = await window.__TAURI__.core.invoke('paste_image_from_clipboard');
      if (imgPath) {
        this._sendInput(paneId, imgPath);
        if (typeof ScreenshotManager !== 'undefined') {
          ScreenshotManager.showToast('图片已粘贴: ' + imgPath);
        }
        this._focusPane(paneId);
        return;
      }
    } catch (_) {}

    // 没有图片，读取文本粘贴
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        this._sendInput(paneId, text);
      }
    } catch (_) {}
    this._focusPane(paneId);
  },

  // 聚焦指定 pane 的终端
  _focusPane(paneId) {
    const pane = this.panes.get(paneId);
    if (pane) pane.terminal.focus();
  },

  // 复制文本到剪贴板（多种方式兜底）
  async _copyText(text) {
    let ok = false;

    // 方式1: Clipboard API
    try {
      await navigator.clipboard.writeText(text);
      ok = true;
    } catch (_) {}

    // 方式2: execCommand fallback
    if (!ok) {
      try {
        const ta = document.createElement('textarea');
        ta.value = text;
        ta.style.cssText = 'position:fixed;left:-9999px;top:-9999px';
        document.body.appendChild(ta);
        ta.select();
        ok = document.execCommand('copy');
        document.body.removeChild(ta);
      } catch (_) {}
    }

    // 方式3: PowerShell fallback
    if (!ok && window.__TAURI__ && window.__TAURI__.core) {
      try {
        await window.__TAURI__.core.invoke('copy_text_to_clipboard', { text });
        ok = true;
      } catch (_) {}
    }

    if (ok && typeof ScreenshotManager !== 'undefined') {
      ScreenshotManager.showToast('已复制');
    }
  },

  writeToPane(paneId, data) {
    const pane = this.panes.get(paneId);
    if (pane) {
      pane.terminal.write(data);
    }
  },

  focusActive() {
    const tab = typeof Tabs !== 'undefined' ? Tabs.getActiveTab() : null;
    if (!tab) return;
    const pane = this.panes.get(tab.activePaneId);
    if (pane) pane.terminal.focus();
  },

  handleResize() {
    this.panes.forEach((pane, paneId) => {
      if (pane.element) {
        try {
          const oldCols = pane.terminal.cols;
          const oldRows = pane.terminal.rows;
          pane.fitAddon.fit();
          const newCols = pane.terminal.cols;
          const newRows = pane.terminal.rows;
          if (newCols !== oldCols || newRows !== oldRows) {
            this._resize(paneId, newCols, newRows);
          }
        } catch (e) {}
      }
    });
  }
};

// 右键菜单
Object.assign(TerminalManager, {
  _showContextMenu(e, paneId) {
    const menu = document.getElementById('context-menu');
    // 单 pane 时隐藏关闭面板选项
    const tab = Tabs.getActiveTab();
    const closePaneItem = menu.querySelector('[data-action="close-pane"]');
    if (closePaneItem) {
      closePaneItem.style.display = (tab && tab.panes.length > 1) ? '' : 'none';
      // 关闭面板前面的分割线
      const sep = closePaneItem.previousElementSibling;
      if (sep && sep.classList.contains('ctx-sep')) {
        sep.style.display = (tab && tab.panes.length > 1) ? '' : 'none';
      }
    }

    // 设置当前 pane 为活跃
    if (tab) {
      tab.activePaneId = paneId;
    }

    // 定位菜单
    menu.classList.remove('hidden');
    const mw = menu.offsetWidth;
    const mh = menu.offsetHeight;
    let x = e.clientX;
    let y = e.clientY;
    if (x + mw > window.innerWidth) x = window.innerWidth - mw - 4;
    if (y + mh > window.innerHeight) y = window.innerHeight - mh - 4;
    menu.style.left = x + 'px';
    menu.style.top = y + 'px';

    // 存储当前 paneId
    menu.dataset.paneId = paneId;
  },
});

// 初始化右键菜单交互
(function initContextMenu() {
  const menu = document.getElementById('context-menu');
  if (!menu) return;

  // 点击菜单项
  menu.addEventListener('click', async (e) => {
    const item = e.target.closest('.ctx-item');
    if (!item) return;
    const action = item.dataset.action;
    const paneId = parseInt(menu.dataset.paneId);
    menu.classList.add('hidden');

    switch (action) {
      case 'copy': {
        const pane = TerminalManager.panes.get(paneId);
        if (pane) {
          const sel = pane.terminal.getSelection();
          if (sel) TerminalManager._copyText(sel);
        }
        break;
      }
      case 'paste':
        await TerminalManager._rightClickPaste(paneId);
        break;
      case 'split-v':
        SplitManager.splitVertical();
        break;
      case 'split-h':
        SplitManager.splitHorizontal();
        break;
      case 'close-pane':
        SplitManager.closeActivePane();
        break;
      case 'open-folder':
        openFolderAndCd();
        break;
    }
  });

  // 点击外部关闭菜单
  document.addEventListener('click', (e) => {
    if (!menu.contains(e.target)) {
      menu.classList.add('hidden');
    }
  });

  // ESC 关闭菜单
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      menu.classList.add('hidden');
    }
  });
})();

let _windowResizeTimer = null;
window.addEventListener('resize', () => {
  if (_windowResizeTimer) clearTimeout(_windowResizeTimer);
  _windowResizeTimer = setTimeout(() => TerminalManager.handleResize(), 50);
});

// 窗口获焦时自动聚焦当前活跃终端
window.addEventListener('focus', () => {
  TerminalManager.focusActive();
});
