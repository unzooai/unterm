//! Draw to an off-screen texture and read the pixels back.
//!
//! The point of these is that they fail on a wrong result rather than a wrong
//! *looking* result. Every bug this layer produced before was of the second
//! kind: it compiled, it submitted, and the window showed something else.
//!
//! They skip when no adapter is available rather than failing, so a machine
//! without a GPU still runs the rest of the suite.

use unterm_render::atlas::{GlyphAtlas, GlyphKey};
use unterm_render::gpu::Renderer;
use unterm_render::quads::{FrameQuads, GlyphQuad, Quad};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

struct Offscreen {
    renderer: Renderer,
    texture: wgpu::Texture,
    width: u32,
    height: u32,
}

fn offscreen(width: u32, height: u32) -> Option<Offscreen> {
    let instance = wgpu::Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok()?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    Some(Offscreen {
        renderer: Renderer::new(device, queue, FORMAT),
        texture,
        width,
        height,
    })
}

impl Offscreen {
    /// Read the whole target back as RGBA.
    fn read_pixels(&self) -> Vec<u8> {
        // Copies out of a texture need rows padded to 256 bytes, so the buffer
        // is wider than the image and has to be un-padded on the way out.
        let unpadded = self.width * 4;
        let padded = unpadded.div_ceil(256) * 256;

        let buffer = self.renderer.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (padded * self.height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder =
            self.renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("readback encoder"),
                });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        self.renderer.queue().submit(Some(encoder.finish()));

        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.renderer.device().poll(wgpu::PollType::Wait);

        let data = slice.get_mapped_range();
        let mut pixels = Vec::with_capacity((unpadded * self.height) as usize);
        for row in 0..self.height {
            let start = (row * padded) as usize;
            pixels.extend_from_slice(&data[start..start + unpadded as usize]);
        }
        drop(data);
        buffer.unmap();
        pixels
    }

    fn pixel(&self, pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
        let index = ((y * self.width + x) * 4) as usize;
        [
            pixels[index],
            pixels[index + 1],
            pixels[index + 2],
            pixels[index + 3],
        ]
    }

    fn view(&self) -> wgpu::TextureView {
        self.texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }
}

fn empty_atlas(renderer: &Renderer) -> wgpu::Texture {
    renderer.upload_atlas(&GlyphAtlas::new(4, 4))
}

#[test]
fn offscreen_an_empty_frame_is_the_clear_colour() {
    let Some(target) = offscreen(8, 8) else {
        return;
    };
    let atlas = empty_atlas(&target.renderer);

    target.renderer.draw(
        &target.view(),
        target.width,
        target.height,
        &FrameQuads::default(),
        &atlas,
        [0.0, 0.0, 1.0, 1.0],
    );

    let pixels = target.read_pixels();
    // A frame with nothing in it still has to clear, or the previous one stays
    // on screen.
    assert_eq!(target.pixel(&pixels, 4, 4), [0, 0, 255, 255]);
}

#[test]
fn offscreen_a_background_quad_lands_where_it_was_put() {
    let Some(target) = offscreen(16, 16) else {
        return;
    };
    let atlas = empty_atlas(&target.renderer);
    let mut quads = FrameQuads::default();
    quads.backgrounds.push(Quad {
        left: 0.0,
        top: 0.0,
        width: 8.0,
        height: 8.0,
        color: [1.0, 0.0, 0.0, 1.0],
    });

    target.renderer.draw(
        &target.view(),
        target.width,
        target.height,
        &quads,
        &atlas,
        [0.0, 0.0, 0.0, 1.0],
    );

    let pixels = target.read_pixels();
    // Inside the quad is red; outside it is still the clear colour. Getting
    // the pixel-to-clip-space mapping wrong shows up here as a quad in the
    // wrong quadrant or flipped vertically.
    assert_eq!(target.pixel(&pixels, 2, 2), [255, 0, 0, 255]);
    assert_eq!(target.pixel(&pixels, 12, 12), [0, 0, 0, 255]);
}

#[test]
fn offscreen_the_top_left_of_a_quad_is_the_top_left_of_the_image() {
    let Some(target) = offscreen(16, 16) else {
        return;
    };
    let atlas = empty_atlas(&target.renderer);
    let mut quads = FrameQuads::default();
    quads.backgrounds.push(Quad {
        left: 0.0,
        top: 0.0,
        width: 4.0,
        height: 4.0,
        color: [0.0, 1.0, 0.0, 1.0],
    });

    target.renderer.draw(
        &target.view(),
        target.width,
        target.height,
        &quads,
        &atlas,
        [0.0, 0.0, 0.0, 1.0],
    );

    let pixels = target.read_pixels();
    // Pixels count down the screen; clip space counts up. A sign error here
    // draws the whole terminal upside down.
    assert_eq!(target.pixel(&pixels, 1, 1), [0, 255, 0, 255]);
    assert_eq!(target.pixel(&pixels, 1, 14), [0, 0, 0, 255]);
}

#[test]
fn offscreen_a_glyph_is_tinted_by_its_colour_and_shaped_by_the_atlas() {
    let Some(target) = offscreen(16, 16) else {
        return;
    };

    // An atlas whose only glyph is fully opaque, so coverage is unambiguous.
    let mut atlas = GlyphAtlas::new(8, 8);
    let slot = atlas.insert(
        GlyphKey {
            face: 0,
            glyph_index: 1,
            pixel_size: 16,
        },
        &unterm_engine::next_core::font_raster::RasterizedGlyph {
            coverage: vec![255; 4 * 4],
            width: 4,
            height: 4,
            bearing_x: 0,
            bearing_y: 0,
            advance_x: 4,
        },
    );
    let atlas_texture = target.renderer.upload_atlas(&atlas);

    let mut quads = FrameQuads::default();
    quads.glyphs.push(GlyphQuad {
        quad: Quad {
            left: 0.0,
            top: 0.0,
            width: 4.0,
            height: 4.0,
            color: [1.0, 1.0, 0.0, 1.0],
        },
        tex_left: slot.x as f32 / atlas.width() as f32,
        tex_top: slot.y as f32 / atlas.height() as f32,
        tex_right: (slot.x + slot.width) as f32 / atlas.width() as f32,
        tex_bottom: (slot.y + slot.height) as f32 / atlas.height() as f32,
    });

    target.renderer.draw(
        &target.view(),
        target.width,
        target.height,
        &quads,
        &atlas_texture,
        [0.0, 0.0, 0.0, 1.0],
    );

    let pixels = target.read_pixels();
    let drawn = target.pixel(&pixels, 1, 1);
    // The atlas carries coverage, not colour: the glyph comes out in the
    // colour the cell asked for. Sampling the wrong channel would give black
    // or white here instead of yellow.
    assert!(drawn[0] > 200 && drawn[1] > 200, "expected yellow, got {drawn:?}");
    assert!(drawn[2] < 60, "expected no blue, got {drawn:?}");
}

#[test]
fn offscreen_a_glyph_draws_over_its_own_background() {
    let Some(target) = offscreen(16, 16) else {
        return;
    };

    let mut atlas = GlyphAtlas::new(8, 8);
    let slot = atlas.insert(
        GlyphKey {
            face: 0,
            glyph_index: 1,
            pixel_size: 16,
        },
        &unterm_engine::next_core::font_raster::RasterizedGlyph {
            coverage: vec![255; 4 * 4],
            width: 4,
            height: 4,
            bearing_x: 0,
            bearing_y: 0,
            advance_x: 4,
        },
    );
    let atlas_texture = target.renderer.upload_atlas(&atlas);

    let mut quads = FrameQuads::default();
    quads.backgrounds.push(Quad {
        left: 0.0,
        top: 0.0,
        width: 4.0,
        height: 4.0,
        color: [1.0, 0.0, 0.0, 1.0],
    });
    quads.glyphs.push(GlyphQuad {
        quad: Quad {
            left: 0.0,
            top: 0.0,
            width: 4.0,
            height: 4.0,
            color: [0.0, 0.0, 1.0, 1.0],
        },
        tex_left: slot.x as f32 / atlas.width() as f32,
        tex_top: slot.y as f32 / atlas.height() as f32,
        tex_right: (slot.x + slot.width) as f32 / atlas.width() as f32,
        tex_bottom: (slot.y + slot.height) as f32 / atlas.height() as f32,
    });

    target.renderer.draw(
        &target.view(),
        target.width,
        target.height,
        &quads,
        &atlas_texture,
        [0.0, 0.0, 0.0, 1.0],
    );

    let pixels = target.read_pixels();
    let drawn = target.pixel(&pixels, 1, 1);
    // Ordering is what puts text on top of its cell without a depth buffer.
    assert!(drawn[2] > 200, "glyph should be on top, got {drawn:?}");
}
