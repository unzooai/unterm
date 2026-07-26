use super::{
    CaptureEngine, CellStyle, CreateSessionRequest, CursorSnapshot, EngineHealthSnapshot,
    HealthEngine, InputEngine, PaneDimensions, PaneLocation, RecordingEngine,
    RecordingExportResult, RecordingStartResult, RecordingStatusSnapshot, RecordingStopResult,
    RenderedScrollbackPng, ScreenEngine, ScreenLine, ScreenSearchMatch, ScreenSnapshot,
    ScrollbackTextRequest, ScrollbackTextSnapshot, SessionActivitySnapshot, SessionEngine,
    SessionSnapshot, ShellSnapshot, SplitDirection, SplitSessionRequest, StyledCell,
    StyledScreenLine, StyledScreenSnapshot, ViewportScrollResult, WindowEngine, WindowFocusResult,
};
use anyhow::{anyhow, Context, Result};
use config::keyassignment::SpawnTabDomain;
use mux::domain::SplitSource;
use mux::pane::{CachePolicy, Pane};
use mux::tab::{SplitDirection as MuxSplitDirection, SplitRequest, SplitSize};
use mux::Mux;
use std::collections::HashMap;
use std::path::Path;
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

impl SessionEngine for WezTermEngine {
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

    fn resize_session(&self, pane_id: usize, cols: usize, rows: usize) -> Result<()> {
        let mux = self.mux()?;
        // A pane that is tiled inside a GUI window gets its geometry from
        // the window size and split layout; resizing only the PTY leaves
        // the model at one size and the visible grid at another.
        let in_gui_layout = mux.iter_windows().into_iter().any(|wid| {
            mux.get_window(wid)
                .map(|window| window.iter().any(|tab| tab.contains_pane(pane_id)))
                .unwrap_or(false)
        });
        if in_gui_layout {
            return Err(anyhow!(
                "Session {} is laid out by the GUI window; its size follows \
                 the window and splits. Resize the window or adjust the \
                 split instead.",
                pane_id
            ));
        }

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

impl ScreenEngine for WezTermEngine {
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
            revision: 0,
            dirty_rows: None,
        })
    }

    fn read_styled_screen(&self, pane_id: usize) -> Result<StyledScreenSnapshot> {
        let screen = self.read_screen(pane_id)?;
        let lines = screen
            .cells
            .iter()
            .map(|line| StyledScreenLine {
                row: line.row,
                cells: line
                    .text
                    .chars()
                    .map(|ch| {
                        let mut buf = [0u8; 4];
                        StyledCell {
                            ch,
                            style: CellStyle::default(),
                            width: termwiz::cell::unicode_column_width(
                                ch.encode_utf8(&mut buf),
                                None,
                            ),
                        }
                    })
                    .collect(),
            })
            .collect();

        Ok(StyledScreenSnapshot {
            lines,
            cursor: screen.cursor,
            cols: screen.cols,
            rows: screen.rows,
            scrollback_rows: screen.scrollback_rows,
            revision: screen.revision,
            dirty_rows: screen.dirty_rows,
        })
    }

    fn read_visible_text(&self, pane_id: usize) -> Result<String> {
        Ok(self.read_screen(pane_id)?.lines.join("\n"))
    }

    fn read_lines(&self, pane_id: usize, start: i64, count: usize) -> Result<Vec<ScreenLine>> {
        let pane = self.pane(pane_id)?;
        let start = start as isize;
        let end = start + count as isize;
        let (first, lines) = pane.get_lines(start..end);

        Ok(lines
            .iter()
            .enumerate()
            .map(|(idx, line)| ScreenLine {
                row: first as i64 + idx as i64,
                text: line.as_str().trim_end().to_string(),
            })
            .collect())
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

    fn read_scrollback_text(
        &self,
        pane_id: usize,
        request: ScrollbackTextRequest,
    ) -> Result<ScrollbackTextSnapshot> {
        let pane = self.pane(pane_id)?;
        let dims = pane.get_dimensions();
        let viewport_bottom = dims.physical_top + dims.viewport_rows as isize;
        let mut start = request
            .start_line
            .map(|n| n as isize)
            .unwrap_or(dims.scrollback_top)
            .max(dims.scrollback_top);
        let end = request
            .end_line
            .map(|n| n as isize)
            .unwrap_or(viewport_bottom)
            .min(viewport_bottom);

        if let Some(tail) = request.tail_lines {
            if tail > 0 {
                start = start.max(end.saturating_sub(tail as isize));
            }
        }

        if end <= start {
            return Ok(ScrollbackTextSnapshot {
                text: String::new(),
                lines: Vec::new(),
                first_row: start as i64,
                row_count: 0,
                cols: dims.cols,
                escapes: request.escapes,
                scrollback_top: dims.scrollback_top as i64,
                physical_top: dims.physical_top as i64,
                viewport_rows: dims.viewport_rows,
            });
        }

        let (first, lines) = pane.get_lines(start..end);
        if request.escapes {
            let text = termwiz_funcs::lines_to_escapes(lines).map_err(|e| anyhow!(e))?;
            Ok(ScrollbackTextSnapshot {
                text,
                lines: Vec::new(),
                first_row: first as i64,
                row_count: (end - start) as i64,
                cols: dims.cols,
                escapes: true,
                scrollback_top: dims.scrollback_top as i64,
                physical_top: dims.physical_top as i64,
                viewport_rows: dims.viewport_rows,
            })
        } else {
            let text_lines: Vec<String> = lines
                .iter()
                .map(|line| line.as_str().trim_end().to_string())
                .collect();
            let text = text_lines.join("\n");
            Ok(ScrollbackTextSnapshot {
                text,
                lines: text_lines,
                first_row: first as i64,
                row_count: (end - start) as i64,
                cols: dims.cols,
                escapes: false,
                scrollback_top: dims.scrollback_top as i64,
                physical_top: dims.physical_top as i64,
                viewport_rows: dims.viewport_rows,
            })
        }
    }

    fn search(
        &self,
        pane_id: usize,
        pattern: &str,
        max_results: usize,
    ) -> Result<Vec<ScreenSearchMatch>> {
        let pane = self.pane(pane_id)?;
        let dims = pane.get_dimensions();
        let start = dims.scrollback_top;
        let end = dims.physical_top + dims.viewport_rows as isize;
        let (first, lines) = pane.get_lines(start..end);

        let mut matches = Vec::new();
        for (idx, line) in lines.iter().enumerate() {
            let text = line.as_str().to_string();
            if let Some(byte_off) = text.find(pattern) {
                matches.push(ScreenSearchMatch {
                    row: first as i64 + idx as i64,
                    col: text[..byte_off].chars().count(),
                    text: text.trim_end().to_string(),
                });
                if matches.len() >= max_results {
                    break;
                }
            }
        }

        Ok(matches)
    }

    fn cursor(&self, pane_id: usize) -> Result<CursorSnapshot> {
        let pane = self.pane(pane_id)?;
        Ok(Self::cursor_snapshot(&pane))
    }
}

impl InputEngine for WezTermEngine {
    fn write_input(&self, pane_id: usize, input: &str) -> Result<()> {
        let pane = self.pane(pane_id)?;
        pane.writer().write_all(input.as_bytes())?;
        Ok(())
    }

    fn paste_input(&self, pane_id: usize, text: &str) -> Result<()> {
        let pane = self.pane(pane_id)?;
        pane.send_paste(text)
    }
}

impl RecordingEngine for WezTermEngine {
    fn start_recording(&self, pane_id: usize) -> Result<RecordingStartResult> {
        let result = crate::recording::start_recording(pane_id as mux::pane::PaneId)?;
        Ok(RecordingStartResult {
            session_id: result.session_id,
            log_path: result.log_path,
            md_path: result.md_path,
        })
    }

    fn stop_recording(&self, pane_id: usize) -> Result<RecordingStopResult> {
        let result = crate::recording::stop_recording(pane_id as mux::pane::PaneId)?;
        Ok(RecordingStopResult {
            session_id: result.session_id,
            ended_at: result.ended_at,
            block_count: result.block_count,
            exit_reason: result.exit_reason,
            md_path: result.md_path,
        })
    }

    fn recording_status(&self, pane_id: usize) -> Result<RecordingStatusSnapshot> {
        Ok(crate::recording::recording_status_snapshot(
            pane_id as mux::pane::PaneId,
        ))
    }

    fn attach_recording_trace(&self, pane_id: usize, trace_id: String) -> Result<Vec<String>> {
        crate::recording::attach_trace(pane_id as mux::pane::PaneId, trace_id)
    }

    fn export_markdown(
        &self,
        pane_id: usize,
        target_path: Option<String>,
    ) -> Result<RecordingExportResult> {
        let result = crate::recording::export_active_recording_markdown(
            pane_id as mux::pane::PaneId,
            target_path.map(std::path::PathBuf::from),
        )?;
        Ok(RecordingExportResult {
            session_id: result.session_id,
            path: result.path,
            bytes: result.bytes,
            block_count: result.block_count,
        })
    }
}

impl HealthEngine for WezTermEngine {
    fn health(&self) -> Result<EngineHealthSnapshot> {
        let mux = Mux::try_get();
        let pane_count = mux.as_ref().map(|mux| mux.iter_panes().len());
        let ready = mux.is_some();
        Ok(EngineHealthSnapshot {
            engine: "wezterm".to_string(),
            ready,
            status: if ready { "ok" } else { "degraded" }.to_string(),
            detail: if ready {
                "WezTerm mux is available".to_string()
            } else {
                "WezTerm mux is not available".to_string()
            },
            pane_count,
        })
    }
}

impl WindowEngine for WezTermEngine {
    fn focus_current_instance_window(&self) -> Result<WindowFocusResult> {
        anyhow::bail!("WezTermEngine window focusing is provided by CurrentTerminalEngine")
    }

    fn active_pane_id(&self) -> Result<Option<u64>> {
        let mux = self.mux()?;
        Ok(WezTermEngine::active_pane_id(self, &mux).map(|pane_id| pane_id as u64))
    }

    fn pane_locations(&self) -> Result<HashMap<u64, PaneLocation>> {
        let mux = self.mux()?;
        let mut locations = HashMap::new();
        for pane in mux.iter_panes() {
            let pane_id = pane.pane_id();
            if let Some((_domain, window_id, tab_id)) =
                mux.resolve_pane_id(pane_id as mux::pane::PaneId)
            {
                locations.insert(pane_id as u64, PaneLocation { window_id, tab_id });
            }
        }
        Ok(locations)
    }

    fn scroll_viewport_to(&self, pane_id: usize, target: isize) -> Result<ViewportScrollResult> {
        let mux = self.mux()?;
        let (_domain, mux_window_id, _tab) = mux
            .resolve_pane_id(pane_id)
            .ok_or_else(|| anyhow!("pane {pane_id} not found in any window"))?;

        let (tx, rx) = std::sync::mpsc::channel();
        promise::spawn::spawn_into_main_thread(async move {
            let result = (|| -> Result<()> {
                use ::window::WindowOps;
                let gui = crate::frontend::front_end()
                    .gui_window_for_mux_window(mux_window_id)
                    .ok_or_else(|| anyhow!("no GUI window for mux window {mux_window_id}"))?;
                gui.window
                    .notify(crate::termwindow::TermWindowNotif::Apply(Box::new(
                        move |term_window| {
                            if let Some(pane) = Mux::get().get_pane(pane_id) {
                                let dims = pane.get_dimensions();
                                let top = (target - dims.viewport_rows as isize / 4)
                                    .max(dims.scrollback_top);
                                term_window.set_viewport(pane_id, Some(top), dims);
                            }
                        },
                    )));
                Ok(())
            })();
            tx.send(result).ok();
        })
        .detach();

        rx.recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| anyhow!("timeout scrolling pane {pane_id} to row {target}"))??;
        Ok(ViewportScrollResult::Scrolled)
    }
}

impl CaptureEngine for WezTermEngine {
    fn render_scrollback_png(
        &self,
        pane_id: Option<usize>,
        path: &Path,
        opts: &crate::scrollshot::ScrollbackPngOptions,
    ) -> Result<RenderedScrollbackPng> {
        let pane = if let Some(pane_id) = pane_id {
            self.pane(pane_id)?
        } else {
            let mux = self.mux()?;
            let pane_id = self
                .active_pane_id(&mux)
                .ok_or_else(|| anyhow!("no active pane available"))?;
            self.pane(pane_id)?
        };
        let session_id = pane.pane_id();
        let image = crate::scrollshot::render_scrollback_png(&pane, path, opts)?;
        Ok(RenderedScrollbackPng { image, session_id })
    }
}
