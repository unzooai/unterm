//! Turning a render commit into drawing commands.
//!
//! Deliberately GPU-free: it fixes the command contract a wgpu backend
//! executes, without making any of it depend on a window, an adapter, a font
//! atlas or a swapchain. That is what makes the whole path testable, and it
//! is why the commands are checked here rather than by looking at a screen.

use crate::consumer::EngineRenderCommitBatch;
use unterm_engine::{CellStyle, RenderRect, RenderTextRun, StyledColor, StyledVerticalAlign};

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum EngineRenderBackendCommand {
    Damage(RenderRect),
    Background {
        rect: RenderRect,
        style: CellStyle,
    },
    Text {
        row: usize,
        col: usize,
        cells: usize,
        rect: RenderRect,
        text: String,
        style: CellStyle,
    },
    Cursor {
        rect: RenderRect,
        visible: bool,
        shape: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct EngineRenderBackendFrame {
    pub pane_id: usize,
    pub submitted: bool,
    pub revision: u64,
    pub requires_full_repaint: bool,
    pub skipped_revisions: u64,
    pub commands: Vec<EngineRenderBackendCommand>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum EngineRenderVertexLayer {
    Background,
    Text,
    Cursor,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineRenderVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub layer: EngineRenderVertexLayer,
    pub command_index: u32,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineRenderBufferPlan {
    pub pane_id: usize,
    pub submitted: bool,
    pub revision: u64,
    pub requires_full_repaint: bool,
    pub damage_rects: Vec<RenderRect>,
    pub text_runs: Vec<RenderTextRun>,
    pub vertices: Vec<EngineRenderVertex>,
    pub indices: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineRenderTextAtlasRun {
    pub row: usize,
    pub col: usize,
    pub cells: usize,
    pub text: String,
    pub rect: RenderRect,
    pub foreground: [f32; 4],
    pub style: CellStyle,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineRenderTextAtlasPlan {
    pub pane_id: usize,
    pub submitted: bool,
    pub revision: u64,
    pub requires_full_repaint: bool,
    pub runs: Vec<EngineRenderTextAtlasRun>,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineRenderShapedGlyph {
    pub row: usize,
    pub col: usize,
    pub cells: usize,
    pub text: String,
    pub rect: RenderRect,
    pub x_advance_px: i32,
    pub x_offset_px: i32,
    pub y_offset_px: i32,
    pub foreground: [f32; 4],
    pub style: CellStyle,
    pub font_idx: usize,
    pub glyph_pos: u32,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineRenderShaperGlyph {
    pub text: String,
    pub only_char: Option<char>,
    pub num_cells: u8,
    pub font_idx: usize,
    pub glyph_pos: u32,
    pub x_advance_px: f64,
    pub x_offset_px: f64,
    pub y_offset_px: f64,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineRenderShapedGlyphPlan {
    pub pane_id: usize,
    pub submitted: bool,
    pub revision: u64,
    pub requires_full_repaint: bool,
    pub glyphs: Vec<EngineRenderShapedGlyph>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct EngineRenderGlyphAtlasKey {
    pub text: String,
    pub cells: usize,
    pub font_idx: Option<usize>,
    pub glyph_pos: Option<u32>,
    pub bold: bool,
    pub faint: bool,
    pub italic: bool,
    pub vertical_align: Option<StyledVerticalAlign>,
}

#[allow(dead_code)]
impl EngineRenderGlyphAtlasKey {
    pub fn from_text(text: String, cells: usize, style: &CellStyle) -> Self {
        Self {
            text,
            cells,
            font_idx: None,
            glyph_pos: None,
            bold: style.bold,
            faint: style.faint,
            italic: style.italic,
            vertical_align: style.vertical_align,
        }
    }

    pub fn from_shaped_glyph(
        text: String,
        cells: usize,
        style: &CellStyle,
        font_idx: usize,
        glyph_pos: u32,
    ) -> Self {
        Self {
            text,
            cells,
            font_idx: Some(font_idx),
            glyph_pos: Some(glyph_pos),
            bold: style.bold,
            faint: style.faint,
            italic: style.italic,
            vertical_align: style.vertical_align,
        }
    }

    pub fn raster_identity(&self) -> Option<(usize, u32)> {
        Some((self.font_idx?, self.glyph_pos?))
    }
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineRenderGlyphAtlasInstance {
    pub key_index: usize,
    pub row: usize,
    pub col: usize,
    pub cells: usize,
    pub rect: RenderRect,
    pub x_advance_px: i32,
    pub x_offset_px: i32,
    pub y_offset_px: i32,
    pub foreground: [f32; 4],
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineRenderGlyphAtlasPlan {
    pub pane_id: usize,
    pub submitted: bool,
    pub revision: u64,
    pub requires_full_repaint: bool,
    pub keys: Vec<EngineRenderGlyphAtlasKey>,
    pub instances: Vec<EngineRenderGlyphAtlasInstance>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct EngineRenderGlyphAtlasPlacement {
    pub key_index: usize,
    pub rect: RenderRect,
    pub source_width_px: usize,
    pub source_height_px: usize,
    pub bearing_x_px: i32,
    pub bearing_y_px: i32,
    pub uses_raster_metrics: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[allow(dead_code)]
pub struct EngineRenderGpuVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub layer: u32,
    pub command_index: u32,
}

impl From<EngineRenderVertex> for EngineRenderGpuVertex {
    fn from(vertex: EngineRenderVertex) -> Self {
        Self {
            position: vertex.position,
            color: vertex.color,
            layer: match vertex.layer {
                EngineRenderVertexLayer::Background => 0,
                EngineRenderVertexLayer::Text => 1,
                EngineRenderVertexLayer::Cursor => 2,
            },
            command_index: vertex.command_index,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineRenderGpuUploadPlan {
    pub pane_id: usize,
    pub submitted: bool,
    pub revision: u64,
    pub requires_full_repaint: bool,
    pub vertices: Vec<EngineRenderGpuVertex>,
    pub indices: Vec<u32>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[allow(dead_code)]
pub struct EngineRenderTexturedGlyphVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub key_index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineRenderTexturedGlyphQuad {
    pub left_px: f32,
    pub top_px: f32,
    pub right_px: f32,
    pub bottom_px: f32,
    pub uv_left_px: usize,
    pub uv_top_px: usize,
    pub uv_right_px: usize,
    pub uv_bottom_px: usize,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineRenderTexturedGlyphLayoutEntry {
    pub key_index: usize,
    pub row: usize,
    pub col: usize,
    pub cells: usize,
    pub text: String,
    pub source_rect: RenderRect,
    pub atlas_rect: RenderRect,
    pub quad: EngineRenderTexturedGlyphQuad,
    pub x_advance_px: i32,
    pub x_offset_px: i32,
    pub y_offset_px: i32,
    pub bearing_x_px: i32,
    pub bearing_y_px: i32,
    pub foreground: [f32; 4],
    pub uses_raster_metrics: bool,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineRenderTexturedGlyphLayoutReport {
    pub pane_id: usize,
    pub submitted: bool,
    pub revision: u64,
    pub requires_full_repaint: bool,
    pub entries: Vec<EngineRenderTexturedGlyphLayoutEntry>,
    pub missing_key_indices: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct EngineRenderTexturedGlyphLayoutIdentity {
    pub row: usize,
    pub col: usize,
    pub cells: usize,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineRenderTexturedGlyphLayoutMismatch {
    pub identity: EngineRenderTexturedGlyphLayoutIdentity,
    pub expected_index: usize,
    pub actual_index: usize,
    pub expected_source_rect: RenderRect,
    pub actual_source_rect: RenderRect,
    pub expected_atlas_rect: RenderRect,
    pub actual_atlas_rect: RenderRect,
    pub expected_quad: EngineRenderTexturedGlyphQuad,
    pub actual_quad: EngineRenderTexturedGlyphQuad,
    pub expected_offsets_px: [i32; 3],
    pub actual_offsets_px: [i32; 3],
    pub expected_bearing_px: [i32; 2],
    pub actual_bearing_px: [i32; 2],
    pub expected_foreground: [f32; 4],
    pub actual_foreground: [f32; 4],
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineRenderTexturedGlyphLayoutDiff {
    pub expected_pane_id: usize,
    pub actual_pane_id: usize,
    pub expected_revision: u64,
    pub actual_revision: u64,
    pub expected_entry_count: usize,
    pub actual_entry_count: usize,
    pub missing_entries: Vec<EngineRenderTexturedGlyphLayoutIdentity>,
    pub unexpected_entries: Vec<EngineRenderTexturedGlyphLayoutIdentity>,
    pub mismatches: Vec<EngineRenderTexturedGlyphLayoutMismatch>,
    pub expected_missing_key_indices: Vec<usize>,
    pub actual_missing_key_indices: Vec<usize>,
}

#[allow(dead_code)]
impl EngineRenderTexturedGlyphLayoutEntry {
    pub fn identity(&self) -> EngineRenderTexturedGlyphLayoutIdentity {
        EngineRenderTexturedGlyphLayoutIdentity {
            row: self.row,
            col: self.col,
            cells: self.cells,
            text: self.text.clone(),
        }
    }
}

#[allow(dead_code)]
impl EngineRenderTexturedGlyphLayoutReport {
    pub fn diff_against(
        &self,
        actual: &EngineRenderTexturedGlyphLayoutReport,
    ) -> EngineRenderTexturedGlyphLayoutDiff {
        let mut matched_actual_indices = Vec::new();
        let mut missing_entries = Vec::new();
        let mut mismatches = Vec::new();

        for (expected_index, expected) in self.entries.iter().enumerate() {
            let identity = expected.identity();
            let Some((actual_index, actual_entry)) =
                actual
                    .entries
                    .iter()
                    .enumerate()
                    .find(|(actual_index, actual_entry)| {
                        !matched_actual_indices.contains(actual_index)
                            && actual_entry.identity() == identity
                    })
            else {
                missing_entries.push(identity);
                continue;
            };

            matched_actual_indices.push(actual_index);
            if expected.source_rect != actual_entry.source_rect
                || expected.atlas_rect != actual_entry.atlas_rect
                || expected.quad != actual_entry.quad
                || expected.x_advance_px != actual_entry.x_advance_px
                || expected.x_offset_px != actual_entry.x_offset_px
                || expected.y_offset_px != actual_entry.y_offset_px
                || expected.bearing_x_px != actual_entry.bearing_x_px
                || expected.bearing_y_px != actual_entry.bearing_y_px
                || expected.foreground != actual_entry.foreground
            {
                mismatches.push(EngineRenderTexturedGlyphLayoutMismatch {
                    identity,
                    expected_index,
                    actual_index,
                    expected_source_rect: expected.source_rect,
                    actual_source_rect: actual_entry.source_rect,
                    expected_atlas_rect: expected.atlas_rect,
                    actual_atlas_rect: actual_entry.atlas_rect,
                    expected_quad: expected.quad,
                    actual_quad: actual_entry.quad,
                    expected_offsets_px: [
                        expected.x_advance_px,
                        expected.x_offset_px,
                        expected.y_offset_px,
                    ],
                    actual_offsets_px: [
                        actual_entry.x_advance_px,
                        actual_entry.x_offset_px,
                        actual_entry.y_offset_px,
                    ],
                    expected_bearing_px: [expected.bearing_x_px, expected.bearing_y_px],
                    actual_bearing_px: [actual_entry.bearing_x_px, actual_entry.bearing_y_px],
                    expected_foreground: expected.foreground,
                    actual_foreground: actual_entry.foreground,
                });
            }
        }

        let unexpected_entries = actual
            .entries
            .iter()
            .enumerate()
            .filter(|(actual_index, _)| !matched_actual_indices.contains(actual_index))
            .map(|(_, entry)| entry.identity())
            .collect();

        EngineRenderTexturedGlyphLayoutDiff {
            expected_pane_id: self.pane_id,
            actual_pane_id: actual.pane_id,
            expected_revision: self.revision,
            actual_revision: actual.revision,
            expected_entry_count: self.entries.len(),
            actual_entry_count: actual.entries.len(),
            missing_entries,
            unexpected_entries,
            mismatches,
            expected_missing_key_indices: self.missing_key_indices.clone(),
            actual_missing_key_indices: actual.missing_key_indices.clone(),
        }
    }
}

#[allow(dead_code)]
impl EngineRenderTexturedGlyphLayoutDiff {
    pub fn is_clean(&self) -> bool {
        self.missing_entries.is_empty()
            && self.unexpected_entries.is_empty()
            && self.mismatches.is_empty()
            && self.expected_missing_key_indices == self.actual_missing_key_indices
    }
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineRenderTexturedGlyphUploadPlan {
    pub pane_id: usize,
    pub submitted: bool,
    pub revision: u64,
    pub requires_full_repaint: bool,
    pub layout: EngineRenderTexturedGlyphLayoutReport,
    pub vertices: Vec<EngineRenderTexturedGlyphVertex>,
    pub indices: Vec<u32>,
    pub missing_key_indices: Vec<usize>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub struct EngineRenderCachedGlyphUploadDiagnostics {
    pub pane_id: usize,
    pub submitted: bool,
    pub revision: u64,
    pub cell_width_px: usize,
    pub cell_height_px: usize,
    pub inserted_key_count: usize,
    pub overflow_key_count: usize,
    pub texture_region_count: usize,
    pub texture_missing_key_count: usize,
    pub layout_entry_count: usize,
    pub layout_missing_key_count: usize,
    pub vertex_count: usize,
    pub index_count: usize,
    pub draw_ready: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum EngineRenderCachedGlyphUploadReadinessIssue {
    NotSubmitted,
    EmptyUpload,
    OverflowKeys,
    TextureMissingKeys,
    LayoutMissingKeys,
    NotDrawReady,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct EngineRenderGlyphAtlasCache {
    pub width_px: usize,
    pub height_px: usize,
    pub padding_px: usize,
    pub next_x_px: usize,
    pub next_y_px: usize,
    pub row_height_px: usize,
    pub placements: Vec<EngineRenderGlyphAtlasPlacement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct EngineRenderGlyphAtlasCacheUpdate {
    pub placements: Vec<EngineRenderGlyphAtlasPlacement>,
    pub inserted_key_indices: Vec<usize>,
    pub overflow_key_indices: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct EngineRenderGlyphAtlasTextureRegion {
    pub key_index: usize,
    pub rect: RenderRect,
    pub width_px: usize,
    pub height_px: usize,
    pub source_width_px: usize,
    pub source_height_px: usize,
    pub bearing_x_px: i32,
    pub bearing_y_px: i32,
    pub uses_raster_metrics: bool,
    pub bytes_rgba: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct EngineRenderGlyphRaster {
    pub bytes_rgba: Vec<u8>,
    pub source_width_px: usize,
    pub source_height_px: usize,
    pub bearing_x_px: i32,
    pub bearing_y_px: i32,
    pub uses_raster_metrics: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct EngineRenderGlyphAtlasTextureUpdatePlan {
    pub pane_id: usize,
    pub revision: u64,
    pub atlas_width_px: usize,
    pub atlas_height_px: usize,
    pub regions: Vec<EngineRenderGlyphAtlasTextureRegion>,
    pub overflow_key_indices: Vec<usize>,
    pub missing_key_indices: Vec<usize>,
}

#[allow(dead_code)]
pub trait EngineRenderGlyphRasterSource {
    fn rasterize_glyph_rgba(
        &self,
        key: &EngineRenderGlyphAtlasKey,
        width_px: usize,
        height_px: usize,
    ) -> Option<Vec<u8>>;

    fn rasterize_glyph_texture(
        &self,
        key: &EngineRenderGlyphAtlasKey,
        width_px: usize,
        height_px: usize,
    ) -> Option<EngineRenderGlyphRaster> {
        Some(EngineRenderGlyphRaster {
            bytes_rgba: self.rasterize_glyph_rgba(key, width_px, height_px)?,
            source_width_px: width_px,
            source_height_px: height_px,
            bearing_x_px: 0,
            bearing_y_px: 0,
            uses_raster_metrics: false,
        })
    }
}

#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct EngineRenderDeterministicGlyphRasterSource;

#[allow(dead_code)]
impl EngineRenderGlyphRasterSource for EngineRenderDeterministicGlyphRasterSource {
    fn rasterize_glyph_rgba(
        &self,
        key: &EngineRenderGlyphAtlasKey,
        width_px: usize,
        height_px: usize,
    ) -> Option<Vec<u8>> {
        Some(placeholder_glyph_texture_bytes(key, width_px, height_px))
    }
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineWgpuPreparedFramePlan {
    pub upload: EngineRenderGpuUploadPlan,
    pub text_atlas: EngineRenderTextAtlasPlan,
    pub glyph_atlas: EngineRenderGlyphAtlasPlan,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub struct EngineWgpuPreparedFrameDiagnostics {
    pub pane_id: usize,
    pub submitted: bool,
    pub revision: u64,
    pub solid_vertex_count: usize,
    pub solid_index_count: usize,
    pub text_run_count: usize,
    pub glyph_key_count: usize,
    pub glyph_instance_count: usize,
    pub replace_ready: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum EngineWgpuPreparedFrameReadinessIssue {
    SolidNotSubmitted,
    EmptySolidUpload,
    TextAtlasMissingGlyphs,
}

#[allow(dead_code)]
impl EngineWgpuPreparedFramePlan {
    pub fn diagnostics(&self) -> EngineWgpuPreparedFrameDiagnostics {
        let issues = self.readiness_issues();
        EngineWgpuPreparedFrameDiagnostics {
            pane_id: self.upload.pane_id,
            submitted: self.upload.submitted,
            revision: self.upload.revision,
            solid_vertex_count: self.upload.vertices.len(),
            solid_index_count: self.upload.indices.len(),
            text_run_count: self.text_atlas.runs.len(),
            glyph_key_count: self.glyph_atlas.keys.len(),
            glyph_instance_count: self.glyph_atlas.instances.len(),
            replace_ready: issues.is_empty(),
        }
    }

    pub fn readiness_issues(&self) -> Vec<EngineWgpuPreparedFrameReadinessIssue> {
        let mut issues = Vec::new();
        if !self.upload.submitted {
            issues.push(EngineWgpuPreparedFrameReadinessIssue::SolidNotSubmitted);
        }
        // A frame with no solid geometry is only empty if it has no glyphs
        // either. Text contributes no solid quads -- its pixels come from the
        // textured glyph pass -- so a text-only frame is perfectly drawable.
        if self.upload.is_empty() && self.text_atlas.runs.is_empty() {
            issues.push(EngineWgpuPreparedFrameReadinessIssue::EmptySolidUpload);
        }
        if !self.text_atlas.runs.is_empty() && self.glyph_atlas.instances.is_empty() {
            issues.push(EngineWgpuPreparedFrameReadinessIssue::TextAtlasMissingGlyphs);
        }
        issues
    }

    pub fn is_replace_ready(&self) -> bool {
        self.readiness_issues().is_empty()
    }

    pub fn textured_glyph_layout_report(
        &self,
        placements: &[EngineRenderGlyphAtlasPlacement],
    ) -> EngineRenderTexturedGlyphLayoutReport {
        EngineRenderTexturedGlyphUploadPlan::layout_report_from_glyph_atlas_plan(
            &self.glyph_atlas,
            placements,
        )
    }

    pub fn diff_textured_glyph_layout_against(
        &self,
        actual: &EngineWgpuPreparedFramePlan,
        expected_placements: &[EngineRenderGlyphAtlasPlacement],
        actual_placements: &[EngineRenderGlyphAtlasPlacement],
    ) -> EngineRenderTexturedGlyphLayoutDiff {
        self.textured_glyph_layout_report(expected_placements)
            .diff_against(&actual.textured_glyph_layout_report(actual_placements))
    }
}

/// Where a pane's frame lands inside the render target.
///
/// next-core builds every frame in pane-local pixels with the pane's top-left
/// at the origin. A window showing one pane can map those straight to clip
/// space, but a split has to shift each pane to its own corner of the same
/// target — otherwise every pane draws over the top-left one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EngineRenderViewportPlacement {
    pub origin_x_px: f32,
    pub origin_y_px: f32,
    pub target_width_px: f32,
    pub target_height_px: f32,
}

#[allow(dead_code)]
impl EngineRenderViewportPlacement {
    /// A pane that owns the whole target.
    pub fn fullscreen(target_width_px: f32, target_height_px: f32) -> Self {
        Self::at(0.0, 0.0, target_width_px, target_height_px)
    }

    /// A pane whose top-left sits at `origin` within the target.
    pub fn at(
        origin_x_px: f32,
        origin_y_px: f32,
        target_width_px: f32,
        target_height_px: f32,
    ) -> Self {
        Self {
            origin_x_px,
            origin_y_px,
            // Guard the divisor, not the origin: a zero-sized target would
            // divide by zero, but a zero origin is the common case.
            target_width_px: target_width_px.max(1.0),
            target_height_px: target_height_px.max(1.0),
        }
    }

    /// Map a pane-local pixel to clip space.
    pub fn to_clip(&self, x: f32, y: f32) -> [f32; 2] {
        [
            ((self.origin_x_px + x) / self.target_width_px) * 2.0 - 1.0,
            1.0 - ((self.origin_y_px + y) / self.target_height_px) * 2.0,
        ]
    }
}

#[allow(dead_code)]
impl EngineRenderGpuUploadPlan {
    pub fn from_buffer_plan(plan: &EngineRenderBufferPlan) -> Self {
        Self {
            pane_id: plan.pane_id,
            submitted: plan.submitted,
            revision: plan.revision,
            requires_full_repaint: plan.requires_full_repaint,
            vertices: plan.vertices.iter().copied().map(Into::into).collect(),
            indices: plan.indices.clone(),
        }
    }

    pub fn from_buffer_plan_for_viewport(
        plan: &EngineRenderBufferPlan,
        viewport_width_px: f32,
        viewport_height_px: f32,
    ) -> Self {
        Self::from_buffer_plan_for_placement(
            plan,
            EngineRenderViewportPlacement::fullscreen(viewport_width_px, viewport_height_px),
        )
    }

    pub fn from_buffer_plan_for_placement(
        plan: &EngineRenderBufferPlan,
        placement: EngineRenderViewportPlacement,
    ) -> Self {
        Self {
            pane_id: plan.pane_id,
            submitted: plan.submitted,
            revision: plan.revision,
            requires_full_repaint: plan.requires_full_repaint,
            vertices: plan
                .vertices
                .iter()
                .copied()
                .map(|vertex| EngineRenderGpuVertex {
                    position: placement.to_clip(vertex.position[0], vertex.position[1]),
                    color: vertex.color,
                    layer: match vertex.layer {
                        EngineRenderVertexLayer::Background => 0,
                        EngineRenderVertexLayer::Text => 1,
                        EngineRenderVertexLayer::Cursor => 2,
                    },
                    command_index: vertex.command_index,
                })
                .collect(),
            indices: plan.indices.clone(),
        }
    }

    pub fn vertex_bytes_len(&self) -> usize {
        self.vertices.len() * std::mem::size_of::<EngineRenderGpuVertex>()
    }

    pub fn index_bytes_len(&self) -> usize {
        self.indices.len() * std::mem::size_of::<u32>()
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() || self.indices.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineWgpuRenderPassPlan {
    pub pane_id: usize,
    pub revision: u64,
    pub draw: bool,
    pub vertex_count: usize,
    pub index_count: usize,
    pub clear_color: Option<[f64; 4]>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineWgpuTexturedGlyphPassPlan {
    pub pane_id: usize,
    pub revision: u64,
    pub draw: bool,
    pub vertex_count: usize,
    pub index_count: usize,
}

#[allow(dead_code)]
impl EngineWgpuRenderPassPlan {
    pub fn from_upload_plan(
        plan: &EngineRenderGpuUploadPlan,
        clear_color: Option<[f64; 4]>,
    ) -> Self {
        Self {
            pane_id: plan.pane_id,
            revision: plan.revision,
            draw: plan.submitted && !plan.is_empty(),
            vertex_count: plan.vertices.len(),
            index_count: plan.indices.len(),
            clear_color,
        }
    }
}

#[allow(dead_code)]
impl EngineWgpuTexturedGlyphPassPlan {
    pub fn from_upload_plan(plan: &EngineRenderTexturedGlyphUploadPlan) -> Self {
        Self {
            pane_id: plan.pane_id,
            revision: plan.revision,
            draw: plan.submitted && !plan.is_empty() && plan.missing_key_indices.is_empty(),
            vertex_count: plan.vertices.len(),
            index_count: plan.indices.len(),
        }
    }
}

#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct EngineWgpuRenderBackend;

#[allow(dead_code)]
impl EngineWgpuRenderBackend {
    pub fn prepare_text_atlas(plan: &EngineRenderBufferPlan) -> EngineRenderTextAtlasPlan {
        EngineRenderTextAtlasPlan::from_buffer_plan(plan)
    }

    pub fn prepare_shaped_glyph_plan(
        plan: &EngineRenderTextAtlasPlan,
        shaped_runs: &[Vec<EngineRenderShaperGlyph>],
    ) -> EngineRenderShapedGlyphPlan {
        EngineRenderShapedGlyphPlan::from_text_atlas_plan_and_shaper_glyphs(plan, shaped_runs)
    }

    pub fn prepare_glyph_atlas(plan: &EngineRenderBufferPlan) -> EngineRenderGlyphAtlasPlan {
        EngineRenderGlyphAtlasPlan::from_text_atlas_plan(&Self::prepare_text_atlas(plan))
    }

    pub fn prepare_glyph_atlas_from_shaped_glyphs(
        plan: &EngineRenderShapedGlyphPlan,
    ) -> EngineRenderGlyphAtlasPlan {
        EngineRenderGlyphAtlasPlan::from_shaped_glyph_plan(plan)
    }

    pub fn prepare_textured_glyph_upload_for_viewport(
        glyphs: &EngineRenderGlyphAtlasPlan,
        placements: &[EngineRenderGlyphAtlasPlacement],
        viewport_width_px: f32,
        viewport_height_px: f32,
        atlas_width_px: f32,
        atlas_height_px: f32,
    ) -> EngineRenderTexturedGlyphUploadPlan {
        Self::prepare_textured_glyph_upload_for_placement(
            glyphs,
            placements,
            EngineRenderViewportPlacement::fullscreen(viewport_width_px, viewport_height_px),
            atlas_width_px,
            atlas_height_px,
        )
    }

    pub fn prepare_textured_glyph_upload_for_placement(
        glyphs: &EngineRenderGlyphAtlasPlan,
        placements: &[EngineRenderGlyphAtlasPlacement],
        viewport: EngineRenderViewportPlacement,
        atlas_width_px: f32,
        atlas_height_px: f32,
    ) -> EngineRenderTexturedGlyphUploadPlan {
        EngineRenderTexturedGlyphUploadPlan::from_glyph_atlas_plan_for_placement(
            glyphs,
            placements,
            viewport,
            atlas_width_px,
            atlas_height_px,
        )
    }

    pub fn prepare_cached_textured_glyph_upload_for_viewport(
        glyphs: &EngineRenderGlyphAtlasPlan,
        cache: &mut EngineRenderGlyphAtlasCache,
        cell_width_px: usize,
        cell_height_px: usize,
        viewport_width_px: f32,
        viewport_height_px: f32,
    ) -> (
        EngineRenderGlyphAtlasCacheUpdate,
        EngineRenderTexturedGlyphUploadPlan,
    ) {
        let update = cache.ensure_glyphs(glyphs, cell_width_px, cell_height_px);
        let upload = Self::prepare_textured_glyph_upload_for_viewport(
            glyphs,
            &update.placements,
            viewport_width_px,
            viewport_height_px,
            cache.width_px as f32,
            cache.height_px as f32,
        );
        (update, upload)
    }

    pub fn prepare_glyph_atlas_texture_update(
        glyphs: &EngineRenderGlyphAtlasPlan,
        update: &EngineRenderGlyphAtlasCacheUpdate,
        atlas_width_px: usize,
        atlas_height_px: usize,
    ) -> EngineRenderGlyphAtlasTextureUpdatePlan {
        EngineRenderGlyphAtlasTextureUpdatePlan::from_cache_update(
            glyphs,
            update,
            atlas_width_px,
            atlas_height_px,
        )
    }

    pub fn prepare_glyph_atlas_texture_update_with_raster_source(
        glyphs: &EngineRenderGlyphAtlasPlan,
        update: &EngineRenderGlyphAtlasCacheUpdate,
        atlas_width_px: usize,
        atlas_height_px: usize,
        raster_source: &dyn EngineRenderGlyphRasterSource,
    ) -> EngineRenderGlyphAtlasTextureUpdatePlan {
        EngineRenderGlyphAtlasTextureUpdatePlan::from_cache_update_with_raster_source(
            glyphs,
            update,
            atlas_width_px,
            atlas_height_px,
            raster_source,
        )
    }

    pub fn prepare_frame_for_viewport(
        plan: &EngineRenderBufferPlan,
        viewport_width_px: f32,
        viewport_height_px: f32,
    ) -> EngineWgpuPreparedFramePlan {
        Self::prepare_frame_for_placement(
            plan,
            EngineRenderViewportPlacement::fullscreen(viewport_width_px, viewport_height_px),
        )
    }

    pub fn prepare_frame_for_placement(
        plan: &EngineRenderBufferPlan,
        viewport: EngineRenderViewportPlacement,
    ) -> EngineWgpuPreparedFramePlan {
        let text_atlas = Self::prepare_text_atlas(plan);
        let glyph_atlas = EngineRenderGlyphAtlasPlan::from_text_atlas_plan(&text_atlas);
        EngineWgpuPreparedFramePlan {
            upload: EngineRenderGpuUploadPlan::from_buffer_plan_for_placement(plan, viewport),
            text_atlas,
            glyph_atlas,
        }
    }

    pub fn prepare_upload(plan: &EngineRenderBufferPlan) -> EngineRenderGpuUploadPlan {
        EngineRenderGpuUploadPlan::from_buffer_plan(plan)
    }

    pub fn prepare_upload_for_viewport(
        plan: &EngineRenderBufferPlan,
        viewport_width_px: f32,
        viewport_height_px: f32,
    ) -> EngineRenderGpuUploadPlan {
        EngineRenderGpuUploadPlan::from_buffer_plan_for_viewport(
            plan,
            viewport_width_px,
            viewport_height_px,
        )
    }

    pub fn prepare_pass(
        &self,
        plan: &EngineRenderGpuUploadPlan,
        clear_color: Option<[f64; 4]>,
    ) -> EngineWgpuRenderPassPlan {
        EngineWgpuRenderPassPlan::from_upload_plan(plan, clear_color)
    }

    pub fn prepare_textured_glyph_pass(
        &self,
        plan: &EngineRenderTexturedGlyphUploadPlan,
    ) -> EngineWgpuTexturedGlyphPassPlan {
        EngineWgpuTexturedGlyphPassPlan::from_upload_plan(plan)
    }
}

#[allow(dead_code)]
impl EngineRenderBufferPlan {
    pub fn from_frame(frame: &EngineRenderBackendFrame) -> Self {
        let mut damage_rects = Vec::new();
        let mut text_runs = Vec::new();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for (command_index, command) in frame.commands.iter().enumerate() {
            match command {
                EngineRenderBackendCommand::Damage(rect) => damage_rects.push(*rect),
                EngineRenderBackendCommand::Background { rect, .. } => {
                    let color = background_color_for_command(command);
                    push_quad_vertices(
                        &mut vertices,
                        &mut indices,
                        *rect,
                        color,
                        EngineRenderVertexLayer::Background,
                        command_index,
                    );
                }
                EngineRenderBackendCommand::Text {
                    row,
                    col,
                    cells,
                    rect,
                    text,
                    style,
                } => {
                    text_runs.push(RenderTextRun {
                        row: *row,
                        col: *col,
                        cells: *cells,
                        text: text.clone(),
                        rect: *rect,
                        style: style.clone(),
                    });
                    // No solid quad for text. The glyph pixels come from the
                    // textured glyph pass, masked by each glyph's coverage;
                    // filling the run's rect here would paint an opaque block
                    // in the foreground colour over the glyphs. (It used to,
                    // back when there was no textured pass to draw them.)
                    //
                    // Losing the glyph pass entirely is not a way to lose the
                    // text: replace readiness requires a matching cached glyph
                    // upload whenever there are text runs, so a frame without
                    // glyphs is not drawn at all and the legacy renderer keeps
                    // the pane.
                    let _ = command_index;
                }
                EngineRenderBackendCommand::Cursor { rect, .. } => {
                    push_quad_vertices(
                        &mut vertices,
                        &mut indices,
                        *rect,
                        [1.0, 1.0, 1.0, 1.0],
                        EngineRenderVertexLayer::Cursor,
                        command_index,
                    );
                }
            }
        }

        Self {
            pane_id: frame.pane_id,
            submitted: frame.submitted,
            revision: frame.revision,
            requires_full_repaint: frame.requires_full_repaint,
            damage_rects,
            text_runs,
            vertices,
            indices,
        }
    }
}

#[allow(dead_code)]
impl EngineRenderTextAtlasPlan {
    pub fn from_buffer_plan(plan: &EngineRenderBufferPlan) -> Self {
        let runs = if plan.submitted {
            plan.text_runs
                .iter()
                .map(|run| EngineRenderTextAtlasRun {
                    row: run.row,
                    col: run.col,
                    cells: run.cells,
                    text: run.text.clone(),
                    rect: run.rect,
                    foreground: styled_color_rgba(run.style.fg).unwrap_or([1.0, 1.0, 1.0, 1.0]),
                    style: run.style.clone(),
                })
                .collect()
        } else {
            Vec::new()
        };

        Self {
            pane_id: plan.pane_id,
            submitted: plan.submitted,
            revision: plan.revision,
            requires_full_repaint: plan.requires_full_repaint,
            runs,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }
}

#[allow(dead_code)]
impl EngineRenderShapedGlyphPlan {
    pub fn from_text_atlas_plan_and_shaper_glyphs(
        plan: &EngineRenderTextAtlasPlan,
        shaped_runs: &[Vec<EngineRenderShaperGlyph>],
    ) -> Self {
        let mut glyphs = Vec::new();

        if plan.submitted {
            for (run, shaper_glyphs) in plan.runs.iter().zip(shaped_runs.iter()) {
                push_shaped_glyphs_from_shaper_glyphs(&mut glyphs, run, shaper_glyphs);
            }
        }

        Self {
            pane_id: plan.pane_id,
            submitted: plan.submitted,
            revision: plan.revision,
            requires_full_repaint: plan.requires_full_repaint,
            glyphs,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }
}

#[allow(dead_code)]
impl EngineRenderGlyphAtlasPlan {
    pub fn from_text_atlas_plan(plan: &EngineRenderTextAtlasPlan) -> Self {
        let mut keys = Vec::new();
        let mut instances = Vec::new();

        if plan.submitted {
            for run in &plan.runs {
                push_glyph_atlas_instances(&mut keys, &mut instances, run);
            }
        }

        Self {
            pane_id: plan.pane_id,
            submitted: plan.submitted,
            revision: plan.revision,
            requires_full_repaint: plan.requires_full_repaint,
            keys,
            instances,
        }
    }

    pub fn from_shaped_glyph_plan(plan: &EngineRenderShapedGlyphPlan) -> Self {
        let mut keys = Vec::new();
        let mut instances = Vec::new();

        if plan.submitted {
            for glyph in &plan.glyphs {
                push_shaped_glyph_atlas_instance(&mut keys, &mut instances, glyph);
            }
        }

        Self {
            pane_id: plan.pane_id,
            submitted: plan.submitted,
            revision: plan.revision,
            requires_full_repaint: plan.requires_full_repaint,
            keys,
            instances,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }
}

#[allow(dead_code)]
impl EngineRenderGlyphAtlasCache {
    pub fn new(width_px: usize, height_px: usize) -> Self {
        Self {
            width_px: width_px.max(1),
            height_px: height_px.max(1),
            padding_px: 1,
            next_x_px: 0,
            next_y_px: 0,
            row_height_px: 0,
            placements: Vec::new(),
        }
    }

    pub fn ensure_glyphs(
        &mut self,
        plan: &EngineRenderGlyphAtlasPlan,
        cell_width_px: usize,
        cell_height_px: usize,
    ) -> EngineRenderGlyphAtlasCacheUpdate {
        let mut inserted_key_indices = Vec::new();
        let mut overflow_key_indices = Vec::new();

        if plan.submitted {
            for (key_index, key) in plan.keys.iter().enumerate() {
                if self.has_placement(key_index) {
                    continue;
                }
                let width = key
                    .cells
                    .max(1)
                    .saturating_mul(cell_width_px.max(1))
                    .saturating_add(self.padding_px.saturating_mul(2));
                let height = cell_height_px
                    .max(1)
                    .saturating_add(self.padding_px.saturating_mul(2));
                match self.allocate(key_index, width, height) {
                    Some(placement) => {
                        self.placements.push(placement);
                        inserted_key_indices.push(key_index);
                    }
                    None => push_unique_usize(&mut overflow_key_indices, key_index),
                }
            }
        }

        EngineRenderGlyphAtlasCacheUpdate {
            placements: self.placements.clone(),
            inserted_key_indices,
            overflow_key_indices,
        }
    }

    fn has_placement(&self, key_index: usize) -> bool {
        self.placements
            .iter()
            .any(|placement| placement.key_index == key_index)
    }

    fn allocate(
        &mut self,
        key_index: usize,
        width_px: usize,
        height_px: usize,
    ) -> Option<EngineRenderGlyphAtlasPlacement> {
        if width_px > self.width_px || height_px > self.height_px {
            return None;
        }
        if self.next_x_px.saturating_add(width_px) > self.width_px {
            self.next_x_px = 0;
            self.next_y_px = self.next_y_px.saturating_add(self.row_height_px);
            self.row_height_px = 0;
        }
        if self.next_y_px.saturating_add(height_px) > self.height_px {
            return None;
        }

        let placement = EngineRenderGlyphAtlasPlacement {
            key_index,
            rect: RenderRect {
                x: self.next_x_px.saturating_add(self.padding_px),
                y: self.next_y_px.saturating_add(self.padding_px),
                width: width_px.saturating_sub(self.padding_px.saturating_mul(2)),
                height: height_px.saturating_sub(self.padding_px.saturating_mul(2)),
            },
            source_width_px: width_px.saturating_sub(self.padding_px.saturating_mul(2)),
            source_height_px: height_px.saturating_sub(self.padding_px.saturating_mul(2)),
            bearing_x_px: 0,
            bearing_y_px: 0,
            uses_raster_metrics: false,
        };
        self.next_x_px = self.next_x_px.saturating_add(width_px);
        self.row_height_px = self.row_height_px.max(height_px);
        Some(placement)
    }

    pub fn apply_texture_update_metrics(
        &mut self,
        update: &EngineRenderGlyphAtlasTextureUpdatePlan,
    ) {
        for region in &update.regions {
            if let Some(placement) = self
                .placements
                .iter_mut()
                .find(|placement| placement.key_index == region.key_index)
            {
                placement.source_width_px = region.source_width_px;
                placement.source_height_px = region.source_height_px;
                placement.bearing_x_px = region.bearing_x_px;
                placement.bearing_y_px = region.bearing_y_px;
                placement.uses_raster_metrics = region.uses_raster_metrics;
            }
        }
    }
}

#[allow(dead_code)]
impl EngineRenderGlyphAtlasTextureUpdatePlan {
    pub fn from_cache_update(
        glyphs: &EngineRenderGlyphAtlasPlan,
        update: &EngineRenderGlyphAtlasCacheUpdate,
        atlas_width_px: usize,
        atlas_height_px: usize,
    ) -> Self {
        Self::from_cache_update_with_raster_source(
            glyphs,
            update,
            atlas_width_px,
            atlas_height_px,
            &EngineRenderDeterministicGlyphRasterSource,
        )
    }

    pub fn from_cache_update_with_raster_source(
        glyphs: &EngineRenderGlyphAtlasPlan,
        update: &EngineRenderGlyphAtlasCacheUpdate,
        atlas_width_px: usize,
        atlas_height_px: usize,
        raster_source: &dyn EngineRenderGlyphRasterSource,
    ) -> Self {
        let mut regions = Vec::new();
        let mut missing_key_indices = Vec::new();

        if glyphs.submitted {
            for key_index in &update.inserted_key_indices {
                let Some(key) = glyphs.keys.get(*key_index) else {
                    push_unique_usize(&mut missing_key_indices, *key_index);
                    continue;
                };
                let Some(placement) = update
                    .placements
                    .iter()
                    .find(|placement| placement.key_index == *key_index)
                else {
                    push_unique_usize(&mut missing_key_indices, *key_index);
                    continue;
                };
                let width_px = placement.rect.width.max(1);
                let height_px = placement.rect.height.max(1);
                let expected_bytes = width_px.saturating_mul(height_px).saturating_mul(4);
                let Some(raster) = raster_source.rasterize_glyph_texture(key, width_px, height_px)
                else {
                    push_unique_usize(&mut missing_key_indices, *key_index);
                    continue;
                };
                if raster.bytes_rgba.len() != expected_bytes {
                    push_unique_usize(&mut missing_key_indices, *key_index);
                    continue;
                }
                regions.push(EngineRenderGlyphAtlasTextureRegion {
                    key_index: *key_index,
                    rect: placement.rect,
                    width_px,
                    height_px,
                    source_width_px: raster.source_width_px,
                    source_height_px: raster.source_height_px,
                    bearing_x_px: raster.bearing_x_px,
                    bearing_y_px: raster.bearing_y_px,
                    uses_raster_metrics: raster.uses_raster_metrics,
                    bytes_rgba: raster.bytes_rgba,
                });
            }
        }

        Self {
            pane_id: glyphs.pane_id,
            revision: glyphs.revision,
            atlas_width_px: atlas_width_px.max(1),
            atlas_height_px: atlas_height_px.max(1),
            regions,
            overflow_key_indices: update.overflow_key_indices.clone(),
            missing_key_indices,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }
}

#[allow(dead_code)]
impl EngineRenderTexturedGlyphUploadPlan {
    pub fn layout_report_from_glyph_atlas_plan(
        plan: &EngineRenderGlyphAtlasPlan,
        placements: &[EngineRenderGlyphAtlasPlacement],
    ) -> EngineRenderTexturedGlyphLayoutReport {
        let mut entries = Vec::new();
        let mut missing_key_indices = Vec::new();

        if plan.submitted {
            for instance in &plan.instances {
                let Some(placement) = placements
                    .iter()
                    .find(|placement| placement.key_index == instance.key_index)
                else {
                    push_unique_usize(&mut missing_key_indices, instance.key_index);
                    continue;
                };
                let text = plan
                    .keys
                    .get(instance.key_index)
                    .map(|key| key.text.clone())
                    .unwrap_or_default();
                entries.push(EngineRenderTexturedGlyphLayoutEntry {
                    key_index: instance.key_index,
                    row: instance.row,
                    col: instance.col,
                    cells: instance.cells,
                    text,
                    source_rect: instance.rect,
                    atlas_rect: placement.rect,
                    quad: textured_glyph_quad_pixels(instance, placement),
                    x_advance_px: instance.x_advance_px,
                    x_offset_px: instance.x_offset_px,
                    y_offset_px: instance.y_offset_px,
                    bearing_x_px: placement.bearing_x_px,
                    bearing_y_px: placement.bearing_y_px,
                    foreground: instance.foreground,
                    uses_raster_metrics: placement.uses_raster_metrics,
                });
            }
        }

        EngineRenderTexturedGlyphLayoutReport {
            pane_id: plan.pane_id,
            submitted: plan.submitted,
            revision: plan.revision,
            requires_full_repaint: plan.requires_full_repaint,
            entries,
            missing_key_indices,
        }
    }

    pub fn from_glyph_atlas_plan_for_viewport(
        plan: &EngineRenderGlyphAtlasPlan,
        placements: &[EngineRenderGlyphAtlasPlacement],
        viewport_width_px: f32,
        viewport_height_px: f32,
        atlas_width_px: f32,
        atlas_height_px: f32,
    ) -> Self {
        Self::from_glyph_atlas_plan_for_placement(
            plan,
            placements,
            EngineRenderViewportPlacement::fullscreen(viewport_width_px, viewport_height_px),
            atlas_width_px,
            atlas_height_px,
        )
    }

    pub fn from_glyph_atlas_plan_for_placement(
        plan: &EngineRenderGlyphAtlasPlan,
        placements: &[EngineRenderGlyphAtlasPlacement],
        viewport: EngineRenderViewportPlacement,
        atlas_width_px: f32,
        atlas_height_px: f32,
    ) -> Self {
        let atlas_width_px = atlas_width_px.max(1.0);
        let atlas_height_px = atlas_height_px.max(1.0);
        let layout = Self::layout_report_from_glyph_atlas_plan(plan, placements);
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        if plan.submitted && layout.missing_key_indices.is_empty() {
            for entry in &layout.entries {
                push_textured_glyph_quad_from_layout(
                    &mut vertices,
                    &mut indices,
                    entry,
                    viewport,
                    atlas_width_px,
                    atlas_height_px,
                );
            }
        }

        Self {
            pane_id: plan.pane_id,
            submitted: plan.submitted,
            revision: plan.revision,
            requires_full_repaint: plan.requires_full_repaint,
            missing_key_indices: layout.missing_key_indices.clone(),
            layout,
            vertices,
            indices,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() || self.indices.is_empty()
    }
}

#[allow(dead_code)]
impl EngineRenderCachedGlyphUploadDiagnostics {
    pub fn from_parts(
        cell_width_px: usize,
        cell_height_px: usize,
        update: &EngineRenderGlyphAtlasCacheUpdate,
        texture_update: &EngineRenderGlyphAtlasTextureUpdatePlan,
        upload: &EngineRenderTexturedGlyphUploadPlan,
    ) -> Self {
        Self {
            pane_id: upload.pane_id,
            submitted: upload.submitted,
            revision: upload.revision,
            cell_width_px,
            cell_height_px,
            inserted_key_count: update.inserted_key_indices.len(),
            overflow_key_count: update.overflow_key_indices.len(),
            texture_region_count: texture_update.regions.len(),
            texture_missing_key_count: texture_update.missing_key_indices.len(),
            layout_entry_count: upload.layout.entries.len(),
            layout_missing_key_count: upload.layout.missing_key_indices.len(),
            vertex_count: upload.vertices.len(),
            index_count: upload.indices.len(),
            draw_ready: upload.submitted
                && !upload.is_empty()
                && update.overflow_key_indices.is_empty()
                && texture_update.missing_key_indices.is_empty()
                && upload.missing_key_indices.is_empty(),
        }
    }

    pub fn readiness_issues(&self) -> Vec<EngineRenderCachedGlyphUploadReadinessIssue> {
        let mut issues = Vec::new();
        if !self.submitted {
            issues.push(EngineRenderCachedGlyphUploadReadinessIssue::NotSubmitted);
        }
        if self.vertex_count == 0 || self.index_count == 0 || self.layout_entry_count == 0 {
            issues.push(EngineRenderCachedGlyphUploadReadinessIssue::EmptyUpload);
        }
        if self.overflow_key_count > 0 {
            issues.push(EngineRenderCachedGlyphUploadReadinessIssue::OverflowKeys);
        }
        if self.texture_missing_key_count > 0 {
            issues.push(EngineRenderCachedGlyphUploadReadinessIssue::TextureMissingKeys);
        }
        if self.layout_missing_key_count > 0 {
            issues.push(EngineRenderCachedGlyphUploadReadinessIssue::LayoutMissingKeys);
        }
        if !self.draw_ready {
            issues.push(EngineRenderCachedGlyphUploadReadinessIssue::NotDrawReady);
        }
        issues
    }

    pub fn is_ready(&self) -> bool {
        self.readiness_issues().is_empty()
    }
}

#[allow(dead_code)]
pub trait EngineRenderBackend {
    fn submit(
        &mut self,
        batch: &EngineRenderCommitBatch,
    ) -> anyhow::Result<EngineRenderBackendFrame>;
}

#[allow(dead_code)]
fn push_quad_vertices(
    vertices: &mut Vec<EngineRenderVertex>,
    indices: &mut Vec<u32>,
    rect: RenderRect,
    color: [f32; 4],
    layer: EngineRenderVertexLayer,
    command_index: usize,
) {
    let base = vertices.len() as u32;
    let left = rect.x as f32;
    let top = rect.y as f32;
    let right = rect.x.saturating_add(rect.width) as f32;
    let bottom = rect.y.saturating_add(rect.height) as f32;
    let command_index = command_index as u32;

    vertices.extend([
        EngineRenderVertex {
            position: [left, top],
            color,
            layer,
            command_index,
        },
        EngineRenderVertex {
            position: [right, top],
            color,
            layer,
            command_index,
        },
        EngineRenderVertex {
            position: [left, bottom],
            color,
            layer,
            command_index,
        },
        EngineRenderVertex {
            position: [right, bottom],
            color,
            layer,
            command_index,
        },
    ]);
    indices.extend([base, base + 1, base + 2, base + 1, base + 2, base + 3]);
}

fn background_color_for_command(command: &EngineRenderBackendCommand) -> [f32; 4] {
    match command {
        EngineRenderBackendCommand::Background { style, .. } => {
            styled_color_rgba(style.bg).unwrap_or([0.0, 0.0, 0.0, 1.0])
        }
        _ => [0.0, 0.0, 0.0, 1.0],
    }
}

fn foreground_color_for_command(command: &EngineRenderBackendCommand) -> [f32; 4] {
    match command {
        EngineRenderBackendCommand::Text { style, .. } => {
            styled_color_rgba(style.fg).unwrap_or([1.0, 1.0, 1.0, 1.0])
        }
        _ => [1.0, 1.0, 1.0, 1.0],
    }
}

fn styled_color_rgba(color: Option<StyledColor>) -> Option<[f32; 4]> {
    match color? {
        StyledColor::Rgb(r, g, b) => {
            Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0])
        }
        StyledColor::Palette(index) => {
            let [r, g, b] = ansi_palette_rgb(index);
            Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0])
        }
    }
}

fn push_glyph_atlas_instances(
    keys: &mut Vec<EngineRenderGlyphAtlasKey>,
    instances: &mut Vec<EngineRenderGlyphAtlasInstance>,
    run: &EngineRenderTextAtlasRun,
) {
    let chars: Vec<char> = run.text.chars().collect();
    if run.cells > 0 && chars.len() == run.cells {
        let base_width = run.rect.width / run.cells;
        let remainder = run.rect.width % run.cells;
        let mut x = run.rect.x;
        for (offset, ch) in chars.into_iter().enumerate() {
            let width = base_width + usize::from(offset < remainder);
            push_glyph_atlas_instance(
                keys,
                instances,
                EngineRenderGlyphAtlasKey::from_text(ch.to_string(), 1, &run.style),
                EngineRenderGlyphAtlasInstance {
                    key_index: 0,
                    row: run.row,
                    col: run.col + offset,
                    cells: 1,
                    rect: RenderRect {
                        x,
                        y: run.rect.y,
                        width,
                        height: run.rect.height,
                    },
                    x_advance_px: width as i32,
                    x_offset_px: 0,
                    y_offset_px: 0,
                    foreground: run.foreground,
                },
            );
            x = x.saturating_add(width);
        }
    } else {
        push_glyph_atlas_instance(
            keys,
            instances,
            EngineRenderGlyphAtlasKey::from_text(run.text.clone(), run.cells, &run.style),
            EngineRenderGlyphAtlasInstance {
                key_index: 0,
                row: run.row,
                col: run.col,
                cells: run.cells,
                rect: run.rect,
                x_advance_px: run.rect.width as i32,
                x_offset_px: 0,
                y_offset_px: 0,
                foreground: run.foreground,
            },
        );
    }
}

fn push_shaped_glyphs_from_shaper_glyphs(
    glyphs: &mut Vec<EngineRenderShapedGlyph>,
    run: &EngineRenderTextAtlasRun,
    shaper_glyphs: &[EngineRenderShaperGlyph],
) {
    let mut pen_x = 0.0f64;
    let mut consumed_cells = 0usize;
    for glyph in shaper_glyphs {
        let cells = usize::from(glyph.num_cells.max(1));
        let x_offset_px = glyph.x_offset_px.round() as i32;
        let y_offset_px = glyph.y_offset_px.round() as i32;
        let x_advance_px = glyph.x_advance_px.round() as i32;
        let x = run.rect.x as i64 + pen_x.round() as i64 + x_offset_px as i64;
        let y = run.rect.y as i64 - y_offset_px as i64;
        let width = x_advance_px.unsigned_abs().max(1) as usize;
        let height = run.rect.height.max(1);
        glyphs.push(EngineRenderShapedGlyph {
            row: run.row,
            col: run.col.saturating_add(consumed_cells),
            cells,
            text: shaped_glyph_text(run, glyph),
            rect: RenderRect {
                x: x.max(0) as usize,
                y: y.max(0) as usize,
                width,
                height,
            },
            x_advance_px,
            x_offset_px,
            y_offset_px,
            foreground: run.foreground,
            style: run.style.clone(),
            font_idx: glyph.font_idx,
            glyph_pos: glyph.glyph_pos,
        });
        pen_x += glyph.x_advance_px;
        consumed_cells = consumed_cells.saturating_add(cells);
    }
}

fn shaped_glyph_text(run: &EngineRenderTextAtlasRun, glyph: &EngineRenderShaperGlyph) -> String {
    glyph
        .only_char
        .map(|ch| ch.to_string())
        .unwrap_or_else(|| run.text.clone())
}

fn push_shaped_glyph_atlas_instance(
    keys: &mut Vec<EngineRenderGlyphAtlasKey>,
    instances: &mut Vec<EngineRenderGlyphAtlasInstance>,
    glyph: &EngineRenderShapedGlyph,
) {
    push_glyph_atlas_instance(
        keys,
        instances,
        EngineRenderGlyphAtlasKey::from_shaped_glyph(
            glyph.text.clone(),
            glyph.cells,
            &glyph.style,
            glyph.font_idx,
            glyph.glyph_pos,
        ),
        EngineRenderGlyphAtlasInstance {
            key_index: 0,
            row: glyph.row,
            col: glyph.col,
            cells: glyph.cells,
            rect: glyph.rect,
            x_advance_px: glyph.x_advance_px,
            x_offset_px: glyph.x_offset_px,
            y_offset_px: glyph.y_offset_px,
            foreground: glyph.foreground,
        },
    );
}

fn push_glyph_atlas_instance(
    keys: &mut Vec<EngineRenderGlyphAtlasKey>,
    instances: &mut Vec<EngineRenderGlyphAtlasInstance>,
    key: EngineRenderGlyphAtlasKey,
    mut instance: EngineRenderGlyphAtlasInstance,
) {
    let key_index = keys.iter().position(|existing| existing == &key);
    instance.key_index = match key_index {
        Some(index) => index,
        None => {
            keys.push(key);
            keys.len() - 1
        }
    };
    instances.push(instance);
}

fn push_textured_glyph_quad_from_layout(
    vertices: &mut Vec<EngineRenderTexturedGlyphVertex>,
    indices: &mut Vec<u32>,
    entry: &EngineRenderTexturedGlyphLayoutEntry,
    viewport: EngineRenderViewportPlacement,
    atlas_width_px: f32,
    atlas_height_px: f32,
) {
    let base = vertices.len() as u32;
    let quad = entry.quad;
    let uv_left = quad.uv_left_px as f32 / atlas_width_px;
    let uv_top = quad.uv_top_px as f32 / atlas_height_px;
    let uv_right = quad.uv_right_px as f32 / atlas_width_px;
    let uv_bottom = quad.uv_bottom_px as f32 / atlas_height_px;
    let key_index = entry.key_index as u32;

    vertices.extend([
        EngineRenderTexturedGlyphVertex {
            position: viewport.to_clip(quad.left_px, quad.top_px),
            uv: [uv_left, uv_top],
            color: entry.foreground,
            key_index,
        },
        EngineRenderTexturedGlyphVertex {
            position: viewport.to_clip(quad.right_px, quad.top_px),
            uv: [uv_right, uv_top],
            color: entry.foreground,
            key_index,
        },
        EngineRenderTexturedGlyphVertex {
            position: viewport.to_clip(quad.left_px, quad.bottom_px),
            uv: [uv_left, uv_bottom],
            color: entry.foreground,
            key_index,
        },
        EngineRenderTexturedGlyphVertex {
            position: viewport.to_clip(quad.right_px, quad.bottom_px),
            uv: [uv_right, uv_bottom],
            color: entry.foreground,
            key_index,
        },
    ]);
    indices.extend([base, base + 1, base + 2, base + 1, base + 2, base + 3]);
}

fn textured_glyph_quad_pixels(
    instance: &EngineRenderGlyphAtlasInstance,
    placement: &EngineRenderGlyphAtlasPlacement,
) -> EngineRenderTexturedGlyphQuad {
    let atlas_rect = placement.rect;
    if placement.uses_raster_metrics {
        let glyph_width = placement
            .source_width_px
            .max(1)
            .min(atlas_rect.width.max(1));
        let glyph_height = placement
            .source_height_px
            .max(1)
            .min(atlas_rect.height.max(1));
        let left = instance.rect.x as f32 + placement.bearing_x_px as f32;
        let top =
            instance.rect.y as f32 + instance.rect.height as f32 - placement.bearing_y_px as f32;
        EngineRenderTexturedGlyphQuad {
            left_px: left,
            top_px: top,
            right_px: left + glyph_width as f32,
            bottom_px: top + glyph_height as f32,
            uv_left_px: atlas_rect.x,
            uv_top_px: atlas_rect.y,
            uv_right_px: atlas_rect.x.saturating_add(glyph_width),
            uv_bottom_px: atlas_rect.y.saturating_add(glyph_height),
        }
    } else {
        EngineRenderTexturedGlyphQuad {
            left_px: instance.rect.x as f32,
            top_px: instance.rect.y as f32,
            right_px: instance.rect.x.saturating_add(instance.rect.width) as f32,
            bottom_px: instance.rect.y.saturating_add(instance.rect.height) as f32,
            uv_left_px: atlas_rect.x,
            uv_top_px: atlas_rect.y,
            uv_right_px: atlas_rect.x.saturating_add(atlas_rect.width),
            uv_bottom_px: atlas_rect.y.saturating_add(atlas_rect.height),
        }
    }
}

fn placeholder_glyph_texture_bytes(
    key: &EngineRenderGlyphAtlasKey,
    width_px: usize,
    height_px: usize,
) -> Vec<u8> {
    let width_px = width_px.max(1);
    let height_px = height_px.max(1);
    let mut bytes = Vec::with_capacity(width_px.saturating_mul(height_px).saturating_mul(4));
    let seed = glyph_key_seed(key);
    let vertical_stem = (seed as usize) % width_px;
    let horizontal_stem = ((seed >> 8) as usize) % height_px;

    for y in 0..height_px {
        for x in 0..width_px {
            let border = x == 0 || y == 0 || x + 1 == width_px || y + 1 == height_px;
            let diagonal = (x + y + seed as usize) % 7 == 0;
            let stem = x == vertical_stem || y == horizontal_stem;
            let mut alpha = if border || diagonal || stem {
                0xff
            } else {
                0x00
            };
            if key.faint {
                alpha /= 2;
            }
            bytes.extend_from_slice(&[0xff, 0xff, 0xff, alpha]);
        }
    }

    bytes
}

pub(crate) fn fit_glyph_rgba_to_atlas_region(
    source_rgba: &[u8],
    source_width_px: usize,
    source_height_px: usize,
    width_px: usize,
    height_px: usize,
    faint: bool,
) -> Vec<u8> {
    let width_px = width_px.max(1);
    let height_px = height_px.max(1);
    let mut bytes = vec![0; width_px.saturating_mul(height_px).saturating_mul(4)];
    if source_width_px == 0 || source_height_px == 0 {
        return bytes;
    }

    let source_stride = source_width_px.saturating_mul(4);
    let expected_source_len = source_stride.saturating_mul(source_height_px);
    if source_rgba.len() < expected_source_len {
        return bytes;
    }

    let copy_width = source_width_px.min(width_px);
    let copy_height = source_height_px.min(height_px);
    let copy_bytes = copy_width.saturating_mul(4);
    for y in 0..copy_height {
        let src_offset = y.saturating_mul(source_stride);
        let dst_offset = y.saturating_mul(width_px).saturating_mul(4);
        bytes[dst_offset..dst_offset + copy_bytes]
            .copy_from_slice(&source_rgba[src_offset..src_offset + copy_bytes]);
    }

    if faint {
        for pixel in bytes.chunks_exact_mut(4) {
            pixel[3] /= 2;
        }
    }

    bytes
}

fn glyph_key_seed(key: &EngineRenderGlyphAtlasKey) -> u32 {
    let mut seed = 0x811c9dc5u32;
    for byte in key.text.as_bytes() {
        seed ^= *byte as u32;
        seed = seed.wrapping_mul(0x01000193);
    }
    seed ^= key.cells as u32;
    if let Some(font_idx) = key.font_idx {
        seed ^= (font_idx as u32).wrapping_mul(0x45d9f3b);
    }
    if let Some(glyph_pos) = key.glyph_pos {
        seed ^= glyph_pos.wrapping_mul(0x119de1f3);
    }
    if key.bold {
        seed ^= 0x1000;
    }
    if key.faint {
        seed ^= 0x2000;
    }
    if key.italic {
        seed ^= 0x4000;
    }
    if key.vertical_align.is_some() {
        seed ^= 0x8000;
    }
    seed
}

fn push_unique_usize(values: &mut Vec<usize>, value: usize) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn ansi_palette_rgb(index: u8) -> [u8; 3] {
    const ANSI_16: [[u8; 3]; 16] = [
        [0x00, 0x00, 0x00],
        [0xcd, 0x31, 0x31],
        [0x0d, 0xa1, 0x0d],
        [0xe5, 0xe5, 0x10],
        [0x24, 0x71, 0xd1],
        [0xbc, 0x3f, 0xbc],
        [0x11, 0xa8, 0xcd],
        [0xe5, 0xe5, 0xe5],
        [0x66, 0x66, 0x66],
        [0xf1, 0x4c, 0x4c],
        [0x23, 0xd1, 0x8b],
        [0xf5, 0xf5, 0x43],
        [0x3b, 0x8e, 0xf3],
        [0xd6, 0x70, 0xd6],
        [0x29, 0xb8, 0xdb],
        [0xff, 0xff, 0xff],
    ];
    if index < 16 {
        ANSI_16[index as usize]
    } else if index < 232 {
        let cube = index - 16;
        let r = cube / 36;
        let g = (cube % 36) / 6;
        let b = cube % 6;
        [
            color_cube_channel(r),
            color_cube_channel(g),
            color_cube_channel(b),
        ]
    } else {
        let gray = 8 + (index - 232) * 10;
        [gray, gray, gray]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fullscreen_placement_maps_target_corners_to_clip_corners() {
        let placement = EngineRenderViewportPlacement::fullscreen(800.0, 600.0);

        assert_eq!(placement.to_clip(0.0, 0.0), [-1.0, 1.0]);
        assert_eq!(placement.to_clip(800.0, 600.0), [1.0, -1.0]);
        assert_eq!(placement.to_clip(400.0, 300.0), [0.0, 0.0]);
    }

    #[test]
    fn offset_placement_shifts_a_pane_into_its_own_corner() {
        // A right-hand split: the pane's own (0,0) is the middle of the
        // window, so pane-local pixels must land in the right half of clip
        // space rather than over the left pane.
        let placement = EngineRenderViewportPlacement::at(400.0, 0.0, 800.0, 600.0);

        assert_eq!(placement.to_clip(0.0, 0.0), [0.0, 1.0]);
        assert_eq!(placement.to_clip(400.0, 600.0), [1.0, -1.0]);

        // A bottom split shifts on y instead.
        let bottom = EngineRenderViewportPlacement::at(0.0, 300.0, 800.0, 600.0);
        assert_eq!(bottom.to_clip(0.0, 0.0), [-1.0, 0.0]);
        assert_eq!(bottom.to_clip(800.0, 300.0), [1.0, -1.0]);
    }

    #[test]
    fn placement_guards_a_zero_sized_target_without_moving_the_origin() {
        let placement = EngineRenderViewportPlacement::at(0.0, 0.0, 0.0, 0.0);

        assert_eq!(placement.target_width_px, 1.0);
        assert_eq!(placement.target_height_px, 1.0);
        assert_eq!(placement.origin_x_px, 0.0);
        assert_eq!(placement.origin_y_px, 0.0);
        assert!(placement.to_clip(0.0, 0.0).iter().all(|v| v.is_finite()));
    }

    #[test]
    fn buffer_plan_for_placement_offsets_every_vertex() {
        let plan = EngineRenderBufferPlan {
            pane_id: 3,
            submitted: true,
            revision: 7,
            requires_full_repaint: false,
            damage_rects: Vec::new(),
            text_runs: Vec::new(),
            vertices: vec![EngineRenderVertex {
                position: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
                layer: EngineRenderVertexLayer::Background,
                command_index: 0,
            }],
            indices: vec![0],
        };

        let fullscreen =
            EngineRenderGpuUploadPlan::from_buffer_plan_for_viewport(&plan, 800.0, 600.0);
        let offset = EngineRenderGpuUploadPlan::from_buffer_plan_for_placement(
            &plan,
            EngineRenderViewportPlacement::at(400.0, 300.0, 800.0, 600.0),
        );

        assert_eq!(fullscreen.vertices[0].position, [-1.0, 1.0]);
        assert_eq!(offset.vertices[0].position, [0.0, 0.0]);
        // Everything except geometry is carried through untouched.
        assert_eq!(offset.pane_id, plan.pane_id);
        assert_eq!(offset.revision, plan.revision);
        assert_eq!(offset.indices, plan.indices);
    }

    #[test]
    fn buffer_plan_for_viewport_still_means_a_fullscreen_placement() {
        let plan = EngineRenderBufferPlan {
            pane_id: 1,
            submitted: true,
            revision: 1,
            requires_full_repaint: true,
            damage_rects: Vec::new(),
            text_runs: Vec::new(),
            vertices: vec![EngineRenderVertex {
                position: [120.0, 45.0],
                color: [0.5, 0.5, 0.5, 1.0],
                layer: EngineRenderVertexLayer::Text,
                command_index: 2,
            }],
            indices: vec![0],
        };

        let viewport =
            EngineRenderGpuUploadPlan::from_buffer_plan_for_viewport(&plan, 640.0, 480.0);
        let placement = EngineRenderGpuUploadPlan::from_buffer_plan_for_placement(
            &plan,
            EngineRenderViewportPlacement::fullscreen(640.0, 480.0),
        );

        assert_eq!(
            viewport.vertices[0].position,
            placement.vertices[0].position
        );
    }

    #[test]
    fn buffer_plan_preserves_text_run_metadata() {
        let style = CellStyle {
            fg: Some(StyledColor::Rgb(0xaa, 0xbb, 0xcc)),
            bg: Some(StyledColor::Rgb(0x01, 0x02, 0x03)),
            ..Default::default()
        };
        let rect = RenderRect {
            x: 8,
            y: 16,
            width: 24,
            height: 16,
        };
        let frame = EngineRenderBackendFrame {
            pane_id: 7,
            submitted: true,
            revision: 42,
            requires_full_repaint: false,
            skipped_revisions: 0,
            commands: vec![EngineRenderBackendCommand::Text {
                row: 1,
                col: 2,
                cells: 3,
                rect,
                text: "abc".to_string(),
                style: style.clone(),
            }],
        };

        let plan = EngineRenderBufferPlan::from_frame(&frame);

        assert_eq!(plan.text_runs.len(), 1);
        assert_eq!(plan.text_runs[0].text, "abc");
        assert_eq!(plan.text_runs[0].row, 1);
        assert_eq!(plan.text_runs[0].col, 2);
        assert_eq!(plan.text_runs[0].cells, 3);
        assert_eq!(plan.text_runs[0].rect, rect);
        assert_eq!(plan.text_runs[0].style, style);
        // A text run contributes no solid geometry: its pixels come from the
        // textured glyph pass, masked by glyph coverage. Emitting a quad here
        // would paint an opaque foreground-coloured block over the glyphs.
        assert!(plan.vertices.is_empty());
        assert!(plan.indices.is_empty());

        let atlas = EngineWgpuRenderBackend::prepare_text_atlas(&plan);
        assert_eq!(atlas.pane_id, 7);
        assert_eq!(atlas.revision, 42);
        assert_eq!(atlas.runs.len(), 1);
        assert_eq!(atlas.runs[0].row, 1);
        assert_eq!(atlas.runs[0].col, 2);
        assert_eq!(atlas.runs[0].cells, 3);
        assert_eq!(atlas.runs[0].text, "abc");
        assert_eq!(atlas.runs[0].rect, rect);
        assert_eq!(
            atlas.runs[0].foreground,
            [
                0xaa as f32 / 255.0,
                0xbb as f32 / 255.0,
                0xcc as f32 / 255.0,
                1.0
            ]
        );
        assert_eq!(atlas.runs[0].style, style);

        let prepared = EngineWgpuRenderBackend::prepare_frame_for_viewport(&plan, 80.0, 40.0);
        assert_eq!(prepared.upload.pane_id, 7);
        assert_eq!(prepared.upload.revision, 42);
        // Same reason: no solid geometry for a text-only frame.
        assert!(prepared.upload.vertices.is_empty());
        assert_eq!(prepared.text_atlas, atlas);
        assert_eq!(prepared.glyph_atlas.pane_id, 7);
        assert_eq!(prepared.glyph_atlas.revision, 42);
        assert_eq!(prepared.glyph_atlas.keys.len(), 3);
        assert_eq!(prepared.glyph_atlas.instances.len(), 3);
        assert!(prepared.is_replace_ready());
        assert!(prepared.readiness_issues().is_empty());
        assert_eq!(
            prepared.diagnostics(),
            EngineWgpuPreparedFrameDiagnostics {
                pane_id: 7,
                submitted: true,
                revision: 42,
                // Text draws through the glyph pass, so a text-only frame
                // carries no solid geometry.
                solid_vertex_count: 0,
                solid_index_count: 0,
                text_run_count: 1,
                glyph_key_count: 3,
                glyph_instance_count: 3,
                replace_ready: true,
            }
        );
    }

    #[test]
    fn text_atlas_plan_skips_unsubmitted_buffer_plan() {
        let plan = EngineRenderBufferPlan {
            pane_id: 9,
            submitted: false,
            revision: 17,
            requires_full_repaint: false,
            damage_rects: Vec::new(),
            text_runs: vec![RenderTextRun {
                row: 0,
                col: 0,
                cells: 1,
                text: "x".to_string(),
                rect: RenderRect {
                    x: 0,
                    y: 0,
                    width: 8,
                    height: 16,
                },
                style: CellStyle::default(),
            }],
            vertices: Vec::new(),
            indices: Vec::new(),
        };

        let atlas = EngineRenderTextAtlasPlan::from_buffer_plan(&plan);

        assert_eq!(atlas.pane_id, 9);
        assert_eq!(atlas.revision, 17);
        assert!(!atlas.submitted);
        assert!(atlas.is_empty());
    }

    #[test]
    fn prepared_frame_diagnostics_report_replace_readiness_issues() {
        let buffer = EngineRenderBufferPlan {
            pane_id: 11,
            submitted: false,
            revision: 31,
            requires_full_repaint: false,
            damage_rects: Vec::new(),
            text_runs: vec![RenderTextRun {
                row: 0,
                col: 0,
                cells: 1,
                text: "x".to_string(),
                rect: RenderRect {
                    x: 0,
                    y: 0,
                    width: 8,
                    height: 16,
                },
                style: CellStyle::default(),
            }],
            vertices: Vec::new(),
            indices: Vec::new(),
        };
        let mut prepared = EngineWgpuRenderBackend::prepare_frame_for_viewport(&buffer, 80.0, 40.0);
        prepared.text_atlas.submitted = true;
        prepared.text_atlas.runs = vec![EngineRenderTextAtlasRun {
            row: 0,
            col: 0,
            cells: 1,
            text: "x".to_string(),
            rect: RenderRect {
                x: 0,
                y: 0,
                width: 8,
                height: 16,
            },
            foreground: [1.0, 1.0, 1.0, 1.0],
            style: CellStyle::default(),
        }];
        prepared.glyph_atlas.submitted = true;

        assert!(!prepared.is_replace_ready());
        assert_eq!(
            prepared.readiness_issues(),
            // No EmptySolidUpload: the frame has text runs, so an empty solid
            // buffer is expected. The precise complaint is that those runs
            // have no glyphs.
            vec![
                EngineWgpuPreparedFrameReadinessIssue::SolidNotSubmitted,
                EngineWgpuPreparedFrameReadinessIssue::TextAtlasMissingGlyphs,
            ]
        );
        assert_eq!(
            prepared.diagnostics(),
            EngineWgpuPreparedFrameDiagnostics {
                pane_id: 11,
                submitted: false,
                revision: 31,
                solid_vertex_count: 0,
                solid_index_count: 0,
                text_run_count: 1,
                glyph_key_count: 0,
                glyph_instance_count: 0,
                replace_ready: false,
            }
        );
    }

    #[test]
    fn glyph_atlas_plan_splits_ascii_runs_and_reuses_keys() {
        let style = CellStyle {
            fg: Some(StyledColor::Palette(2)),
            bold: true,
            ..Default::default()
        };
        let frame = EngineRenderBackendFrame {
            pane_id: 3,
            submitted: true,
            revision: 10,
            requires_full_repaint: true,
            skipped_revisions: 0,
            commands: vec![EngineRenderBackendCommand::Text {
                row: 4,
                col: 5,
                cells: 3,
                rect: RenderRect {
                    x: 50,
                    y: 80,
                    width: 31,
                    height: 16,
                },
                text: "aba".to_string(),
                style: style.clone(),
            }],
        };
        let buffer = EngineRenderBufferPlan::from_frame(&frame);

        let atlas = EngineWgpuRenderBackend::prepare_glyph_atlas(&buffer);

        assert_eq!(atlas.pane_id, 3);
        assert!(atlas.submitted);
        assert!(atlas.requires_full_repaint);
        assert_eq!(
            atlas.keys,
            vec![
                EngineRenderGlyphAtlasKey::from_text("a".to_string(), 1, &style),
                EngineRenderGlyphAtlasKey::from_text("b".to_string(), 1, &style),
            ]
        );
        assert_eq!(atlas.instances.len(), 3);
        assert_eq!(atlas.instances[0].key_index, 0);
        assert_eq!(atlas.instances[0].row, 4);
        assert_eq!(atlas.instances[0].col, 5);
        assert_eq!(
            atlas.instances[0].rect,
            RenderRect {
                x: 50,
                y: 80,
                width: 11,
                height: 16,
            }
        );
        assert_eq!(atlas.instances[1].key_index, 1);
        assert_eq!(atlas.instances[1].col, 6);
        assert_eq!(atlas.instances[1].rect.x, 61);
        assert_eq!(atlas.instances[1].rect.width, 10);
        assert_eq!(atlas.instances[2].key_index, 0);
        assert_eq!(atlas.instances[2].col, 7);
        assert_eq!(atlas.instances[2].rect.x, 71);
        assert_eq!(atlas.instances[2].rect.width, 10);
        assert_eq!(
            atlas.instances[0].foreground,
            [
                0x0d as f32 / 255.0,
                0xa1 as f32 / 255.0,
                0x0d as f32 / 255.0,
                1.0
            ]
        );
    }

    #[test]
    fn glyph_atlas_plan_keeps_non_cell_aligned_runs_intact() {
        let frame = EngineRenderBackendFrame {
            pane_id: 4,
            submitted: true,
            revision: 11,
            requires_full_repaint: false,
            skipped_revisions: 0,
            commands: vec![EngineRenderBackendCommand::Text {
                row: 0,
                col: 1,
                cells: 2,
                rect: RenderRect {
                    x: 8,
                    y: 0,
                    width: 16,
                    height: 16,
                },
                text: "你".to_string(),
                style: CellStyle::default(),
            }],
        };
        let buffer = EngineRenderBufferPlan::from_frame(&frame);

        let atlas = EngineWgpuRenderBackend::prepare_glyph_atlas(&buffer);

        assert_eq!(atlas.keys.len(), 1);
        assert_eq!(atlas.keys[0].text, "你");
        assert_eq!(atlas.keys[0].cells, 2);
        assert_eq!(atlas.instances.len(), 1);
        assert_eq!(atlas.instances[0].col, 1);
        assert_eq!(atlas.instances[0].cells, 2);
        assert_eq!(
            atlas.instances[0].rect,
            RenderRect {
                x: 8,
                y: 0,
                width: 16,
                height: 16,
            }
        );
    }

    #[test]
    fn shaped_glyph_plan_feeds_raster_identity_into_glyph_atlas() {
        let style = CellStyle {
            italic: true,
            ..Default::default()
        };
        let plan = EngineRenderShapedGlyphPlan {
            pane_id: 14,
            submitted: true,
            revision: 21,
            requires_full_repaint: true,
            glyphs: vec![
                shaped_glyph("a", 0, 0, 2, 42, style.clone()),
                shaped_glyph("a", 0, 1, 2, 42, style.clone()),
                shaped_glyph("a", 0, 2, 2, 43, style.clone()),
            ],
        };

        let atlas = EngineWgpuRenderBackend::prepare_glyph_atlas_from_shaped_glyphs(&plan);

        assert_eq!(atlas.pane_id, 14);
        assert!(atlas.submitted);
        assert!(atlas.requires_full_repaint);
        assert_eq!(atlas.keys.len(), 2);
        assert_eq!(atlas.keys[0].raster_identity(), Some((2, 42)));
        assert_eq!(atlas.keys[1].raster_identity(), Some((2, 43)));
        assert_eq!(atlas.instances.len(), 3);
        assert_eq!(atlas.instances[0].key_index, 0);
        assert_eq!(atlas.instances[1].key_index, 0);
        assert_eq!(atlas.instances[2].key_index, 1);
        assert_eq!(atlas.instances[2].col, 2);
    }

    #[test]
    fn glyph_infos_build_shaped_glyph_plan_for_atlas_input() {
        let style = CellStyle {
            fg: Some(StyledColor::Rgb(0x33, 0x44, 0x55)),
            bold: true,
            ..Default::default()
        };
        let text_atlas = EngineRenderTextAtlasPlan {
            pane_id: 15,
            submitted: true,
            revision: 22,
            requires_full_repaint: false,
            runs: vec![EngineRenderTextAtlasRun {
                row: 1,
                col: 2,
                cells: 2,
                text: "ab".to_string(),
                rect: RenderRect {
                    x: 20,
                    y: 40,
                    width: 18,
                    height: 16,
                },
                foreground: [0.2, 0.3, 0.4, 1.0],
                style,
            }],
        };
        let glyph_infos = vec![vec![
            glyph_info("a", 'a', 0, 101, 9.0, 0.0, 0.0, 1),
            glyph_info("b", 'b', 1, 202, 9.0, 1.0, 2.0, 1),
        ]];

        let shaped = EngineWgpuRenderBackend::prepare_shaped_glyph_plan(&text_atlas, &glyph_infos);
        let atlas = EngineWgpuRenderBackend::prepare_glyph_atlas_from_shaped_glyphs(&shaped);

        assert_eq!(shaped.pane_id, 15);
        assert_eq!(shaped.glyphs.len(), 2);
        assert_eq!(shaped.glyphs[0].rect.x, 20);
        assert_eq!(shaped.glyphs[0].rect.width, 9);
        assert_eq!(shaped.glyphs[1].rect.x, 30);
        assert_eq!(shaped.glyphs[1].rect.y, 38);
        assert_eq!(shaped.glyphs[1].x_advance_px, 9);
        assert_eq!(shaped.glyphs[1].x_offset_px, 1);
        assert_eq!(shaped.glyphs[1].y_offset_px, 2);
        assert_eq!(shaped.glyphs[1].col, 3);
        assert_eq!(shaped.glyphs[1].font_idx, 1);
        assert_eq!(shaped.glyphs[1].glyph_pos, 202);
        assert_eq!(atlas.keys[0].raster_identity(), Some((0, 101)));
        assert_eq!(atlas.keys[1].raster_identity(), Some((1, 202)));
        assert_eq!(atlas.instances[1].rect.y, 38);
        assert_eq!(atlas.instances[1].x_advance_px, 9);
        assert_eq!(atlas.instances[1].x_offset_px, 1);
        assert_eq!(atlas.instances[1].y_offset_px, 2);
    }

    #[test]
    fn glyph_atlas_key_ignores_foreground_color() {
        let mut red = CellStyle {
            fg: Some(StyledColor::Rgb(0xff, 0x00, 0x00)),
            italic: true,
            ..Default::default()
        };
        let mut blue = red.clone();
        blue.fg = Some(StyledColor::Rgb(0x00, 0x00, 0xff));
        red.hyperlink = Some("https://example.test/a".to_string());
        blue.hyperlink = Some("https://example.test/b".to_string());

        let frame = EngineRenderBackendFrame {
            pane_id: 5,
            submitted: true,
            revision: 12,
            requires_full_repaint: false,
            skipped_revisions: 0,
            commands: vec![
                EngineRenderBackendCommand::Text {
                    row: 0,
                    col: 0,
                    cells: 1,
                    rect: RenderRect {
                        x: 0,
                        y: 0,
                        width: 8,
                        height: 16,
                    },
                    text: "x".to_string(),
                    style: red,
                },
                EngineRenderBackendCommand::Text {
                    row: 0,
                    col: 1,
                    cells: 1,
                    rect: RenderRect {
                        x: 8,
                        y: 0,
                        width: 8,
                        height: 16,
                    },
                    text: "x".to_string(),
                    style: blue,
                },
            ],
        };
        let buffer = EngineRenderBufferPlan::from_frame(&frame);

        let atlas = EngineWgpuRenderBackend::prepare_glyph_atlas(&buffer);

        assert_eq!(atlas.keys.len(), 1);
        assert_eq!(atlas.keys[0].text, "x");
        assert!(atlas.keys[0].italic);
        assert_eq!(atlas.instances.len(), 2);
        assert_eq!(atlas.instances[0].key_index, 0);
        assert_eq!(atlas.instances[1].key_index, 0);
        assert_ne!(atlas.instances[0].foreground, atlas.instances[1].foreground);
    }

    #[test]
    fn textured_glyph_upload_maps_instances_to_clip_space_and_uvs() {
        let style = CellStyle {
            fg: Some(StyledColor::Rgb(0x80, 0x40, 0x20)),
            ..Default::default()
        };
        let frame = EngineRenderBackendFrame {
            pane_id: 6,
            submitted: true,
            revision: 13,
            requires_full_repaint: false,
            skipped_revisions: 0,
            commands: vec![EngineRenderBackendCommand::Text {
                row: 0,
                col: 0,
                cells: 1,
                rect: RenderRect {
                    x: 10,
                    y: 5,
                    width: 20,
                    height: 10,
                },
                text: "z".to_string(),
                style,
            }],
        };
        let buffer = EngineRenderBufferPlan::from_frame(&frame);
        let glyphs = EngineWgpuRenderBackend::prepare_glyph_atlas(&buffer);

        let upload = EngineWgpuRenderBackend::prepare_textured_glyph_upload_for_viewport(
            &glyphs,
            &[EngineRenderGlyphAtlasPlacement {
                key_index: 0,
                rect: RenderRect {
                    x: 16,
                    y: 8,
                    width: 8,
                    height: 16,
                },
                source_width_px: 8,
                source_height_px: 16,
                bearing_x_px: 0,
                bearing_y_px: 0,
                uses_raster_metrics: false,
            }],
            100.0,
            50.0,
            64.0,
            32.0,
        );

        assert_eq!(upload.pane_id, 6);
        assert!(upload.submitted);
        assert!(upload.missing_key_indices.is_empty());
        assert_eq!(upload.vertices.len(), 4);
        assert_eq!(upload.indices, vec![0, 1, 2, 1, 2, 3]);
        assert_f32_pair_close(upload.vertices[0].position, [-0.8, 0.8]);
        assert_f32_pair_close(upload.vertices[1].position, [-0.4, 0.8]);
        assert_f32_pair_close(upload.vertices[2].position, [-0.8, 0.4]);
        assert_f32_pair_close(upload.vertices[3].position, [-0.4, 0.4]);
        assert_eq!(upload.vertices[0].uv, [0.25, 0.25]);
        assert_eq!(upload.vertices[1].uv, [0.375, 0.25]);
        assert_eq!(upload.vertices[2].uv, [0.25, 0.75]);
        assert_eq!(upload.vertices[3].uv, [0.375, 0.75]);
        assert_eq!(upload.layout.entries.len(), 1);
        assert_eq!(upload.layout.entries[0].text, "z");
        assert_eq!(upload.layout.entries[0].quad.left_px, 10.0);
        assert_eq!(upload.layout.entries[0].quad.right_px, 30.0);
        assert_eq!(upload.layout.entries[0].quad.uv_right_px, 24);
        assert_eq!(
            upload.vertices[0].color,
            [
                0x80 as f32 / 255.0,
                0x40 as f32 / 255.0,
                0x20 as f32 / 255.0,
                1.0
            ]
        );
        assert_eq!(upload.vertices[0].key_index, 0);
    }

    #[test]
    fn textured_glyph_upload_uses_raster_metrics_for_bitmap_quad() {
        let frame = EngineRenderBackendFrame {
            pane_id: 32,
            submitted: true,
            revision: 33,
            requires_full_repaint: false,
            skipped_revisions: 0,
            commands: vec![EngineRenderBackendCommand::Text {
                row: 0,
                col: 0,
                cells: 1,
                rect: RenderRect {
                    x: 10,
                    y: 5,
                    width: 20,
                    height: 10,
                },
                text: "g".to_string(),
                style: CellStyle::default(),
            }],
        };
        let buffer = EngineRenderBufferPlan::from_frame(&frame);
        let glyphs = EngineWgpuRenderBackend::prepare_glyph_atlas(&buffer);

        let upload = EngineWgpuRenderBackend::prepare_textured_glyph_upload_for_viewport(
            &glyphs,
            &[EngineRenderGlyphAtlasPlacement {
                key_index: 0,
                rect: RenderRect {
                    x: 16,
                    y: 8,
                    width: 20,
                    height: 16,
                },
                source_width_px: 7,
                source_height_px: 9,
                bearing_x_px: -2,
                bearing_y_px: 8,
                uses_raster_metrics: true,
            }],
            100.0,
            50.0,
            64.0,
            32.0,
        );

        assert_eq!(upload.vertices.len(), 4);
        assert_f32_pair_close(upload.vertices[0].position, [-0.84, 0.72]);
        assert_f32_pair_close(upload.vertices[1].position, [-0.7, 0.72]);
        assert_f32_pair_close(upload.vertices[2].position, [-0.84, 0.36]);
        assert_f32_pair_close(upload.vertices[3].position, [-0.7, 0.36]);
        assert_eq!(upload.vertices[0].uv, [0.25, 0.25]);
        assert_eq!(upload.vertices[1].uv, [23.0 / 64.0, 0.25]);
        assert_eq!(upload.vertices[2].uv, [0.25, 17.0 / 32.0]);
        assert_eq!(upload.vertices[3].uv, [23.0 / 64.0, 17.0 / 32.0]);
    }

    #[test]
    fn textured_glyph_quad_pixels_matches_baseline_bitmap_formula() {
        let instance = EngineRenderGlyphAtlasInstance {
            key_index: 0,
            row: 0,
            col: 0,
            cells: 1,
            rect: RenderRect {
                x: 10,
                y: 3,
                width: 20,
                height: 12,
            },
            x_advance_px: 9,
            x_offset_px: 1,
            y_offset_px: 2,
            foreground: [1.0, 1.0, 1.0, 1.0],
        };
        let placement = EngineRenderGlyphAtlasPlacement {
            key_index: 0,
            rect: RenderRect {
                x: 32,
                y: 48,
                width: 20,
                height: 18,
            },
            source_width_px: 8,
            source_height_px: 10,
            bearing_x_px: -1,
            bearing_y_px: 7,
            uses_raster_metrics: true,
        };

        let quad = textured_glyph_quad_pixels(&instance, &placement);

        assert_eq!(quad.left_px, 9.0);
        assert_eq!(quad.top_px, 8.0);
        assert_eq!(quad.right_px, 17.0);
        assert_eq!(quad.bottom_px, 18.0);
        assert_eq!(quad.uv_left_px, 32);
        assert_eq!(quad.uv_top_px, 48);
        assert_eq!(quad.uv_right_px, 40);
        assert_eq!(quad.uv_bottom_px, 58);
    }

    #[test]
    fn textured_glyph_upload_reports_missing_atlas_placements() {
        let frame = EngineRenderBackendFrame {
            pane_id: 7,
            submitted: true,
            revision: 14,
            requires_full_repaint: false,
            skipped_revisions: 0,
            commands: vec![EngineRenderBackendCommand::Text {
                row: 0,
                col: 0,
                cells: 2,
                rect: RenderRect {
                    x: 0,
                    y: 0,
                    width: 16,
                    height: 16,
                },
                text: "aa".to_string(),
                style: CellStyle::default(),
            }],
        };
        let buffer = EngineRenderBufferPlan::from_frame(&frame);
        let glyphs = EngineWgpuRenderBackend::prepare_glyph_atlas(&buffer);

        let upload = EngineWgpuRenderBackend::prepare_textured_glyph_upload_for_viewport(
            &glyphs,
            &[],
            80.0,
            40.0,
            128.0,
            128.0,
        );

        assert!(upload.is_empty());
        assert_eq!(upload.missing_key_indices, vec![0]);
    }

    #[test]
    fn textured_glyph_layout_diff_is_clean_for_identical_reports() {
        let report = sample_textured_layout_report();

        let diff = report.diff_against(&report);

        assert!(diff.is_clean());
        assert_eq!(diff.expected_entry_count, 2);
        assert_eq!(diff.actual_entry_count, 2);
    }

    #[test]
    fn textured_glyph_layout_diff_reports_missing_unexpected_and_mismatch() {
        let expected = sample_textured_layout_report();
        let mut actual = expected.clone();
        actual.revision = 22;
        actual.entries[0].quad.left_px += 1.0;
        actual.entries[0].x_offset_px += 1;
        actual.entries.remove(1);
        actual
            .entries
            .push(sample_textured_layout_entry(3, 9, 1, "x"));
        actual.missing_key_indices = vec![7];

        let diff = expected.diff_against(&actual);

        assert!(!diff.is_clean());
        assert_eq!(diff.expected_pane_id, 40);
        assert_eq!(diff.actual_revision, 22);
        assert_eq!(
            diff.missing_entries,
            vec![EngineRenderTexturedGlyphLayoutIdentity {
                row: 2,
                col: 4,
                cells: 1,
                text: "b".to_string(),
            }]
        );
        assert_eq!(
            diff.unexpected_entries,
            vec![EngineRenderTexturedGlyphLayoutIdentity {
                row: 3,
                col: 9,
                cells: 1,
                text: "x".to_string(),
            }]
        );
        assert_eq!(diff.mismatches.len(), 1);
        assert_eq!(diff.mismatches[0].identity.text, "a");
        assert_eq!(diff.mismatches[0].expected_quad.left_px, 16.0);
        assert_eq!(diff.mismatches[0].actual_quad.left_px, 17.0);
        assert_eq!(diff.mismatches[0].expected_offsets_px, [8, 0, 0]);
        assert_eq!(diff.mismatches[0].actual_offsets_px, [8, 1, 0]);
        assert_eq!(diff.expected_missing_key_indices, Vec::<usize>::new());
        assert_eq!(diff.actual_missing_key_indices, vec![7]);
    }

    #[test]
    fn prepared_frame_plan_exposes_textured_glyph_layout_parity() {
        let frame = EngineRenderBackendFrame {
            pane_id: 41,
            submitted: true,
            revision: 23,
            requires_full_repaint: false,
            skipped_revisions: 0,
            commands: vec![EngineRenderBackendCommand::Text {
                row: 0,
                col: 0,
                cells: 2,
                rect: RenderRect {
                    x: 0,
                    y: 0,
                    width: 16,
                    height: 16,
                },
                text: "ab".to_string(),
                style: CellStyle::default(),
            }],
        };
        let buffer = EngineRenderBufferPlan::from_frame(&frame);
        let prepared = EngineWgpuRenderBackend::prepare_frame_for_viewport(&buffer, 80.0, 40.0);
        let placements = vec![
            EngineRenderGlyphAtlasPlacement {
                key_index: 0,
                rect: RenderRect {
                    x: 1,
                    y: 1,
                    width: 8,
                    height: 16,
                },
                source_width_px: 8,
                source_height_px: 16,
                bearing_x_px: 0,
                bearing_y_px: 0,
                uses_raster_metrics: false,
            },
            EngineRenderGlyphAtlasPlacement {
                key_index: 1,
                rect: RenderRect {
                    x: 11,
                    y: 1,
                    width: 8,
                    height: 16,
                },
                source_width_px: 8,
                source_height_px: 16,
                bearing_x_px: 0,
                bearing_y_px: 0,
                uses_raster_metrics: false,
            },
        ];

        let diff = prepared.diff_textured_glyph_layout_against(&prepared, &placements, &placements);

        assert!(diff.is_clean());
        assert_eq!(diff.expected_entry_count, 2);
        assert_eq!(diff.actual_entry_count, 2);
    }

    #[test]
    fn prepared_frame_plan_layout_parity_reports_frame_level_drift() {
        let expected_frame = EngineRenderBackendFrame {
            pane_id: 42,
            submitted: true,
            revision: 24,
            requires_full_repaint: false,
            skipped_revisions: 0,
            commands: vec![EngineRenderBackendCommand::Text {
                row: 0,
                col: 0,
                cells: 2,
                rect: RenderRect {
                    x: 0,
                    y: 0,
                    width: 16,
                    height: 16,
                },
                text: "ab".to_string(),
                style: CellStyle::default(),
            }],
        };
        let actual_frame = EngineRenderBackendFrame {
            revision: 25,
            commands: vec![EngineRenderBackendCommand::Text {
                row: 0,
                col: 1,
                cells: 2,
                rect: RenderRect {
                    x: 8,
                    y: 0,
                    width: 16,
                    height: 16,
                },
                text: "ab".to_string(),
                style: CellStyle::default(),
            }],
            ..expected_frame.clone()
        };
        let expected_buffer = EngineRenderBufferPlan::from_frame(&expected_frame);
        let actual_buffer = EngineRenderBufferPlan::from_frame(&actual_frame);
        let expected =
            EngineWgpuRenderBackend::prepare_frame_for_viewport(&expected_buffer, 80.0, 40.0);
        let actual =
            EngineWgpuRenderBackend::prepare_frame_for_viewport(&actual_buffer, 80.0, 40.0);
        let placements = vec![
            EngineRenderGlyphAtlasPlacement {
                key_index: 0,
                rect: RenderRect {
                    x: 1,
                    y: 1,
                    width: 8,
                    height: 16,
                },
                source_width_px: 8,
                source_height_px: 16,
                bearing_x_px: 0,
                bearing_y_px: 0,
                uses_raster_metrics: false,
            },
            EngineRenderGlyphAtlasPlacement {
                key_index: 1,
                rect: RenderRect {
                    x: 11,
                    y: 1,
                    width: 8,
                    height: 16,
                },
                source_width_px: 8,
                source_height_px: 16,
                bearing_x_px: 0,
                bearing_y_px: 0,
                uses_raster_metrics: false,
            },
        ];

        let diff = expected.diff_textured_glyph_layout_against(&actual, &placements, &placements);

        assert!(!diff.is_clean());
        assert_eq!(diff.expected_revision, 24);
        assert_eq!(diff.actual_revision, 25);
        assert_eq!(diff.missing_entries.len(), 2);
        assert_eq!(diff.unexpected_entries.len(), 2);
    }

    #[test]
    fn textured_glyph_pass_skips_missing_key_uploads() {
        let upload = EngineRenderTexturedGlyphUploadPlan {
            pane_id: 12,
            submitted: true,
            revision: 19,
            requires_full_repaint: false,
            layout: empty_textured_layout_report(12, 19, vec![0]),
            vertices: vec![EngineRenderTexturedGlyphVertex::default(); 4],
            indices: vec![0, 1, 2, 1, 2, 3],
            missing_key_indices: vec![0],
        };

        let pass = EngineWgpuTexturedGlyphPassPlan::from_upload_plan(&upload);

        assert_eq!(pass.pane_id, 12);
        assert_eq!(pass.revision, 19);
        assert!(!pass.draw);
        assert_eq!(pass.vertex_count, 4);
        assert_eq!(pass.index_count, 6);
    }

    #[test]
    fn textured_glyph_pass_draws_complete_uploads() {
        let upload = EngineRenderTexturedGlyphUploadPlan {
            pane_id: 13,
            submitted: true,
            revision: 20,
            requires_full_repaint: false,
            layout: empty_textured_layout_report(13, 20, Vec::new()),
            vertices: vec![EngineRenderTexturedGlyphVertex::default(); 4],
            indices: vec![0, 1, 2, 1, 2, 3],
            missing_key_indices: Vec::new(),
        };

        let pass = EngineWgpuTexturedGlyphPassPlan::from_upload_plan(&upload);

        assert!(pass.draw);
        assert_eq!(pass.vertex_count, 4);
        assert_eq!(pass.index_count, 6);
    }

    #[test]
    fn glyph_atlas_cache_allocates_and_reuses_placements() {
        let frame = EngineRenderBackendFrame {
            pane_id: 8,
            submitted: true,
            revision: 15,
            requires_full_repaint: false,
            skipped_revisions: 0,
            commands: vec![EngineRenderBackendCommand::Text {
                row: 0,
                col: 0,
                cells: 3,
                rect: RenderRect {
                    x: 0,
                    y: 0,
                    width: 24,
                    height: 16,
                },
                text: "aba".to_string(),
                style: CellStyle::default(),
            }],
        };
        let buffer = EngineRenderBufferPlan::from_frame(&frame);
        let glyphs = EngineWgpuRenderBackend::prepare_glyph_atlas(&buffer);
        let mut cache = EngineRenderGlyphAtlasCache::new(64, 32);

        let update = cache.ensure_glyphs(&glyphs, 8, 16);

        assert_eq!(update.inserted_key_indices, vec![0, 1]);
        assert!(update.overflow_key_indices.is_empty());
        assert_eq!(update.placements.len(), 2);
        assert_eq!(
            update.placements[0],
            EngineRenderGlyphAtlasPlacement {
                key_index: 0,
                rect: RenderRect {
                    x: 1,
                    y: 1,
                    width: 8,
                    height: 16,
                },
                source_width_px: 8,
                source_height_px: 16,
                bearing_x_px: 0,
                bearing_y_px: 0,
                uses_raster_metrics: false,
            }
        );
        assert_eq!(
            update.placements[1],
            EngineRenderGlyphAtlasPlacement {
                key_index: 1,
                rect: RenderRect {
                    x: 11,
                    y: 1,
                    width: 8,
                    height: 16,
                },
                source_width_px: 8,
                source_height_px: 16,
                bearing_x_px: 0,
                bearing_y_px: 0,
                uses_raster_metrics: false,
            }
        );

        let repeat = cache.ensure_glyphs(&glyphs, 8, 16);
        assert!(repeat.inserted_key_indices.is_empty());
        assert!(repeat.overflow_key_indices.is_empty());
        assert_eq!(repeat.placements, update.placements);
    }

    #[test]
    fn glyph_atlas_cache_wraps_rows_and_reports_overflow() {
        let mut cache = EngineRenderGlyphAtlasCache::new(20, 36);
        let plan = EngineRenderGlyphAtlasPlan {
            pane_id: 9,
            submitted: true,
            revision: 16,
            requires_full_repaint: false,
            keys: vec![
                glyph_key("a", 1),
                glyph_key("b", 1),
                glyph_key("c", 1),
                glyph_key("wide", 3),
            ],
            instances: Vec::new(),
        };

        let update = cache.ensure_glyphs(&plan, 8, 16);

        assert_eq!(update.inserted_key_indices, vec![0, 1, 2]);
        assert_eq!(update.overflow_key_indices, vec![3]);
        assert_eq!(update.placements[0].rect.x, 1);
        assert_eq!(update.placements[0].rect.y, 1);
        assert_eq!(update.placements[1].rect.x, 11);
        assert_eq!(update.placements[1].rect.y, 1);
        assert_eq!(update.placements[2].rect.x, 1);
        assert_eq!(update.placements[2].rect.y, 19);
    }

    #[test]
    fn cached_textured_glyph_upload_uses_cache_placements() {
        let style = CellStyle {
            fg: Some(StyledColor::Rgb(0x10, 0x20, 0x30)),
            ..Default::default()
        };
        let frame = EngineRenderBackendFrame {
            pane_id: 10,
            submitted: true,
            revision: 17,
            requires_full_repaint: true,
            skipped_revisions: 0,
            commands: vec![EngineRenderBackendCommand::Text {
                row: 0,
                col: 0,
                cells: 1,
                rect: RenderRect {
                    x: 0,
                    y: 0,
                    width: 8,
                    height: 16,
                },
                text: "q".to_string(),
                style,
            }],
        };
        let buffer = EngineRenderBufferPlan::from_frame(&frame);
        let glyphs = EngineWgpuRenderBackend::prepare_glyph_atlas(&buffer);
        let mut cache = EngineRenderGlyphAtlasCache::new(32, 32);

        let (update, upload) =
            EngineWgpuRenderBackend::prepare_cached_textured_glyph_upload_for_viewport(
                &glyphs, &mut cache, 8, 16, 80.0, 40.0,
            );

        assert_eq!(update.inserted_key_indices, vec![0]);
        assert!(update.overflow_key_indices.is_empty());
        assert_eq!(upload.vertices.len(), 4);
        assert_eq!(upload.indices.len(), 6);
        assert!(upload.missing_key_indices.is_empty());
        assert_eq!(upload.vertices[0].uv, [1.0 / 32.0, 1.0 / 32.0]);
        assert_eq!(upload.vertices[3].uv, [9.0 / 32.0, 17.0 / 32.0]);
        assert!(upload.requires_full_repaint);
    }

    #[test]
    fn glyph_atlas_texture_update_prepares_inserted_regions() {
        let frame = EngineRenderBackendFrame {
            pane_id: 11,
            submitted: true,
            revision: 18,
            requires_full_repaint: true,
            skipped_revisions: 0,
            commands: vec![EngineRenderBackendCommand::Text {
                row: 0,
                col: 0,
                cells: 2,
                rect: RenderRect {
                    x: 0,
                    y: 0,
                    width: 16,
                    height: 16,
                },
                text: "az".to_string(),
                style: CellStyle::default(),
            }],
        };
        let buffer = EngineRenderBufferPlan::from_frame(&frame);
        let glyphs = EngineWgpuRenderBackend::prepare_glyph_atlas(&buffer);
        let mut cache = EngineRenderGlyphAtlasCache::new(64, 32);
        let update = cache.ensure_glyphs(&glyphs, 8, 16);

        let texture_update =
            EngineWgpuRenderBackend::prepare_glyph_atlas_texture_update(&glyphs, &update, 64, 32);

        assert_eq!(texture_update.pane_id, 11);
        assert_eq!(texture_update.revision, 18);
        assert_eq!(texture_update.atlas_width_px, 64);
        assert_eq!(texture_update.atlas_height_px, 32);
        assert_eq!(texture_update.regions.len(), 2);
        assert!(texture_update.overflow_key_indices.is_empty());
        assert!(texture_update.missing_key_indices.is_empty());
        assert_eq!(texture_update.regions[0].key_index, 0);
        assert_eq!(texture_update.regions[0].rect, update.placements[0].rect);
        assert_eq!(texture_update.regions[0].width_px, 8);
        assert_eq!(texture_update.regions[0].height_px, 16);
        assert_eq!(texture_update.regions[0].bytes_rgba.len(), 8 * 16 * 4);
        assert_eq!(
            &texture_update.regions[0].bytes_rgba[0..3],
            &[0xff, 0xff, 0xff]
        );
        assert!(texture_update.regions[0]
            .bytes_rgba
            .chunks_exact(4)
            .any(|pixel| pixel[3] != 0));

        let repeat = cache.ensure_glyphs(&glyphs, 8, 16);
        let repeat_texture =
            EngineWgpuRenderBackend::prepare_glyph_atlas_texture_update(&glyphs, &repeat, 64, 32);
        assert!(repeat_texture.is_empty());
    }

    #[test]
    fn glyph_atlas_texture_update_uses_raster_source_bytes() {
        struct SolidRasterSource;

        impl EngineRenderGlyphRasterSource for SolidRasterSource {
            fn rasterize_glyph_rgba(
                &self,
                _key: &EngineRenderGlyphAtlasKey,
                width_px: usize,
                height_px: usize,
            ) -> Option<Vec<u8>> {
                let mut bytes = Vec::with_capacity(width_px * height_px * 4);
                for _ in 0..width_px * height_px {
                    bytes.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
                }
                Some(bytes)
            }
        }

        let glyphs = EngineRenderGlyphAtlasPlan {
            pane_id: 12,
            submitted: true,
            revision: 19,
            requires_full_repaint: false,
            keys: vec![glyph_key("r", 1)],
            instances: Vec::new(),
        };
        let mut cache = EngineRenderGlyphAtlasCache::new(32, 32);
        let update = cache.ensure_glyphs(&glyphs, 8, 16);

        let texture_update =
            EngineWgpuRenderBackend::prepare_glyph_atlas_texture_update_with_raster_source(
                &glyphs,
                &update,
                32,
                32,
                &SolidRasterSource,
            );

        assert_eq!(texture_update.regions.len(), 1);
        assert!(texture_update.missing_key_indices.is_empty());
        assert_eq!(texture_update.regions[0].source_width_px, 8);
        assert_eq!(texture_update.regions[0].source_height_px, 16);
        assert_eq!(texture_update.regions[0].bearing_x_px, 0);
        assert_eq!(texture_update.regions[0].bearing_y_px, 0);
        assert_eq!(
            &texture_update.regions[0].bytes_rgba[0..4],
            &[0x11, 0x22, 0x33, 0x44]
        );
    }

    #[test]
    fn glyph_atlas_texture_update_preserves_raster_source_metrics() {
        struct MetricsRasterSource;

        impl EngineRenderGlyphRasterSource for MetricsRasterSource {
            fn rasterize_glyph_rgba(
                &self,
                key: &EngineRenderGlyphAtlasKey,
                width_px: usize,
                height_px: usize,
            ) -> Option<Vec<u8>> {
                self.rasterize_glyph_texture(key, width_px, height_px)
                    .map(|raster| raster.bytes_rgba)
            }

            fn rasterize_glyph_texture(
                &self,
                _key: &EngineRenderGlyphAtlasKey,
                width_px: usize,
                height_px: usize,
            ) -> Option<EngineRenderGlyphRaster> {
                Some(EngineRenderGlyphRaster {
                    bytes_rgba: vec![0xaa; width_px * height_px * 4],
                    source_width_px: 11,
                    source_height_px: 13,
                    bearing_x_px: -2,
                    bearing_y_px: 9,
                    uses_raster_metrics: true,
                })
            }
        }

        let glyphs = EngineRenderGlyphAtlasPlan {
            pane_id: 31,
            submitted: true,
            revision: 32,
            requires_full_repaint: false,
            keys: vec![glyph_key("m", 1)],
            instances: Vec::new(),
        };
        let mut cache = EngineRenderGlyphAtlasCache::new(32, 32);
        let update = cache.ensure_glyphs(&glyphs, 8, 16);

        let texture_update =
            EngineWgpuRenderBackend::prepare_glyph_atlas_texture_update_with_raster_source(
                &glyphs,
                &update,
                32,
                32,
                &MetricsRasterSource,
            );

        assert_eq!(texture_update.regions.len(), 1);
        assert_eq!(texture_update.regions[0].source_width_px, 11);
        assert_eq!(texture_update.regions[0].source_height_px, 13);
        assert_eq!(texture_update.regions[0].bearing_x_px, -2);
        assert_eq!(texture_update.regions[0].bearing_y_px, 9);
    }

    #[test]
    fn glyph_atlas_texture_update_rejects_bad_raster_source_bytes() {
        struct BadRasterSource;

        impl EngineRenderGlyphRasterSource for BadRasterSource {
            fn rasterize_glyph_rgba(
                &self,
                _key: &EngineRenderGlyphAtlasKey,
                _width_px: usize,
                _height_px: usize,
            ) -> Option<Vec<u8>> {
                Some(vec![0xff, 0xff, 0xff])
            }
        }

        let glyphs = EngineRenderGlyphAtlasPlan {
            pane_id: 13,
            submitted: true,
            revision: 20,
            requires_full_repaint: false,
            keys: vec![glyph_key("bad", 1)],
            instances: Vec::new(),
        };
        let mut cache = EngineRenderGlyphAtlasCache::new(32, 32);
        let update = cache.ensure_glyphs(&glyphs, 8, 16);

        let texture_update =
            EngineWgpuRenderBackend::prepare_glyph_atlas_texture_update_with_raster_source(
                &glyphs,
                &update,
                32,
                32,
                &BadRasterSource,
            );

        assert!(texture_update.regions.is_empty());
        assert_eq!(texture_update.missing_key_indices, vec![0]);
    }

    #[test]
    fn font_glyph_raster_region_fit_crops_pads_and_applies_faint_alpha() {
        let source = vec![
            0x10, 0x11, 0x12, 0x80, 0x20, 0x21, 0x22, 0xff, 0x30, 0x31, 0x32, 0x40, 0x40, 0x41,
            0x42, 0x20,
        ];

        let bytes = fit_glyph_rgba_to_atlas_region(&source, 2, 2, 3, 1, true);

        assert_eq!(bytes.len(), 3 * 4);
        assert_eq!(&bytes[0..4], &[0x10, 0x11, 0x12, 0x40]);
        assert_eq!(&bytes[4..8], &[0x20, 0x21, 0x22, 0x7f]);
        assert_eq!(&bytes[8..12], &[0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn font_glyph_raster_region_fit_returns_transparent_for_bad_source() {
        let bytes = fit_glyph_rgba_to_atlas_region(&[0xff, 0xff], 2, 2, 2, 2, false);

        assert_eq!(bytes.len(), 2 * 2 * 4);
        assert!(bytes.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn glyph_atlas_key_tracks_optional_shaped_raster_identity() {
        let style = CellStyle {
            bold: true,
            ..Default::default()
        };

        let text_key = EngineRenderGlyphAtlasKey::from_text("a".to_string(), 1, &style);
        let shaped_key =
            EngineRenderGlyphAtlasKey::from_shaped_glyph("a".to_string(), 1, &style, 2, 42);
        let other_glyph =
            EngineRenderGlyphAtlasKey::from_shaped_glyph("a".to_string(), 1, &style, 2, 43);

        assert_eq!(text_key.raster_identity(), None);
        assert_eq!(shaped_key.raster_identity(), Some((2, 42)));
        assert_ne!(text_key, shaped_key);
        assert_ne!(shaped_key, other_glyph);
    }

    fn assert_f32_pair_close(actual: [f32; 2], expected: [f32; 2]) {
        assert!(
            (actual[0] - expected[0]).abs() < 0.0001,
            "x mismatch: actual={:?} expected={:?}",
            actual,
            expected
        );
        assert!(
            (actual[1] - expected[1]).abs() < 0.0001,
            "y mismatch: actual={:?} expected={:?}",
            actual,
            expected
        );
    }

    fn empty_textured_layout_report(
        pane_id: usize,
        revision: u64,
        missing_key_indices: Vec<usize>,
    ) -> EngineRenderTexturedGlyphLayoutReport {
        EngineRenderTexturedGlyphLayoutReport {
            pane_id,
            submitted: true,
            revision,
            requires_full_repaint: false,
            entries: Vec::new(),
            missing_key_indices,
        }
    }

    fn sample_textured_layout_report() -> EngineRenderTexturedGlyphLayoutReport {
        EngineRenderTexturedGlyphLayoutReport {
            pane_id: 40,
            submitted: true,
            revision: 21,
            requires_full_repaint: false,
            entries: vec![
                sample_textured_layout_entry(1, 2, 1, "a"),
                sample_textured_layout_entry(2, 4, 1, "b"),
            ],
            missing_key_indices: Vec::new(),
        }
    }

    fn sample_textured_layout_entry(
        row: usize,
        col: usize,
        cells: usize,
        text: &str,
    ) -> EngineRenderTexturedGlyphLayoutEntry {
        let key_index = col;
        EngineRenderTexturedGlyphLayoutEntry {
            key_index,
            row,
            col,
            cells,
            text: text.to_string(),
            source_rect: RenderRect {
                x: col * 8,
                y: row * 16,
                width: cells * 8,
                height: 16,
            },
            atlas_rect: RenderRect {
                x: key_index * 10,
                y: 4,
                width: 8,
                height: 16,
            },
            quad: EngineRenderTexturedGlyphQuad {
                left_px: (col * 8) as f32,
                top_px: (row * 16) as f32,
                right_px: (col * 8 + cells * 8) as f32,
                bottom_px: (row * 16 + 16) as f32,
                uv_left_px: key_index * 10,
                uv_top_px: 4,
                uv_right_px: key_index * 10 + 8,
                uv_bottom_px: 20,
            },
            x_advance_px: (cells * 8) as i32,
            x_offset_px: 0,
            y_offset_px: 0,
            bearing_x_px: 0,
            bearing_y_px: 0,
            foreground: [1.0, 1.0, 1.0, 1.0],
            uses_raster_metrics: true,
        }
    }

    fn glyph_key(text: &str, cells: usize) -> EngineRenderGlyphAtlasKey {
        EngineRenderGlyphAtlasKey::from_text(text.to_string(), cells, &CellStyle::default())
    }

    fn shaped_glyph(
        text: &str,
        row: usize,
        col: usize,
        font_idx: usize,
        glyph_pos: u32,
        style: CellStyle,
    ) -> EngineRenderShapedGlyph {
        EngineRenderShapedGlyph {
            row,
            col,
            cells: 1,
            text: text.to_string(),
            rect: RenderRect {
                x: col * 8,
                y: row * 16,
                width: 8,
                height: 16,
            },
            x_advance_px: 8,
            x_offset_px: 0,
            y_offset_px: 0,
            foreground: [1.0, 1.0, 1.0, 1.0],
            style,
            font_idx,
            glyph_pos,
        }
    }

    fn glyph_info(
        text: &str,
        only_char: char,
        font_idx: usize,
        glyph_pos: u32,
        x_advance: f64,
        x_offset: f64,
        y_offset: f64,
        num_cells: u8,
    ) -> EngineRenderShaperGlyph {
        EngineRenderShaperGlyph {
            text: text.to_string(),
            only_char: Some(only_char),
            num_cells,
            font_idx,
            glyph_pos,
            x_advance_px: x_advance,
            x_offset_px: x_offset,
            y_offset_px: y_offset,
        }
    }
}

fn color_cube_channel(value: u8) -> u8 {
    if value == 0 {
        0
    } else {
        55 + value * 40
    }
}

#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct CommandListRenderBackend;

impl EngineRenderBackend for CommandListRenderBackend {
    fn submit(
        &mut self,
        batch: &EngineRenderCommitBatch,
    ) -> anyhow::Result<EngineRenderBackendFrame> {
        let mut commands = Vec::new();
        if let Some(submission) = batch.commit.submission.as_ref() {
            commands.extend(
                submission
                    .damage_rects
                    .iter()
                    .copied()
                    .map(EngineRenderBackendCommand::Damage),
            );
            commands.extend(submission.background_quads.iter().map(|quad| {
                EngineRenderBackendCommand::Background {
                    rect: quad.rect,
                    style: quad.style.clone(),
                }
            }));
            commands.extend(submission.text_runs.iter().map(|run| {
                EngineRenderBackendCommand::Text {
                    row: run.row,
                    col: run.col,
                    cells: run.cells,
                    rect: run.rect,
                    text: run.text.clone(),
                    style: run.style.clone(),
                }
            }));
            if let Some(cursor) = submission.cursor.as_ref() {
                commands.push(EngineRenderBackendCommand::Cursor {
                    rect: cursor.rect,
                    visible: cursor.visible,
                    shape: cursor.shape.clone(),
                });
            }
        }

        Ok(EngineRenderBackendFrame {
            pane_id: batch.pane_id,
            submitted: batch.stats.submit && !commands.is_empty(),
            revision: batch.stats.revision,
            requires_full_repaint: batch.stats.requires_full_repaint,
            skipped_revisions: batch.stats.skipped_revisions,
            commands,
        })
    }
}
