// Shell Profile 管理
const Profiles = {
  shells: [],

  // 图标映射
  ICONS: {
    powershell: '💠',
    cmd: '⬛',
    git: '🔶',
    azure: '☁️',
    vs: '🟣',
    linux: '🐧',
    terminal: '⬛',
  },

  async detect() {
    try {
      // 调用 Tauri 后端检测可用 shell
      this.shells = await window.__TAURI__.core.invoke('detect_shells');
    } catch (e) {
      console.warn('Shell 检测失败，使用默认值:', e);
      this.shells = [
        { name: 'PowerShell', command: 'powershell.exe', icon: 'powershell' },
      ];
    }
    return this.shells;
  },

  getDefault() {
    return this.shells[0] || { name: 'Shell', command: null, icon: 'terminal' };
  },

  getByIndex(index) {
    return this.shells[index] || null;
  },

  renderDropdown() {
    const list = document.getElementById('shell-list');
    list.innerHTML = '';
    this.shells.forEach((shell, idx) => {
      const item = document.createElement('div');
      item.className = 'shell-item';
      const icon = this.ICONS[shell.icon] || '⬛';
      const shortcut = idx < 9 ? `Ctrl+Shift+${idx + 1}` : '';
      item.innerHTML = `
        <span class="shell-item-icon">${icon}</span>
        <span class="shell-item-name">${shell.name}</span>
        ${shortcut ? `<span class="shell-item-shortcut">${shortcut}</span>` : ''}
      `;
      item.addEventListener('click', () => {
        Tabs.createTab(shell.name, shell.command);
        document.getElementById('shell-dropdown').classList.add('hidden');
      });
      list.appendChild(item);
    });

    // 底部：设置 + 命令面板（分隔线后）
    const sep = document.createElement('div');
    sep.className = 'shell-dropdown-sep';
    list.appendChild(sep);

    const settingsItem = document.createElement('div');
    settingsItem.className = 'shell-item';
    settingsItem.innerHTML = `
      <span class="shell-item-icon">⚙</span>
      <span class="shell-item-name">设置</span>
      <span class="shell-item-shortcut">Ctrl+,</span>
    `;
    settingsItem.addEventListener('click', () => {
      document.getElementById('shell-dropdown').classList.add('hidden');
      document.getElementById('menu-settings').click();
    });
    list.appendChild(settingsItem);
  }
};
