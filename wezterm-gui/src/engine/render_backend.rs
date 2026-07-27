//! Backend-facing render command preparation for engine render commits.
//!
//! This is deliberately GPU-free: it fixes the command contract that a future
//! wgpu backend will execute, without making tests depend on a window, adapter,
//! font atlas, or swapchain.

use super::{CellStyle, EngineRenderCommitBatch, RenderRect};
use wgpu::util::DeviceExt;

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum EngineRenderBackendCommand {
    Damage(RenderRect),
    Background {
        rect: RenderRect,
        style: CellStyle,
    },
    Text {
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
    pub vertices: Vec<EngineRenderVertex>,
    pub indices: Vec<u32>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[allow(dead_code)]
pub struct EngineRenderGpuVertex {
    pub position: [f32; 2],
    pub layer: u32,
    pub command_index: u32,
}

impl From<EngineRenderVertex> for EngineRenderGpuVertex {
    fn from(vertex: EngineRenderVertex) -> Self {
        Self {
            position: vertex.position,
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

#[allow(dead_code)]
pub struct EngineWgpuRenderBuffers {
    pub pane_id: usize,
    pub revision: u64,
    pub vertex_count: usize,
    pub index_count: usize,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
}

#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct EngineWgpuRenderBackend;

#[allow(dead_code)]
impl EngineWgpuRenderBackend {
    pub fn prepare_upload(plan: &EngineRenderBufferPlan) -> EngineRenderGpuUploadPlan {
        EngineRenderGpuUploadPlan::from_buffer_plan(plan)
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
}

#[allow(dead_code)]
impl EngineRenderBufferPlan {
    pub fn from_frame(frame: &EngineRenderBackendFrame) -> Self {
        let mut damage_rects = Vec::new();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for (command_index, command) in frame.commands.iter().enumerate() {
            match command {
                EngineRenderBackendCommand::Damage(rect) => damage_rects.push(*rect),
                EngineRenderBackendCommand::Background { rect, .. } => {
                    push_quad_vertices(
                        &mut vertices,
                        &mut indices,
                        *rect,
                        EngineRenderVertexLayer::Background,
                        command_index,
                    );
                }
                EngineRenderBackendCommand::Text { rect, .. } => {
                    push_quad_vertices(
                        &mut vertices,
                        &mut indices,
                        *rect,
                        EngineRenderVertexLayer::Text,
                        command_index,
                    );
                }
                EngineRenderBackendCommand::Cursor { rect, .. } => {
                    push_quad_vertices(
                        &mut vertices,
                        &mut indices,
                        *rect,
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
            vertices,
            indices,
        }
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
            layer,
            command_index,
        },
        EngineRenderVertex {
            position: [right, top],
            layer,
            command_index,
        },
        EngineRenderVertex {
            position: [left, bottom],
            layer,
            command_index,
        },
        EngineRenderVertex {
            position: [right, bottom],
            layer,
            command_index,
        },
    ]);
    indices.extend([base, base + 1, base + 2, base + 1, base + 2, base + 3]);
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
