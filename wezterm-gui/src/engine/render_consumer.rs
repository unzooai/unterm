//! Renderer-side consumer state for engine-neutral render commits.
//!
//! This keeps the future next-core GUI renderer on the `ScreenEngine`
//! contract. The actual GPU layer should consume the returned commit batch
//! rather than reaching into next-core screen internals.

use super::{
    CommandListRenderBackend, EngineRenderBackend, EngineRenderBufferPlan, RenderCellMetrics,
    RenderCommitPlan, RenderConsumerState, RenderRect, ScreenEngine,
};

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

#[derive(Clone, Debug)]
pub struct EngineRenderConsumer {
    pane_id: usize,
    metrics: RenderCellMetrics,
    state: RenderConsumerState,
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
