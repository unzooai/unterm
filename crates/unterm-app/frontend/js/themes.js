// 主题管理器
const ThemeManager = {
  themes: {
    // VS Code 默认深色 — 最经典的开发者暗色主题
    'vscode-dark': {
      name: 'VS Code Dark',
      css: {
        '--base': '#1e1e1e', '--mantle': '#272727', '--crust': '#303030',
        '--surface0': '#2d2d2d', '--surface1': '#3c3c3c', '--surface2': '#4e4e4e',
        '--overlay0': '#6e6e6e', '--text': '#d4d4d4', '--subtext0': '#9e9e9e',
        '--subtext1': '#b0b0b0', '--blue': '#569cd6', '--green': '#6a9955',
        '--red': '#f44747', '--yellow': '#dcdcaa', '--peach': '#ce9178',
        '--mauve': '#c586c0', '--teal': '#4ec9b0',
      },
      xterm: {
        background: '#1e1e1e', foreground: '#d4d4d4', cursor: '#aeafad',
        selectionBackground: '#264f78',
        black: '#000000', red: '#cd3131', green: '#0dbc79', yellow: '#e5e510',
        blue: '#2472c8', magenta: '#bc3fbc', cyan: '#11a8cd', white: '#e5e5e5',
        brightBlack: '#666666', brightRed: '#f14c4c', brightGreen: '#23d18b',
        brightYellow: '#f5f543', brightBlue: '#3b8eea', brightMagenta: '#d670d6',
        brightCyan: '#29b8db', brightWhite: '#e5e5e5',
      },
    },
    // Catppuccin Mocha — 温暖柔和的深色
    'catppuccin-mocha': {
      name: 'Catppuccin Mocha',
      css: {
        '--base': '#1e1e2e', '--mantle': '#272738', '--crust': '#313244',
        '--surface0': '#3b3c50', '--surface1': '#45475a', '--surface2': '#585b70',
        '--overlay0': '#6c7086', '--text': '#cdd6f4', '--subtext0': '#a6adc8',
        '--subtext1': '#bac2de', '--blue': '#89b4fa', '--green': '#a6e3a1',
        '--red': '#f38ba8', '--yellow': '#f9e2af', '--peach': '#fab387',
        '--mauve': '#cba6f7', '--teal': '#94e2d5',
      },
      xterm: {
        background: '#1e1e2e', foreground: '#cdd6f4', cursor: '#f5e0dc',
        selectionBackground: '#45475a',
        black: '#45475a', red: '#f38ba8', green: '#a6e3a1', yellow: '#f9e2af',
        blue: '#89b4fa', magenta: '#cba6f7', cyan: '#94e2d5', white: '#bac2de',
        brightBlack: '#585b70', brightRed: '#f38ba8', brightGreen: '#a6e3a1',
        brightYellow: '#f9e2af', brightBlue: '#89b4fa', brightMagenta: '#cba6f7',
        brightCyan: '#94e2d5', brightWhite: '#a6adc8',
      },
    },
    // One Dark Pro — Atom 编辑器经典配色
    'one-dark': {
      name: 'One Dark Pro',
      css: {
        '--base': '#282c34', '--mantle': '#31353d', '--crust': '#3a3e47',
        '--surface0': '#2c313a', '--surface1': '#353b45', '--surface2': '#3e4451',
        '--overlay0': '#545862', '--text': '#abb2bf', '--subtext0': '#828997',
        '--subtext1': '#9da5b4', '--blue': '#61afef', '--green': '#98c379',
        '--red': '#e06c75', '--yellow': '#e5c07b', '--peach': '#d19a66',
        '--mauve': '#c678dd', '--teal': '#56b6c2',
      },
      xterm: {
        background: '#282c34', foreground: '#abb2bf', cursor: '#528bff',
        selectionBackground: '#3e4451',
        black: '#1e2127', red: '#e06c75', green: '#98c379', yellow: '#d19a66',
        blue: '#61afef', magenta: '#c678dd', cyan: '#56b6c2', white: '#abb2bf',
        brightBlack: '#5c6370', brightRed: '#e06c75', brightGreen: '#98c379',
        brightYellow: '#d19a66', brightBlue: '#61afef', brightMagenta: '#c678dd',
        brightCyan: '#56b6c2', brightWhite: '#ffffff',
      },
    },
    // GitHub Dark — GitHub 官方深色
    'github-dark': {
      name: 'GitHub Dark',
      css: {
        '--base': '#0d1117', '--mantle': '#161b22', '--crust': '#1f2428',
        '--surface0': '#21262d', '--surface1': '#2d333b', '--surface2': '#30363d',
        '--overlay0': '#484f58', '--text': '#e6edf3', '--subtext0': '#8b949e',
        '--subtext1': '#b1bac4', '--blue': '#58a6ff', '--green': '#3fb950',
        '--red': '#f85149', '--yellow': '#d29922', '--peach': '#db6d28',
        '--mauve': '#bc8cff', '--teal': '#39d353',
      },
      xterm: {
        background: '#0d1117', foreground: '#e6edf3', cursor: '#e6edf3',
        selectionBackground: '#264f78',
        black: '#484f58', red: '#ff7b72', green: '#3fb950', yellow: '#d29922',
        blue: '#58a6ff', magenta: '#bc8cff', cyan: '#39d353', white: '#b1bac4',
        brightBlack: '#6e7681', brightRed: '#ffa198', brightGreen: '#56d364',
        brightYellow: '#e3b341', brightBlue: '#79c0ff', brightMagenta: '#d2a8ff',
        brightCyan: '#56d364', brightWhite: '#f0f6fc',
      },
    },
    // Dracula — 经典紫调
    'dracula': {
      name: 'Dracula',
      css: {
        '--base': '#282a36', '--mantle': '#323440', '--crust': '#3b3d4a',
        '--surface0': '#44475a', '--surface1': '#4d5066', '--surface2': '#565973',
        '--overlay0': '#6272a4', '--text': '#f8f8f2', '--subtext0': '#bfbfbf',
        '--subtext1': '#e0e0e0', '--blue': '#8be9fd', '--green': '#50fa7b',
        '--red': '#ff5555', '--yellow': '#f1fa8c', '--peach': '#ffb86c',
        '--mauve': '#bd93f9', '--teal': '#8be9fd',
      },
      xterm: {
        background: '#282a36', foreground: '#f8f8f2', cursor: '#f8f8f2',
        selectionBackground: '#44475a',
        black: '#21222c', red: '#ff5555', green: '#50fa7b', yellow: '#f1fa8c',
        blue: '#bd93f9', magenta: '#ff79c6', cyan: '#8be9fd', white: '#f8f8f2',
        brightBlack: '#6272a4', brightRed: '#ff6e6e', brightGreen: '#69ff94',
        brightYellow: '#ffffa5', brightBlue: '#d6acff', brightMagenta: '#ff92df',
        brightCyan: '#a4ffff', brightWhite: '#ffffff',
      },
    },
    // Windows Terminal 默认 — Campbell 配色
    'campbell': {
      name: 'Windows Terminal',
      css: {
        '--base': '#0c0c0c', '--mantle': '#181818', '--crust': '#202020',
        '--surface0': '#2a2a2a', '--surface1': '#333333', '--surface2': '#3a3a3a',
        '--overlay0': '#555555', '--text': '#cccccc', '--subtext0': '#999999',
        '--subtext1': '#bbbbbb', '--blue': '#3b78ff', '--green': '#16c60c',
        '--red': '#e74856', '--yellow': '#f9f1a5', '--peach': '#f9f1a5',
        '--mauve': '#b4009e', '--teal': '#61d6d6',
      },
      xterm: {
        background: '#0c0c0c', foreground: '#cccccc', cursor: '#ffffff',
        selectionBackground: '#264f78',
        black: '#0c0c0c', red: '#c50f1f', green: '#13a10e', yellow: '#c19c00',
        blue: '#0037da', magenta: '#881798', cyan: '#3a96dd', white: '#cccccc',
        brightBlack: '#767676', brightRed: '#e74856', brightGreen: '#16c60c',
        brightYellow: '#f9f1a5', brightBlue: '#3b78ff', brightMagenta: '#b4009e',
        brightCyan: '#61d6d6', brightWhite: '#f2f2f2',
      },
    },
    // Solarized Dark — 经典低对比度
    'solarized-dark': {
      name: 'Solarized Dark',
      css: {
        '--base': '#002b36', '--mantle': '#073642', '--crust': '#0a4050',
        '--surface0': '#0d4a5a', '--surface1': '#1a5066', '--surface2': '#2a6070',
        '--overlay0': '#586e75', '--text': '#839496', '--subtext0': '#657b83',
        '--subtext1': '#93a1a1', '--blue': '#268bd2', '--green': '#859900',
        '--red': '#dc322f', '--yellow': '#b58900', '--peach': '#cb4b16',
        '--mauve': '#6c71c4', '--teal': '#2aa198',
      },
      xterm: {
        background: '#002b36', foreground: '#839496', cursor: '#839496',
        selectionBackground: '#073642',
        black: '#073642', red: '#dc322f', green: '#859900', yellow: '#b58900',
        blue: '#268bd2', magenta: '#d33682', cyan: '#2aa198', white: '#eee8d5',
        brightBlack: '#586e75', brightRed: '#cb4b16', brightGreen: '#586e75',
        brightYellow: '#657b83', brightBlue: '#839496', brightMagenta: '#6c71c4',
        brightCyan: '#93a1a1', brightWhite: '#fdf6e3',
      },
    },
    // macOS Terminal — 经典 macOS 终端风格
    'macos-terminal': {
      name: 'macOS Terminal',
      css: {
        '--base': '#1e1e1e', '--mantle': '#272727', '--crust': '#303030',
        '--surface0': '#2a2a2a', '--surface1': '#363636', '--surface2': '#424242',
        '--overlay0': '#636363', '--text': '#e0e0e0', '--subtext0': '#a0a0a0',
        '--subtext1': '#c0c0c0', '--blue': '#58a6ff', '--green': '#87d441',
        '--red': '#ff6b6b', '--yellow': '#ffd93d', '--peach': '#ffa94d',
        '--mauve': '#cc77ff', '--teal': '#63e6be',
      },
      xterm: {
        background: '#1e1e1e', foreground: '#e0e0e0', cursor: '#e0e0e0',
        selectionBackground: '#3a3a3a',
        black: '#000000', red: '#c91b00', green: '#00c200', yellow: '#c7c400',
        blue: '#0225c7', magenta: '#ca30c7', cyan: '#00c5c7', white: '#c7c7c7',
        brightBlack: '#686868', brightRed: '#ff6e67', brightGreen: '#5ffa68',
        brightYellow: '#fffc67', brightBlue: '#6871ff', brightMagenta: '#ff77ff',
        brightCyan: '#60fdff', brightWhite: '#ffffff',
      },
    },
  },

  current: (() => {
    const ua = navigator.userAgent || navigator.platform || '';
    if (ua.includes('Win')) return 'campbell';
    if (ua.includes('Mac')) return 'macos-terminal';
    return 'vscode-dark'; // Linux 默认
  })(),

  apply(themeName) {
    const theme = this.themes[themeName];
    if (!theme) return;
    this.current = themeName;

    const root = document.documentElement;
    for (const [prop, value] of Object.entries(theme.css)) {
      root.style.setProperty(prop, value);
    }

    if (typeof TerminalManager !== 'undefined') {
      TerminalManager.panes.forEach((pane) => {
        pane.terminal.options.theme = theme.xterm;
      });
    }

    try { localStorage.setItem('unterm-theme', themeName); } catch (e) {}
  },

  getXtermTheme() {
    return this.themes[this.current].xterm;
  },

  list() {
    return Object.entries(this.themes).map(([id, t]) => ({ id, name: t.name }));
  },

  load() {
    try {
      const saved = localStorage.getItem('unterm-theme');
      if (saved && this.themes[saved]) {
        this.current = saved;
      }
    } catch (e) {}
    this.apply(this.current);
  },

  // 强制重置为指定主题（清除缓存）
  reset(themeName) {
    themeName = themeName || 'campbell';
    try { localStorage.removeItem('unterm-theme'); } catch (e) {}
    this.apply(themeName);
  },
};
