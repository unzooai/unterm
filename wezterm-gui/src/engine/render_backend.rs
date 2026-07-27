//! Backend-facing render command preparation for engine render commits.
//!
//! This is deliberately GPU-free: it fixes the command contract that a future
//! wgpu backend will execute, without making tests depend on a window, adapter,
//! font atlas, or swapchain.

use super::{
    CellStyle, EngineRenderCommitBatch, RenderRect, RenderTextRun, StyledColor, StyledVerticalAlign,
};
use wgpu::util::DeviceExt;

const NEXT_CORE_RENDER_SHADER: &str = r#"
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

const NEXT_CORE_TEXTURED_GLYPH_SHADER: &str = r#"
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@group(0) @binding(0) var glyph_atlas_tex: texture_2d<f32>;
@group(0) @binding(1) var glyph_atlas_sampler: sampler;

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) _key_index: u32,
) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.uv = uv;
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let glyph = textureSample(glyph_atlas_tex, glyph_atlas_sampler, in.uv);
    return vec4<f32>(in.color.rgb, in.color.a * glyph.a);
}
"#;

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
    pub foreground: [f32; 4],
    pub style: CellStyle,
    pub font_idx: usize,
    pub glyph_pos: u32,
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

#[allow(dead_code)]
impl EngineRenderGpuVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x4,
        2 => Uint32,
        3 => Uint32,
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
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

#[allow(dead_code)]
impl EngineRenderTexturedGlyphVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x4,
        3 => Uint32,
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct EngineRenderTexturedGlyphUploadPlan {
    pub pane_id: usize,
    pub submitted: bool,
    pub revision: u64,
    pub requires_full_repaint: bool,
    pub vertices: Vec<EngineRenderTexturedGlyphVertex>,
    pub indices: Vec<u32>,
    pub missing_key_indices: Vec<usize>,
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
    pub bytes_rgba: Vec<u8>,
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
        let viewport_width_px = viewport_width_px.max(1.0);
        let viewport_height_px = viewport_height_px.max(1.0);
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
                    position: [
                        (vertex.position[0] / viewport_width_px) * 2.0 - 1.0,
                        1.0 - (vertex.position[1] / viewport_height_px) * 2.0,
                    ],
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct EngineWgpuPipelineConfig {
    pub target_format: wgpu::TextureFormat,
}

#[allow(dead_code)]
pub struct EngineWgpuRenderBuffers {
    pub pane_id: usize,
    pub revision: u64,
    pub vertex_count: usize,
    pub index_count: usize,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
}

#[allow(dead_code)]
pub struct EngineWgpuTexturedGlyphBuffers {
    pub pane_id: usize,
    pub revision: u64,
    pub vertex_count: usize,
    pub index_count: usize,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
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

    fn load_op(&self) -> wgpu::LoadOp<wgpu::Color> {
        match self.clear_color {
            Some([r, g, b, a]) => wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
            None => wgpu::LoadOp::Load,
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
        EngineRenderTexturedGlyphUploadPlan::from_glyph_atlas_plan_for_viewport(
            glyphs,
            placements,
            viewport_width_px,
            viewport_height_px,
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
        let text_atlas = Self::prepare_text_atlas(plan);
        let glyph_atlas = EngineRenderGlyphAtlasPlan::from_text_atlas_plan(&text_atlas);
        EngineWgpuPreparedFramePlan {
            upload: Self::prepare_upload_for_viewport(plan, viewport_width_px, viewport_height_px),
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

    pub fn create_pipeline(
        &self,
        device: &wgpu::Device,
        config: EngineWgpuPipelineConfig,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("next-core render shader"),
            source: wgpu::ShaderSource::Wgsl(NEXT_CORE_RENDER_SHADER.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("next-core render pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("next-core render pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[EngineRenderGpuVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        })
    }

    pub fn create_textured_glyph_pipeline(
        &self,
        device: &wgpu::Device,
        config: EngineWgpuPipelineConfig,
        glyph_texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("next-core textured glyph shader"),
            source: wgpu::ShaderSource::Wgsl(NEXT_CORE_TEXTURED_GLYPH_SHADER.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("next-core textured glyph pipeline layout"),
            bind_group_layouts: &[glyph_texture_bind_group_layout],
            push_constant_ranges: &[],
        });
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("next-core textured glyph pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[EngineRenderTexturedGlyphVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        })
    }

    pub fn upload(
        &self,
        device: &wgpu::Device,
        plan: &EngineRenderGpuUploadPlan,
    ) -> Option<EngineWgpuRenderBuffers> {
        if !plan.submitted || plan.is_empty() {
            return None;
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("next-core render vertex buffer"),
            contents: bytemuck::cast_slice(&plan.vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("next-core render index buffer"),
            contents: bytemuck::cast_slice(&plan.indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        Some(EngineWgpuRenderBuffers {
            pane_id: plan.pane_id,
            revision: plan.revision,
            vertex_count: plan.vertices.len(),
            index_count: plan.indices.len(),
            vertex_buffer,
            index_buffer,
        })
    }

    pub fn upload_textured_glyphs(
        &self,
        device: &wgpu::Device,
        plan: &EngineRenderTexturedGlyphUploadPlan,
    ) -> Option<EngineWgpuTexturedGlyphBuffers> {
        if !plan.submitted || plan.is_empty() || !plan.missing_key_indices.is_empty() {
            return None;
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("next-core textured glyph vertex buffer"),
            contents: bytemuck::cast_slice(&plan.vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("next-core textured glyph index buffer"),
            contents: bytemuck::cast_slice(&plan.indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        Some(EngineWgpuTexturedGlyphBuffers {
            pane_id: plan.pane_id,
            revision: plan.revision,
            vertex_count: plan.vertices.len(),
            index_count: plan.indices.len(),
            vertex_buffer,
            index_buffer,
        })
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

    pub fn encode_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        pipeline: &wgpu::RenderPipeline,
        buffers: &EngineWgpuRenderBuffers,
        plan: &EngineWgpuRenderPassPlan,
    ) -> bool {
        if !plan.draw || plan.index_count == 0 || plan.vertex_count == 0 {
            return false;
        }

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("next-core render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: plan.load_op(),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        render_pass.set_pipeline(pipeline);
        render_pass.set_vertex_buffer(0, buffers.vertex_buffer.slice(..));
        render_pass.set_index_buffer(buffers.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..plan.index_count as u32, 0, 0..1);
        true
    }

    pub fn encode_textured_glyph_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        pipeline: &wgpu::RenderPipeline,
        glyph_texture_bind_group: &wgpu::BindGroup,
        buffers: &EngineWgpuTexturedGlyphBuffers,
        plan: &EngineWgpuTexturedGlyphPassPlan,
    ) -> bool {
        if !plan.draw || plan.index_count == 0 || plan.vertex_count == 0 {
            return false;
        }

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("next-core textured glyph render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, glyph_texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, buffers.vertex_buffer.slice(..));
        render_pass.set_index_buffer(buffers.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..plan.index_count as u32, 0, 0..1);
        true
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
                    let color = foreground_color_for_command(command);
                    push_quad_vertices(
                        &mut vertices,
                        &mut indices,
                        *rect,
                        color,
                        EngineRenderVertexLayer::Text,
                        command_index,
                    );
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
        };
        self.next_x_px = self.next_x_px.saturating_add(width_px);
        self.row_height_px = self.row_height_px.max(height_px);
        Some(placement)
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
                let Some(bytes_rgba) = raster_source.rasterize_glyph_rgba(key, width_px, height_px)
                else {
                    push_unique_usize(&mut missing_key_indices, *key_index);
                    continue;
                };
                if bytes_rgba.len() != expected_bytes {
                    push_unique_usize(&mut missing_key_indices, *key_index);
                    continue;
                }
                regions.push(EngineRenderGlyphAtlasTextureRegion {
                    key_index: *key_index,
                    rect: placement.rect,
                    width_px,
                    height_px,
                    bytes_rgba,
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
    pub fn from_glyph_atlas_plan_for_viewport(
        plan: &EngineRenderGlyphAtlasPlan,
        placements: &[EngineRenderGlyphAtlasPlacement],
        viewport_width_px: f32,
        viewport_height_px: f32,
        atlas_width_px: f32,
        atlas_height_px: f32,
    ) -> Self {
        let viewport_width_px = viewport_width_px.max(1.0);
        let viewport_height_px = viewport_height_px.max(1.0);
        let atlas_width_px = atlas_width_px.max(1.0);
        let atlas_height_px = atlas_height_px.max(1.0);
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
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
                push_textured_glyph_quad(
                    &mut vertices,
                    &mut indices,
                    instance,
                    placement.rect,
                    viewport_width_px,
                    viewport_height_px,
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
            vertices,
            indices,
            missing_key_indices,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() || self.indices.is_empty()
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
                foreground: run.foreground,
            },
        );
    }
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

fn push_textured_glyph_quad(
    vertices: &mut Vec<EngineRenderTexturedGlyphVertex>,
    indices: &mut Vec<u32>,
    instance: &EngineRenderGlyphAtlasInstance,
    atlas_rect: RenderRect,
    viewport_width_px: f32,
    viewport_height_px: f32,
    atlas_width_px: f32,
    atlas_height_px: f32,
) {
    let base = vertices.len() as u32;
    let left = instance.rect.x as f32;
    let top = instance.rect.y as f32;
    let right = instance.rect.x.saturating_add(instance.rect.width) as f32;
    let bottom = instance.rect.y.saturating_add(instance.rect.height) as f32;
    let uv_left = atlas_rect.x as f32 / atlas_width_px;
    let uv_top = atlas_rect.y as f32 / atlas_height_px;
    let uv_right = atlas_rect.x.saturating_add(atlas_rect.width) as f32 / atlas_width_px;
    let uv_bottom = atlas_rect.y.saturating_add(atlas_rect.height) as f32 / atlas_height_px;
    let key_index = instance.key_index as u32;

    vertices.extend([
        EngineRenderTexturedGlyphVertex {
            position: viewport_to_clip(left, top, viewport_width_px, viewport_height_px),
            uv: [uv_left, uv_top],
            color: instance.foreground,
            key_index,
        },
        EngineRenderTexturedGlyphVertex {
            position: viewport_to_clip(right, top, viewport_width_px, viewport_height_px),
            uv: [uv_right, uv_top],
            color: instance.foreground,
            key_index,
        },
        EngineRenderTexturedGlyphVertex {
            position: viewport_to_clip(left, bottom, viewport_width_px, viewport_height_px),
            uv: [uv_left, uv_bottom],
            color: instance.foreground,
            key_index,
        },
        EngineRenderTexturedGlyphVertex {
            position: viewport_to_clip(right, bottom, viewport_width_px, viewport_height_px),
            uv: [uv_right, uv_bottom],
            color: instance.foreground,
            key_index,
        },
    ]);
    indices.extend([base, base + 1, base + 2, base + 1, base + 2, base + 3]);
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

fn viewport_to_clip(x: f32, y: f32, viewport_width_px: f32, viewport_height_px: f32) -> [f32; 2] {
    [
        (x / viewport_width_px) * 2.0 - 1.0,
        1.0 - (y / viewport_height_px) * 2.0,
    ]
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
        assert_eq!(plan.vertices.len(), 4);
        assert_eq!(plan.indices, vec![0, 1, 2, 1, 2, 3]);

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
        assert_eq!(prepared.upload.vertices.len(), 4);
        assert_eq!(prepared.text_atlas, atlas);
        assert_eq!(prepared.glyph_atlas.pane_id, 7);
        assert_eq!(prepared.glyph_atlas.revision, 42);
        assert_eq!(prepared.glyph_atlas.keys.len(), 3);
        assert_eq!(prepared.glyph_atlas.instances.len(), 3);
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
    fn textured_glyph_pass_skips_missing_key_uploads() {
        let upload = EngineRenderTexturedGlyphUploadPlan {
            pane_id: 12,
            submitted: true,
            revision: 19,
            requires_full_repaint: false,
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
        assert_eq!(
            &texture_update.regions[0].bytes_rgba[0..4],
            &[0x11, 0x22, 0x33, 0x44]
        );
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
            foreground: [1.0, 1.0, 1.0, 1.0],
            style,
            font_idx,
            glyph_pos,
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
