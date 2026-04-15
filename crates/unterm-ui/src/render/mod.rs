//! wgpu + glyphon 文字渲染模块

use anyhow::Result;
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution,
    Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use std::sync::Arc;
use wgpu;

/// 终端配色（Catppuccin Mocha）
pub const BG_COLOR: wgpu::Color = wgpu::Color {
    r: 0.118, g: 0.118, b: 0.180, a: 1.0, // #1e1e2e
};
pub const TEXT_COLOR: Color = Color::rgb(205, 214, 244); // #cdd6f4

/// GPU 渲染器
pub struct Renderer {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub viewport: Viewport,
    pub text_atlas: TextAtlas,
    pub text_renderer: TextRenderer,
    pub text_buffer: Buffer,
    pub width: u32,
    pub height: u32,
}

impl Renderer {
    /// 初始化 wgpu + glyphon 渲染管线
    pub fn new(window: Arc<winit::window::Window>) -> Result<Self> {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        // 初始化 wgpu
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

        // 初始化 glyphon 文字渲染
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

        // 创建文本缓冲区
        let mut text_buffer = Buffer::new(&mut font_system, Metrics::new(16.0, 20.0));
        text_buffer.set_size(&mut font_system, Some(width as f32), Some(height as f32));

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
            text_buffer,
            width,
            height,
        })
    }

    /// 设置要渲染的文本内容
    pub fn set_text(&mut self, text: &str) {
        self.text_buffer.set_text(
            &mut self.font_system,
            text,
            Attrs::new().family(Family::Monospace).color(TEXT_COLOR),
            Shaping::Advanced,
        );
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
        self.viewport.update(&self.queue, Resolution { width, height });
        self.text_buffer.set_size(
            &mut self.font_system,
            Some(width as f32),
            Some(height as f32),
        );
    }

    /// 渲染一帧
    pub fn draw_frame(&mut self) -> Result<()> {
        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 准备文字渲染
        self.text_renderer.prepare(
            &self.device,
            &self.queue,
            &mut self.font_system,
            &mut self.text_atlas,
            &self.viewport,
            [TextArea {
                buffer: &self.text_buffer,
                left: 8.0,
                top: 8.0,
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    right: self.width as i32,
                    bottom: self.height as i32,
                },
                default_color: TEXT_COLOR,
                custom_glyphs: &[],
            }],
            &mut self.swash_cache,
        )?;

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("unterm-encoder") },
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

            self.text_renderer.render(&self.text_atlas, &self.viewport, &mut pass)?;
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }
}
