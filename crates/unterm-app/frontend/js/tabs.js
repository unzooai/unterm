// Tab 管理
const Tabs = {
  tabs: [],
  activeTabId: null,
  nextId: 1,
  nextPaneId: 1,

  init() {
    // 新建 Tab 按钮
    const newTabBtn = document.getElementById('new-tab-btn');
    newTabBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      const dropdown = document.getElementById('shell-dropdown');
      if (dropdown.classList.contains('hidden')) {
        Profiles.renderDropdown();
        dropdown.classList.remove('hidden');
      } else {
        dropdown.classList.add('hidden');
      }
    });

    // 点击其他地方关闭下拉菜单
    document.addEventListener('click', () => {
      document.getElementById('shell-dropdown').classList.add('hidden');
    });
  },

  createTab(title, shell, cwd) {
    const id = this.nextId++;
    const paneId = this.nextPaneId++;

    const tab = {
      id,
      title: title || `Tab ${id}`,
      shell: shell || null,
      panes: [{ id: paneId, terminal: null }],
      activePaneId: paneId,
      layout: null, // 由 SplitManager 管理
    };

    this.tabs.push(tab);
    this.switchTab(id);
    this.renderTabs();

    // 创建终端 pane
    TerminalManager.createPane(paneId, shell, cwd);

    return tab;
  },

  closeTab(tabId) {
    const idx = this.tabs.findIndex(t => t.id === tabId);
    if (idx === -1) return;

    const tab = this.tabs[idx];
    tab.panes.forEach(p => TerminalManager.destroyPane(p.id));

    this.tabs.splice(idx, 1);

    if (this.tabs.length === 0) {
      const defaultShell = Profiles.getDefault();
      this.createTab(defaultShell.name, defaultShell.command);
      return;
    }

    if (this.activeTabId === tabId) {
      const newIdx = Math.min(idx, this.tabs.length - 1);
      this.switchTab(this.tabs[newIdx].id);
    }
    this.renderTabs();
  },

  switchTab(tabId) {
    this.activeTabId = tabId;
    this.renderTabs();
    this.renderTerminalArea();
    // 切换 tab 后聚焦终端
    setTimeout(() => TerminalManager.focusActive(), 50);
  },

  nextTab() {
    const idx = this.tabs.findIndex(t => t.id === this.activeTabId);
    const next = (idx + 1) % this.tabs.length;
    this.switchTab(this.tabs[next].id);
  },

  prevTab() {
    const idx = this.tabs.findIndex(t => t.id === this.activeTabId);
    const prev = (idx - 1 + this.tabs.length) % this.tabs.length;
    this.switchTab(this.tabs[prev].id);
  },

  getActiveTab() {
    return this.tabs.find(t => t.id === this.activeTabId);
  },

  renderTabs() {
    const container = document.getElementById('tabs-container');
    container.innerHTML = '';

    this.tabs.forEach(tab => {
      const el = document.createElement('div');
      el.className = `tab${tab.id === this.activeTabId ? ' active' : ''}`;
      const titleSpan = document.createElement('span');
      titleSpan.className = 'tab-title';
      titleSpan.textContent = tab.title;
      const closeBtn = document.createElement('button');
      closeBtn.className = 'tab-close';
      closeBtn.title = '\u5173\u95ED';
      closeBtn.textContent = '\u00D7';
      el.appendChild(titleSpan);
      el.appendChild(closeBtn);

      el.addEventListener('click', (e) => {
        if (!e.target.classList.contains('tab-close')) {
          this.switchTab(tab.id);
        }
      });

      closeBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        this.closeTab(tab.id);
      });

      container.appendChild(el);
    });
  },

  renderTerminalArea() {
    const area = document.getElementById('terminal-area');
    area.innerHTML = '';

    const tab = this.getActiveTab();
    if (!tab) return;

    if (!tab.layout) {
      tab.layout = { type: 'pane', id: tab.panes[0].id };
    }
    const rootEl = SplitManager.renderNode(tab.layout, tab);
    if (rootEl) area.appendChild(rootEl);
  }
};
