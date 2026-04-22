// 分屏管理 — 树形布局，支持嵌套分屏
//
// 布局数据结构:
//   叶节点: { type: 'pane', id: number }
//   分裂节点: { type: 'split', direction: 'row'|'column', children: [node, node] }
//
const SplitManager = {
  MAX_PANES: 4,

  // 左右分屏
  splitHorizontal() {
    this._splitActivePane('row');
  },

  // 上下分屏
  splitVertical() {
    this._splitActivePane('column');
  },

  _splitActivePane(direction) {
    const tab = Tabs.getActiveTab();
    if (!tab) return;

    if (!tab.layout) {
      tab.layout = { type: 'pane', id: tab.panes[0].id };
    }

    const count = this._countPanes(tab.layout);
    if (count >= this.MAX_PANES) {
      console.warn(`已达到最大分屏数 ${this.MAX_PANES}`);
      return;
    }

    const newPaneId = Tabs.nextPaneId++;
    const activePaneId = tab.activePaneId;

    // 在树中替换活跃 pane 为 split 节点
    tab.layout = this._replaceNode(tab.layout, activePaneId, {
      type: 'split',
      direction,
      children: [
        { type: 'pane', id: activePaneId },
        { type: 'pane', id: newPaneId },
      ]
    });

    tab.panes.push({ id: newPaneId, terminal: null });
    tab.activePaneId = newPaneId;

    // ─── 直接操作 DOM，不重建已有终端 ───

    // 找到当前活跃 pane 的 DOM 元素
    const existingPaneEl = document.getElementById(`pane-${activePaneId}`);
    if (!existingPaneEl) return;

    const parent = existingPaneEl.parentElement;

    // 创建 split 容器
    const container = document.createElement('div');
    container.className = 'split-container';
    container.style.display = 'flex';
    container.style.flex = '1';
    container.style.flexDirection = direction;
    container.style.overflow = 'hidden';
    container.style.minWidth = '0';
    container.style.minHeight = '0';

    // 用 container 替换 existingPaneEl
    parent.replaceChild(container, existingPaneEl);

    // 把原来的 pane 放进 container
    container.appendChild(existingPaneEl);

    // 给原来的 pane 加关闭按钮（如果之前没有）
    if (!existingPaneEl.querySelector('.pane-close-btn')) {
      this._addCloseBtn(existingPaneEl, tab, activePaneId);
    }

    // 加分隔线
    const divider = document.createElement('div');
    divider.className = `split-divider ${direction === 'row' ? 'horizontal' : 'vertical'}`;
    container.appendChild(divider);

    // 创建新 pane 元素
    const newPaneEl = document.createElement('div');
    newPaneEl.className = 'pane active';
    newPaneEl.id = `pane-${newPaneId}`;
    newPaneEl.addEventListener('click', () => {
      tab.activePaneId = newPaneId;
      document.getElementById('terminal-area')
        .querySelectorAll('.pane').forEach(el => el.classList.remove('active'));
      newPaneEl.classList.add('active');
    });
    this._addCloseBtn(newPaneEl, tab, newPaneId);
    container.appendChild(newPaneEl);

    // 取消原 pane 的 active
    existingPaneEl.classList.remove('active');

    // 为新 pane 创建终端
    TerminalManager.createPane(newPaneId, tab.shell);
  },

  closeActivePane() {
    const tab = Tabs.getActiveTab();
    if (!tab) return;

    if (tab.panes.length <= 1) {
      Tabs.closeTab(tab.id);
      return;
    }

    const closingId = tab.activePaneId;

    // 更新布局树
    tab.layout = this._removeNode(tab.layout, closingId);

    // ─── 直接操作 DOM，不重建 ───
    const closingEl = document.getElementById(`pane-${closingId}`);
    if (closingEl) {
      const splitContainer = closingEl.parentElement;
      // splitContainer 应该是 .split-container
      if (splitContainer && splitContainer.classList.contains('split-container')) {
        const containerParent = splitContainer.parentElement;

        // 找到兄弟节点（跳过 divider 和关闭的 pane）
        let siblingEl = null;
        for (const child of splitContainer.children) {
          if (child !== closingEl && !child.classList.contains('split-divider')) {
            siblingEl = child;
            break;
          }
        }

        if (siblingEl && containerParent) {
          // 用兄弟节点替换整个 split-container
          containerParent.replaceChild(siblingEl, splitContainer);

          // 如果只剩一个 pane（非嵌套），移除其关闭按钮
          if (tab.panes.length === 2) { // 关闭前还有2个，关闭后只剩1个
            const closeBtn = siblingEl.querySelector('.pane-close-btn');
            if (closeBtn) closeBtn.remove();
          }
        }
      }
    }

    // 销毁终端
    TerminalManager.destroyPane(closingId);
    const idx = tab.panes.findIndex(p => p.id === closingId);
    tab.panes.splice(idx, 1);

    // 切换到相邻 pane
    const newIdx = Math.min(idx, tab.panes.length - 1);
    tab.activePaneId = tab.panes[newIdx].id;

    // 设置 active
    const activeEl = document.getElementById(`pane-${tab.activePaneId}`);
    if (activeEl) {
      document.getElementById('terminal-area')
        .querySelectorAll('.pane').forEach(el => el.classList.remove('active'));
      activeEl.classList.add('active');
    }

    // 触发 resize（剩余 pane 尺寸变了，ResizeObserver 应该自动触发）
    // 保险起见手动也触发一次
    setTimeout(() => TerminalManager.handleResize(), 50);
  },

  // ─── 树操作工具 ───

  _countPanes(node) {
    if (node.type === 'pane') return 1;
    return node.children.reduce((s, c) => s + this._countPanes(c), 0);
  },

  _replaceNode(node, paneId, replacement) {
    if (node.type === 'pane') {
      return node.id === paneId ? replacement : node;
    }
    return {
      ...node,
      children: node.children.map(c => this._replaceNode(c, paneId, replacement))
    };
  },

  _removeNode(node, paneId) {
    if (node.type === 'pane') return node;
    if (node.type === 'split') {
      for (let i = 0; i < node.children.length; i++) {
        if (node.children[i].type === 'pane' && node.children[i].id === paneId) {
          return node.children[1 - i];
        }
      }
      return {
        ...node,
        children: node.children.map(c => this._removeNode(c, paneId))
      };
    }
    return node;
  },

  // renderNode 仅用于 tab 切换时重建（switchTab）
  renderNode(node, tab) {
    if (node.type === 'pane') {
      const paneEl = document.createElement('div');
      const isSplit = tab.panes.length > 1;
      paneEl.className = `pane${isSplit && node.id === tab.activePaneId ? ' active' : ''}`;
      paneEl.id = `pane-${node.id}`;
      paneEl.addEventListener('click', () => {
        tab.activePaneId = node.id;
        document.getElementById('terminal-area')
          .querySelectorAll('.pane').forEach(el => el.classList.remove('active'));
        if (tab.panes.length > 1) {
          paneEl.classList.add('active');
        }
        // 点击 pane 时聚焦终端
        const p = TerminalManager.panes.get(node.id);
        if (p) p.terminal.focus();
      });

      if (TerminalManager.panes.has(node.id)) {
        TerminalManager.attachToPane(node.id, paneEl);
      }

      if (tab.panes.length > 1) {
        this._addCloseBtn(paneEl, tab, node.id);
      }

      return paneEl;
    }

    if (node.type === 'split') {
      const container = document.createElement('div');
      container.className = 'split-container';
      container.style.display = 'flex';
      container.style.flex = '1';
      container.style.flexDirection = node.direction;
      container.style.overflow = 'hidden';
      container.style.minWidth = '0';
      container.style.minHeight = '0';

      node.children.forEach((child, i) => {
        if (i > 0) {
          const divider = document.createElement('div');
          divider.className = `split-divider ${node.direction === 'row' ? 'horizontal' : 'vertical'}`;
          container.appendChild(divider);
        }
        container.appendChild(this.renderNode(child, tab));
      });

      return container;
    }
  },

  _addCloseBtn(paneEl, tab, paneId) {
    const closeBtn = document.createElement('button');
    closeBtn.className = 'pane-close-btn';
    closeBtn.title = '关闭面板 (Ctrl+Shift+X)';
    closeBtn.textContent = '✕';
    closeBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      tab.activePaneId = paneId;
      SplitManager.closeActivePane();
    });
    paneEl.appendChild(closeBtn);
  }
};
