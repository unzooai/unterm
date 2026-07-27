//! Renderer-side consumer state for engine-neutral render commits.
//!
//! This keeps the future next-core GUI renderer on the `ScreenEngine`
//! contract. The actual GPU layer should consume the returned commit batch
//! rather than reaching into next-core screen internals.

use super::{
    CommandListRenderBackend, EngineRenderBackend, EngineRenderBufferPlan, RenderCellMetrics,
    RenderCommitPlan, RenderConsumerState, RenderRect, ScreenEngine,
};
use crate::engine::render_backend::{
    EngineRenderCachedGlyphUploadDiagnostics, EngineWgpuPreparedFrameDiagnostics,
    EngineWgpuPreparedFramePlan,
};
use std::collections::HashMap;
use std::rc::Rc;
use wezterm_font::LoadedFont;

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
    pub font: Option<Rc<LoadedFont>>,
}

#[allow(dead_code)]
impl EngineRenderPreparedPaneFrame {
    pub fn from_parts(
        batch: EngineRenderBufferBatch,
        prepared: EngineWgpuPreparedFramePlan,
        replace_requested: bool,
        cached_glyph_upload: Option<&EngineRenderCachedGlyphUploadDiagnostics>,
        font: Option<Rc<LoadedFont>>,
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
            font,
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
        if self.buffer_plan.vertices.is_empty() || self.buffer_plan.indices.is_empty() {
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
