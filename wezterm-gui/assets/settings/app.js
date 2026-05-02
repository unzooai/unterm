// Unterm Web Settings — Alpine.js controller.
//
// All `/api/*` requests carry the bootstrap auth token loaded once at
// startup from `/bootstrap.json`. Same-origin, no cookies, no CORS.
//
// i18n: a single dictionary is fetched at boot from `/api/i18n` and used by
// every `t(key)` call. When the user picks a different language the page
// reloads so all strings render in the new locale.

function untermSettings() {
  return {
    token: '',
    health: { ok: false },
    state: {
      version: '',
      hostname: '',
      pid: '',
      started_at: '',
      ports: { mcp: '?', http: '?' },
      theme: 'standard',
      project: { path: '', slug: '' },
      sessions_path: '',
    },
    proxy: {
      enabled: false,
      mode: 'off',
      http_proxy: null,
      socks_proxy: null,
      no_proxy: '',
      health: null,
    },
    proxyForm: { http_proxy: '', socks_proxy: '', no_proxy: '' },
    recording: { active: false },
    sessions: [],
    sessionMarkdown: null,
    currentSessionId: null,
    toasts: [],
    nextToast: 1,

    // i18n state
    lang: 'en-US',
    dict: {},
    availableLocales: [{ code: 'en-US', name: 'English' }],

    active: 'general',
    get nav() {
      return [
        { id: 'general', label: this.t('web.nav.general') },
        { id: 'appearance', label: this.t('web.nav.appearance') },
        { id: 'proxy', label: this.t('web.nav.proxy') },
        { id: 'scrollback', label: this.t('web.nav.scrollback') },
        { id: 'compat', label: this.t('web.nav.compat') },
        { id: 'recording', label: this.t('web.nav.recording'), badge: !this._recordingSeen },
        { id: 'project', label: this.t('web.nav.project') },
        { id: 'about', label: this.t('web.nav.about') },
      ];
    },
    _recordingSeen: false,

    // Scrollback config — number of lines kept in each pane's history
    // buffer. Existing panes keep their old buffer until they're closed,
    // because the per-pane VecDeque capacity is fixed at construction; we
    // surface that with `appliedAt` so the UI can show "restart to apply
    // to existing panes" right after Save.
    scrollback: {
      lines: 10000,
      default: 10000,
      min: 100,
      max: 999_999_999,
      saving: false,
      appliedAt: null,
    },

    // Compatibility config — what to advertise as TERM_PROGRAM into spawned
    // shells. Default "Unterm" keeps our brand identity; some third-party
    // tools (Gemini CLI, certain IDE detectors) only whitelist a fixed
    // set of terminal names, so users can spoof to dodge those checks.
    // `appliedAt` flips a hint asking the user to open a new tab — the
    // running shells keep their old TERM_PROGRAM until respawned.
    compat: {
      term_program: "Unterm",
      default: "Unterm",
      presets: ["Unterm", "WezTerm", "Apple_Terminal", "iTerm.app", "xterm"],
      saving: false,
      appliedAt: null,
    },

    themes: [
      {
        id: 'standard',
        name: 'Standard',
        scheme: 'Catppuccin Mocha',
        desc: 'Balanced dark terminal style',
        swatches: ['#1e1e2e', '#cba6f7', '#a6e3a1', '#f9e2af', '#89b4fa'],
      },
      {
        id: 'midnight',
        name: 'Midnight',
        scheme: 'Tokyo Night',
        desc: 'Low-glare blue-black workspace',
        swatches: ['#1a1b26', '#7aa2f7', '#9ece6a', '#bb9af7', '#f7768e'],
      },
      {
        id: 'daylight',
        name: 'Daylight',
        scheme: 'Builtin Solarized Light',
        desc: 'Readable light mode for bright rooms',
        swatches: ['#fdf6e3', '#268bd2', '#859900', '#b58900', '#dc322f'],
      },
      {
        id: 'classic',
        name: 'Classic',
        scheme: 'Builtin Tango Dark',
        desc: 'Plain high-contrast terminal colors',
        swatches: ['#000000', '#3465a4', '#4e9a06', '#c4a000', '#cc0000'],
      },
    ],

    /// Lookup helper. Returns the translated string for `key` or the key
    /// itself if the dictionary doesn't carry it.
    t(key) {
      const v = this.dict[key];
      return typeof v === 'string' ? v : key;
    },

    async boot() {
      try {
        const res = await fetch('/bootstrap.json');
        const j = await res.json();
        this.token = j.auth_token || '';
      } catch (e) {
        this.toast('Could not load bootstrap.json — backend offline?', 'error');
      }
      // Load i18n state before anything else so the rest of the boot path
      // can render translated text.
      await this.loadI18n();
      await this.refresh();
      this.pollHealth();
      setInterval(() => this.pollHealth(), 5000);
    },

    async loadI18n() {
      try {
        const res = await fetch('/api/i18n', {
          headers: { Authorization: 'Bearer ' + this.token },
        });
        if (!res.ok) return;
        const j = await res.json();
        this.lang = j.current || 'en-US';
        this.dict = j.dict || {};
        this.availableLocales = j.available || this.availableLocales;
        document.documentElement.lang = this.lang;
        document.title = this.t('settings.title') || 'Unterm Settings';
      } catch (e) {
        // Fall through — t(key) returns the key itself when dict is empty.
      }
    },

    async changeLang(code) {
      try {
        await this.api('POST', '/api/i18n', { lang: code });
        this.toast(this.t('web.toast.lang_changed'));
        // Reload the SPA so every binding re-evaluates against the new dict.
        // We give the toast a moment to render before reloading.
        setTimeout(() => window.location.reload(), 250);
      } catch (e) {
        this.toast(this.t('web.toast.lang_failed').replace('{err}', e.message), 'error');
      }
    },

    async api(method, path, body) {
      const opts = {
        method,
        headers: {
          'Content-Type': 'application/json',
          Authorization: 'Bearer ' + this.token,
        },
      };
      if (body !== undefined) opts.body = JSON.stringify(body);
      const res = await fetch(path, opts);
      if (!res.ok) {
        let msg = res.status + ' ' + res.statusText;
        try { msg = (await res.json()).error || msg; } catch (e) {}
        throw new Error(msg);
      }
      const ct = res.headers.get('content-type') || '';
      if (ct.includes('application/json')) return res.json();
      return res.text();
    },

    async refresh() {
      try {
        const s = await this.api('GET', '/api/state');
        this.state = Object.assign({}, this.state, s);
        if (s.proxy) this.proxy = s.proxy;
        if (s.proxy) {
          this.proxyForm = {
            http_proxy: s.proxy.http_proxy || '',
            socks_proxy: s.proxy.socks_proxy || '',
            no_proxy: s.proxy.no_proxy || '',
          };
        }
        if (s.recording) this.recording = s.recording;
        if (s.scrollback) {
          // Don't clobber `saving` / `appliedAt` UI flags — only sync the
          // numeric fields the server is the source of truth for.
          this.scrollback.lines = s.scrollback.lines ?? this.scrollback.lines;
          this.scrollback.default = s.scrollback.default ?? this.scrollback.default;
          this.scrollback.max = s.scrollback.max ?? this.scrollback.max;
        }
      } catch (e) {
        this.toast(this.t('web.toast.refresh').replace('{err}', e.message), 'error');
      }
      await this.loadSessions();
      await this.loadCompat();
    },

    async saveScrollback() {
      // Clamp client-side so we don't fire off requests that we know the
      // server will reject — keeps the round-trip cheap and the toast
      // friendlier than raw 400s.
      const n = Math.max(this.scrollback.min,
                Math.min(this.scrollback.max, Number(this.scrollback.lines) | 0));
      this.scrollback.lines = n;
      this.scrollback.saving = true;
      try {
        const j = await this.api('POST', '/api/scrollback', { lines: n });
        this.scrollback.appliedAt = new Date().toISOString();
        this.toast(this.t('web.toast.scrollback_saved').replace('{n}', String(j.lines)));
      } catch (e) {
        this.toast(this.t('web.toast.scrollback_failed').replace('{err}', e.message), 'error');
      } finally {
        this.scrollback.saving = false;
      }
    },

    resetScrollback() {
      this.scrollback.lines = this.scrollback.default;
      this.saveScrollback();
    },

    async loadCompat() {
      try {
        const j = await this.api('GET', '/api/compat');
        this.compat.term_program = j.term_program ?? this.compat.term_program;
        this.compat.default = j.default ?? this.compat.default;
        if (Array.isArray(j.presets)) this.compat.presets = j.presets;
      } catch (e) { /* leave defaults */ }
    },

    async saveCompat() {
      const value = (this.compat.term_program || '').trim() || this.compat.default;
      this.compat.term_program = value;
      this.compat.saving = true;
      try {
        const j = await this.api('POST', '/api/compat', { term_program: value });
        this.compat.appliedAt = new Date().toISOString();
        this.toast(this.t('web.toast.compat_saved').replace('{name}', j.term_program));
      } catch (e) {
        this.toast(this.t('web.toast.compat_failed').replace('{err}', e.message), 'error');
      } finally {
        this.compat.saving = false;
      }
    },

    pickCompatPreset(name) {
      this.compat.term_program = name;
      this.saveCompat();
    },

    resetCompat() {
      this.compat.term_program = this.compat.default;
      this.saveCompat();
    },

    async pollHealth() {
      try {
        const j = await this.api('GET', '/api/health');
        this.health = { ok: !!j.ok };
      } catch (e) {
        this.health = { ok: false };
      }
    },

    async loadSessions() {
      try {
        const j = await this.api('GET', '/api/sessions');
        this.sessions = (j.sessions || []).slice().reverse();
      } catch (e) {
        this.sessions = [];
      }
    },

    select(id) {
      this.active = id;
      if (id === 'recording') this._recordingSeen = true;
    },

    async applyTheme(id) {
      try {
        await this.api('POST', '/api/theme', { name: id });
        this.state.theme = id;
        this.toast(this.t('web.toast.theme_applied').replace('{id}', id));
      } catch (e) {
        this.toast(this.t('web.toast.theme_failed').replace('{err}', e.message), 'error');
      }
    },

    async toggleProxy(enabled) {
      try {
        await this.api('POST', '/api/proxy', { enabled });
        await this.refresh();
        this.toast(enabled ? this.t('web.toast.proxy_enabled') : this.t('web.toast.proxy_disabled'));
      } catch (e) {
        this.toast(this.t('web.toast.proxy_failed').replace('{err}', e.message), 'error');
      }
    },

    async saveProxyManual() {
      try {
        await this.api('POST', '/api/proxy', {
          enabled: true,
          http_proxy: this.proxyForm.http_proxy || undefined,
          socks_proxy: this.proxyForm.socks_proxy || undefined,
          no_proxy: this.proxyForm.no_proxy || undefined,
        });
        await this.refresh();
        this.toast(this.t('web.toast.proxy_saved'));
      } catch (e) {
        this.toast(this.t('web.toast.proxy_failed').replace('{err}', e.message), 'error');
      }
    },

    async openSession(s) {
      try {
        const md = await this.api(
          'GET',
          '/api/sessions/' + encodeURIComponent(s.unterm_session_id) + '/markdown'
        );
        this.sessionMarkdown = md;
        this.currentSessionId = s.unterm_session_id;
      } catch (e) {
        this.toast(this.t('web.toast.session_failed').replace('{err}', e.message), 'error');
      }
    },

    toast(text, kind = 'ok') {
      const id = this.nextToast++;
      this.toasts.push({ id, text, kind });
      setTimeout(() => {
        this.toasts = this.toasts.filter((t) => t.id !== id);
      }, 3500);
    },
  };
}
