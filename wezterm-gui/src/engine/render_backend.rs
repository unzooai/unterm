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

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct EngineRenderGlyphAtlasKey {
    pub text: String,
    pub cells: usize,
    pub bold: bool,
    pub faint: bool,
    pub italic: bool,
    pub vertical_align: Option<StyledVerticalAlign>,
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

    pub fn prepare_pass(
        &self,
        plan: &EngineRenderGpuUploadPlan,
        clear_color: Option<[f64; 4]>,
    ) -> EngineWgpuRenderPassPlan {
        EngineWgpuRenderPassPlan::from_upload_plan(plan, clear_color)
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

    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
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
                EngineRenderGlyphAtlasKey {
                    text: ch.to_string(),
                    cells: 1,
                    bold: run.style.bold,
                    faint: run.style.faint,
                    italic: run.style.italic,
                    vertical_align: run.style.vertical_align,
                },
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
            EngineRenderGlyphAtlasKey {
                text: run.text.clone(),
                cells: run.cells,
                bold: run.style.bold,
                faint: run.style.faint,
                italic: run.style.italic,
                vertical_align: run.style.vertical_align,
            },
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
                EngineRenderGlyphAtlasKey {
                    text: "a".to_string(),
                    cells: 1,
                    bold: true,
                    faint: false,
                    italic: false,
                    vertical_align: None,
                },
                EngineRenderGlyphAtlasKey {
                    text: "b".to_string(),
                    cells: 1,
                    bold: true,
                    faint: false,
                    italic: false,
                    vertical_align: None,
                },
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
