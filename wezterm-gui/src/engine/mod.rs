//! GUI adapter for the engine-neutral terminal layer.
//!
//! The neutral traits and next-core implementation live in `unterm-engine`.
//! This module keeps the current WezTerm adapter available to GUI callers while
//! letting product services migrate away from WezTerm internals.

pub mod render_backend;
pub mod render_consumer;
pub mod wezterm;

use std::collections::HashMap;
use std::path::Path;
use window::WindowOps;

#[allow(unused_imports)]
pub use render_backend::{
    CommandListRenderBackend, EngineRenderBackend, EngineRenderBackendCommand,
    EngineRenderBackendFrame, EngineRenderBufferPlan, EngineRenderGpuUploadPlan,
    EngineRenderGpuVertex, EngineRenderVertexLayer, EngineWgpuPipelineConfig,
    EngineWgpuRenderBackend, EngineWgpuRenderPassPlan,
};
#[allow(unused_imports)]
pub use render_consumer::{
    EngineRenderBufferBatch, EngineRenderCommitBatch, EngineRenderCommitStats, EngineRenderConsumer,
};

#[allow(unused_imports)]
pub use unterm_engine::{
    next_core, CellStyle, CreateSessionRequest, CursorSnapshot, DirtyRows, EngineHealthSnapshot,
    HealthEngine, InputEngine, LaunchEnvBinding, LaunchEnvSource, LaunchPolicyDecision,
    LaunchPolicyDecisionSnapshot, LaunchPolicySnapshot, PaneDimensions, RecordingEngine,
    RecordingExportResult, RecordingStartResult, RecordingStatusSnapshot, RecordingStopResult,
    RenderBackgroundQuad, RenderCellMetrics, RenderCellRun, RenderCellRunGeometry,
    RenderCommitPlan, RenderConsumerState, RenderCursorDraw, RenderCursorGeometry,
    RenderCursorQuad, RenderDrawPlan, RenderFrameSnapshot, RenderGeometryPlan, RenderGlyphRun,
    RenderGlyphRunGeometry, RenderRect, RenderSubmissionPlan, RenderTextRun, ScreenEngine,
    ScreenLine, ScreenSearchMatch, ScreenSnapshot, ScrollbackTextRequest, ScrollbackTextSnapshot,
    SessionActivitySnapshot, SessionEngine, SessionSnapshot, ShellSnapshot, SplitDirection,
    SplitSessionRequest, StyledCell, StyledColor, StyledScreenLine, StyledScreenSnapshot,
    StyledScrollbackSnapshot, TerminalEngine,
};

#[derive(Clone, Debug)]
pub struct WindowFocusResult {
    pub mux_window_id: usize,
    pub window_engine: &'static str,
    pub uses_host_window: bool,
}

#[derive(Clone, Debug)]
pub struct WindowTitleResult {
    pub title: Option<String>,
    pub window_engine: &'static str,
    pub title_owner: &'static str,
    pub metadata_owner: &'static str,
    pub native_window_lifecycle: &'static str,
    pub applied_to_native_window: bool,
    pub uses_host_window: bool,
}

#[derive(Clone, Debug)]
pub struct PaneLocation {
    pub window_id: usize,
    pub tab_id: usize,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum ViewportScrollResult {
    Scrolled,
    Unsupported { reason: String },
}

pub trait WindowEngine {
    fn focus_current_instance_window(&self) -> anyhow::Result<WindowFocusResult>;
    fn set_current_instance_title(
        &self,
        title: Option<String>,
    ) -> anyhow::Result<WindowTitleResult>;
    fn active_pane_id(&self) -> anyhow::Result<Option<u64>>;
    fn pane_locations(&self) -> anyhow::Result<HashMap<u64, PaneLocation>>;
    fn scroll_viewport_to(
        &self,
        pane_id: usize,
        target: isize,
    ) -> anyhow::Result<ViewportScrollResult>;
}

pub struct RenderedScrollbackPng {
    pub image: crate::scrollshot::ScrollbackPng,
    pub session_id: usize,
    pub renderer: serde_json::Value,
}

pub fn wezterm_scrollback_renderer_metadata() -> serde_json::Value {
    serde_json::json!({
        "engine": "wezterm",
        "renderer": "wezterm-pane",
        "source": "pane-styled-cells",
        "uses_wezterm_pane": true,
        "standalone": false,
        "styled": true,
        "palette": "pane-resolved",
        "supported_styles": [
            "fg",
            "bg",
            "inverse",
            "underline",
            "bold",
            "italic",
            "theme_palette",
        ],
        "missing_parity": [],
    })
}

pub fn next_core_scrollback_renderer_metadata() -> serde_json::Value {
    serde_json::json!({
        "engine": "next-core",
        "renderer": "standalone-styled",
        "source": "engine-styled-scrollback",
        "uses_wezterm_pane": false,
        "standalone": true,
        "styled": true,
        "palette": "config-resolved",
        "supported_styles": [
            "fg",
            "bg",
            "palette_index",
            "rgb",
            "inverse",
            "underline",
            "theme_palette",
            "bold",
            "italic",
        ],
        "missing_parity": [],
    })
}

pub trait CaptureEngine {
    fn capture_screen_image(&self, include_base64: bool) -> anyhow::Result<serde_json::Value>;
    fn capture_window_image(
        &self,
        title_filter: Option<&str>,
        pid_filter: Option<u32>,
        include_base64: bool,
    ) -> anyhow::Result<serde_json::Value>;

    fn render_scrollback_png(
        &self,
        pane_id: Option<usize>,
        path: &Path,
        opts: &crate::scrollshot::ScrollbackPngOptions,
    ) -> anyhow::Result<RenderedScrollbackPng>;
}

#[derive(Clone, Copy, Debug)]
pub enum CurrentTerminalEngine {
    WezTerm(wezterm::WezTermEngine),
    NextCore(unterm_engine::next_core::NextCoreEngine),
}

fn selected_engine_name_from_env(value: Option<&str>) -> &'static str {
    match value.map(str::trim) {
        Some(value) if value.eq_ignore_ascii_case("next-core") => "next-core",
        _ => "wezterm",
    }
}

pub fn selected_engine_name() -> &'static str {
    selected_engine_name_from_env(std::env::var("UNTERM_ENGINE").ok().as_deref())
}

pub fn current() -> CurrentTerminalEngine {
    match selected_engine_name() {
        "next-core" => CurrentTerminalEngine::NextCore(next_core()),
        _ => CurrentTerminalEngine::WezTerm(wezterm::WezTermEngine),
    }
}

impl CurrentTerminalEngine {
    pub fn name(&self) -> &'static str {
        match self {
            Self::WezTerm(_) => "wezterm",
            Self::NextCore(_) => "next-core",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        next_core, selected_engine_name_from_env, CommandListRenderBackend, CreateSessionRequest,
        CurrentTerminalEngine, EngineRenderBackend, EngineRenderBackendCommand,
        EngineRenderBufferBatch, EngineRenderBufferPlan, EngineRenderConsumer,
        EngineRenderGpuUploadPlan, EngineRenderGpuVertex, EngineRenderVertexLayer,
        EngineWgpuPipelineConfig, EngineWgpuRenderBackend, EngineWgpuRenderPassPlan,
        LaunchPolicySnapshot, RenderCellMetrics, RenderConsumerState, ScreenEngine, SessionEngine,
    };

    #[test]
    fn selects_wezterm_by_default() {
        assert_eq!(selected_engine_name_from_env(None), "wezterm");
        assert_eq!(selected_engine_name_from_env(Some("")), "wezterm");
        assert_eq!(selected_engine_name_from_env(Some("wezterm")), "wezterm");
    }

    #[test]
    fn selects_next_core_from_env() {
        assert_eq!(
            selected_engine_name_from_env(Some("next-core")),
            "next-core"
        );
        assert_eq!(
            selected_engine_name_from_env(Some("NEXT-CORE")),
            "next-core"
        );
        assert_eq!(
            selected_engine_name_from_env(Some(" next-core ")),
            "next-core"
        );
    }

    #[test]
    fn next_core_facade_reads_render_commit_plan() {
        let engine = CurrentTerminalEngine::NextCore(next_core());
        let session = engine
            .create_session(CreateSessionRequest {
                cols: 20,
                rows: 4,
                command_dir: None,
                command: Some(quiet_wait_command_for_test()),
                env: Vec::new(),
                launch_policy: LaunchPolicySnapshot::default(),
            })
            .expect("create next-core session");
        let mut consumer = RenderConsumerState::new();

        let first = engine
            .read_render_commit_plan(
                session.id,
                RenderCellMetrics {
                    cell_width_px: 8,
                    cell_height_px: 16,
                },
                &mut consumer,
            )
            .expect("read render commit plan through facade");

        assert!(first.submit);
        assert!(first.requires_full_repaint);
        assert!(first.submission.is_some());
        engine
            .destroy_session(session.id)
            .expect("destroy next-core test session");
    }

    #[test]
    fn engine_render_consumer_skips_repeated_next_core_revision() {
        let engine = CurrentTerminalEngine::NextCore(next_core());
        let session = engine
            .create_session(CreateSessionRequest {
                cols: 20,
                rows: 4,
                command_dir: None,
                command: Some(quiet_wait_command_for_test()),
                env: Vec::new(),
                launch_policy: LaunchPolicySnapshot::default(),
            })
            .expect("create next-core session");
        let mut consumer = EngineRenderConsumer::new(
            session.id,
            RenderCellMetrics {
                cell_width_px: 8,
                cell_height_px: 16,
            },
        );

        let first = consumer
            .read_commit(&engine)
            .expect("read first render commit batch");
        assert!(first.stats.submit);
        assert!(first.stats.requires_full_repaint);
        assert_eq!(first.stats.damage_rect_count, 1);
        assert!(first.commit.submission.is_some());

        let repeat = consumer
            .read_commit(&engine)
            .expect("read repeated render commit batch");
        assert!(!repeat.stats.submit);
        assert_eq!(repeat.stats.previous_revision, Some(first.stats.revision));
        assert!(repeat.commit.submission.is_none());
        engine
            .destroy_session(session.id)
            .expect("destroy next-core test session");
    }

    #[test]
    fn command_list_backend_prepares_next_core_commit_commands() {
        let engine = CurrentTerminalEngine::NextCore(next_core());
        let session = engine
            .create_session(CreateSessionRequest {
                cols: 20,
                rows: 4,
                command_dir: None,
                command: Some(quiet_wait_command_for_test()),
                env: Vec::new(),
                launch_policy: LaunchPolicySnapshot::default(),
            })
            .expect("create next-core session");
        let mut consumer = EngineRenderConsumer::new(
            session.id,
            RenderCellMetrics {
                cell_width_px: 8,
                cell_height_px: 16,
            },
        );
        let mut backend = CommandListRenderBackend::default();

        let first = consumer
            .read_commit(&engine)
            .expect("read first render commit batch");
        let frame = backend
            .submit(&first)
            .expect("prepare backend command list");

        assert!(frame.submitted);
        assert_eq!(frame.pane_id, session.id);
        assert_eq!(frame.revision, first.stats.revision);
        assert!(matches!(
            frame.commands.first(),
            Some(EngineRenderBackendCommand::Damage(_))
        ));
        assert!(frame
            .commands
            .iter()
            .any(|command| matches!(command, EngineRenderBackendCommand::Background { .. })));
        let buffer_plan = EngineRenderBufferPlan::from_frame(&frame);
        assert_eq!(buffer_plan.pane_id, session.id);
        assert_eq!(
            buffer_plan.damage_rects.len(),
            first.stats.damage_rect_count
        );
        assert_eq!(buffer_plan.vertices.len() % 4, 0);
        assert_eq!(buffer_plan.indices.len() % 6, 0);
        assert_eq!(&buffer_plan.indices[0..6], &[0, 1, 2, 1, 2, 3]);
        assert!(buffer_plan
            .vertices
            .iter()
            .any(|vertex| vertex.layer == EngineRenderVertexLayer::Background));
        assert!(buffer_plan
            .vertices
            .iter()
            .any(|vertex| vertex.color == [0.0, 0.0, 0.0, 1.0]));
        let upload_plan = EngineWgpuRenderBackend::prepare_upload(&buffer_plan);
        assert_eq!(upload_plan.pane_id, session.id);
        assert_eq!(upload_plan.revision, frame.revision);
        assert_eq!(upload_plan.vertices.len(), buffer_plan.vertices.len());
        assert_eq!(upload_plan.indices, buffer_plan.indices);
        assert_eq!(
            upload_plan.vertex_bytes_len(),
            upload_plan.vertices.len() * std::mem::size_of::<EngineRenderGpuVertex>()
        );
        assert_eq!(
            upload_plan.index_bytes_len(),
            upload_plan.indices.len() * std::mem::size_of::<u32>()
        );
        assert!(upload_plan.vertices.iter().any(|vertex| vertex.layer == 0));
        assert!(upload_plan
            .vertices
            .iter()
            .any(|vertex| vertex.color == [0.0, 0.0, 0.0, 1.0]));
        let viewport_upload = EngineWgpuRenderBackend::prepare_upload_for_viewport(
            &buffer_plan,
            20.0 * 8.0,
            4.0 * 16.0,
        );
        assert!(viewport_upload.vertices.iter().all(|vertex| {
            vertex.position[0] >= -1.0
                && vertex.position[0] <= 1.0
                && vertex.position[1] >= -1.0
                && vertex.position[1] <= 1.0
        }));
        assert_eq!(
            EngineRenderGpuVertex::desc().array_stride,
            std::mem::size_of::<EngineRenderGpuVertex>() as wgpu::BufferAddress
        );
        let pipeline_config = EngineWgpuPipelineConfig {
            target_format: wgpu::TextureFormat::Rgba8UnormSrgb,
        };
        assert_eq!(
            pipeline_config.target_format,
            wgpu::TextureFormat::Rgba8UnormSrgb
        );
        let pass_plan =
            EngineWgpuRenderPassPlan::from_upload_plan(&upload_plan, Some([0.0, 0.0, 0.0, 1.0]));
        assert!(pass_plan.draw);
        assert_eq!(pass_plan.pane_id, session.id);
        assert_eq!(pass_plan.revision, frame.revision);
        assert_eq!(pass_plan.vertex_count, upload_plan.vertices.len());
        assert_eq!(pass_plan.index_count, upload_plan.indices.len());

        let repeat = consumer
            .read_commit(&engine)
            .expect("read repeated render commit batch");
        let skipped = backend
            .submit(&repeat)
            .expect("prepare repeated backend frame");
        let skipped_buffer = EngineRenderBufferPlan::from_frame(&skipped);
        assert!(!skipped.submitted);
        assert!(skipped.commands.is_empty());
        assert!(skipped_buffer.vertices.is_empty());
        assert!(skipped_buffer.indices.is_empty());
        let skipped_upload = EngineRenderGpuUploadPlan::from_buffer_plan(&skipped_buffer);
        assert!(skipped_upload.is_empty());
        let skipped_pass = EngineWgpuRenderBackend::default().prepare_pass(&skipped_upload, None);
        assert!(!skipped_pass.draw);
        assert_eq!(skipped_pass.vertex_count, 0);
        assert_eq!(skipped_pass.index_count, 0);
        engine
            .destroy_session(session.id)
            .expect("destroy next-core test session");
    }

    #[test]
    fn engine_render_consumer_reads_next_core_buffer_plan() {
        let engine = CurrentTerminalEngine::NextCore(next_core());
        let session = engine
            .create_session(CreateSessionRequest {
                cols: 20,
                rows: 4,
                command_dir: None,
                command: Some(quiet_wait_command_for_test()),
                env: Vec::new(),
                launch_policy: LaunchPolicySnapshot::default(),
            })
            .expect("create next-core session");
        let mut consumer = EngineRenderConsumer::new(
            session.id,
            RenderCellMetrics {
                cell_width_px: 8,
                cell_height_px: 16,
            },
        );

        let first: EngineRenderBufferBatch = consumer
            .read_buffer_plan(&engine)
            .expect("read first render buffer plan");
        assert!(first.stats.submit);
        assert!(first.buffer_plan.submitted);
        assert_eq!(first.pane_id, session.id);
        assert_eq!(first.buffer_plan.pane_id, session.id);
        assert_eq!(first.buffer_plan.revision, first.stats.revision);
        assert_eq!(
            first.buffer_plan.damage_rects.len(),
            first.stats.damage_rect_count
        );
        assert!(!first.buffer_plan.vertices.is_empty());
        assert!(!first.buffer_plan.indices.is_empty());

        let repeat = consumer
            .read_buffer_plan(&engine)
            .expect("read repeated render buffer plan");
        assert!(!repeat.stats.submit);
        assert!(!repeat.buffer_plan.submitted);
        assert_eq!(repeat.stats.previous_revision, Some(first.stats.revision));
        assert!(repeat.buffer_plan.vertices.is_empty());
        assert!(repeat.buffer_plan.indices.is_empty());
        engine
            .destroy_session(session.id)
            .expect("destroy next-core test session");
    }

    fn quiet_wait_command_for_test() -> portable_pty::CommandBuilder {
        #[cfg(windows)]
        {
            let mut command = portable_pty::CommandBuilder::new("cmd.exe");
            command.args(["/c", "ping -n 5 127.0.0.1 >nul"]);
            command
        }
        #[cfg(not(windows))]
        {
            let mut command = portable_pty::CommandBuilder::new("sh");
            command.args(["-c", "sleep 5"]);
            command
        }
    }
}

impl SessionEngine for CurrentTerminalEngine {
    fn list_sessions(&self) -> anyhow::Result<Vec<SessionSnapshot>> {
        match self {
            Self::WezTerm(engine) => engine.list_sessions(),
            Self::NextCore(engine) => engine.list_sessions(),
        }
    }

    fn get_session(&self, pane_id: usize) -> anyhow::Result<SessionSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.get_session(pane_id),
            Self::NextCore(engine) => engine.get_session(pane_id),
        }
    }

    fn create_session(&self, request: CreateSessionRequest) -> anyhow::Result<SessionSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.create_session(request),
            Self::NextCore(engine) => engine.create_session(request),
        }
    }

    fn split_session(&self, request: SplitSessionRequest) -> anyhow::Result<SessionSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.split_session(request),
            Self::NextCore(engine) => engine.split_session(request),
        }
    }

    fn focus_session(&self, pane_id: usize) -> anyhow::Result<()> {
        match self {
            Self::WezTerm(engine) => engine.focus_session(pane_id),
            Self::NextCore(engine) => engine.focus_session(pane_id),
        }
    }

    fn shell(&self, pane_id: usize) -> anyhow::Result<ShellSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.shell(pane_id),
            Self::NextCore(engine) => engine.shell(pane_id),
        }
    }

    fn activity(&self, pane_id: usize) -> anyhow::Result<SessionActivitySnapshot> {
        match self {
            Self::WezTerm(engine) => engine.activity(pane_id),
            Self::NextCore(engine) => engine.activity(pane_id),
        }
    }

    fn resize_session(&self, pane_id: usize, cols: usize, rows: usize) -> anyhow::Result<()> {
        match self {
            Self::WezTerm(engine) => engine.resize_session(pane_id, cols, rows),
            Self::NextCore(engine) => engine.resize_session(pane_id, cols, rows),
        }
    }

    fn destroy_session(&self, pane_id: usize) -> anyhow::Result<()> {
        match self {
            Self::WezTerm(engine) => engine.destroy_session(pane_id),
            Self::NextCore(engine) => engine.destroy_session(pane_id),
        }
    }
}

impl ScreenEngine for CurrentTerminalEngine {
    fn read_screen(&self, pane_id: usize) -> anyhow::Result<ScreenSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.read_screen(pane_id),
            Self::NextCore(engine) => engine.read_screen(pane_id),
        }
    }

    fn read_styled_screen(&self, pane_id: usize) -> anyhow::Result<StyledScreenSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.read_styled_screen(pane_id),
            Self::NextCore(engine) => engine.read_styled_screen(pane_id),
        }
    }

    fn read_render_frame(
        &self,
        pane_id: usize,
        since_revision: Option<u64>,
    ) -> anyhow::Result<RenderFrameSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.read_render_frame(pane_id, since_revision),
            Self::NextCore(engine) => engine.read_render_frame(pane_id, since_revision),
        }
    }

    fn read_render_draw_plan(
        &self,
        pane_id: usize,
        since_revision: Option<u64>,
    ) -> anyhow::Result<RenderDrawPlan> {
        match self {
            Self::WezTerm(engine) => engine.read_render_draw_plan(pane_id, since_revision),
            Self::NextCore(engine) => engine.read_render_draw_plan(pane_id, since_revision),
        }
    }

    fn read_render_commit_plan(
        &self,
        pane_id: usize,
        metrics: RenderCellMetrics,
        consumer: &mut RenderConsumerState,
    ) -> anyhow::Result<RenderCommitPlan> {
        match self {
            Self::WezTerm(engine) => engine.read_render_commit_plan(pane_id, metrics, consumer),
            Self::NextCore(engine) => engine.read_render_commit_plan(pane_id, metrics, consumer),
        }
    }

    fn read_visible_text(&self, pane_id: usize) -> anyhow::Result<String> {
        match self {
            Self::WezTerm(engine) => engine.read_visible_text(pane_id),
            Self::NextCore(engine) => engine.read_visible_text(pane_id),
        }
    }

    fn read_lines(
        &self,
        pane_id: usize,
        start: i64,
        count: usize,
    ) -> anyhow::Result<Vec<ScreenLine>> {
        match self {
            Self::WezTerm(engine) => engine.read_lines(pane_id, start, count),
            Self::NextCore(engine) => engine.read_lines(pane_id, start, count),
        }
    }

    fn read_scrollback(&self, pane_id: usize, limit: usize) -> anyhow::Result<Vec<String>> {
        match self {
            Self::WezTerm(engine) => engine.read_scrollback(pane_id, limit),
            Self::NextCore(engine) => engine.read_scrollback(pane_id, limit),
        }
    }

    fn read_scrollback_text(
        &self,
        pane_id: usize,
        request: ScrollbackTextRequest,
    ) -> anyhow::Result<ScrollbackTextSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.read_scrollback_text(pane_id, request),
            Self::NextCore(engine) => engine.read_scrollback_text(pane_id, request),
        }
    }

    fn search(
        &self,
        pane_id: usize,
        pattern: &str,
        max_results: usize,
    ) -> anyhow::Result<Vec<ScreenSearchMatch>> {
        match self {
            Self::WezTerm(engine) => engine.search(pane_id, pattern, max_results),
            Self::NextCore(engine) => engine.search(pane_id, pattern, max_results),
        }
    }

    fn cursor(&self, pane_id: usize) -> anyhow::Result<CursorSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.cursor(pane_id),
            Self::NextCore(engine) => engine.cursor(pane_id),
        }
    }
}

impl InputEngine for CurrentTerminalEngine {
    fn write_input(&self, pane_id: usize, input: &str) -> anyhow::Result<()> {
        match self {
            Self::WezTerm(engine) => engine.write_input(pane_id, input),
            Self::NextCore(engine) => engine.write_input(pane_id, input),
        }
    }

    fn paste_input(&self, pane_id: usize, text: &str) -> anyhow::Result<()> {
        match self {
            Self::WezTerm(engine) => engine.paste_input(pane_id, text),
            Self::NextCore(engine) => engine.paste_input(pane_id, text),
        }
    }
}

impl RecordingEngine for CurrentTerminalEngine {
    fn start_recording(&self, pane_id: usize) -> anyhow::Result<RecordingStartResult> {
        match self {
            Self::WezTerm(engine) => engine.start_recording(pane_id),
            Self::NextCore(engine) => engine.start_recording(pane_id),
        }
    }

    fn stop_recording(&self, pane_id: usize) -> anyhow::Result<RecordingStopResult> {
        match self {
            Self::WezTerm(engine) => engine.stop_recording(pane_id),
            Self::NextCore(engine) => engine.stop_recording(pane_id),
        }
    }

    fn recording_status(&self, pane_id: usize) -> anyhow::Result<RecordingStatusSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.recording_status(pane_id),
            Self::NextCore(engine) => engine.recording_status(pane_id),
        }
    }

    fn attach_recording_trace(
        &self,
        pane_id: usize,
        trace_id: String,
    ) -> anyhow::Result<Vec<String>> {
        match self {
            Self::WezTerm(engine) => engine.attach_recording_trace(pane_id, trace_id),
            Self::NextCore(engine) => engine.attach_recording_trace(pane_id, trace_id),
        }
    }

    fn export_markdown(
        &self,
        pane_id: usize,
        target_path: Option<String>,
    ) -> anyhow::Result<RecordingExportResult> {
        match self {
            Self::WezTerm(engine) => engine.export_markdown(pane_id, target_path),
            Self::NextCore(engine) => engine.export_markdown(pane_id, target_path),
        }
    }
}

impl HealthEngine for CurrentTerminalEngine {
    fn health(&self) -> anyhow::Result<EngineHealthSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.health(),
            Self::NextCore(engine) => engine.health(),
        }
    }
}

impl WindowEngine for CurrentTerminalEngine {
    fn focus_current_instance_window(&self) -> anyhow::Result<WindowFocusResult> {
        focus_current_instance_window()
    }

    fn set_current_instance_title(
        &self,
        title: Option<String>,
    ) -> anyhow::Result<WindowTitleResult> {
        set_current_instance_title(title)
    }

    fn active_pane_id(&self) -> anyhow::Result<Option<u64>> {
        match self {
            Self::WezTerm(engine) => engine.active_pane_id(),
            Self::NextCore(engine) => Ok(engine
                .list_sessions()?
                .into_iter()
                .find(|session| session.is_active)
                .map(|session| session.id as u64)),
        }
    }

    fn pane_locations(&self) -> anyhow::Result<HashMap<u64, PaneLocation>> {
        match self {
            Self::WezTerm(engine) => engine.pane_locations(),
            Self::NextCore(engine) => Ok(engine
                .list_sessions()?
                .into_iter()
                .map(|session| {
                    (
                        session.id as u64,
                        PaneLocation {
                            window_id: 0,
                            tab_id: session.id,
                        },
                    )
                })
                .collect()),
        }
    }

    fn scroll_viewport_to(
        &self,
        pane_id: usize,
        target: isize,
    ) -> anyhow::Result<ViewportScrollResult> {
        match self {
            Self::WezTerm(engine) => engine.scroll_viewport_to(pane_id, target),
            Self::NextCore(engine) => {
                engine.scroll_viewport_to(pane_id, target)?;
                Ok(ViewportScrollResult::Scrolled)
            }
        }
    }
}

impl CaptureEngine for CurrentTerminalEngine {
    fn capture_screen_image(&self, include_base64: bool) -> anyhow::Result<serde_json::Value> {
        crate::mcp::handler::capture_screen_image(include_base64)
    }

    fn capture_window_image(
        &self,
        title_filter: Option<&str>,
        pid_filter: Option<u32>,
        include_base64: bool,
    ) -> anyhow::Result<serde_json::Value> {
        crate::mcp::handler::capture_window_image(title_filter, pid_filter, include_base64)
    }

    fn render_scrollback_png(
        &self,
        pane_id: Option<usize>,
        path: &Path,
        opts: &crate::scrollshot::ScrollbackPngOptions,
    ) -> anyhow::Result<RenderedScrollbackPng> {
        match self {
            Self::WezTerm(engine) => engine.render_scrollback_png(pane_id, path, opts),
            Self::NextCore(engine) => {
                let pane_id = match pane_id {
                    Some(pane_id) => pane_id,
                    None => engine
                        .list_sessions()?
                        .into_iter()
                        .find(|session| session.is_active)
                        .map(|session| session.id)
                        .ok_or_else(|| anyhow::anyhow!("no active next-core session"))?,
                };
                let styled = engine.read_styled_scrollback(
                    pane_id,
                    ScrollbackTextRequest {
                        start_line: None,
                        end_line: None,
                        tail_lines: Some(opts.max_rows as i64),
                        escapes: false,
                    },
                )?;
                let total_rows = styled.physical_top + styled.viewport_rows as i64;
                let image = crate::scrollshot::render_styled_scrollback_png(
                    &styled.lines,
                    styled.cols,
                    styled.first_row,
                    styled.first_row > styled.scrollback_top
                        || total_rows.saturating_sub(styled.scrollback_top) > styled.row_count,
                    path,
                    opts,
                )?;
                Ok(RenderedScrollbackPng {
                    image,
                    session_id: pane_id,
                    renderer: next_core_scrollback_renderer_metadata(),
                })
            }
        }
    }
}

fn focus_current_instance_window() -> anyhow::Result<WindowFocusResult> {
    let window = crate::frontend::try_front_end()
        .and_then(|fe| fe.gui_windows().into_iter().next())
        .ok_or_else(|| anyhow::anyhow!("no GUI window is registered for this instance"))?;
    window.window.focus();
    Ok(WindowFocusResult {
        mux_window_id: window.mux_window_id,
        window_engine: "wezterm-host",
        uses_host_window: true,
    })
}

fn set_current_instance_title(title: Option<String>) -> anyhow::Result<WindowTitleResult> {
    crate::server_info::set_title(title.clone())?;
    Ok(WindowTitleResult {
        title,
        window_engine: "wezterm-host",
        title_owner: "server_info",
        metadata_owner: "product_registry",
        native_window_lifecycle: "host_owned",
        applied_to_native_window: false,
        uses_host_window: true,
    })
}
