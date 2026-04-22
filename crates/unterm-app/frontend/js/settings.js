// 设置管理器
const SettingsManager = {
  defaults: {
    shell: '',
    font: 'Cascadia Mono',
    fontSize: 16,
    cursorStyle: 'bar',
    cursorBlink: true,
  },

  settings: {},

  load() {
    try {
      const saved = localStorage.getItem('unterm-settings');
      this.settings = saved ? JSON.parse(saved) : {};
    } catch (e) {
      this.settings = {};
    }
    // 合并默认值
    for (const [key, val] of Object.entries(this.defaults)) {
      if (this.settings[key] === undefined) {
        this.settings[key] = val;
      }
    }
  },

  save() {
    try {
      localStorage.setItem('unterm-settings', JSON.stringify(this.settings));
    } catch (e) {}
  },

  get(key) {
    return this.settings[key] !== undefined ? this.settings[key] : this.defaults[key];
  },

  set(key, value) {
    this.settings[key] = value;
    this.save();
  },

  applyToTerminals() {
    if (typeof TerminalManager === 'undefined') return;
    const fontFamily = `'${this.get('font')}', 'Cascadia Mono', 'Consolas', 'Courier New', monospace`;
    const fontSize = this.get('fontSize');
    const cursorStyle = this.get('cursorStyle');
    const cursorBlink = this.get('cursorBlink');

    TerminalManager.panes.forEach((pane, paneId) => {
      pane.terminal.options.fontFamily = fontFamily;
      pane.terminal.options.fontSize = fontSize;
      pane.terminal.options.cursorStyle = cursorStyle;
      pane.terminal.options.cursorBlink = cursorBlink;
      if (pane.element) {
        try {
          pane.fitAddon.fit();
          TerminalManager._resize(paneId, pane.terminal.cols, pane.terminal.rows);
        } catch (e) {}
      }
    });
  },
};

// 代理管理器（仅手动模式）
const ProxyManager = {
  status: null,

  async load() {
    try {
      this.status = await window.__TAURI__.core.invoke('proxy_get_status');
    } catch (e) {
      try {
        const saved = localStorage.getItem('unterm-proxy');
        if (saved) {
          const legacy = JSON.parse(saved);
          this.status = {
            mode: legacy.enabled ? 'manual' : 'off',
            manual: { http: legacy.http || '', socks: legacy.socks || '' },
          };
        }
      } catch (_) {}
    }
    if (!this.status) {
      this.status = {
        mode: 'off',
        manual: { http: '', socks: '' },
      };
    }
    this._updateBadge();
  },

  isEnabled() {
    return this.status && this.status.mode !== 'off';
  },

  async getEnvVars() {
    try {
      return await window.__TAURI__.core.invoke('proxy_get_env_vars');
    } catch (e) {
      if (!this.status || this.status.mode === 'off') return null;
      if (this.status.mode === 'manual') {
        const env = {};
        if (this.status.manual.http) {
          env['HTTP_PROXY'] = this.status.manual.http;
          env['http_proxy'] = this.status.manual.http;
          env['HTTPS_PROXY'] = this.status.manual.http;
          env['https_proxy'] = this.status.manual.http;
        }
        if (this.status.manual.socks) {
          env['ALL_PROXY'] = this.status.manual.socks;
          env['all_proxy'] = this.status.manual.socks;
        }
        return Object.keys(env).length > 0 ? env : null;
      }
      return null;
    }
  },

  async setMode(mode) {
    try {
      this.status = await window.__TAURI__.core.invoke('proxy_set_mode', { mode });
    } catch (e) {
      if (this.status) this.status.mode = mode;
    }
    this._updateBadge();
  },

  async setManual(http, socks) {
    try {
      await window.__TAURI__.core.invoke('proxy_set_manual', { http, socks });
      if (this.status) {
        this.status.manual = { http, socks };
      }
    } catch (e) {}
  },

  _updateBadge() {
    const badge = document.getElementById('proxy-status');
    if (!badge) return;
    if (this.isEnabled()) {
      badge.classList.add('active');
    } else {
      badge.classList.remove('active');
    }
  },
};

// 面板开关辅助
function openOverlay(id) {
  const el = document.getElementById(id);
  if (el) el.classList.remove('hidden');
}

function closeOverlay(id) {
  const el = document.getElementById(id);
  if (el) el.classList.add('hidden');
}

function closeAllOverlays() {
  document.querySelectorAll('.overlay').forEach((el) => {
    el.classList.add('hidden');
  });
}

// 设置面板逻辑
function initSettingsPanel() {
  const fontInput = document.getElementById('setting-font');
  const fontSizeInput = document.getElementById('setting-fontsize');
  const cursorSelect = document.getElementById('setting-cursor');
  const cursorBlinkInput = document.getElementById('setting-cursor-blink');
  const shellSelect = document.getElementById('setting-shell');

  function populateSettings() {
    fontInput.value = SettingsManager.get('font');
    fontSizeInput.value = SettingsManager.get('fontSize');
    cursorSelect.value = SettingsManager.get('cursorStyle');
    cursorBlinkInput.checked = SettingsManager.get('cursorBlink');

    shellSelect.innerHTML = '';
    if (typeof Profiles !== 'undefined' && Profiles.shells.length > 0) {
      Profiles.shells.forEach((shell) => {
        const opt = document.createElement('option');
        opt.value = shell.command;
        opt.textContent = shell.name;
        shellSelect.appendChild(opt);
      });
      const savedShell = SettingsManager.get('shell');
      if (savedShell) {
        shellSelect.value = savedShell;
      }
    }
  }

  fontInput.addEventListener('input', () => {
    SettingsManager.set('font', fontInput.value);
    SettingsManager.applyToTerminals();
  });

  fontSizeInput.addEventListener('input', () => {
    const val = parseInt(fontSizeInput.value, 10);
    if (val >= 8 && val <= 32) {
      SettingsManager.set('fontSize', val);
      SettingsManager.applyToTerminals();
    }
  });

  cursorSelect.addEventListener('change', () => {
    SettingsManager.set('cursorStyle', cursorSelect.value);
    SettingsManager.applyToTerminals();
  });

  cursorBlinkInput.addEventListener('change', () => {
    SettingsManager.set('cursorBlink', cursorBlinkInput.checked);
    SettingsManager.applyToTerminals();
  });

  shellSelect.addEventListener('change', () => {
    SettingsManager.set('shell', shellSelect.value);
  });

  document.getElementById('menu-settings').addEventListener('click', () => {
    populateSettings();
    openOverlay('settings-overlay');
    document.getElementById('app-menu').classList.add('hidden');
  });
}

// 代理面板逻辑（仅手动模式）
function initProxyPanel() {
  const modeSelect = document.getElementById('proxy-mode');
  const manualPanel = document.getElementById('proxy-manual-panel');
  const httpInput = document.getElementById('proxy-http');
  const socksInput = document.getElementById('proxy-socks');

  function updatePanelVisibility(mode) {
    manualPanel.classList.toggle('hidden', mode !== 'manual');
  }

  async function populateProxy() {
    await ProxyManager.load();
    const status = ProxyManager.status;
    if (!status) return;

    modeSelect.value = (status.mode === 'off' || status.mode === 'manual') ? status.mode : 'off';
    updatePanelVisibility(modeSelect.value);

    httpInput.value = status.manual.http || '';
    socksInput.value = status.manual.socks || '';
  }

  modeSelect.addEventListener('change', async () => {
    await ProxyManager.setMode(modeSelect.value);
    updatePanelVisibility(modeSelect.value);
  });

  let manualTimer = null;
  function saveManual() {
    clearTimeout(manualTimer);
    manualTimer = setTimeout(() => {
      ProxyManager.setManual(httpInput.value, socksInput.value);
    }, 500);
  }
  httpInput.addEventListener('input', saveManual);
  socksInput.addEventListener('input', saveManual);

  document.getElementById('menu-proxy').addEventListener('click', () => {
    populateProxy();
    openOverlay('proxy-overlay');
    document.getElementById('app-menu').classList.add('hidden');
  });
}

// 关于面板逻辑
function initAboutPanel() {
  document.getElementById('menu-about').addEventListener('click', () => {
    openOverlay('about-overlay');
    document.getElementById('app-menu').classList.add('hidden');
  });
}

// 管理员模式
function initAdminAction() {
  document.getElementById('menu-admin').addEventListener('click', async () => {
    document.getElementById('app-menu').classList.add('hidden');
    try {
      const shell = Profiles.getDefault();
      await window.__TAURI__.core.invoke('open_admin_shell', {
        shell: shell.command,
      });
      if (typeof ScreenshotManager !== 'undefined') {
        ScreenshotManager.showToast('正在以管理员身份重启 Unterm...');
      }
    } catch (e) {
      console.error('[Unterm] 打开管理员 Shell 失败:', e);
      if (typeof ScreenshotManager !== 'undefined') {
        ScreenshotManager.showToast('打开失败: ' + (e.message || e));
      }
    }
  });
}

// 通用遮罩层交互
function initOverlayInteractions() {
  document.querySelectorAll('.overlay-close[data-close]').forEach((btn) => {
    btn.addEventListener('click', () => {
      closeOverlay(btn.dataset.close);
    });
  });

  document.querySelectorAll('.overlay').forEach((overlay) => {
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) {
        overlay.classList.add('hidden');
      }
    });
  });

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      closeAllOverlays();
    }
  });
}
