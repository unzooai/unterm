//! What a renderer keeps between frames.
//!
//! Holds a pane's last-drawn revision and its cell metrics, so a frame that
//! changed nothing costs nothing. Keeping this on the `ScreenEngine` contract
//! rather than reaching into the screen is what lets a renderer be written
//! against the engine instead of against next-core's internals.

use crate::backend::{
    CommandListRenderBackend, EngineRenderBackend, EngineRenderBufferPlan,
    EngineRenderCachedGlyphUploadDiagnostics, EngineWgpuPreparedFrameDiagnostics,
    EngineWgpuPreparedFramePlan,
};
use unterm_engine::{
    RenderCellMetrics, RenderCommitPlan, RenderConsumerState, RenderRect, ScreenEngine,
};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineRenderCommitStats {
    pub submit: bool,
    pub previous_revision: Option<u64>,
    pub revision: u64,
    pub skipped_revisions: u64,
    pub requires_full_repaint: bool,
    pub full: bool,
    pub viewport: Option<RenderRect>,
    pub damage_rect_count: usize,
    pub background_quad_count: usize,
    pub text_run_count: usize,
    pub cursor_visible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineRenderCommitBatch {
    pub pane_id: usize,
    pub commit: RenderCommitPlan,
    pub stats: EngineRenderCommitStats,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EngineRenderBufferBatch {
    pub pane_id: usize,
    pub stats: EngineRenderCommitStats,
    pub buffer_plan: EngineRenderBufferPlan,
}

#[allow(dead_code)]
pub struct EngineRenderPreparedPaneFrame {
    pub batch: EngineRenderBufferBatch,
    pub prepared: EngineWgpuPreparedFramePlan,
    pub replace_diagnostics: EngineRenderPaneReplaceDiagnostics,
}

#[allow(dead_code)]
impl EngineRenderPreparedPaneFrame {
    pub fn from_parts(
        batch: EngineRenderBufferBatch,
        prepared: EngineWgpuPreparedFramePlan,
        replace_requested: bool,
        cached_glyph_upload: Option<&EngineRenderCachedGlyphUploadDiagnostics>,
    ) -> Self {
        let prepared_diagnostics = prepared.diagnostics();
        let replace_diagnostics = EngineRenderPaneReplaceDiagnostics::from_parts(
            replace_requested,
            Some(&batch),
            Some(&prepared_diagnostics),
            cached_glyph_upload,
        );

        Self {
            batch,
            prepared,
            replace_diagnostics,
        }
    }

    pub fn replace_diagnostics_for_request(
        replace_requested: bool,
        frame: Option<&Self>,
    ) -> Option<EngineRenderPaneReplaceDiagnostics> {
        match (replace_requested, frame) {
            (true, Some(frame)) => Some(frame.replace_diagnostics),
            (true, None) => Some(EngineRenderPaneReplaceDiagnostics::requested_missing_frame()),
            (false, _) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum EngineRenderBufferReadinessIssue {
    CommitNotSubmitted,
    BufferNotSubmitted,
    EmptyBuffer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct EngineRenderPaneReplaceDiagnostics {
    pub requested: bool,
    pub pane_id: Option<usize>,
    pub revision: Option<u64>,
    pub batch_present: bool,
    pub batch_ready: bool,
    pub batch_readiness_issue_count: usize,
    pub prepared_frame_present: bool,
    pub prepared_frame_ready: bool,
    pub prepared_frame_matches_batch: bool,
    pub prepared_frame_readiness_issue_count: usize,
    pub cached_glyph_upload_required: bool,
    pub cached_glyph_upload_present: bool,
    pub cached_glyph_upload_ready: bool,
    pub cached_glyph_upload_matches_batch: bool,
    pub cached_glyph_upload_readiness_issue_count: usize,
    pub replace_ready: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum EngineRenderPaneReplaceReadinessIssue {
    NotRequested,
    MissingBufferBatch,
    BufferBatchNotReady,
    MissingPreparedFrame,
    PreparedFrameNotReady,
    PreparedFrameBatchMismatch,
    MissingCachedGlyphUpload,
    CachedGlyphUploadNotReady,
    CachedGlyphUploadBatchMismatch,
}

#[derive(Clone, Debug)]
pub struct EngineRenderConsumer {
    pane_id: usize,
    metrics: RenderCellMetrics,
    state: RenderConsumerState,
}

#[derive(Clone, Debug, Default)]
pub struct EngineRenderConsumerSet {
    consumers: HashMap<usize, EngineRenderConsumer>,
}

#[allow(dead_code)]
impl EngineRenderConsumer {
    pub fn new(pane_id: usize, metrics: RenderCellMetrics) -> Self {
        Self {
            pane_id,
            metrics,
            state: RenderConsumerState::new(),
        }
    }

    pub fn pane_id(&self) -> usize {
        self.pane_id
    }

    pub fn metrics(&self) -> RenderCellMetrics {
        self.metrics
    }

    pub fn submitted_revision(&self) -> Option<u64> {
        self.state.submitted_revision()
    }

    pub fn resize_cells(&mut self, metrics: RenderCellMetrics) {
        self.metrics = metrics;
    }

    pub fn read_commit<E: ScreenEngine + ?Sized>(
        &mut self,
        engine: &E,
    ) -> anyhow::Result<EngineRenderCommitBatch> {
        let commit = engine.read_render_commit_plan(self.pane_id, self.metrics, &mut self.state)?;
        Ok(EngineRenderCommitBatch {
            pane_id: self.pane_id,
            stats: EngineRenderCommitStats::from_commit(&commit),
            commit,
        })
    }

    pub fn read_buffer_plan<E: ScreenEngine + ?Sized>(
        &mut self,
        engine: &E,
    ) -> anyhow::Result<EngineRenderBufferBatch> {
        let batch = self.read_commit(engine)?;
        let mut backend = CommandListRenderBackend::default();
        let frame = backend.submit(&batch)?;
        let buffer_plan = EngineRenderBufferPlan::from_frame(&frame);
        Ok(EngineRenderBufferBatch {
            pane_id: batch.pane_id,
            stats: batch.stats,
            buffer_plan,
        })
    }
}

#[allow(dead_code)]
impl EngineRenderBufferBatch {
    pub fn readiness_issues(&self) -> Vec<EngineRenderBufferReadinessIssue> {
        let mut issues = Vec::new();
        if !self.stats.submit {
            issues.push(EngineRenderBufferReadinessIssue::CommitNotSubmitted);
        }
        if !self.buffer_plan.submitted {
            issues.push(EngineRenderBufferReadinessIssue::BufferNotSubmitted);
        }
        // Text runs draw through the textured glyph pass and contribute no
        // solid vertices, so a frame that is all text has an empty solid
        // buffer and is still perfectly drawable.
        if (self.buffer_plan.vertices.is_empty() || self.buffer_plan.indices.is_empty())
            && self.buffer_plan.text_runs.is_empty()
        {
            issues.push(EngineRenderBufferReadinessIssue::EmptyBuffer);
        }
        issues
    }

    pub fn is_draw_ready(&self) -> bool {
        self.readiness_issues().is_empty()
    }
}

#[allow(dead_code)]
impl EngineRenderPaneReplaceDiagnostics {
    pub fn requested_missing_frame() -> Self {
        Self::from_parts(true, None, None, None)
    }

    pub fn from_parts(
        requested: bool,
        batch: Option<&EngineRenderBufferBatch>,
        prepared_frame: Option<&EngineWgpuPreparedFrameDiagnostics>,
        cached_glyph_upload: Option<&EngineRenderCachedGlyphUploadDiagnostics>,
    ) -> Self {
        let batch_ready = batch.is_some_and(|batch| batch.is_draw_ready());
        let batch_readiness_issue_count = batch.map_or(0, |batch| batch.readiness_issues().len());
        let prepared_frame_ready =
            prepared_frame.is_some_and(|diagnostics| diagnostics.replace_ready);
        let prepared_frame_matches_batch = match (batch, prepared_frame) {
            (Some(batch), Some(diagnostics)) => {
                diagnostics.pane_id == batch.pane_id && diagnostics.revision == batch.stats.revision
            }
            _ => false,
        };
        let prepared_frame_readiness_issue_count =
            prepared_frame.map_or(0, |diagnostics| usize::from(!diagnostics.replace_ready));
        let cached_glyph_upload_required = prepared_frame.is_some_and(|diagnostics| {
            diagnostics.text_run_count > 0
                || diagnostics.glyph_key_count > 0
                || diagnostics.glyph_instance_count > 0
        });
        let cached_glyph_upload_ready =
            cached_glyph_upload.is_some_and(|diagnostics| diagnostics.is_ready());
        let cached_glyph_upload_matches_batch = match (batch, cached_glyph_upload) {
            (Some(batch), Some(diagnostics)) => {
                diagnostics.pane_id == batch.pane_id && diagnostics.revision == batch.stats.revision
            }
            _ => false,
        };
        let cached_glyph_upload_readiness_issue_count =
            cached_glyph_upload.map_or(0, |diagnostics| diagnostics.readiness_issues().len());
        let glyph_ready = !cached_glyph_upload_required
            || (cached_glyph_upload_ready && cached_glyph_upload_matches_batch);

        Self {
            requested,
            pane_id: batch.map(|batch| batch.pane_id),
            revision: batch.map(|batch| batch.stats.revision),
            batch_present: batch.is_some(),
            batch_ready,
            batch_readiness_issue_count,
            prepared_frame_present: prepared_frame.is_some(),
            prepared_frame_ready,
            prepared_frame_matches_batch,
            prepared_frame_readiness_issue_count,
            cached_glyph_upload_required,
            cached_glyph_upload_present: cached_glyph_upload.is_some(),
            cached_glyph_upload_ready,
            cached_glyph_upload_matches_batch,
            cached_glyph_upload_readiness_issue_count,
            replace_ready: requested
                && batch_ready
                && prepared_frame_ready
                && prepared_frame_matches_batch
                && glyph_ready,
        }
    }

    pub fn readiness_issues(&self) -> Vec<EngineRenderPaneReplaceReadinessIssue> {
        let mut issues = Vec::new();
        if !self.requested {
            issues.push(EngineRenderPaneReplaceReadinessIssue::NotRequested);
        }
        if !self.batch_present {
            issues.push(EngineRenderPaneReplaceReadinessIssue::MissingBufferBatch);
        } else if !self.batch_ready {
            issues.push(EngineRenderPaneReplaceReadinessIssue::BufferBatchNotReady);
        }
        if !self.prepared_frame_present {
            issues.push(EngineRenderPaneReplaceReadinessIssue::MissingPreparedFrame);
        } else if !self.prepared_frame_ready {
            issues.push(EngineRenderPaneReplaceReadinessIssue::PreparedFrameNotReady);
        } else if !self.prepared_frame_matches_batch {
            issues.push(EngineRenderPaneReplaceReadinessIssue::PreparedFrameBatchMismatch);
        }
        if self.cached_glyph_upload_required && !self.cached_glyph_upload_present {
            issues.push(EngineRenderPaneReplaceReadinessIssue::MissingCachedGlyphUpload);
        } else if self.cached_glyph_upload_required && !self.cached_glyph_upload_ready {
            issues.push(EngineRenderPaneReplaceReadinessIssue::CachedGlyphUploadNotReady);
        } else if self.cached_glyph_upload_required && !self.cached_glyph_upload_matches_batch {
            issues.push(EngineRenderPaneReplaceReadinessIssue::CachedGlyphUploadBatchMismatch);
        }
        issues
    }

    pub fn should_replace_legacy_pane(&self) -> bool {
        self.replace_ready
    }

    pub fn should_log_replace_fallback(&self) -> bool {
        self.requested && !self.replace_ready
    }

    pub fn readiness_issue_count(&self) -> usize {
        self.readiness_issues().len()
    }
}

#[allow(dead_code)]
impl EngineRenderConsumerSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.consumers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.consumers.is_empty()
    }

    pub fn contains_pane(&self, pane_id: usize) -> bool {
        self.consumers.contains_key(&pane_id)
    }

    pub fn remove_pane(&mut self, pane_id: usize) -> Option<EngineRenderConsumer> {
        self.consumers.remove(&pane_id)
    }

    pub fn clear(&mut self) {
        self.consumers.clear();
    }

    pub fn consumer(&self, pane_id: usize) -> Option<&EngineRenderConsumer> {
        self.consumers.get(&pane_id)
    }

    pub fn consumer_mut(
        &mut self,
        pane_id: usize,
        metrics: RenderCellMetrics,
    ) -> &mut EngineRenderConsumer {
        let consumer = self
            .consumers
            .entry(pane_id)
            .or_insert_with(|| EngineRenderConsumer::new(pane_id, metrics));
        if consumer.metrics() != metrics {
            consumer.resize_cells(metrics);
        }
        consumer
    }

    pub fn read_buffer_plan<E: ScreenEngine + ?Sized>(
        &mut self,
        engine: &E,
        pane_id: usize,
        metrics: RenderCellMetrics,
    ) -> anyhow::Result<EngineRenderBufferBatch> {
        self.consumer_mut(pane_id, metrics).read_buffer_plan(engine)
    }
}

impl EngineRenderCommitStats {
    pub fn from_commit(commit: &RenderCommitPlan) -> Self {
        let submission = commit.submission.as_ref();
        Self {
            submit: commit.submit,
            previous_revision: commit.previous_revision,
            revision: commit.revision,
            skipped_revisions: commit.skipped_revisions,
            requires_full_repaint: commit.requires_full_repaint,
            full: submission.is_some_and(|submission| submission.full),
            viewport: submission.map(|submission| submission.viewport),
            damage_rect_count: submission.map_or(0, |submission| submission.damage_rects.len()),
            background_quad_count: submission
                .map_or(0, |submission| submission.background_quads.len()),
            text_run_count: submission.map_or(0, |submission| submission.text_runs.len()),
            cursor_visible: submission
                .and_then(|submission| submission.cursor.as_ref().map(|cursor| cursor.visible))
                .unwrap_or(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::*;
    use unterm_engine::{
        next_core, CreateSessionRequest, LaunchPolicySnapshot,
        SessionEngine,
    };

    #[test]
    fn next_core_facade_reads_render_commit_plan() {
        let engine = next_core();
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
        let engine = next_core();
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

        let (repeat, submitted) = read_unchanged(
            first.stats.revision,
            || {
                consumer
                    .read_commit(&engine)
                    .expect("read repeated render commit batch")
            },
            |batch| batch.stats.submit,
            |batch| batch.stats.revision,
        );
        assert_eq!(repeat.stats.previous_revision, Some(submitted));
        assert!(repeat.commit.submission.is_none());
        engine
            .destroy_session(session.id)
            .expect("destroy next-core test session");
    }

    #[test]
    fn command_list_backend_prepares_next_core_commit_commands() {
        let engine = next_core();
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
        assert_eq!(buffer_plan.text_runs.len(), first.stats.text_run_count);
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
            upload_plan.vertex_bytes_len(),
            upload_plan.vertices.len() * std::mem::size_of::<EngineRenderGpuVertex>()
        );
        let pass_plan =
            EngineWgpuRenderPassPlan::from_upload_plan(&upload_plan, Some([0.0, 0.0, 0.0, 1.0]));
        assert!(pass_plan.draw);
        assert_eq!(pass_plan.pane_id, session.id);
        assert_eq!(pass_plan.revision, frame.revision);
        assert_eq!(pass_plan.vertex_count, upload_plan.vertices.len());
        assert_eq!(pass_plan.index_count, upload_plan.indices.len());

        let (repeat, _) = read_unchanged(
            first.stats.revision,
            || {
                consumer
                    .read_commit(&engine)
                    .expect("read repeated render commit batch")
            },
            |batch| batch.stats.submit,
            |batch| batch.stats.revision,
        );
        let skipped = backend
            .submit(&repeat)
            .expect("prepare repeated backend frame");
        let skipped_buffer = EngineRenderBufferPlan::from_frame(&skipped);
        assert!(!skipped.submitted);
        assert!(skipped.commands.is_empty());
        assert!(skipped_buffer.text_runs.is_empty());
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
        let engine = next_core();
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
        assert_eq!(
            first.buffer_plan.text_runs.len(),
            first.stats.text_run_count
        );
        assert!(!first.buffer_plan.vertices.is_empty());
        assert!(!first.buffer_plan.indices.is_empty());
        assert!(first.is_draw_ready());
        assert!(first.readiness_issues().is_empty());

        let (repeat, submitted) = read_unchanged(
            first.stats.revision,
            || {
                consumer
                    .read_buffer_plan(&engine)
                    .expect("read repeated render buffer plan")
            },
            |batch| batch.stats.submit,
            |batch| batch.stats.revision,
        );
        assert!(!repeat.buffer_plan.submitted);
        assert_eq!(repeat.stats.previous_revision, Some(submitted));
        assert!(repeat.buffer_plan.text_runs.is_empty());
        assert!(repeat.buffer_plan.vertices.is_empty());
        assert!(repeat.buffer_plan.indices.is_empty());
        assert!(!repeat.is_draw_ready());
        assert_eq!(repeat.readiness_issues().len(), 3);
        let missing_frame =
            EngineRenderPaneReplaceDiagnostics::from_parts(true, Some(&first), None, None);
        assert!(!missing_frame.replace_ready);
        assert_eq!(
            missing_frame.readiness_issues(),
            vec![EngineRenderPaneReplaceReadinessIssue::MissingPreparedFrame]
        );
        let requested_missing_frame = EngineRenderPaneReplaceDiagnostics::requested_missing_frame();
        assert!(!requested_missing_frame.replace_ready);
        assert_eq!(
            requested_missing_frame.readiness_issues(),
            vec![
                EngineRenderPaneReplaceReadinessIssue::MissingBufferBatch,
                EngineRenderPaneReplaceReadinessIssue::MissingPreparedFrame
            ]
        );
        let not_requested = EngineRenderPaneReplaceDiagnostics::from_parts(
            false,
            Some(&first),
            Some(&EngineWgpuPreparedFrameDiagnostics {
                pane_id: first.pane_id,
                submitted: true,
                revision: first.stats.revision,
                solid_vertex_count: first.buffer_plan.vertices.len(),
                solid_index_count: first.buffer_plan.indices.len(),
                text_run_count: first.buffer_plan.text_runs.len(),
                glyph_key_count: 0,
                glyph_instance_count: 0,
                replace_ready: true,
            }),
            None,
        );
        assert!(not_requested.prepared_frame_matches_batch);
        assert_eq!(
            not_requested.readiness_issues(),
            vec![EngineRenderPaneReplaceReadinessIssue::NotRequested]
        );
        engine
            .destroy_session(session.id)
            .expect("destroy next-core test session");
    }

    #[test]
    fn engine_render_prepared_pane_frame_builds_replace_diagnostics() {
        let batch = EngineRenderBufferBatch {
            pane_id: 42,
            stats: EngineRenderCommitStats {
                submit: true,
                previous_revision: None,
                revision: 9,
                skipped_revisions: 0,
                requires_full_repaint: true,
                full: true,
                viewport: None,
                damage_rect_count: 0,
                background_quad_count: 1,
                text_run_count: 0,
                cursor_visible: false,
            },
            buffer_plan: EngineRenderBufferPlan {
                pane_id: 42,
                submitted: true,
                revision: 9,
                requires_full_repaint: true,
                damage_rects: Vec::new(),
                text_runs: Vec::new(),
                vertices: vec![EngineRenderVertex {
                    position: [0.0, 0.0],
                    color: [1.0, 1.0, 1.0, 1.0],
                    layer: EngineRenderVertexLayer::Background,
                    command_index: 0,
                }],
                indices: vec![0],
            },
        };
        let prepared =
            EngineWgpuRenderBackend::prepare_frame_for_viewport(&batch.buffer_plan, 80.0, 40.0);

        let frame = EngineRenderPreparedPaneFrame::from_parts(batch, prepared, true, None);

        assert!(frame.replace_diagnostics.replace_ready);
        assert!(frame.replace_diagnostics.should_replace_legacy_pane());
        assert!(!frame.replace_diagnostics.should_log_replace_fallback());
        assert_eq!(frame.replace_diagnostics.pane_id, Some(42));
        assert_eq!(frame.replace_diagnostics.revision, Some(9));
        assert!(frame.replace_diagnostics.prepared_frame_matches_batch);
        assert_eq!(frame.replace_diagnostics.readiness_issue_count(), 0);
        assert!(frame.replace_diagnostics.readiness_issues().is_empty());

        let frame_diagnostics =
            EngineRenderPreparedPaneFrame::replace_diagnostics_for_request(true, Some(&frame))
                .expect("requested frame diagnostics");
        assert!(frame_diagnostics.should_replace_legacy_pane());

        let missing_frame_diagnostics =
            EngineRenderPreparedPaneFrame::replace_diagnostics_for_request(true, None)
                .expect("requested missing-frame diagnostics");
        assert!(missing_frame_diagnostics.should_log_replace_fallback());
        assert_eq!(
            missing_frame_diagnostics.readiness_issues(),
            vec![
                EngineRenderPaneReplaceReadinessIssue::MissingBufferBatch,
                EngineRenderPaneReplaceReadinessIssue::MissingPreparedFrame,
            ]
        );

        assert!(
            EngineRenderPreparedPaneFrame::replace_diagnostics_for_request(false, Some(&frame))
                .is_none()
        );
    }

    #[test]
    fn engine_render_consumer_set_reuses_state_and_resizes_metrics() {
        let engine = next_core();
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
        let mut consumers = EngineRenderConsumerSet::new();
        let metrics = RenderCellMetrics {
            cell_width_px: 8,
            cell_height_px: 16,
        };

        let first = consumers
            .read_buffer_plan(&engine, session.id, metrics)
            .expect("read first render buffer plan");
        assert!(first.stats.submit);
        assert_eq!(consumers.len(), 1);
        assert!(consumers.contains_pane(session.id));
        assert_eq!(
            consumers
                .consumer(session.id)
                .and_then(|consumer| consumer.submitted_revision()),
            Some(first.stats.revision)
        );

        let (repeat, _) = read_unchanged(
            first.stats.revision,
            || {
                consumers
                    .read_buffer_plan(&engine, session.id, metrics)
                    .expect("read repeated render buffer plan")
            },
            |batch| batch.stats.submit,
            |batch| batch.stats.revision,
        );
        assert!(repeat.buffer_plan.vertices.is_empty());
        assert_eq!(consumers.len(), 1);

        let resized_metrics = RenderCellMetrics {
            cell_width_px: 9,
            cell_height_px: 18,
        };
        let resized = consumers
            .read_buffer_plan(&engine, session.id, resized_metrics)
            .expect("read resized render buffer plan");
        assert!(resized.stats.submit);
        assert!(resized.stats.requires_full_repaint);
        assert_eq!(
            consumers
                .consumer(session.id)
                .map(|consumer| consumer.metrics()),
            Some(resized_metrics)
        );

        assert!(consumers.remove_pane(session.id).is_some());
        assert!(consumers.is_empty());
        engine
            .destroy_session(session.id)
            .expect("destroy next-core test session");
    }

    /// Read until one read finds nothing new, and hand that read back.
    ///
    /// A shell on a fresh pty writes its setup sequences over several
    /// milliseconds, so *which* read is the one that finds nothing changed is
    /// a matter of timing. The property under test is what such a read does,
    /// so these tests find one and assert about it rather than assuming the
    /// second read will be it. The revision returned alongside is the last one
    /// that was submitted, which is what an unchanged read reports having seen.
    fn read_unchanged<B>(
        seen: u64,
        mut read: impl FnMut() -> B,
        submitted: impl Fn(&B) -> bool,
        revision: impl Fn(&B) -> u64,
    ) -> (B, u64) {
        let mut last = seen;
        for _ in 0..400 {
            let batch = read();
            if !submitted(&batch) {
                return (batch, last);
            }
            last = revision(&batch);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("next-core never stopped writing to the pane");
    }

    /// A shell that stays quiet and stays alive.
    ///
    /// It has to outlive the whole test: when the command exits the PTY closes
    /// and next-core bumps its revision, which is the very thing the
    /// "reading twice changes nothing" tests assert does not happen. A short
    /// wait made them fail whenever the suite ran slower than the wait.
    fn quiet_wait_command_for_test() -> portable_pty::CommandBuilder {
        #[cfg(windows)]
        {
            let mut command = portable_pty::CommandBuilder::new("cmd.exe");
            command.args(["/c", "ping -n 600 127.0.0.1 >nul"]);
            command
        }
        #[cfg(not(windows))]
        {
            let mut command = portable_pty::CommandBuilder::new("sh");
            command.args(["-c", "sleep 600"]);
            command
        }
    }
}
