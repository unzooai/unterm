use super::{
    CreateSessionRequest, CursorSnapshot, PaneDimensions, ScreenLine, ScreenSnapshot,
    SessionActivitySnapshot, SessionSnapshot, ShellSnapshot, SplitDirection, SplitSessionRequest,
    TerminalEngine,
};
use anyhow::{anyhow, Context, Result};
use config::keyassignment::SpawnTabDomain;
use mux::domain::SplitSource;
use mux::pane::{CachePolicy, Pane};
use mux::tab::{SplitDirection as MuxSplitDirection, SplitRequest, SplitSize};
use mux::Mux;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default)]
pub struct WezTermEngine;

impl WezTermEngine {
    fn mux(&self) -> Result<Arc<Mux>> {
        Mux::try_get().ok_or_else(|| anyhow!("Mux not available"))
    }

    fn pane(&self, pane_id: usize) -> Result<Arc<dyn Pane>> {
        self.mux()?
            .get_pane(pane_id)
            .ok_or_else(|| anyhow!("Session {} not found", pane_id))
    }

    fn active_pane_id(&self, mux: &Mux) -> Option<usize> {
        mux.iter_windows()
            .into_iter()
            .find_map(|wid| mux.get_active_tab_for_window(wid))
            .and_then(|tab| tab.get_active_pane())
            .map(|pane| pane.pane_id())
    }

    fn cursor_snapshot(pane: &Arc<dyn Pane>) -> CursorSnapshot {
        let cursor = pane.get_cursor_position();
        CursorSnapshot {
            x: cursor.x,
            y: cursor.y,
            visible: cursor.visibility == termwiz::surface::CursorVisibility::Visible,
            shape: format!("{:?}", cursor.shape),
        }
    }

    fn dimensions(pane: &Arc<dyn Pane>) -> PaneDimensions {
        let dims = pane.get_dimensions();
        PaneDimensions {
            cols: dims.cols,
            viewport_rows: dims.viewport_rows,
            scrollback_rows: dims.scrollback_rows,
        }
    }

    fn shell_snapshot(pane: &Arc<dyn Pane>) -> ShellSnapshot {
        let process_name = pane
            .get_foreground_process_name(CachePolicy::AllowStale)
            .unwrap_or_default();

        let shell_type = if process_name.is_empty() {
            "unknown"
        } else {
            let name = process_name
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(&process_name)
                .to_lowercase();
            if name.contains("pwsh") || name.contains("powershell") {
                "powershell"
            } else if name.contains("cmd") {
                "cmd"
            } else if name.contains("bash") {
                "bash"
            } else if name.contains("zsh") {
                "zsh"
            } else if name.contains("fish") {
                "fish"
            } else if name.contains("nu") {
                "nushell"
            } else {
                "unknown"
            }
        }
        .to_string();

        let cwd = pane
            .get_current_working_dir(CachePolicy::AllowStale)
            .map(|u| u.to_string());

        ShellSnapshot {
            shell_type,
            process_name,
            cwd,
        }
    }

    fn session_snapshot(pane: &Arc<dyn Pane>, active_pane_id: Option<usize>) -> SessionSnapshot {
        let dims = Self::dimensions(pane);
        SessionSnapshot {
            id: pane.pane_id(),
            title: pane.get_title(),
            cols: dims.cols,
            rows: dims.viewport_rows,
            scrollback_rows: dims.scrollback_rows,
            cursor: Self::cursor_snapshot(pane),
            is_dead: pane.is_dead(),
            is_active: Some(pane.pane_id()) == active_pane_id,
            domain_id: pane.domain_id(),
            shell: Self::shell_snapshot(pane),
        }
    }
}

impl TerminalEngine for WezTermEngine {
    fn list_sessions(&self) -> Result<Vec<SessionSnapshot>> {
        let mux = self.mux()?;
        let active_pane_id = self.active_pane_id(&mux);

        let mut panes = mux.iter_panes();
        panes.sort_by_key(|pane| pane.pane_id());

        Ok(panes
            .iter()
            .map(|pane| Self::session_snapshot(pane, active_pane_id))
            .collect())
    }

    fn get_session(&self, pane_id: usize) -> Result<SessionSnapshot> {
        let mux = self.mux()?;
        let active_pane_id = self.active_pane_id(&mux);
        let pane = mux
            .get_pane(pane_id)
            .ok_or_else(|| anyhow!("Session {} not found", pane_id))?;
        Ok(Self::session_snapshot(&pane, active_pane_id))
    }

    fn create_session(&self, request: CreateSessionRequest) -> Result<SessionSnapshot> {
        let size = wezterm_term::TerminalSize {
            rows: request.rows,
            cols: request.cols,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 0,
        };

        let (tx, rx) = std::sync::mpsc::channel();
        promise::spawn::spawn_into_main_thread(async move {
            promise::spawn::spawn(async move {
                let result = async {
                    let mux = Mux::get();
                    let window_id = mux
                        .iter_windows()
                        .into_iter()
                        .next()
                        .ok_or_else(|| anyhow!("No windows available"))?;

                    let (_tab, pane, _wid) = mux
                        .spawn_tab_or_window(
                            Some(window_id),
                            SpawnTabDomain::DefaultDomain,
                            request.command,
                            request.command_dir,
                            size,
                            None,
                            String::new(),
                            None,
                        )
                        .await
                        .context("spawn_tab_or_window")?;

                    let active_pane_id = WezTermEngine.active_pane_id(&mux);
                    Ok::<SessionSnapshot, anyhow::Error>(WezTermEngine::session_snapshot(
                        &pane,
                        active_pane_id,
                    ))
                }
                .await;
                tx.send(result).ok();
            })
            .detach();
        })
        .detach();

        rx.recv_timeout(std::time::Duration::from_secs(10))
            .map_err(|_| anyhow!("Timeout waiting for session creation"))?
    }

    fn split_session(&self, request: SplitSessionRequest) -> Result<SessionSnapshot> {
        let (direction, target_is_second) = match request.direction {
            SplitDirection::Right => (MuxSplitDirection::Horizontal, true),
            SplitDirection::Left => (MuxSplitDirection::Horizontal, false),
            SplitDirection::Down => (MuxSplitDirection::Vertical, true),
            SplitDirection::Up => (MuxSplitDirection::Vertical, false),
        };

        let split_request = SplitRequest {
            direction,
            target_is_second,
            top_level: false,
            size: SplitSize::Percent(request.size_percent),
        };

        let (tx, rx) = std::sync::mpsc::channel();
        promise::spawn::spawn_into_main_thread(async move {
            promise::spawn::spawn(async move {
                let result = async {
                    let mux = Mux::get();
                    let (pane, _size) = mux
                        .split_pane(
                            request.source_pane_id,
                            split_request,
                            SplitSource::Spawn {
                                command: None,
                                command_dir: request.command_dir,
                            },
                            SpawnTabDomain::DefaultDomain,
                        )
                        .await
                        .context("split_pane")?;

                    let active_pane_id = WezTermEngine.active_pane_id(&mux);
                    Ok::<SessionSnapshot, anyhow::Error>(WezTermEngine::session_snapshot(
                        &pane,
                        active_pane_id,
                    ))
                }
                .await;
                tx.send(result).ok();
            })
            .detach();
        })
        .detach();

        rx.recv_timeout(std::time::Duration::from_secs(10))
            .map_err(|_| anyhow!("Timeout waiting for session.split"))?
    }

    fn focus_session(&self, pane_id: usize) -> Result<()> {
        self.mux()?
            .focus_pane_and_containing_tab(pane_id)
            .with_context(|| format!("focus pane {pane_id}"))?;
        Ok(())
    }

    fn shell(&self, pane_id: usize) -> Result<ShellSnapshot> {
        let pane = self.pane(pane_id)?;
        Ok(Self::shell_snapshot(&pane))
    }

    fn activity(&self, pane_id: usize) -> Result<SessionActivitySnapshot> {
        let shell = self.shell(pane_id)?;
        Ok(SessionActivitySnapshot {
            idle: shell.shell_type != "unknown",
            foreground_process: shell.process_name,
        })
    }

    fn read_screen(&self, pane_id: usize) -> Result<ScreenSnapshot> {
        let pane = self.pane(pane_id)?;
        let dims = pane.get_dimensions();
        let first_row = dims.physical_top;
        let last_row = first_row + dims.viewport_rows as isize;
        let (first, lines) = pane.get_lines(first_row..last_row);

        let mut text_lines = Vec::with_capacity(lines.len());
        let mut cells = Vec::with_capacity(lines.len());
        for (row_idx, line) in lines.iter().enumerate() {
            let text = line.as_str().trim_end().to_string();
            text_lines.push(text.clone());
            cells.push(ScreenLine {
                row: first as i64 + row_idx as i64,
                text,
            });
        }

        Ok(ScreenSnapshot {
            lines: text_lines,
            cells,
            cursor: Self::cursor_snapshot(&pane),
            cols: dims.cols,
            rows: dims.viewport_rows,
            scrollback_rows: dims.scrollback_rows,
        })
    }

    fn read_scrollback(&self, pane_id: usize, limit: usize) -> Result<Vec<String>> {
        let pane = self.pane(pane_id)?;
        let dims = pane.get_dimensions();
        let end = dims.physical_top;
        let start = (end - limit as isize).max(0);
        let (_first, lines) = pane.get_lines(start..end);

        Ok(lines
            .iter()
            .map(|line| line.as_str().trim_end().to_string())
            .filter(|text| !text.is_empty())
            .collect())
    }

    fn cursor(&self, pane_id: usize) -> Result<CursorSnapshot> {
        let pane = self.pane(pane_id)?;
        Ok(Self::cursor_snapshot(&pane))
    }

    fn write_input(&self, pane_id: usize, input: &str) -> Result<()> {
        let pane = self.pane(pane_id)?;
        pane.writer().write_all(input.as_bytes())?;
        Ok(())
    }

    fn resize_session(&self, pane_id: usize, cols: usize, rows: usize) -> Result<()> {
        let pane = self.pane(pane_id)?;
        let size = wezterm_term::TerminalSize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 0,
        };
        pane.resize(size)?;
        Ok(())
    }

    fn destroy_session(&self, pane_id: usize) -> Result<()> {
        let pane = self.pane(pane_id)?;
        pane.kill();
        Ok(())
    }
}
