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

    // Identity profiles state. Backed by /api/profile/*. Loaded on first
    // visit to the Profiles tab (lazy — most users won't touch profiles).
    profiles: {
      loaded: false,
      loading: false,
      list: [],
      defaultId: null,
      sshInclude: { installed: false, user_config_path: '', include_path: '' },
      newForm: { open: false, display_name: '', accent_color: '#10b981' },
      expandedId: null,
      // Per-profile inline forms keyed by profile id. We keep them
      // distinct per profile rather than a single global so expanding
      // two cards doesn't crosswire their inputs.
      secretForms: {},    // id -> { env_name: '', value: '' }
      gitForms: {},       // id -> { user_name: '', user_email: '' }
      sshForms: {},       // id -> { host: '', key_path: '' }
    },

    // Onboarding wizard state. Used by both first-launch detection and
    // the manual "Run wizard" button. One-step UI: scan, present
    // candidates with checkboxes + paste boxes, click "Create".
    wizard: {
      active: false,
      loading: false,
      candidates: [],
      // Index → boolean. Default true for candidates with values, false
      // for those needing manual paste (avoids accidental empty submit).
      selected: {},
      manualValues: {},   // index → user-entered value for has_value=false candidates
      profileName: 'Personal',
      accentColor: '#10b981',
    },
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
        { id: 'profiles', label: this.t('web.nav.profiles') },
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

    // Update check state — populated from /api/updates which reads
    // ~/.unterm/update_check.json. The background poller writes that
    // file every 6 h; we just surface it. `dismissed` is a session-local
    // flag (sessionStorage) so the user can hush the banner for one
    // browser session without clobbering the underlying disk state —
    // next refresh / next deploy / next manual check brings it back.
    updates: {
      upgrade_available: false,
      latest_tag: "",
      current_pkg: "",
      checked_at: "",
      dismissed: false,
      checking: false,
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
      await this.loadUpdates();
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

    async loadUpdates() {
      try {
        const j = await this.api('GET', '/api/updates');
        this.updates.upgrade_available = !!j.upgrade_available;
        this.updates.latest_tag = j.latest_tag ?? '';
        this.updates.current_pkg = j.current_pkg ?? '';
        this.updates.checked_at = j.checked_at ?? '';
        // Honor session-scoped dismissal (clicked × on the banner this
        // browser session). It resets on full reload — intentional, so
        // the user can't permanently silence themselves out of seeing
        // future versions.
        if (sessionStorage.getItem('unterm_update_dismissed') === this.updates.latest_tag) {
          this.updates.dismissed = true;
          this.updates.upgrade_available = false;
        }
      } catch (e) {
        // network blip on first load — leave defaults, don't toast spam
      }
    },

    async checkUpdatesNow() {
      this.updates.checking = true;
      try {
        const j = await this.api('POST', '/api/updates/check');
        this.updates.upgrade_available = !!j.upgrade_available;
        this.updates.latest_tag = j.latest_tag ?? '';
        this.updates.current_pkg = j.current_pkg ?? '';
        this.updates.checked_at = j.checked_at ?? '';
        this.updates.dismissed = false; // manual recheck unhushes
        sessionStorage.removeItem('unterm_update_dismissed');
        const msg = this.updates.upgrade_available
          ? this.t('web.toast.update_available').replace('{tag}', this.updates.latest_tag)
          : this.t('web.toast.update_uptodate');
        this.toast(msg);
      } catch (e) {
        this.toast(this.t('web.toast.update_failed').replace('{err}', e.message), 'error');
      } finally {
        this.updates.checking = false;
      }
    },

    dismissUpdate() {
      this.updates.dismissed = true;
      // Pin the dismissal to the specific tag — if a yet-newer version
      // arrives later, the banner re-emerges.
      sessionStorage.setItem('unterm_update_dismissed', this.updates.latest_tag);
      this.updates.upgrade_available = false;
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
      // Lazy-load profiles on first visit so users who never touch the
      // tab don't pay for the registry-load + sniffer scan.
      if (id === 'profiles' && !this.profiles.loaded) this.loadProfiles();
    },

    // ---- Identity profiles ----
    //
    // All mutations re-fetch the full list afterward rather than
    // optimistically patching local state. Profile data is small (one
    // TOML file per profile) and the registry-load on the server side
    // is fast, so the safety of "what's displayed always matches what's
    // on disk" wins over micro-optimization.

    async loadProfiles() {
      this.profiles.loading = true;
      try {
        const [data, ssh] = await Promise.all([
          this.api('GET', '/api/profile/list'),
          this.api('GET', '/api/profile/ssh-include-status'),
        ]);
        this.profiles.list = data.profiles || [];
        this.profiles.defaultId = data.default;
        this.profiles.sshInclude = ssh;
        // Seed inline forms for each profile so x-model bindings have
        // something to write into. Missing entries → empty defaults.
        for (const p of this.profiles.list) {
          if (!this.profiles.secretForms[p.id]) this.profiles.secretForms[p.id] = { env_name: '', value: '' };
          if (!this.profiles.gitForms[p.id]) this.profiles.gitForms[p.id] = { user_name: p.git.user_name, user_email: p.git.user_email };
          if (!this.profiles.sshForms[p.id]) this.profiles.sshForms[p.id] = { host: '', key_path: '' };
        }
        this.profiles.loaded = true;
      } catch (e) {
        this.toast(this.t('web.toast.profile_load_failed').replace('{err}', e.message), 'error');
      } finally {
        this.profiles.loading = false;
      }
    },

    async createProfile() {
      const dn = this.profiles.newForm.display_name.trim();
      if (!dn) {
        this.toast(this.t('web.profiles.toast.name_required'), 'error');
        return;
      }
      try {
        await this.api('POST', '/api/profile/create', {
          display_name: dn,
          accent_color: this.profiles.newForm.accent_color || undefined,
        });
        this.profiles.newForm = { open: false, display_name: '', accent_color: '#10b981' };
        await this.loadProfiles();
        this.toast(this.t('web.profiles.toast.created').replace('{name}', dn));
      } catch (e) {
        this.toast(this.t('web.profiles.toast.create_failed').replace('{err}', e.message), 'error');
      }
    },

    async deleteProfile(id, displayName) {
      if (!confirm(this.t('web.profiles.confirm.delete').replace('{name}', displayName))) return;
      try {
        await this.api('DELETE', '/api/profile/' + encodeURIComponent(id));
        await this.loadProfiles();
        this.toast(this.t('web.profiles.toast.deleted').replace('{name}', displayName));
      } catch (e) {
        this.toast(this.t('web.profiles.toast.delete_failed').replace('{err}', e.message), 'error');
      }
    },

    async setProfileAsDefault(id) {
      try {
        await this.api('POST', '/api/profile/set-default', { id });
        await this.loadProfiles();
        this.toast(this.t('web.profiles.toast.default_set'));
      } catch (e) {
        this.toast(this.t('web.profiles.toast.default_failed').replace('{err}', e.message), 'error');
      }
    },

    async addSecret(id) {
      const f = this.profiles.secretForms[id];
      if (!f || !f.env_name.trim() || !f.value) {
        this.toast(this.t('web.profiles.toast.secret_required'), 'error');
        return;
      }
      try {
        await this.api('POST', '/api/profile/' + encodeURIComponent(id) + '/secret', {
          env_name: f.env_name.trim(),
          value: f.value,
        });
        this.profiles.secretForms[id] = { env_name: '', value: '' };
        await this.loadProfiles();
        this.toast(this.t('web.profiles.toast.secret_saved'));
      } catch (e) {
        this.toast(this.t('web.profiles.toast.secret_failed').replace('{err}', e.message), 'error');
      }
    },

    async deleteSecret(id, envName) {
      if (!confirm(this.t('web.profiles.confirm.secret_delete').replace('{env}', envName))) return;
      try {
        await this.api(
          'DELETE',
          '/api/profile/' + encodeURIComponent(id) + '/secret/' + encodeURIComponent(envName),
        );
        await this.loadProfiles();
      } catch (e) {
        this.toast(this.t('web.profiles.toast.secret_failed').replace('{err}', e.message), 'error');
      }
    },

    async saveGitIdentity(id) {
      const g = this.profiles.gitForms[id] || {};
      try {
        await this.api('PUT', '/api/profile/' + encodeURIComponent(id), {
          git: { user_name: g.user_name || '', user_email: g.user_email || '' },
        });
        await this.loadProfiles();
        this.toast(this.t('web.profiles.toast.git_saved'));
      } catch (e) {
        this.toast(this.t('web.profiles.toast.git_failed').replace('{err}', e.message), 'error');
      }
    },

    async addSshRoute(id, profile) {
      const f = this.profiles.sshForms[id];
      if (!f || !f.host.trim() || !f.key_path.trim()) {
        this.toast(this.t('web.profiles.toast.ssh_required'), 'error');
        return;
      }
      const merged = Object.assign({}, profile.ssh, { [f.host.trim()]: f.key_path.trim() });
      try {
        await this.api('PUT', '/api/profile/' + encodeURIComponent(id), { ssh: merged });
        this.profiles.sshForms[id] = { host: '', key_path: '' };
        await this.loadProfiles();
        this.toast(this.t('web.profiles.toast.ssh_saved'));
      } catch (e) {
        this.toast(this.t('web.profiles.toast.ssh_failed').replace('{err}', e.message), 'error');
      }
    },

    async removeSshRoute(id, profile, host) {
      const next = Object.assign({}, profile.ssh);
      delete next[host];
      try {
        await this.api('PUT', '/api/profile/' + encodeURIComponent(id), { ssh: next });
        await this.loadProfiles();
      } catch (e) {
        this.toast(this.t('web.profiles.toast.ssh_failed').replace('{err}', e.message), 'error');
      }
    },

    async installSshInclude() {
      try {
        const r = await this.api('POST', '/api/profile/install-ssh-include');
        await this.loadProfiles();
        this.toast(
          r.already_present
            ? this.t('web.profiles.toast.ssh_include_already')
            : this.t('web.profiles.toast.ssh_include_installed'),
        );
      } catch (e) {
        this.toast(this.t('web.profiles.toast.ssh_include_failed').replace('{err}', e.message), 'error');
      }
    },

    // ---- Onboarding wizard ----

    async runImportWizard() {
      this.wizard.active = true;
      this.wizard.loading = true;
      this.wizard.candidates = [];
      this.wizard.selected = {};
      this.wizard.manualValues = {};
      try {
        const r = await this.api('GET', '/api/profile/import-scan');
        this.wizard.candidates = r.candidates || [];
        // Default-select candidates whose values we can extract; leave
        // manual-paste candidates unchecked so the user has to opt in.
        for (let i = 0; i < this.wizard.candidates.length; i++) {
          this.wizard.selected[i] = this.wizard.candidates[i].has_value;
        }
      } catch (e) {
        this.toast(this.t('web.profiles.toast.scan_failed').replace('{err}', e.message), 'error');
      } finally {
        this.wizard.loading = false;
      }
    },

    cancelWizard() {
      this.wizard.active = false;
    },

    async finalizeWizard() {
      const profileName = this.wizard.profileName.trim() || 'Personal';
      const picks = this.wizard.candidates
        .map((c, i) => ({ c, i }))
        .filter(({ i }) => this.wizard.selected[i]);

      if (picks.length === 0) {
        this.toast(this.t('web.profiles.wizard.toast.nothing_selected'), 'error');
        return;
      }

      // Validate: every selected has-no-value candidate needs a manual
      // value the user pasted in. Surface ALL missing in one go so they
      // can fill them and resubmit, rather than fighting one at a time.
      const missing = picks.filter(({ c, i }) => !c.has_value && !this.wizard.manualValues[i]);
      if (missing.length > 0) {
        this.toast(
          this.t('web.profiles.wizard.toast.missing_value').replace('{count}', missing.length),
          'error',
        );
        return;
      }

      try {
        // Step 1: create the profile.
        const created = await this.api('POST', '/api/profile/create', {
          display_name: profileName,
          accent_color: this.wizard.accentColor,
        });
        const id = created.id;

        // Step 2: for each picked candidate that has a value, we have
        // to fetch it from the server now (the scan API doesn't return
        // raw values for safety). For has_value=false, use the manual
        // paste. For has_value=true, run a single-source rescan and
        // match by env_name to pull the value out.
        // Simplification: in the SPA, has_value=true candidates send
        // env_name to the server which re-runs the matching sniffer
        // and stores the value directly (server already has access).
        // For now, since /api/profile/import-scan doesn't expose
        // values, has_value=true picks need a follow-up "import-fetch"
        // call. Wire that in when the time comes. v1 simpler path:
        // ONLY accept manual paste, so we don't need the value-fetch
        // endpoint. has_value=true serves as a discovery hint only.
        for (const { c, i } of picks) {
          if (!this.wizard.manualValues[i]) continue; // need a value to store
          await this.api('POST', '/api/profile/' + encodeURIComponent(id) + '/secret', {
            env_name: c.suggested_env_name,
            value: this.wizard.manualValues[i],
          });
        }

        // Step 3: for SSH-type candidates, also write the host→key
        // routing into the profile's [ssh] table.
        const sshPicks = picks.filter(({ c }) => c.source === 'ssh' && c.host);
        if (sshPicks.length > 0) {
          const sshMap = {};
          for (const { c } of sshPicks) {
            // c.label includes the key path after the arrow; pull it.
            const m = c.label.match(/→\s+(.+)$/);
            if (m && c.host) sshMap[c.host] = m[1];
          }
          if (Object.keys(sshMap).length > 0) {
            await this.api('PUT', '/api/profile/' + encodeURIComponent(id), { ssh: sshMap });
          }
        }

        this.wizard.active = false;
        await this.loadProfiles();
        this.toast(
          this.t('web.profiles.wizard.toast.success').replace('{name}', profileName),
        );
      } catch (e) {
        this.toast(this.t('web.profiles.wizard.toast.failed').replace('{err}', e.message), 'error');
      }
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
