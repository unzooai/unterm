//! A Domain represents an instance of a multiplexer.
//! For example, the gui frontend has its own domain,
//! and we can connect to a domain hosted by a mux server
//! that may be local, running "remotely" inside a WSL
//! container or actually remote, running on the other end
//! of an ssh session somewhere.

use crate::pane::{alloc_pane_id, Pane, PaneId};
use crate::tab::{SplitRequest, Tab, TabId};
use crate::window::WindowId;
use crate::Mux;
use anyhow::{bail, Context};
use async_trait::async_trait;
use config::keyassignment::{SpawnCommand, SpawnTabDomain};
use config::ExecDomain;
use downcast_rs::{impl_downcast, Downcast};
use portable_pty::CommandBuilder;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use wezterm_term::TerminalSize;

static DOMAIN_ID: ::std::sync::atomic::AtomicUsize = ::std::sync::atomic::AtomicUsize::new(0);
pub type DomainId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainState {
    Detached,
    Attached,
}

pub fn alloc_domain_id() -> DomainId {
    DOMAIN_ID.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed)
}

#[derive(Debug, Clone, PartialEq)]
pub enum SplitSource {
    Spawn {
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
    },
    MovePane(PaneId),
}

#[async_trait(?Send)]
pub trait Domain: Downcast + Send + Sync {
    /// Spawn a new command within this domain
    async fn spawn(
        &self,
        size: TerminalSize,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
        window: WindowId,
    ) -> anyhow::Result<Arc<Tab>> {
        let pane = self
            .spawn_pane(size, command, command_dir)
            .await
            .context("spawn")?;

        let tab = Arc::new(Tab::new(&size));
        tab.assign_pane(&pane);

        let mux = Mux::get();
        mux.add_tab_and_active_pane(&tab)?;
        mux.add_tab_to_window(&tab, window)?;

        Ok(tab)
    }

    async fn split_pane(
        &self,
        source: SplitSource,
        tab: TabId,
        pane_id: PaneId,
        split_request: SplitRequest,
    ) -> anyhow::Result<Arc<dyn Pane>> {
        let mux = Mux::get();
        let tab = match mux.get_tab(tab) {
            Some(t) => t,
            None => anyhow::bail!("Invalid tab id {}", tab),
        };

        let pane_index = match tab
            .iter_panes_ignoring_zoom()
            .iter()
            .find(|p| p.pane.pane_id() == pane_id)
        {
            Some(p) => p.index,
            None => anyhow::bail!("invalid pane id {}", pane_id),
        };

        let split_size = match tab.compute_split_size(pane_index, split_request) {
            Some(s) => s,
            None => anyhow::bail!("invalid pane index {}", pane_index),
        };

        let pane = match source {
            SplitSource::Spawn {
                command,
                command_dir,
            } => {
                self.spawn_pane(split_size.second, command, command_dir)
                    .await?
            }
            SplitSource::MovePane(src_pane_id) => {
                let (_domain, _window, src_tab) = mux
                    .resolve_pane_id(src_pane_id)
                    .ok_or_else(|| anyhow::anyhow!("pane {} not found", src_pane_id))?;
                let src_tab = match mux.get_tab(src_tab) {
                    Some(t) => t,
                    None => anyhow::bail!("Invalid tab id {}", src_tab),
                };

                let pane = src_tab.remove_pane(src_pane_id).ok_or_else(|| {
                    anyhow::anyhow!("pane {} not found in its containing tab!?", src_pane_id)
                })?;

                if src_tab.is_dead() {
                    mux.remove_tab(src_tab.tab_id());
                }

                pane
            }
        };

        // pane_index may have changed if src_pane was also in the same tab
        let final_pane_index = match tab
            .iter_panes_ignoring_zoom()
            .iter()
            .find(|p| p.pane.pane_id() == pane_id)
        {
            Some(p) => p.index,
            None => anyhow::bail!("invalid pane id {}", pane_id),
        };

        tab.split_and_insert(final_pane_index, split_request, Arc::clone(&pane))?;
        Ok(pane)
    }

    async fn spawn_pane(
        &self,
        size: TerminalSize,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
    ) -> anyhow::Result<Arc<dyn Pane>>;

    /// The mux will call this method on the domain of the pane that
    /// is being moved to give the domain a chance to handle the movement.
    /// If this method returns Ok(None), then the mux will handle the
    /// movement itself by mutating its local Tabs and Windows.
    async fn move_pane_to_new_tab(
        &self,
        _pane_id: PaneId,
        _window_id: Option<WindowId>,
        _workspace_for_new_window: Option<String>,
    ) -> anyhow::Result<Option<(Arc<Tab>, WindowId)>> {
        Ok(None)
    }

    /// Returns false if the `spawn` method will never succeed.
    /// There are some internal placeholder domains that are
    /// pre-created with local UI that we do not want to allow
    /// to show in the launcher/menu as launchable items.
    fn spawnable(&self) -> bool {
        true
    }

    /// Returns true if the `detach` method can be used
    /// to detach the domain, preserving the associated
    /// panes, or false if the `detach` method will never
    /// succeed
    fn detachable(&self) -> bool;

    /// Returns the domain id, which is useful for obtaining
    /// a handle on the domain later.
    fn domain_id(&self) -> DomainId;

    /// Returns the name of the domain.
    /// Should be a short identifier.
    fn domain_name(&self) -> &str;

    /// Returns a label describing the domain.
    async fn domain_label(&self) -> String {
        self.domain_name().to_string()
    }

    /// Re-attach to any tabs that might be pre-existing in this domain
    async fn attach(&self, window_id: Option<WindowId>) -> anyhow::Result<()>;

    /// Detach all tabs
    fn detach(&self) -> anyhow::Result<()>;

    /// Indicates the state of the domain
    fn state(&self) -> DomainState;
}
impl_downcast!(Domain);

/// The production local domain. Session processes and terminal state live in
/// unterm-core; the GUI only owns the Pane projection used for rendering.
pub struct CoreDomain {
    id: DomainId,
    name: String,
    base_command: Option<CommandBuilder>,
    exec_domain: Option<ExecDomain>,
    serial: Option<(String, Option<usize>)>,
}

impl CoreDomain {
    pub fn new(name: &str) -> Self {
        Self {
            id: alloc_domain_id(),
            name: name.to_string(),
            base_command: None,
            exec_domain: None,
            serial: None,
        }
    }
    pub fn with_command(name: &str, command: CommandBuilder) -> Self {
        Self {
            id: alloc_domain_id(),
            name: name.to_string(),
            base_command: Some(command),
            exec_domain: None,
            serial: None,
        }
    }
    pub fn with_exec_domain(exec_domain: ExecDomain) -> Self {
        Self {
            id: alloc_domain_id(),
            name: exec_domain.name.clone(),
            base_command: None,
            exec_domain: Some(exec_domain),
            serial: None,
        }
    }
    pub fn with_serial(name: &str, port: String, baud: Option<usize>) -> Self {
        Self {
            id: alloc_domain_id(),
            name: name.to_string(),
            base_command: None,
            exec_domain: None,
            serial: Some((port, baud)),
        }
    }
}

#[async_trait(?Send)]
impl Domain for CoreDomain {
    async fn spawn_pane(
        &self,
        size: TerminalSize,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
    ) -> anyhow::Result<Arc<dyn Pane>> {
        let pane_id = alloc_pane_id();
        let mut command = match (self.base_command.clone(), command) {
            (Some(mut base), Some(command)) => {
                base.get_argv_mut()
                    .extend(command.get_argv().iter().cloned());
                Some(base)
            }
            (Some(base), None) => Some(base),
            (None, command) => command,
        };
        if let Some(exec_domain) = &self.exec_domain {
            let mut exec_command = command.unwrap_or_else(CommandBuilder::new_default_prog);
            self.fixup_exec_command(&mut exec_command, exec_domain)
                .await?;
            command = Some(exec_command);
        }
        let extra = self
            .serial
            .as_ref()
            .map(|(port, baud)| serde_json::json!({"transport":"serial","port":port,"baud":baud}))
            .unwrap_or(serde_json::Value::Null);
        let pane = crate::corepane::CorePane::try_spawn_with_params(
            pane_id,
            self.id,
            size,
            command,
            command_dir,
            extra,
        )?
        .ok_or_else(|| anyhow::anyhow!("unterm-core is not available"))?;
        Mux::get().add_pane(&pane)?;
        Ok(pane)
    }

    fn domain_id(&self) -> DomainId {
        self.id
    }
    fn domain_name(&self) -> &str {
        &self.name
    }
    async fn attach(&self, _window_id: Option<WindowId>) -> anyhow::Result<()> {
        Ok(())
    }
    fn detachable(&self) -> bool {
        false
    }
    fn detach(&self) -> anyhow::Result<()> {
        bail!("detach not implemented for CoreDomain")
    }
    fn state(&self) -> DomainState {
        DomainState::Attached
    }
}

impl CoreDomain {
    async fn fixup_exec_command(
        &self,
        cmd: &mut CommandBuilder,
        exec_domain: &ExecDomain,
    ) -> anyhow::Result<()> {
        let args = cmd
            .get_argv()
            .iter()
            .map(|arg| {
                arg.to_str()
                    .ok_or_else(|| anyhow::anyhow!("command argument is not utf8"))
                    .map(str::to_owned)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let mut env = HashMap::new();
        for (key, value) in cmd.iter_full_env_as_str() {
            env.insert(key.to_string(), value.to_string());
        }
        let cwd = cmd.get_cwd().map(PathBuf::from);
        let spawn_command = SpawnCommand {
            label: None,
            domain: SpawnTabDomain::DomainName(exec_domain.name.clone()),
            args: (!args.is_empty()).then_some(args),
            set_environment_variables: env,
            cwd,
            position: None,
        };
        let fixed: SpawnCommand = config::with_lua_config_on_main_thread(|lua| async {
            let lua = lua.ok_or_else(|| anyhow::anyhow!("missing lua context"))?;
            let value = config::lua::emit_async_callback(
                &*lua,
                (exec_domain.fixup_command.clone(), (spawn_command.clone())),
            )
            .await?;
            luahelper::from_lua_value_dynamic(value).with_context(|| {
                format!(
                    "interpreting SpawnCommand from ExecDomain {}",
                    exec_domain.name
                )
            })
        })
        .await?;
        cmd.get_argv_mut().clear();
        if let Some(args) = fixed.args {
            cmd.get_argv_mut()
                .extend(args.into_iter().map(OsString::from));
        }
        cmd.env_clear();
        for (key, value) in fixed.set_environment_variables {
            cmd.env(key, value);
        }
        cmd.clear_cwd();
        if let Some(cwd) = fixed.cwd {
            cmd.cwd(cwd);
        }
        Ok(())
    }
}
