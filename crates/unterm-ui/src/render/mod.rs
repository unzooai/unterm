//! wgpu + fontdue glyph atlas 终端渲染模块
//!
//! 每个字符预光栅化到纹理图集，以 GPU 四边形精确渲染到像素网格。
//! 对标 Alacritty/WezTerm 的渲染质量。

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use fontdue::Font;
use std::collections::HashMap;
use std::sync::Arc;
use wgpu;
use wgpu::util::DeviceExt;

// ─── 配色 (Campbell — Windows Terminal 默认) ──────────────────────────────────

/// 终端背景 #0C0C0C (Campbell Black)
const TERM_BG: (u8, u8, u8) = (12, 12, 12);
/// 默认前景 #CCCCCC (Campbell White)
const TEXT_COLOR: (u8, u8, u8) = (204, 204, 204);
/// Tab 栏背景 — Windows Terminal 标题栏深色
const TAB_BG: (u8, u8, u8) = (32, 32, 32);
/// 激活 Tab 背景 — 与终端背景一致
const ACTIVE_TAB_BG: (u8, u8, u8) = (12, 12, 12);
/// Tab 文本（激活）
const TAB_TEXT: (u8, u8, u8) = (255, 255, 255);
/// Tab 文本（非激活）
const INACTIVE_TEXT: (u8, u8, u8) = (150, 150, 150);
/// 状态栏背景
const STATUS_BG: (u8, u8, u8) = (30, 30, 30);
/// 状态栏文本
const STATUS_TEXT: (u8, u8, u8) = (118, 118, 118);

// ─── 字体 ─────────────────────────────────────────────────────────────────────

/// 逻辑字体大小（px）— Windows Terminal 默认 12pt = 16px
const FONT_SIZE: f32 = 16.0;
/// 逻辑行高（px）
const LINE_HEIGHT: f32 = 20.0;

// ─── 数据结构 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct PaneRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct PaneContent {
    pub pane_id: u64,
    pub rect: PaneRect,
    pub text: String,
    pub is_active: bool,
    pub title: Option<String>,
    pub structured: Option<StructuredContent>,
    /// 滚动偏移（0 = 底部/实时，正数 = 向上滚了多少行）
    pub viewport_offset: usize,
}

#[derive(Debug, Clone)]
pub struct TabInfo {
    pub id: u64,
    pub title: String,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct StructuredContent {
    pub cells: Vec<Vec<RenderCell>>,
    pub cursor: RenderCursor,
    /// scrollback 缓冲（最近 500 行，index 0 = 最早）
    pub scrollback: Vec<Vec<RenderCell>>,
    /// core 端总 scrollback 行数
    pub scrollback_total: usize,
}

#[derive(Debug, Clone)]
pub struct RenderCell {
    pub ch: char,
    pub fg: (u8, u8, u8),
    pub bg: Option<(u8, u8, u8)>,
    pub bold: bool,
    pub dim: bool,
    pub is_wide_continuation: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RenderCursor {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
}

#[derive(Debug, Clone)]
pub struct MenuRenderData {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub items: Vec<MenuRenderItem>,
    pub hovered: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct MenuRenderItem {
    pub label: String,
    pub shortcut: Option<String>,
    pub is_separator: bool,
}

// ─── 256 色表 (Campbell) ──────────────────────────────────────────────────────

pub fn indexed_color_to_rgb(index: u8) -> (u8, u8, u8) {
    match index {
        0 => (12, 12, 12),
        1 => (197, 15, 31),
        2 => (19, 161, 14),
        3 => (193, 156, 0),
        4 => (0, 55, 218),
        5 => (136, 23, 152),
        6 => (58, 150, 221),
        7 => (204, 204, 204),
        8 => (118, 118, 118),
        9 => (231, 72, 86),
        10 => (22, 198, 12),
        11 => (249, 241, 165),
        12 => (59, 120, 255),
        13 => (180, 0, 158),
        14 => (97, 214, 214),
        15 => (242, 242, 242),
        16..=231 => {
            let n = index - 16;
            let r = (n / 36) * 51;
            let g = ((n % 36) / 6) * 51;
            let b = (n % 6) * 51;
            (r, g, b)
        }
        232..=255 => {
            let v = 8 + (index - 232) * 10;
            (v, v, v)
        }
    }
}

// ─── GPU 顶点 ─────────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: 8,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: 16,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x4,
            },
        ],
    };
}

// ─── Glyph Atlas ──────────────────────────────────────────────────────────────

const ATLAS_SIZE: u32 = 1024;

struct GlyphEntry {
    /// UV 坐标 (归一化 0..1)
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
    /// 字形度量
    width: f32,
    height: f32,
    offset_x: f32,
    offset_y: f32,
}

struct GlyphAtlas {
    font: Font,
    /// CJK 回退字体（微软雅黑等）
    fallback_font: Option<Font>,
    font_size: f32,
    cache: HashMap<char, GlyphEntry>,
    /// CPU 侧图集数据 (R8)
    pixels: Vec<u8>,
    /// 打包游标
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    /// 字符 cell 宽高
    pub cell_width: f32,
    pub cell_height: f32,
    dirty: bool,
}

impl GlyphAtlas {
    fn new(font_size: f32) -> Self {
        // 加载 Cascadia Mono > Consolas > 内置回退
        let font_data = Self::load_system_font();
        let font = Font::from_bytes(
            font_data.as_slice(),
            fontdue::FontSettings {
                scale: font_size,
                ..Default::default()
            },
        )
        .expect("无法解析字体");

        // 加载 CJK 回退字体
        let fallback_font = Self::load_cjk_fallback_font(font_size);

        // 用 'M' 计算 cell 尺寸
        let metrics = font.metrics('M', font_size);
        let cell_width = metrics.advance_width.ceil();
        let cell_height = font_size.ceil();

        Self {
            font,
            fallback_font,
            font_size,
            cache: HashMap::new(),
            pixels: vec![0u8; (ATLAS_SIZE * ATLAS_SIZE) as usize],
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
            cell_width,
            cell_height,
            dirty: false,
        }
    }

    fn load_system_font() -> Vec<u8> {
        let candidates = [
            "C:\\Windows\\Fonts\\CascadiaMono.ttf",
            "C:\\Windows\\Fonts\\CascadiaCode.ttf",
            "C:\\Windows\\Fonts\\consola.ttf",
            "C:\\Windows\\Fonts\\cour.ttf",
        ];
        for path in &candidates {
            if let Ok(data) = std::fs::read(path) {
                tracing::info!("加载字体: {}", path);
                return data;
            }
        }
        tracing::warn!("未找到系统等宽字体，使用回退");
        include_bytes!("C:\\Windows\\Fonts\\consola.ttf").to_vec()
    }

    fn load_cjk_fallback_font(font_size: f32) -> Option<Font> {
        let candidates = [
            "C:\\Windows\\Fonts\\msyh.ttc",    // 微软雅黑
            "C:\\Windows\\Fonts\\msyhbd.ttc",   // 微软雅黑粗体
            "C:\\Windows\\Fonts\\simsun.ttc",   // 宋体
            "C:\\Windows\\Fonts\\simhei.ttf",   // 黑体
        ];
        for path in &candidates {
            if let Ok(data) = std::fs::read(path) {
                match Font::from_bytes(
                    data.as_slice(),
                    fontdue::FontSettings {
                        scale: font_size,
                        collection_index: 0,
                        ..Default::default()
                    },
                ) {
                    Ok(f) => {
                        tracing::info!("加载 CJK 回退字体: {}", path);
                        return Some(f);
                    }
                    Err(e) => {
                        tracing::warn!("无法解析 CJK 字体 {}: {}", path, e);
                    }
                }
            }
        }
        tracing::warn!("未找到 CJK 回退字体");
        None
    }

    /// 获取或光栅化字形
    fn get_or_rasterize(&mut self, ch: char) -> Option<&GlyphEntry> {
        if !self.cache.contains_key(&ch) {
            self.rasterize(ch);
        }
        self.cache.get(&ch)
    }

    fn rasterize(&mut self, ch: char) {
        // 检测主字体是否包含该字形（glyph index 0 = .notdef = 缺失）
        let has_glyph = self.font.lookup_glyph_index(ch) != 0;

        let (metrics, bitmap) = if !has_glyph {
            // 主字体没有该字形，尝试回退字体
            if let Some(ref fallback) = self.fallback_font {
                fallback.rasterize(ch, self.font_size)
            } else {
                self.font.rasterize(ch, self.font_size)
            }
        } else {
            self.font.rasterize(ch, self.font_size)
        };

        if metrics.width == 0 || metrics.height == 0 {
            self.cache.insert(ch, GlyphEntry {
                u0: 0.0, v0: 0.0, u1: 0.0, v1: 0.0,
                width: 0.0, height: 0.0,
                offset_x: metrics.xmin as f32,
                offset_y: metrics.ymin as f32,
            });
            return;
        }

        let w = metrics.width as u32;
        let h = metrics.height as u32;

        // 换行检测
        if self.cursor_x + w + 1 > ATLAS_SIZE {
            self.cursor_x = 0;
            self.cursor_y += self.row_height + 1;
            self.row_height = 0;
        }
        if self.cursor_y + h > ATLAS_SIZE {
            tracing::warn!("Glyph atlas 已满，无法添加 '{}'", ch);
            return;
        }

        // 复制像素到图集
        for row in 0..h {
            for col in 0..w {
                let src = (row * w + col) as usize;
                let dst = ((self.cursor_y + row) * ATLAS_SIZE + self.cursor_x + col) as usize;
                self.pixels[dst] = bitmap[src];
            }
        }

        let entry = GlyphEntry {
            u0: self.cursor_x as f32 / ATLAS_SIZE as f32,
            v0: self.cursor_y as f32 / ATLAS_SIZE as f32,
            u1: (self.cursor_x + w) as f32 / ATLAS_SIZE as f32,
            v1: (self.cursor_y + h) as f32 / ATLAS_SIZE as f32,
            width: w as f32,
            height: h as f32,
            offset_x: metrics.xmin as f32,
            offset_y: metrics.ymin as f32,
        };

        self.cursor_x += w + 1;
        self.row_height = self.row_height.max(h);
        self.cache.insert(ch, entry);
        self.dirty = true;
    }
}

// ─── WGSL Shader ──────────────────────────────────────────────────────────────

const SHADER_SRC: &str = r#"
struct Uniforms {
    screen_size: vec2<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var atlas_tex: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    let ndc = vec2<f32>(
        in.pos.x / uniforms.screen_size.x * 2.0 - 1.0,
        1.0 - in.pos.y / uniforms.screen_size.y * 2.0,
    );
    out.pos = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment fn fs_bg(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}

@fragment fn fs_text(in: VsOut) -> @location(0) vec4<f32> {
    let alpha = textureSample(atlas_tex, atlas_sampler, in.uv).r;
    if (alpha < 0.01) { discard; }
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
"#;

// ─── Renderer ─────────────────────────────────────────────────────────────────

pub struct Renderer {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,

    bg_pipeline: wgpu::RenderPipeline,
    text_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,
    atlas_texture: wgpu::Texture,

    atlas: GlyphAtlas,

    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
    clear_color: wgpu::Color,
}

impl Renderer {
    pub fn new(window: Arc<winit::window::Window>) -> Result<Self> {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        let scale_factor = window.scale_factor() as f32;

        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window)?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .ok_or_else(|| anyhow::anyhow!("无法找到合适的 GPU 适配器"))?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("unterm"),
                ..Default::default()
            },
            None,
        ))?;
        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // Surface format
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().find(|f| f.is_srgb()).copied().unwrap_or(caps.formats[0]);

        let clear_color = if format.is_srgb() {
            fn to_lin(v: f64) -> f64 {
                let s = v / 255.0;
                if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
            }
            wgpu::Color { r: to_lin(TERM_BG.0 as f64), g: to_lin(TERM_BG.1 as f64), b: to_lin(TERM_BG.2 as f64), a: 1.0 }
        } else {
            wgpu::Color { r: TERM_BG.0 as f64 / 255.0, g: TERM_BG.1 as f64 / 255.0, b: TERM_BG.2 as f64 / 255.0, a: 1.0 }
        };

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // Glyph atlas
        let phys_font_size = (FONT_SIZE * scale_factor).round();
        let atlas = GlyphAtlas::new(phys_font_size);

        // Atlas GPU texture
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph-atlas"),
            size: wgpu::Extent3d { width: ATLAS_SIZE, height: ATLAS_SIZE, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Uniform buffer
        let uniform_data = [width as f32, height as f32];
        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniforms"),
            contents: bytemuck::cast_slice(&uniform_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let atlas_view = atlas_texture.create_view(&Default::default());

        // Bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bind-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind-group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });

        // Shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let mk_pipeline = |frag: &str, blend: Option<wgpu::BlendState>| -> wgpu::RenderPipeline {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(frag),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::LAYOUT],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(frag),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };

        let bg_pipeline = mk_pipeline("fs_bg", None);
        let text_pipeline = mk_pipeline("fs_text", Some(wgpu::BlendState::ALPHA_BLENDING));

        Ok(Self {
            device, queue, surface, surface_config,
            bg_pipeline, text_pipeline, bind_group_layout, bind_group,
            uniform_buf, atlas_texture,
            atlas, width, height, scale_factor, clear_color,
        })
    }

    pub fn cell_width(&self) -> f32 {
        self.atlas.cell_width
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 { return; }
        self.width = width;
        self.height = height;
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
        self.queue.write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&[width as f32, height as f32]));
    }

    /// 渲染一帧
    pub fn draw_frame(
        &mut self,
        tabs: &[TabInfo],
        panes: &[PaneContent],
        status_text: &str,
        status_rect: PaneRect,
        tab_bar_rect: PaneRect,
        menu: Option<&MenuRenderData>,
    ) -> Result<()> {
        let sf = self.scale_factor;
        let cw = self.atlas.cell_width;
        let ch = (LINE_HEIGHT * sf).round();

        let mut bg_verts: Vec<Vertex> = Vec::new();
        let mut fg_verts: Vec<Vertex> = Vec::new();
        // 菜单 overlay 独立顶点缓冲（在常规内容之上绘制，避免终端文字穿透菜单背景）
        let mut menu_bg_verts: Vec<Vertex> = Vec::new();
        let mut menu_fg_verts: Vec<Vertex> = Vec::new();

        // ── Tab 栏背景 ──
        push_rect(&mut bg_verts, tab_bar_rect.x * sf, tab_bar_rect.y * sf,
            tab_bar_rect.width * sf, tab_bar_rect.height * sf, TAB_BG);

        // ── 各 Tab ──
        let tab_h = tab_bar_rect.height * sf;
        let tab_text_y = (tab_bar_rect.y * sf + (tab_h - ch) / 2.0).round();
        let mut tab_x = (tab_bar_rect.x + 8.0) * sf;
        for tab in tabs {
            let label = format!(" {}  \u{00D7} ", tab.title); // × 关闭按钮
            let tab_w = display_width(&label) as f32 * cw + 8.0 * sf;
            if tab.is_active {
                push_rect(&mut bg_verts, tab_x, tab_bar_rect.y * sf, tab_w, tab_h, ACTIVE_TAB_BG);
                self.draw_text_line(&mut fg_verts, &label, tab_x + 4.0 * sf, tab_text_y, TAB_TEXT);
            } else {
                self.draw_text_line(&mut fg_verts, &label, tab_x + 4.0 * sf, tab_text_y, INACTIVE_TEXT);
            }
            tab_x += tab_w + 2.0 * sf;
        }
        // + 按钮
        self.draw_text_line(&mut fg_verts, " + ", tab_x + 4.0 * sf, tab_text_y, INACTIVE_TEXT);

        // ˅ 下拉菜单按钮 (在 Tab 栏最右侧)
        let dropdown_x = (tab_bar_rect.x + tab_bar_rect.width - 36.0) * sf;
        self.draw_text_line(&mut fg_verts, " \u{2228} ", dropdown_x, tab_text_y, INACTIVE_TEXT);

        // ── 状态栏背景 ──
        push_rect(&mut bg_verts, status_rect.x * sf, status_rect.y * sf,
            status_rect.width * sf, status_rect.height * sf, STATUS_BG);

        // ── 状态栏文本 ──
        let sx = (status_rect.x + 8.0) * sf;
        let sy = (status_rect.y + 2.0) * sf;
        self.draw_text_line(&mut fg_verts, status_text, sx, sy, STATUS_TEXT);

        // ── 各 Pane ──
        let multi_pane = panes.len() > 1;
        for pane in panes {
            let px = pane.rect.x * sf;
            let py = pane.rect.y * sf;
            let pw = pane.rect.width * sf;
            let ph = pane.rect.height * sf;

            // 多窗格时，为焦点窗格绘制高亮边框
            if multi_pane && pane.is_active {
                let border = (1.0 * sf).max(1.0);
                let accent: (u8, u8, u8) = (0, 120, 215); // Windows Terminal 蓝色
                push_rect(&mut bg_verts, px, py, pw, border, accent);           // top
                push_rect(&mut bg_verts, px, py + ph - border, pw, border, accent); // bottom
                push_rect(&mut bg_verts, px, py, border, ph, accent);           // left
                push_rect(&mut bg_verts, px + pw - border, py, border, ph, accent); // right
            } else if multi_pane {
                // 非焦点窗格绘制暗灰色边框
                let border = (1.0 * sf).max(1.0);
                let dim: (u8, u8, u8) = (50, 50, 50);
                push_rect(&mut bg_verts, px, py, pw, border, dim);
                push_rect(&mut bg_verts, px, py + ph - border, pw, border, dim);
                push_rect(&mut bg_verts, px, py, border, ph, dim);
                push_rect(&mut bg_verts, px + pw - border, py, border, ph, dim);
            }

            // 多窗格时绘制 × 关闭按钮（右上角）
            if multi_pane {
                let btn_size = 20.0 * sf;
                let btn_margin = 4.0 * sf;
                let btn_x = px + pw - btn_size - btn_margin;
                let btn_y = py + btn_margin;
                // 按钮背景
                let btn_bg = if pane.is_active { (60, 60, 60) } else { (45, 45, 45) };
                push_rect(&mut bg_verts, btn_x, btn_y, btn_size, btn_size, btn_bg);
                // × 符号
                let x_color = (180, 180, 180);
                let cx = btn_x + btn_size * 0.5;
                let cy = btn_y + btn_size * 0.5;
                let arm = btn_size * 0.28;
                let thick = (1.5 * sf).max(1.0);
                // 左上→右下对角线（用细矩形近似）
                for i in 0..((arm * 2.0) as i32) {
                    let t = i as f32 - arm;
                    push_rect(&mut fg_verts, cx + t - thick * 0.5, cy + t - thick * 0.5, thick, thick, x_color);
                }
                // 右上→左下对角线
                for i in 0..((arm * 2.0) as i32) {
                    let t = i as f32 - arm;
                    push_rect(&mut fg_verts, cx - t - thick * 0.5, cy + t - thick * 0.5, thick, thick, x_color);
                }
            }

            if let Some(ref s) = pane.structured {
                let pad_x = (8.0 * sf).round();
                let pad_y = (4.0 * sf).round();
                let offset = pane.viewport_offset;
                let sb = &s.scrollback;
                let sb_len = sb.len();
                let screen_rows = s.cells.len();

                if offset == 0 {
                    // ── 实时视图：直接渲染 s.cells（原始逻辑）──
                    for (row_idx, row) in s.cells.iter().enumerate() {
                        let y = py + pad_y + row_idx as f32 * ch;
                        if y + ch > py + ph { break; }

                        for (col_idx, cell) in row.iter().enumerate() {
                            if cell.is_wide_continuation { continue; }
                            let x = px + pad_x + col_idx as f32 * cw;
                            if x + cw > px + pw { break; }

                            if let Some(bg) = cell.bg {
                                push_rect(&mut bg_verts, x, y, cw, ch, bg);
                            }

                            let is_cursor = s.cursor.visible
                                && row_idx == s.cursor.row as usize
                                && col_idx == s.cursor.col as usize;
                            if is_cursor {
                                push_rect(&mut bg_verts, x, y, cw, ch, TEXT_COLOR);
                            }

                            let fg = if is_cursor { TERM_BG } else { cell.fg };
                            if cell.ch != ' ' && cell.ch != '\0' {
                                self.draw_glyph(&mut fg_verts, cell.ch, x, y, ch, fg);
                            }
                        }
                    }
                } else {
                    // ── 回看视图：从 scrollback + cells 合成 ──
                    let total_lines = sb_len + screen_rows;
                    let view_end = total_lines.saturating_sub(offset);
                    let view_start = view_end.saturating_sub(screen_rows);

                    for vis_row in 0..screen_rows {
                        let abs_row = view_start + vis_row;
                        if abs_row >= total_lines { break; }
                        let y = py + pad_y + vis_row as f32 * ch;
                        if y + ch > py + ph { break; }

                        let row = if abs_row < sb_len {
                            &sb[abs_row]
                        } else if abs_row - sb_len < s.cells.len() {
                            &s.cells[abs_row - sb_len]
                        } else {
                            continue;
                        };

                        for (col_idx, cell) in row.iter().enumerate() {
                            if cell.is_wide_continuation { continue; }
                            let x = px + pad_x + col_idx as f32 * cw;
                            if x + cw > px + pw { break; }

                            if let Some(bg) = cell.bg {
                                push_rect(&mut bg_verts, x, y, cw, ch, bg);
                            }

                            let fg = cell.fg;
                            if cell.ch != ' ' && cell.ch != '\0' {
                                self.draw_glyph(&mut fg_verts, cell.ch, x, y, ch, fg);
                            }
                        }
                    }
                }

                // 滚动条（有 scrollback 时显示）
                if sb_len > 0 {
                    let total_lines = sb_len + screen_rows;
                    let scrollbar_w = 6.0 * sf;
                    let scrollbar_x = px + pw - scrollbar_w;
                    push_rect(&mut bg_verts, scrollbar_x, py, scrollbar_w, ph, (30, 30, 30));
                    let thumb_h = (screen_rows as f32 / total_lines.max(1) as f32 * ph).max(20.0 * sf);
                    let scroll_ratio = 1.0 - (offset as f32 / sb_len.max(1) as f32);
                    let thumb_y = py + (ph - thumb_h) * scroll_ratio;
                    push_rect(&mut bg_verts, scrollbar_x, thumb_y, scrollbar_w, thumb_h, (80, 80, 80));
                }
            } else {
                // 纯文本回退
                let pad_x = (8.0 * sf).round();
                let pad_y = (2.0 * sf).round();
                let color = if pane.is_active { TEXT_COLOR } else { INACTIVE_TEXT };
                let mut x = px + pad_x;
                let mut y = py + pad_y;
                for line in pane.text.lines() {
                    self.draw_text_line(&mut fg_verts, line, x, y, color);
                    y += ch;
                    x = px + pad_x;
                }
            }
        }

        // ── 菜单覆盖层（独立缓冲，在常规内容之上绘制）──
        if let Some(menu) = menu {
            let mx = menu.x * sf;
            let my = menu.y * sf;
            let mw = menu.width * sf;
            let item_h = 28.0 * sf;
            let sep_h = 9.0 * sf;
            let pad = 8.0 * sf;

            // Calculate total menu height
            let total_h: f32 = menu.items.iter().map(|item| {
                if item.is_separator { sep_h } else { item_h }
            }).sum::<f32>() + pad * 2.0;

            // Menu background (dark)
            push_rect(&mut menu_bg_verts, mx, my, mw, total_h, (45, 45, 45));
            // Menu border
            push_rect(&mut menu_bg_verts, mx, my, mw, 1.0, (70, 70, 70));
            push_rect(&mut menu_bg_verts, mx, my + total_h - 1.0, mw, 1.0, (70, 70, 70));
            push_rect(&mut menu_bg_verts, mx, my, 1.0, total_h, (70, 70, 70));
            push_rect(&mut menu_bg_verts, mx + mw - 1.0, my, 1.0, total_h, (70, 70, 70));

            let mut iy = my + pad;
            for (idx, item) in menu.items.iter().enumerate() {
                if item.is_separator {
                    // Separator line
                    let sep_y = iy + sep_h / 2.0;
                    push_rect(&mut menu_bg_verts, mx + pad, sep_y, mw - pad * 2.0, 1.0, (70, 70, 70));
                    iy += sep_h;
                } else {
                    // Hover highlight
                    if menu.hovered == Some(idx) {
                        push_rect(&mut menu_bg_verts, mx + 4.0, iy, mw - 8.0, item_h, (65, 65, 65));
                    }
                    // Label
                    self.draw_text_line(&mut menu_fg_verts, &item.label, mx + pad * 2.0, iy + (item_h - ch) / 2.0, (240, 240, 240));
                    // Shortcut (right-aligned)
                    if let Some(ref shortcut) = item.shortcut {
                        let shortcut_w = display_width(shortcut) as f32 * cw;
                        let sx = mx + mw - shortcut_w - pad * 2.0;
                        self.draw_text_line(&mut menu_fg_verts, shortcut, sx, iy + (item_h - ch) / 2.0, (140, 140, 140));
                    }
                    iy += item_h;
                }
            }
        }

        // ── 上传 atlas（如果有新字形）──
        if self.atlas.dirty {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.atlas_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &self.atlas.pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(ATLAS_SIZE),
                    rows_per_image: Some(ATLAS_SIZE),
                },
                wgpu::Extent3d { width: ATLAS_SIZE, height: ATLAS_SIZE, depth_or_array_layers: 1 },
            );
            self.atlas.dirty = false;
        }

        // ── GPU 渲染 ──
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.surface_config);
                self.surface.get_current_texture()?
            }
            Err(e) => return Err(e.into()),
        };
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            // Pass 1: 常规背景
            if !bg_verts.is_empty() {
                let buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("bg"), contents: bytemuck::cast_slice(&bg_verts),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                pass.set_pipeline(&self.bg_pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, buf.slice(..));
                pass.draw(0..bg_verts.len() as u32, 0..1);
            }

            // Pass 2: 常规文字
            if !fg_verts.is_empty() {
                let buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("fg"), contents: bytemuck::cast_slice(&fg_verts),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                pass.set_pipeline(&self.text_pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, buf.slice(..));
                pass.draw(0..fg_verts.len() as u32, 0..1);
            }

            // Pass 3+4: 菜单 overlay（仅在菜单打开时创建缓冲区）
            if !menu_bg_verts.is_empty() {
                let buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("m-bg"), contents: bytemuck::cast_slice(&menu_bg_verts),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                pass.set_pipeline(&self.bg_pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, buf.slice(..));
                pass.draw(0..menu_bg_verts.len() as u32, 0..1);
            }
            if !menu_fg_verts.is_empty() {
                let buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("m-fg"), contents: bytemuck::cast_slice(&menu_fg_verts),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                pass.set_pipeline(&self.text_pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, buf.slice(..));
                pass.draw(0..menu_fg_verts.len() as u32, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }

    /// 绘制单个字形四边形
    fn draw_glyph(&mut self, verts: &mut Vec<Vertex>, ch: char, x: f32, y: f32, cell_h: f32, color: (u8, u8, u8)) {
        let entry = match self.atlas.get_or_rasterize(ch) {
            Some(e) => e,
            None => return,
        };
        if entry.width == 0.0 { return; }

        let baseline = cell_h * 0.8; // 基线约在 cell 高度的 80%
        let gx = (x + entry.offset_x).round();
        let gy = (y + baseline - entry.offset_y - entry.height).round();
        let gw = entry.width;
        let gh = entry.height;
        let (u0, v0, u1, v1) = (entry.u0, entry.v0, entry.u1, entry.v1);

        let c = color_to_linear(color);

        verts.extend_from_slice(&[
            Vertex { pos: [gx,      gy],      uv: [u0, v0], color: c },
            Vertex { pos: [gx + gw, gy],      uv: [u1, v0], color: c },
            Vertex { pos: [gx + gw, gy + gh], uv: [u1, v1], color: c },
            Vertex { pos: [gx,      gy],      uv: [u0, v0], color: c },
            Vertex { pos: [gx + gw, gy + gh], uv: [u1, v1], color: c },
            Vertex { pos: [gx,      gy + gh], uv: [u0, v1], color: c },
        ]);
    }

    /// 绘制一行文本（CJK 宽字符占 2 cell）
    fn draw_text_line(&mut self, verts: &mut Vec<Vertex>, text: &str, mut x: f32, y: f32, color: (u8, u8, u8)) {
        let ch = (LINE_HEIGHT * self.scale_factor).round();
        let cw = self.atlas.cell_width;
        for c in text.chars() {
            if c != ' ' && c != '\0' {
                self.draw_glyph(verts, c, x, y, ch, color);
            }
            if is_wide_char(c) {
                x += cw * 2.0;
            } else {
                x += cw;
            }
        }
    }
}

/// sRGB → linear 转换（单通道）
#[inline]
fn srgb_to_linear(s: f32) -> f32 {
    if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
}

/// sRGB (u8,u8,u8) → linear [f32;4]（适配 Bgra8UnormSrgb surface）
#[inline]
fn color_to_linear(color: (u8, u8, u8)) -> [f32; 4] {
    [
        srgb_to_linear(color.0 as f32 / 255.0),
        srgb_to_linear(color.1 as f32 / 255.0),
        srgb_to_linear(color.2 as f32 / 255.0),
        1.0,
    ]
}

/// 判断字符是否为全角/宽字符（CJK 等占 2 个 cell）
pub fn is_wide_char(c: char) -> bool {
    let cp = c as u32;
    matches!(cp,
        0x1100..=0x115F | 0x231A..=0x231B | 0x2329..=0x232A |
        0x23E9..=0x23F3 | 0x23F8..=0x23FA | 0x25FD..=0x25FE |
        0x2614..=0x2615 | 0x2648..=0x2653 | 0x267F | 0x2693 |
        0x26A1 | 0x26AA..=0x26AB | 0x26BD..=0x26BE |
        0x26C4..=0x26C5 | 0x26CE | 0x26D4 | 0x26EA |
        0x26F2..=0x26F3 | 0x26F5 | 0x26FA | 0x26FD |
        0x2702 | 0x2705 | 0x2708..=0x270D | 0x270F |
        0x2E80..=0x303E | 0x3040..=0x33BF | 0x3400..=0x4DBF |
        0x4E00..=0x9FFF | 0xA000..=0xA4CF | 0xA960..=0xA97F |
        0xAC00..=0xD7AF | 0xD7B0..=0xD7FF | 0xF900..=0xFAFF |
        0xFE10..=0xFE19 | 0xFE30..=0xFE6F | 0xFF01..=0xFF60 |
        0xFFE0..=0xFFE6 | 0x1F004 | 0x1F0CF | 0x1F18E |
        0x1F191..=0x1F19A | 0x1F1E0..=0x1F1FF |
        0x1F200..=0x1F202 | 0x1F210..=0x1F23B |
        0x1F240..=0x1F248 | 0x1F250..=0x1F251 |
        0x1F300..=0x1F9FF | 0x20000..=0x2FA1F | 0x30000..=0x3134F
    )
}

/// 计算字符串的显示宽度（以等宽 cell 为单位，CJK 字符算 2）
pub fn display_width(s: &str) -> usize {
    s.chars().map(|c| if is_wide_char(c) { 2 } else { 1 }).sum()
}

/// 推入一个纯色矩形（2 个三角形 = 6 个顶点）
fn push_rect(verts: &mut Vec<Vertex>, x: f32, y: f32, w: f32, h: f32, color: (u8, u8, u8)) {
    let c = color_to_linear(color);
    let uv = [0.0, 0.0]; // bg 不使用纹理
    verts.extend_from_slice(&[
        Vertex { pos: [x,     y],     uv, color: c },
        Vertex { pos: [x + w, y],     uv, color: c },
        Vertex { pos: [x + w, y + h], uv, color: c },
        Vertex { pos: [x,     y],     uv, color: c },
        Vertex { pos: [x + w, y + h], uv, color: c },
        Vertex { pos: [x,     y + h], uv, color: c },
    ]);
}
