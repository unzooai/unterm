// 截图管理器
const ScreenshotManager = {
  init() {
    if (!document.getElementById('toast-container')) {
      const t = document.createElement('div');
      t.id = 'toast-container';
      t.className = 'toast';
      document.body.appendChild(t);
    }
    this._initImagePaste();
  },

  showToast(msg, dur) {
    dur = dur || 2500;
    const t = document.getElementById('toast-container');
    if (!t) return;
    t.textContent = msg;
    t.classList.add('show');
    setTimeout(() => t.classList.remove('show'), dur);
  },

  // 截图选区：直接打开系统截图工具
  async captureRegion() {
    try {
      await window.__TAURI__.core.invoke('open_snipping_tool');
      this.showToast('已打开系统截图工具（Win+Shift+S）');
    } catch (_) {
      this.showToast('请按 Win+Shift+S 使用系统截图');
    }
    // 轮询检测截图工具进程退出后自动聚焦终端
    this._waitSnippingToolExit();
  },

  // 等待截图工具退出，然后聚焦终端
  _waitSnippingToolExit() {
    let attempts = 0;
    const check = setInterval(async () => {
      attempts++;
      if (attempts > 30) { clearInterval(check); return; } // 30秒超时
      try {
        const running = await window.__TAURI__.core.invoke('is_snipping_tool_running');
        if (!running && attempts > 1) {
          clearInterval(check);
          this._forceFocusTerminal();
        }
      } catch (_) {
        clearInterval(check);
      }
    }, 1000);
  },

  // 隐藏窗口截图：隐藏窗口 → 打开截图工具 → 自动恢复
  async captureHideWindow() {
    const { getCurrentWindow } = window.__TAURI__.window;
    const appWindow = getCurrentWindow();

    try {
      // 隐藏窗口
      await appWindow.hide();
      await new Promise(r => setTimeout(r, 300));

      // 打开系统截图工具
      try {
        await window.__TAURI__.core.invoke('open_snipping_tool');
      } catch (_) {}

      // 监听窗口恢复：用户完成截图后点击任务栏图标会触发
      // 同时设置最长 30 秒超时自动恢复
      let restored = false;
      const restore = async () => {
        if (restored) return;
        restored = true;
        try {
          await appWindow.show();
          await appWindow.setFocus();
          const focus = () => TerminalManager.focusActive();
          focus();
          requestAnimationFrame(focus);
          setTimeout(focus, 200);
        } catch (_) {}
      };

      setTimeout(restore, 30000);

      // 每秒检测截图工具进程
      const checkInterval = setInterval(async () => {
        if (restored) { clearInterval(checkInterval); return; }
        try {
          const running = await window.__TAURI__.core.invoke('is_snipping_tool_running');
          if (!running) {
            clearInterval(checkInterval);
            await new Promise(r => setTimeout(r, 500));
            restore();
          }
        } catch (_) {
          clearInterval(checkInterval);
          setTimeout(restore, 3000);
        }
      }, 1000);

    } catch (e) {
      try { await appWindow.show(); await appWindow.setFocus(); } catch (_) {}
    }
  },

  async capture(hideWindow) {
    if (hideWindow) {
      await this.captureHideWindow();
    } else {
      await this.captureRegion();
    }
  },

  // 强制聚焦终端：直接找到 xterm 隐藏 textarea 并 focus
  _forceFocusTerminal(paneId) {
    const focus = () => {
      const id = paneId !== undefined ? paneId : (Tabs.getActiveTab() || {}).activePaneId;
      if (id === undefined) return;
      const pane = TerminalManager.panes.get(id);
      if (!pane) return;
      // 直接聚焦 xterm 内部的 helper-textarea（键盘输入接收器）
      const el = pane.element || document.getElementById(`pane-${id}`);
      if (el) {
        const textarea = el.querySelector('.xterm-helper-textarea');
        if (textarea) { textarea.focus(); return; }
      }
      // 兜底：用 xterm API
      pane.terminal.focus();
    };
    // 先 blur 当前焦点元素，避免抢回焦点
    if (document.activeElement) document.activeElement.blur();
    focus();
    requestAnimationFrame(focus);
    setTimeout(focus, 50);
    setTimeout(focus, 200);
  },

  // 图片粘贴：检测剪贴板图片 → 保存文件 → 插入路径到终端
  _initImagePaste() {
    const self = this;

    async function handleImagePaste(e) {
      if (!window.__TAURI__ || !window.__TAURI__.core) return;
      try {
        const b64 = await window.__TAURI__.core.invoke('paste_image_as_base64');
        if (b64) {
          e.preventDefault();
          e.stopPropagation();

          const tab = typeof Tabs !== 'undefined' ? Tabs.getActiveTab() : null;
          if (!tab || tab.activePaneId === undefined) return;

          const filePath = await window.__TAURI__.core.invoke('save_screenshot', {
            imageData: b64,
          });
          TerminalManager._sendInput(tab.activePaneId, filePath);
          self.showToast('图片已保存: ' + filePath);
          // 强制聚焦：先让 Tauri 窗口获焦，再聚焦 xterm 终端
          self._forceFocusTerminal(tab.activePaneId);
        }
      } catch (_) {}
    }

    // Ctrl+V
    document.addEventListener('keydown', (e) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'v') {
        handleImagePaste(e);
      }
    }, true);

    // paste 事件
    document.addEventListener('paste', (e) => {
      handleImagePaste(e);
    }, true);
  },
};
