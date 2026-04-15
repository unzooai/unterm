//! wgpu + glyphon 多区域文字渲染模块
//!
//! 支持 Tab 栏 + 多 Pane + 状态栏的布局渲染。

use anyhow::Result;
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use std::sync::Arc;
use wgpu;

// ─── 配色常量（Catppuccin Mocha 扩展）───────────────────────────────────────

/// 主背景色 #1e1e2e
pub const BG_COLOR: wgpu::Color = wgpu::Color {
    r: 0.118,
    g: 0.118,
    b: 0.180,
    a: 1.0,
};

/// 默认文本色 #cdd6f4
const TEXT_COLOR: (u8, u8, u8) = (205, 214, 244);

/// 激活 Pane 边框色 #89b4fa（蓝色）
const _BORDER_ACTIVE: (u8, u8, u8) = (137, 180, 250);

/// 非激活 Pane 边框色 #45475a（暗灰）
const _BORDER_INACTIVE: (u8, u8, u8) = (69, 71, 90);

/// Tab 栏背景 #181825（比主背景更深）
const _TAB_BG: (u8, u8, u8) = (24, 24, 37);

/// 激活 Tab 背景 #313244
const _TAB_ACTIVE_BG: (u8, u8, u8) = (49, 50, 68);

/// Tab 文本色 #cdd6f4
const TAB_TEXT: (u8, u8, u8) = (205, 214, 244);

/// 非激活 Pane 文本色（稍暗）#a6adc8 Subtext0
const INACTIVE_TEXT: (u8, u8, u8) = (166, 173, 200);

/// 状态栏背景 #181825
const _STATUS_BG: (u8, u8, u8) = (24, 24, 37);

/// 状态栏文本色 #a6adc8 Subtext0
const STATUS_TEXT: (u8, u8, u8) = (166, 173, 200);

// ─── 字体度量 ────────────────────────────────────────────────────────────────

/// 字体大小（像素）
const FONT_SIZE: f32 = 16.0;
/// 行高（像素）
const LINE_HEIGHT: f32 = 20.0;

// ─── 数据结构 ────────────────────────────────────────────────────────────────

/// 像素级矩形区域
#[derive(Debug, Clone, Copy)]
pub struct PaneRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// 单个 Pane 的渲染内容
#[derive(Debug, Clone)]
pub struct PaneContent {
    /// Pane 唯一标识
    pub pane_id: u64,
    /// 像素区域
    pub rect: PaneRect,
    /// 终端文本内容
    pub text: String,
    /// 是否为当前激活 Pane（影响文本颜色等）
    pub is_active: bool,
    /// 可选标题
    pub title: Option<String>,
}

/// Tab 栏中的单个 Tab 信息
#[derive(Debug, Clone)]
pub struct TabInfo {
    pub id: u64,
    pub title: String,
    pub is_active: bool,
}

// ─── 渲染器 ──────────────────────────────────────────────────────────────────

/// GPU 渲染器——支持多 Pane 区域渲染
pub struct Renderer {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,

    // glyphon 文字渲染基础设施
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,

    // 多区域文本缓冲区
    /// Tab 栏文本缓冲区
    tab_buffer: Buffer,
    /// 各 Pane 文本缓冲区（动态数量）
    pane_buffers: Vec<Buffer>,
    /// 状态栏文本缓冲区
    status_buffer: Buffer,

    pub width: u32,
    pub height: u32,
}

/// 辅助函数：更新 glyphon Buffer 的文本内容和尺寸
fn update_buffer(
    font_system: &mut FontSystem,
    buffer: &mut Buffer,
    text: &str,
    width: Option<f32>,
    height: Option<f32>,
) {
    buffer.set_text(
        font_system,
        text,
        Attrs::new().family(Family::Monospace),
        Shaping::Advanced,
    );
    buffer.set_size(font_system, width, height);
    buffer.shape_until_scroll(font_system, false);
}

impl Renderer {
    /// 初始化 wgpu + glyphon 渲染管线
    pub fn new(window: Arc<winit::window::Window>) -> Result<Self> {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        // ── wgpu 初始化 ──
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window)?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .ok_or_else(|| anyhow::anyhow!("无法找到合适的 GPU 适配器"))?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("unterm-device"),
                ..Default::default()
            },
            None,
        ))?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let swapchain_format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // ── glyphon 文字渲染初始化 ──
        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(&device);
        let mut text_atlas = TextAtlas::new(&device, &queue, &cache, swapchain_format);
        let mut viewport = Viewport::new(&device, &cache);
        viewport.update(&queue, Resolution { width, height });
        let text_renderer = TextRenderer::new(
            &mut text_atlas,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );

        let metrics = Metrics::new(FONT_SIZE, LINE_HEIGHT);

        // 创建各区域的文本缓冲区
        let mut tab_buffer = Buffer::new(&mut font_system, metrics);
        tab_buffer.set_size(&mut font_system, Some(width as f32), Some(LINE_HEIGHT));

        let mut status_buffer = Buffer::new(&mut font_system, metrics);
        status_buffer.set_size(&mut font_system, Some(width as f32), Some(LINE_HEIGHT));

        // Pane 缓冲区初始为空，draw_frame 时按需创建
        let pane_buffers: Vec<Buffer> = Vec::new();

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            font_system,
            swash_cache,
            viewport,
            text_atlas,
            text_renderer,
            tab_buffer,
            pane_buffers,
            status_buffer,
            width,
            height,
        })
    }

    /// 处理窗口大小变化
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.width = width;
        self.height = height;
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
        self.viewport
            .update(&self.queue, Resolution { width, height });
    }

    /// 渲染完整一帧：Tab 栏 + 多个 Pane + 状态栏
    ///
    /// # 参数
    /// - `tabs`：Tab 栏数据
    /// - `panes`：各 Pane 的渲染内容
    /// - `status_text`：状态栏文本
    /// - `status_rect`：状态栏像素区域
    /// - `tab_bar_rect`：Tab 栏像素区域
    pub fn draw_frame(
        &mut self,
        tabs: &[TabInfo],
        panes: &[PaneContent],
        status_text: &str,
        status_rect: PaneRect,
        tab_bar_rect: PaneRect,
    ) -> Result<()> {
        let metrics = Metrics::new(FONT_SIZE, LINE_HEIGHT);

        // ── 1. 准备 Tab 栏文本 ──
        let tab_text = tabs
            .iter()
            .map(|t| {
                if t.is_active {
                    format!("[{}]", t.title)
                } else {
                    t.title.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" | ");

        update_buffer(
            &mut self.font_system,
            &mut self.tab_buffer,
            &tab_text,
            Some(tab_bar_rect.width),
            Some(tab_bar_rect.height),
        );

        // ── 2. 确保 pane_buffers 数量与 panes 匹配 ──
        while self.pane_buffers.len() < panes.len() {
            self.pane_buffers
                .push(Buffer::new(&mut self.font_system, metrics));
        }
        self.pane_buffers.truncate(panes.len());

        // ── 3. 更新每个 Pane 的文本内容 ──
        for (i, pane) in panes.iter().enumerate() {
            update_buffer(
                &mut self.font_system,
                &mut self.pane_buffers[i],
                &pane.text,
                Some(pane.rect.width),
                Some(pane.rect.height),
            );
        }

        // ── 4. 准备状态栏文本 ──
        update_buffer(
            &mut self.font_system,
            &mut self.status_buffer,
            status_text,
            Some(status_rect.width),
            Some(status_rect.height),
        );

        // ── 5. 构建 TextArea 数组 ──
        let mut text_areas: Vec<TextArea> = Vec::with_capacity(2 + panes.len());

        // Tab 栏
        text_areas.push(TextArea {
            buffer: &self.tab_buffer,
            left: tab_bar_rect.x + 8.0,
            top: tab_bar_rect.y + 6.0,
            scale: 1.0,
            bounds: TextBounds {
                left: tab_bar_rect.x as i32,
                top: tab_bar_rect.y as i32,
                right: (tab_bar_rect.x + tab_bar_rect.width) as i32,
                bottom: (tab_bar_rect.y + tab_bar_rect.height) as i32,
            },
            default_color: Color::rgb(TAB_TEXT.0, TAB_TEXT.1, TAB_TEXT.2),
            custom_glyphs: &[],
        });

        // 各 Pane
        for (i, pane) in panes.iter().enumerate() {
            let (r, g, b) = if pane.is_active {
                TEXT_COLOR
            } else {
                INACTIVE_TEXT
            };
            text_areas.push(TextArea {
                buffer: &self.pane_buffers[i],
                left: pane.rect.x + 4.0,
                top: pane.rect.y + 4.0,
                scale: 1.0,
                bounds: TextBounds {
                    left: pane.rect.x as i32,
                    top: pane.rect.y as i32,
                    right: (pane.rect.x + pane.rect.width) as i32,
                    bottom: (pane.rect.y + pane.rect.height) as i32,
                },
                default_color: Color::rgb(r, g, b),
                custom_glyphs: &[],
            });
        }

        // 状态栏
        text_areas.push(TextArea {
            buffer: &self.status_buffer,
            left: status_rect.x + 8.0,
            top: status_rect.y + 2.0,
            scale: 1.0,
            bounds: TextBounds {
                left: status_rect.x as i32,
                top: status_rect.y as i32,
                right: (status_rect.x + status_rect.width) as i32,
                bottom: (status_rect.y + status_rect.height) as i32,
            },
            default_color: Color::rgb(STATUS_TEXT.0, STATUS_TEXT.1, STATUS_TEXT.2),
            custom_glyphs: &[],
        });

        // ── 6. glyphon prepare（提交文本到 GPU）──
        self.text_renderer.prepare(
            &self.device,
            &self.queue,
            &mut self.font_system,
            &mut self.text_atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
        )?;

        // ── 7. wgpu 渲染通道 ──
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("unterm-encoder"),
            },
        );

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("unterm-render-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(BG_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            self.text_renderer
                .render(&self.text_atlas, &self.viewport, &mut pass)?;
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }
}
