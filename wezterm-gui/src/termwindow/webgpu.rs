use crate::engine::render_backend::fit_glyph_rgba_to_atlas_region;
use crate::engine::{
    EngineRenderBufferBatch, EngineRenderBufferPlan, EngineRenderCachedGlyphUploadDiagnostics,
    EngineRenderGlyphAtlasCache, EngineRenderGlyphAtlasCacheUpdate, EngineRenderGlyphAtlasPlan,
    EngineRenderGlyphAtlasTextureRegion, EngineRenderGlyphAtlasTextureUpdatePlan,
    EngineRenderGlyphRaster, EngineRenderGlyphRasterSource, EngineRenderGpuUploadPlan,
    EngineRenderPreparedPaneFrame, EngineRenderShaperGlyph, EngineRenderTextAtlasPlan,
    EngineRenderTexturedGlyphLayoutDiff, EngineRenderTexturedGlyphUploadPlan,
    EngineWgpuPipelineConfig, EngineWgpuPreparedFramePlan, EngineWgpuRenderBackend,
};
use crate::quad::Vertex;
use anyhow::anyhow;
use config::{ConfigHandle, GpuInfo, WebGpuPowerPreference};
use std::cell::RefCell;
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::Arc;
use unterm_engine::{CellStyle, StyledBlink, StyledColor, StyledUnderline, StyledVerticalAlign};
use wezterm_font::shaper::Direction;
use wezterm_font::{GlyphInfo, LoadedFont};
use wgpu::util::DeviceExt;
use window::bitmaps::Texture2d;
use window::raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle,
};
use window::{BitmapImage, Dimensions, Rect, Window};

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShaderUniform {
    pub foreground_text_hsb: [f32; 3],
    pub milliseconds: u32,
    pub projection: [[f32; 4]; 4],
    // sampler2D atlas_nearest_sampler;
    // sampler2D atlas_linear_sampler;
}

pub struct WebGpuState {
    pub adapter_info: wgpu::AdapterInfo,
    pub downlevel_caps: wgpu::DownlevelCapabilities,
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: Arc<wgpu::Queue>,
    pub config: RefCell<wgpu::SurfaceConfiguration>,
    pub dimensions: RefCell<Dimensions>,
    pub render_pipeline: wgpu::RenderPipeline,
    pub next_core_render_backend: EngineWgpuRenderBackend,
    pub next_core_render_pipeline: wgpu::RenderPipeline,
    pub next_core_textured_glyph_pipeline: wgpu::RenderPipeline,
    next_core_glyph_atlases: RefCell<NextCoreGlyphAtlasState>,
    next_core_glyph_texture: NextCoreGlyphTexture,
    next_core_glyph_texture_bind_group: wgpu::BindGroup,
    shader_uniform_bind_group_layout: wgpu::BindGroupLayout,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub texture_nearest_sampler: wgpu::Sampler,
    pub texture_linear_sampler: wgpu::Sampler,
    pub handle: RawHandlePair,
}

const NEXT_CORE_GLYPH_ATLAS_WIDTH_PX: usize = 2048;
const NEXT_CORE_GLYPH_ATLAS_HEIGHT_PX: usize = 2048;

#[derive(Clone, Debug, Default)]
pub struct NextCoreGlyphAtlasState {
    panes: HashMap<usize, NextCorePaneGlyphAtlasState>,
    shaped_glyph_atlases: HashMap<usize, NextCoreShapedGlyphAtlasCacheEntry>,
}

#[derive(Clone, Debug)]
struct NextCorePaneGlyphAtlasState {
    cell_width_px: usize,
    cell_height_px: usize,
    cache: EngineRenderGlyphAtlasCache,
}

#[derive(Clone, Debug)]
struct NextCoreShapedGlyphAtlasCacheEntry {
    revision: u64,
    font_id: usize,
    fingerprint: u64,
    glyphs: EngineRenderGlyphAtlasPlan,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct NextCoreCachedGlyphUpload {
    pub pane_id: usize,
    pub revision: u64,
    pub cell_width_px: usize,
    pub cell_height_px: usize,
    pub update: EngineRenderGlyphAtlasCacheUpdate,
    pub texture_update: EngineRenderGlyphAtlasTextureUpdatePlan,
    pub upload: EngineRenderTexturedGlyphUploadPlan,
}

#[allow(dead_code)]
pub struct NextCoreWebGpuPaneDrawFrame {
    pub engine_frame: EngineRenderPreparedPaneFrame,
    font: Option<Rc<LoadedFont>>,
}

#[derive(Clone, Debug)]
struct NextCoreFontGlyphRasterSource {
    font: Rc<LoadedFont>,
}

impl NextCoreFontGlyphRasterSource {
    fn new(font: Rc<LoadedFont>) -> Self {
        Self { font }
    }
}

impl EngineRenderGlyphRasterSource for NextCoreFontGlyphRasterSource {
    fn rasterize_glyph_rgba(
        &self,
        key: &crate::engine::EngineRenderGlyphAtlasKey,
        width_px: usize,
        height_px: usize,
    ) -> Option<Vec<u8>> {
        let (font_idx, glyph_pos) = key.raster_identity()?;
        let glyph = self.font.rasterize_glyph(glyph_pos, font_idx).ok()?;
        Some(fit_glyph_rgba_to_atlas_region(
            &glyph.data,
            glyph.width,
            glyph.height,
            width_px,
            height_px,
            key.faint,
        ))
    }

    fn rasterize_glyph_texture(
        &self,
        key: &crate::engine::EngineRenderGlyphAtlasKey,
        width_px: usize,
        height_px: usize,
    ) -> Option<EngineRenderGlyphRaster> {
        let (font_idx, glyph_pos) = key.raster_identity()?;
        let glyph = self.font.rasterize_glyph(glyph_pos, font_idx).ok()?;
        Some(EngineRenderGlyphRaster {
            bytes_rgba: fit_glyph_rgba_to_atlas_region(
                &glyph.data,
                glyph.width,
                glyph.height,
                width_px,
                height_px,
                key.faint,
            ),
            source_width_px: glyph.width,
            source_height_px: glyph.height,
            bearing_x_px: glyph.bearing_x.get().round() as i32,
            bearing_y_px: glyph.bearing_y.get().round() as i32,
            uses_raster_metrics: true,
        })
    }
}

fn shaper_glyphs_from_glyph_infos(glyph_infos: Vec<GlyphInfo>) -> Vec<EngineRenderShaperGlyph> {
    glyph_infos
        .into_iter()
        .map(|info| EngineRenderShaperGlyph {
            text: info.text,
            only_char: info.only_char,
            num_cells: info.num_cells,
            font_idx: info.font_idx,
            glyph_pos: info.glyph_pos,
            x_advance_px: info.x_advance.get(),
            x_offset_px: info.x_offset.get(),
            y_offset_px: info.y_offset.get(),
        })
        .collect()
}

#[allow(dead_code)]
impl NextCoreCachedGlyphUpload {
    pub fn diagnostics(&self) -> EngineRenderCachedGlyphUploadDiagnostics {
        EngineRenderCachedGlyphUploadDiagnostics::from_parts(
            self.cell_width_px,
            self.cell_height_px,
            &self.update,
            &self.texture_update,
            &self.upload,
        )
    }

    pub fn diff_layout_against(
        &self,
        actual: &NextCoreCachedGlyphUpload,
    ) -> EngineRenderTexturedGlyphLayoutDiff {
        self.upload.layout.diff_against(&actual.upload.layout)
    }

    pub fn has_clean_layout_parity_with(&self, actual: &NextCoreCachedGlyphUpload) -> bool {
        self.diff_layout_against(actual).is_clean()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub struct NextCoreGlyphTextureUploadStats {
    pub region_count: usize,
    pub byte_count: usize,
}

pub struct NextCoreGlyphTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
    queue: Arc<wgpu::Queue>,
}

pub struct RawHandlePair {
    window: RawWindowHandle,
    display: RawDisplayHandle,
}

impl RawHandlePair {
    fn new(window: &Window) -> Self {
        Self {
            window: window.window_handle().expect("window handle").as_raw(),
            display: window.display_handle().expect("display handle").as_raw(),
        }
    }
}

impl HasWindowHandle for RawHandlePair {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        unsafe { Ok(WindowHandle::borrow_raw(self.window)) }
    }
}

impl HasDisplayHandle for RawHandlePair {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        unsafe { Ok(DisplayHandle::borrow_raw(self.display)) }
    }
}

pub struct WebGpuTexture {
    texture: wgpu::Texture,
    width: u32,
    height: u32,
    queue: Arc<wgpu::Queue>,
}

impl std::ops::Deref for WebGpuTexture {
    type Target = wgpu::Texture;
    fn deref(&self) -> &Self::Target {
        &self.texture
    }
}

impl Texture2d for WebGpuTexture {
    fn write(&self, rect: Rect, im: &dyn BitmapImage) {
        let (im_width, im_height) = im.image_dimensions();

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: rect.min_x() as u32,
                    y: rect.min_y() as u32,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            im.pixel_data_slice(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(im_width as u32 * 4),
                rows_per_image: Some(im_height as u32),
            },
            wgpu::Extent3d {
                width: im_width as u32,
                height: im_height as u32,
                depth_or_array_layers: 1,
            },
        );
    }

    fn read(&self, _rect: Rect, _im: &mut dyn BitmapImage) {
        unimplemented!();
    }

    fn width(&self) -> usize {
        self.width as usize
    }

    fn height(&self) -> usize {
        self.height as usize
    }
}

impl WebGpuTexture {
    pub fn new(width: u32, height: u32, state: &WebGpuState) -> anyhow::Result<Self> {
        let limit = state.device.limits().max_texture_dimension_2d;

        if width > limit || height > limit {
            // Ideally, wgpu would have a fallible create_texture method,
            // but it doesn't: instead it will panic if the requested
            // dimension is too large.
            // So we check the limit ourselves here.
            // <https://github.com/wezterm/wezterm/issues/3713>
            anyhow::bail!(
                "texture dimensions {width}x{height} exceeed the \
                 max dimension {limit} supported by your GPU"
            );
        }

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let view_formats = if state
            .downlevel_caps
            .flags
            .contains(wgpu::DownlevelFlags::SURFACE_VIEW_FORMATS)
        {
            vec![format, format.remove_srgb_suffix()]
        } else {
            vec![]
        };
        let texture = state.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("Texture Atlas"),
            view_formats: &view_formats,
        });
        Ok(Self {
            texture,
            width,
            height,
            queue: Arc::clone(&state.queue),
        })
    }
}

impl NextCoreGlyphTexture {
    #[allow(dead_code)]
    pub fn new(state: &WebGpuState) -> anyhow::Result<Self> {
        Self::new_with_device(
            NEXT_CORE_GLYPH_ATLAS_WIDTH_PX as u32,
            NEXT_CORE_GLYPH_ATLAS_HEIGHT_PX as u32,
            &state.device,
            &state.queue,
        )
    }

    fn new_with_device(
        width: u32,
        height: u32,
        device: &wgpu::Device,
        queue: &Arc<wgpu::Queue>,
    ) -> anyhow::Result<Self> {
        let limit = device.limits().max_texture_dimension_2d;
        if width > limit || height > limit {
            anyhow::bail!(
                "next-core glyph atlas texture dimensions {width}x{height} exceed the \
                 max dimension {limit} supported by your GPU"
            );
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("next-core glyph texture atlas"),
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Ok(Self {
            texture,
            view,
            width,
            height,
            queue: Arc::clone(queue),
        })
    }

    fn create_bind_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
            label: Some("next-core glyph texture bind group"),
        })
    }

    pub fn upload_update(
        &self,
        update: &EngineRenderGlyphAtlasTextureUpdatePlan,
    ) -> anyhow::Result<NextCoreGlyphTextureUploadStats> {
        if update.atlas_width_px != self.width as usize
            || update.atlas_height_px != self.height as usize
        {
            anyhow::bail!(
                "next-core glyph texture update atlas size {}x{} does not match texture {}x{}",
                update.atlas_width_px,
                update.atlas_height_px,
                self.width,
                self.height
            );
        }
        let stats = validate_next_core_glyph_texture_regions(
            self.width as usize,
            self.height as usize,
            &update.regions,
        )?;
        for region in &update.regions {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: region.rect.x as u32,
                        y: region.rect.y as u32,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &region.bytes_rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(region.width_px as u32 * 4),
                    rows_per_image: Some(region.height_px as u32),
                },
                wgpu::Extent3d {
                    width: region.width_px as u32,
                    height: region.height_px as u32,
                    depth_or_array_layers: 1,
                },
            );
        }
        Ok(stats)
    }
}

fn validate_next_core_glyph_texture_regions(
    atlas_width_px: usize,
    atlas_height_px: usize,
    regions: &[EngineRenderGlyphAtlasTextureRegion],
) -> anyhow::Result<NextCoreGlyphTextureUploadStats> {
    let mut byte_count = 0usize;
    for region in regions {
        if region.width_px == 0 || region.height_px == 0 {
            anyhow::bail!(
                "next-core glyph texture region {} has zero size {}x{}",
                region.key_index,
                region.width_px,
                region.height_px
            );
        }
        if region.rect.x.saturating_add(region.width_px) > atlas_width_px
            || region.rect.y.saturating_add(region.height_px) > atlas_height_px
        {
            anyhow::bail!(
                "next-core glyph texture region {} overflows atlas: rect=({},{} {}x{}) atlas={}x{}",
                region.key_index,
                region.rect.x,
                region.rect.y,
                region.width_px,
                region.height_px,
                atlas_width_px,
                atlas_height_px
            );
        }
        let expected_len = region
            .width_px
            .saturating_mul(region.height_px)
            .saturating_mul(4);
        if region.bytes_rgba.len() != expected_len {
            anyhow::bail!(
                "next-core glyph texture region {} has {} bytes, expected {}",
                region.key_index,
                region.bytes_rgba.len(),
                expected_len
            );
        }
        byte_count = byte_count.saturating_add(region.bytes_rgba.len());
    }
    Ok(NextCoreGlyphTextureUploadStats {
        region_count: regions.len(),
        byte_count,
    })
}

#[allow(dead_code)]
impl NextCoreGlyphAtlasState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.panes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.panes.is_empty() && self.shaped_glyph_atlases.is_empty()
    }

    pub fn remove_pane(&mut self, pane_id: usize) {
        self.panes.remove(&pane_id);
        self.shaped_glyph_atlases.remove(&pane_id);
    }

    pub fn clear(&mut self) {
        self.panes.clear();
        self.shaped_glyph_atlases.clear();
    }

    pub fn prepare_shaped_glyph_atlas_with_shaper<F>(
        &mut self,
        text_atlas: &EngineRenderTextAtlasPlan,
        font_id: usize,
        mut shape_run: F,
    ) -> Option<EngineRenderGlyphAtlasPlan>
    where
        F: FnMut(&str) -> Option<Vec<EngineRenderShaperGlyph>>,
    {
        if !text_atlas.submitted || text_atlas.runs.is_empty() {
            return None;
        }

        let fingerprint = text_atlas_fingerprint(text_atlas);
        if let Some(entry) = self.shaped_glyph_atlases.get(&text_atlas.pane_id) {
            if entry.revision == text_atlas.revision
                && entry.font_id == font_id
                && entry.fingerprint == fingerprint
            {
                return Some(entry.glyphs.clone());
            }
        }

        let mut shaped_runs = Vec::with_capacity(text_atlas.runs.len());
        for run in &text_atlas.runs {
            shaped_runs.push(shape_run(&run.text)?);
        }
        let shaped = EngineWgpuRenderBackend::prepare_shaped_glyph_plan(text_atlas, &shaped_runs);
        if shaped.is_empty() {
            return None;
        }
        let glyphs = EngineWgpuRenderBackend::prepare_glyph_atlas_from_shaped_glyphs(&shaped);
        self.shaped_glyph_atlases.insert(
            text_atlas.pane_id,
            NextCoreShapedGlyphAtlasCacheEntry {
                revision: text_atlas.revision,
                font_id,
                fingerprint,
                glyphs: glyphs.clone(),
            },
        );
        Some(glyphs)
    }

    pub fn prepare_cached_upload(
        &mut self,
        plan: &EngineRenderBufferPlan,
        glyphs: &EngineRenderGlyphAtlasPlan,
        viewport_width_px: f32,
        viewport_height_px: f32,
    ) -> Option<NextCoreCachedGlyphUpload> {
        if !plan.submitted || plan.text_runs.is_empty() {
            return None;
        }
        let (cell_width_px, cell_height_px) = infer_cell_metrics_from_buffer_plan(plan)?;
        if glyphs.is_empty() {
            return None;
        }
        let pane = self
            .panes
            .entry(plan.pane_id)
            .or_insert_with(|| NextCorePaneGlyphAtlasState::new(cell_width_px, cell_height_px));
        if pane.cell_width_px != cell_width_px || pane.cell_height_px != cell_height_px {
            *pane = NextCorePaneGlyphAtlasState::new(cell_width_px, cell_height_px);
        }
        let update = pane
            .cache
            .ensure_glyphs(glyphs, cell_width_px, cell_height_px);
        let texture_update = EngineWgpuRenderBackend::prepare_glyph_atlas_texture_update(
            glyphs,
            &update,
            NEXT_CORE_GLYPH_ATLAS_WIDTH_PX,
            NEXT_CORE_GLYPH_ATLAS_HEIGHT_PX,
        );
        pane.cache.apply_texture_update_metrics(&texture_update);
        let upload = EngineWgpuRenderBackend::prepare_textured_glyph_upload_for_viewport(
            glyphs,
            &pane.cache.placements,
            viewport_width_px,
            viewport_height_px,
            NEXT_CORE_GLYPH_ATLAS_WIDTH_PX as f32,
            NEXT_CORE_GLYPH_ATLAS_HEIGHT_PX as f32,
        );
        Some(NextCoreCachedGlyphUpload {
            pane_id: plan.pane_id,
            revision: plan.revision,
            cell_width_px,
            cell_height_px,
            update,
            texture_update,
            upload,
        })
    }

    pub fn prepare_cached_upload_with_raster_source(
        &mut self,
        plan: &EngineRenderBufferPlan,
        glyphs: &EngineRenderGlyphAtlasPlan,
        viewport_width_px: f32,
        viewport_height_px: f32,
        raster_source: &dyn EngineRenderGlyphRasterSource,
    ) -> Option<NextCoreCachedGlyphUpload> {
        if !plan.submitted || plan.text_runs.is_empty() {
            return None;
        }
        let (cell_width_px, cell_height_px) = infer_cell_metrics_from_buffer_plan(plan)?;
        if glyphs.is_empty() {
            return None;
        }
        let pane = self
            .panes
            .entry(plan.pane_id)
            .or_insert_with(|| NextCorePaneGlyphAtlasState::new(cell_width_px, cell_height_px));
        if pane.cell_width_px != cell_width_px || pane.cell_height_px != cell_height_px {
            *pane = NextCorePaneGlyphAtlasState::new(cell_width_px, cell_height_px);
        }
        let update = pane
            .cache
            .ensure_glyphs(glyphs, cell_width_px, cell_height_px);
        let texture_update =
            EngineWgpuRenderBackend::prepare_glyph_atlas_texture_update_with_raster_source(
                glyphs,
                &update,
                NEXT_CORE_GLYPH_ATLAS_WIDTH_PX,
                NEXT_CORE_GLYPH_ATLAS_HEIGHT_PX,
                raster_source,
            );
        pane.cache.apply_texture_update_metrics(&texture_update);
        let upload = EngineWgpuRenderBackend::prepare_textured_glyph_upload_for_viewport(
            glyphs,
            &pane.cache.placements,
            viewport_width_px,
            viewport_height_px,
            NEXT_CORE_GLYPH_ATLAS_WIDTH_PX as f32,
            NEXT_CORE_GLYPH_ATLAS_HEIGHT_PX as f32,
        );
        Some(NextCoreCachedGlyphUpload {
            pane_id: plan.pane_id,
            revision: plan.revision,
            cell_width_px,
            cell_height_px,
            update,
            texture_update,
            upload,
        })
    }
}

impl NextCorePaneGlyphAtlasState {
    fn new(cell_width_px: usize, cell_height_px: usize) -> Self {
        Self {
            cell_width_px,
            cell_height_px,
            cache: EngineRenderGlyphAtlasCache::new(
                NEXT_CORE_GLYPH_ATLAS_WIDTH_PX,
                NEXT_CORE_GLYPH_ATLAS_HEIGHT_PX,
            ),
        }
    }
}

fn infer_cell_metrics_from_buffer_plan(plan: &EngineRenderBufferPlan) -> Option<(usize, usize)> {
    plan.text_runs.iter().find_map(|run| {
        if run.cells == 0 || run.rect.width == 0 || run.rect.height == 0 {
            return None;
        }
        Some(((run.rect.width / run.cells).max(1), run.rect.height.max(1)))
    })
}

fn text_atlas_fingerprint(text_atlas: &EngineRenderTextAtlasPlan) -> u64 {
    let mut hasher = DefaultHasher::new();
    text_atlas.pane_id.hash(&mut hasher);
    text_atlas.submitted.hash(&mut hasher);
    text_atlas.revision.hash(&mut hasher);
    text_atlas.requires_full_repaint.hash(&mut hasher);
    text_atlas.runs.len().hash(&mut hasher);
    for run in &text_atlas.runs {
        run.row.hash(&mut hasher);
        run.col.hash(&mut hasher);
        run.cells.hash(&mut hasher);
        run.text.hash(&mut hasher);
        run.rect.x.hash(&mut hasher);
        run.rect.y.hash(&mut hasher);
        run.rect.width.hash(&mut hasher);
        run.rect.height.hash(&mut hasher);
        for channel in run.foreground {
            channel.to_bits().hash(&mut hasher);
        }
        cell_style_fingerprint(&run.style).hash(&mut hasher);
    }
    hasher.finish()
}

fn cell_style_fingerprint(style: &CellStyle) -> u64 {
    let mut hasher = DefaultHasher::new();
    style.bold.hash(&mut hasher);
    style.faint.hash(&mut hasher);
    style.italic.hash(&mut hasher);
    style.underline.hash(&mut hasher);
    styled_underline_fingerprint(style.underline_style).hash(&mut hasher);
    styled_color_fingerprint(style.underline_color).hash(&mut hasher);
    style.strikethrough.hash(&mut hasher);
    style.hidden.hash(&mut hasher);
    style.overline.hash(&mut hasher);
    styled_blink_fingerprint(style.blink).hash(&mut hasher);
    styled_vertical_align_fingerprint(style.vertical_align).hash(&mut hasher);
    style.inverse.hash(&mut hasher);
    styled_color_fingerprint(style.fg).hash(&mut hasher);
    styled_color_fingerprint(style.bg).hash(&mut hasher);
    style.hyperlink.hash(&mut hasher);
    hasher.finish()
}

fn styled_color_fingerprint(color: Option<StyledColor>) -> u64 {
    match color {
        None => 0,
        Some(StyledColor::Palette(index)) => 0x100 | u64::from(index),
        Some(StyledColor::Rgb(r, g, b)) => {
            0x200 | (u64::from(r) << 16) | (u64::from(g) << 8) | u64::from(b)
        }
    }
}

fn styled_blink_fingerprint(blink: Option<StyledBlink>) -> u8 {
    match blink {
        None => 0,
        Some(StyledBlink::Slow) => 1,
        Some(StyledBlink::Rapid) => 2,
    }
}

fn styled_underline_fingerprint(underline: Option<StyledUnderline>) -> u8 {
    match underline {
        None => 0,
        Some(StyledUnderline::Single) => 1,
        Some(StyledUnderline::Double) => 2,
        Some(StyledUnderline::Curly) => 3,
        Some(StyledUnderline::Dotted) => 4,
        Some(StyledUnderline::Dashed) => 5,
    }
}

fn styled_vertical_align_fingerprint(vertical_align: Option<StyledVerticalAlign>) -> u8 {
    match vertical_align {
        None => 0,
        Some(StyledVerticalAlign::SuperScript) => 1,
        Some(StyledVerticalAlign::SubScript) => 2,
    }
}

pub fn adapter_info_to_gpu_info(info: wgpu::AdapterInfo) -> GpuInfo {
    GpuInfo {
        name: info.name,
        vendor: Some(info.vendor),
        device: Some(info.device),
        device_type: format!("{:?}", info.device_type),
        driver: if info.driver.is_empty() {
            None
        } else {
            Some(info.driver)
        },
        driver_info: if info.driver_info.is_empty() {
            None
        } else {
            Some(info.driver_info)
        },
        backend: format!("{:?}", info.backend),
    }
}

fn compute_compatibility_list(
    instance: &wgpu::Instance,
    backends: wgpu::Backends,
    surface: &wgpu::Surface,
) -> Vec<String> {
    instance
        .enumerate_adapters(backends)
        .into_iter()
        .map(|a| {
            let info = adapter_info_to_gpu_info(a.get_info());
            let compatible = a.is_surface_supported(&surface);
            format!(
                "{}, compatible={}",
                info.to_string(),
                if compatible { "yes" } else { "NO" }
            )
        })
        .collect()
}

impl WebGpuState {
    pub async fn new(
        window: &Window,
        dimensions: Dimensions,
        config: &ConfigHandle,
    ) -> anyhow::Result<Self> {
        let handle = RawHandlePair::new(window);
        Self::new_impl(handle, dimensions, config).await
    }

    pub async fn new_impl(
        handle: RawHandlePair,
        dimensions: Dimensions,
        config: &ConfigHandle,
    ) -> anyhow::Result<Self> {
        let backends = wgpu::Backends::all();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&handle)?)?
        };

        let mut adapter: Option<wgpu::Adapter> = None;

        if let Some(preference) = &config.webgpu_preferred_adapter {
            for a in instance.enumerate_adapters(backends) {
                if !a.is_surface_supported(&surface) {
                    let info = adapter_info_to_gpu_info(a.get_info());
                    log::warn!("{} is not compatible with surface", info.to_string());
                    continue;
                }

                let info = a.get_info();

                if preference.name != info.name {
                    continue;
                }

                if preference.device_type != format!("{:?}", info.device_type) {
                    continue;
                }

                if preference.backend != format!("{:?}", info.backend) {
                    continue;
                }

                if let Some(driver) = &preference.driver {
                    if *driver != info.driver {
                        continue;
                    }
                }
                if let Some(vendor) = &preference.vendor {
                    if *vendor != info.vendor {
                        continue;
                    }
                }
                if let Some(device) = &preference.device {
                    if *device != info.device {
                        continue;
                    }
                }

                adapter.replace(a);
                break;
            }

            if adapter.is_none() {
                let adapters = compute_compatibility_list(&instance, backends, &surface);
                log::warn!(
                    "Your webgpu preferred adapter '{}' was either not \
                     found or is not compatible with your display. Available:\n{}",
                    preference.to_string(),
                    adapters.join("\n")
                );
            }
        }

        if adapter.is_none() {
            adapter = Some(
                instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: match config.webgpu_power_preference {
                            WebGpuPowerPreference::HighPerformance => {
                                wgpu::PowerPreference::HighPerformance
                            }
                            WebGpuPowerPreference::LowPower => wgpu::PowerPreference::LowPower,
                        },
                        compatible_surface: Some(&surface),
                        force_fallback_adapter: config.webgpu_force_fallback_adapter,
                    })
                    .await?,
            );
        }

        let adapter = adapter.ok_or_else(|| {
            let adapters = compute_compatibility_list(&instance, backends, &surface);
            anyhow!(
                "no compatible adapter found. Available:\n{}",
                adapters.join("\n")
            )
        })?;

        let adapter_info = adapter.get_info();
        log::trace!("Using adapter: {adapter_info:?}");
        let caps = surface.get_capabilities(&adapter);
        log::trace!("caps: {caps:?}");
        let downlevel_caps = adapter.get_downlevel_capabilities();
        log::trace!("downlevel_caps: {downlevel_caps:?}");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::downlevel_defaults()
                }
                .using_resolution(adapter.limits()),
                label: None,
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let queue = Arc::new(queue);

        // Explicitly request an SRGB format, if available
        let pref_format_srgb = caps.formats[0].add_srgb_suffix();
        let format = if caps.formats.contains(&pref_format_srgb) {
            pref_format_srgb
        } else {
            caps.formats[0]
        };

        // Need to check that this is supported, as trying to set
        // view_formats without it will cause surface.configure
        // to panic
        // <https://github.com/wezterm/wezterm/issues/3565>
        let view_formats = if downlevel_caps
            .flags
            .contains(wgpu::DownlevelFlags::SURFACE_VIEW_FORMATS)
        {
            vec![format.add_srgb_suffix(), format.remove_srgb_suffix()]
        } else {
            vec![]
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            // Clamp to at least 1x1. Configuring a surface with a 0 width or
            // height panics in wgpu ("Invalid surface"). The resize path below
            // already guards against this (issue #2881), but the INITIAL
            // configure here did not — so on a GPU-less host (VM / RDP / cloud
            // Windows) where the window can report 0 dimensions at WebGpu-init
            // time, Unterm crashed on launch instead of coming up. A 1x1
            // surface is valid; the resize path reconfigures it with the real
            // size as soon as the window is measured.
            width: (dimensions.pixel_width as u32).max(1),
            height: (dimensions.pixel_height as u32).max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: if caps
                .alpha_modes
                .contains(&wgpu::CompositeAlphaMode::PostMultiplied)
            {
                wgpu::CompositeAlphaMode::PostMultiplied
            } else if caps
                .alpha_modes
                .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
            {
                wgpu::CompositeAlphaMode::PreMultiplied
            } else {
                wgpu::CompositeAlphaMode::Auto
            },
            view_formats,
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::include_wgsl!("../shader.wgsl"));

        let shader_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("ShaderUniform bind group layout"),
            });

        let texture_nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let texture_linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture bind group layout"),
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &shader_uniform_bind_group_layout,
                    &texture_bind_group_layout,
                    &texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
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
        });
        let next_core_render_backend = EngineWgpuRenderBackend::default();
        let next_core_render_pipeline = next_core_render_backend.create_pipeline(
            &device,
            EngineWgpuPipelineConfig {
                target_format: config.format,
            },
        );
        let next_core_glyph_texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("next-core glyph texture bind group layout"),
            });
        let next_core_textured_glyph_pipeline = next_core_render_backend
            .create_textured_glyph_pipeline(
                &device,
                EngineWgpuPipelineConfig {
                    target_format: config.format,
                },
                &next_core_glyph_texture_bind_group_layout,
            );
        let next_core_glyph_texture = NextCoreGlyphTexture::new_with_device(
            NEXT_CORE_GLYPH_ATLAS_WIDTH_PX as u32,
            NEXT_CORE_GLYPH_ATLAS_HEIGHT_PX as u32,
            &device,
            &queue,
        )?;
        let next_core_glyph_texture_bind_group = next_core_glyph_texture.create_bind_group(
            &device,
            &next_core_glyph_texture_bind_group_layout,
            &texture_linear_sampler,
        );

        Ok(Self {
            adapter_info,
            downlevel_caps,
            surface,
            device,
            queue,
            config: RefCell::new(config),
            dimensions: RefCell::new(dimensions),
            render_pipeline,
            next_core_render_backend,
            next_core_render_pipeline,
            next_core_textured_glyph_pipeline,
            next_core_glyph_atlases: RefCell::new(NextCoreGlyphAtlasState::new()),
            next_core_glyph_texture,
            next_core_glyph_texture_bind_group,
            handle,
            shader_uniform_bind_group_layout,
            texture_bind_group_layout,
            texture_nearest_sampler,
            texture_linear_sampler,
        })
    }

    pub fn create_uniform(&self, uniform: ShaderUniform) -> wgpu::BindGroup {
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ShaderUniform Buffer"),
                contents: bytemuck::cast_slice(&[uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.shader_uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("ShaderUniform Bind Group"),
        })
    }

    pub fn next_core_render_parts(&self) -> (&EngineWgpuRenderBackend, &wgpu::RenderPipeline) {
        (
            &self.next_core_render_backend,
            &self.next_core_render_pipeline,
        )
    }

    #[allow(dead_code)]
    pub fn remove_next_core_glyph_atlas_pane(&self, pane_id: usize) {
        self.next_core_glyph_atlases
            .borrow_mut()
            .remove_pane(pane_id);
    }

    #[allow(dead_code)]
    pub fn encode_next_core_upload(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        upload: &EngineRenderGpuUploadPlan,
        clear_color: Option<[f64; 4]>,
    ) -> bool {
        let Some(buffers) = self.next_core_render_backend.upload(&self.device, upload) else {
            return false;
        };
        let pass = self
            .next_core_render_backend
            .prepare_pass(upload, clear_color);
        self.next_core_render_backend.encode_pass(
            encoder,
            target,
            &self.next_core_render_pipeline,
            &buffers,
            &pass,
        )
    }

    #[allow(dead_code)]
    pub fn encode_next_core_textured_glyph_upload(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        upload: &EngineRenderTexturedGlyphUploadPlan,
    ) -> bool {
        let Some(buffers) = self
            .next_core_render_backend
            .upload_textured_glyphs(&self.device, upload)
        else {
            return false;
        };
        let pass = self
            .next_core_render_backend
            .prepare_textured_glyph_pass(upload);
        self.next_core_render_backend.encode_textured_glyph_pass(
            encoder,
            target,
            &self.next_core_textured_glyph_pipeline,
            &self.next_core_glyph_texture_bind_group,
            &buffers,
            &pass,
        )
    }

    fn next_core_viewport_pixels(&self) -> (f32, f32) {
        let dimensions = self.dimensions.borrow();
        (
            dimensions.pixel_width.max(1) as f32,
            dimensions.pixel_height.max(1) as f32,
        )
    }

    pub fn prepare_next_core_pane_frame(
        &self,
        batch: EngineRenderBufferBatch,
        font: Option<Rc<LoadedFont>>,
        replace_requested: bool,
    ) -> NextCoreWebGpuPaneDrawFrame {
        let (viewport_width_px, viewport_height_px) = self.next_core_viewport_pixels();
        let prepared = EngineWgpuRenderBackend::prepare_frame_for_viewport(
            &batch.buffer_plan,
            viewport_width_px,
            viewport_height_px,
        );
        let cached_glyph_upload = replace_requested
            .then(|| {
                self.next_core_cached_glyph_upload_diagnostics_for_prepared(
                    &batch.buffer_plan,
                    &prepared,
                    viewport_width_px,
                    viewport_height_px,
                    font.clone(),
                )
            })
            .flatten();

        let engine_frame = EngineRenderPreparedPaneFrame::from_parts(
            batch,
            prepared,
            replace_requested,
            cached_glyph_upload.as_ref(),
        );

        NextCoreWebGpuPaneDrawFrame { engine_frame, font }
    }

    fn next_core_cached_glyph_upload_diagnostics_for_prepared(
        &self,
        plan: &EngineRenderBufferPlan,
        prepared: &EngineWgpuPreparedFramePlan,
        viewport_width_px: f32,
        viewport_height_px: f32,
        font: Option<Rc<LoadedFont>>,
    ) -> Option<EngineRenderCachedGlyphUploadDiagnostics> {
        let mut glyph_state = self.next_core_glyph_atlases.borrow().clone();
        let shaped_glyph_atlas = font.as_ref().and_then(|font| {
            glyph_state.prepare_shaped_glyph_atlas_with_shaper(
                &prepared.text_atlas,
                font.id(),
                |text| {
                    font.shape(
                        text,
                        || {},
                        |_| {},
                        None,
                        Direction::LeftToRight,
                        None,
                        None,
                    )
                    .ok()
                    .map(shaper_glyphs_from_glyph_infos)
                },
            )
        });
        let font_raster_source = font.map(NextCoreFontGlyphRasterSource::new);
        let glyph_upload = if let (Some(shaped_glyph_atlas), Some(font_raster_source)) =
            (shaped_glyph_atlas.as_ref(), font_raster_source.as_ref())
        {
            glyph_state.prepare_cached_upload_with_raster_source(
                plan,
                shaped_glyph_atlas,
                viewport_width_px,
                viewport_height_px,
                font_raster_source,
            )
        } else {
            glyph_state.prepare_cached_upload(
                plan,
                &prepared.glyph_atlas,
                viewport_width_px,
                viewport_height_px,
            )
        };
        glyph_upload.map(|upload| upload.diagnostics())
    }

    #[allow(dead_code)]
    pub fn encode_next_core_pane_frame(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        frame: NextCoreWebGpuPaneDrawFrame,
        clear_color: Option<[f64; 4]>,
    ) -> bool {
        let (viewport_width_px, viewport_height_px) = self.next_core_viewport_pixels();
        self.encode_prepared_next_core_pane_frame_with_font(
            encoder,
            target,
            &frame.engine_frame.batch.buffer_plan,
            &frame.engine_frame.prepared,
            viewport_width_px,
            viewport_height_px,
            clear_color,
            frame.font,
        )
    }

    fn encode_prepared_next_core_pane_frame_with_font(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        plan: &EngineRenderBufferPlan,
        prepared: &EngineWgpuPreparedFramePlan,
        viewport_width_px: f32,
        viewport_height_px: f32,
        clear_color: Option<[f64; 4]>,
        font: Option<Rc<LoadedFont>>,
    ) -> bool {
        let frame_diagnostics = prepared.diagnostics();
        let frame_readiness_issue_count = prepared.readiness_issues().len();
        log::trace!(
            "next-core prepared frame pane={} revision={} submitted={} solid_vertices={} solid_indices={} text_runs={} glyph_keys={} glyph_instances={} replace_ready={} readiness_issues={}",
            frame_diagnostics.pane_id,
            frame_diagnostics.revision,
            frame_diagnostics.submitted,
            frame_diagnostics.solid_vertex_count,
            frame_diagnostics.solid_index_count,
            frame_diagnostics.text_run_count,
            frame_diagnostics.glyph_key_count,
            frame_diagnostics.glyph_instance_count,
            frame_diagnostics.replace_ready,
            frame_readiness_issue_count
        );
        if !prepared.text_atlas.is_empty() {
            log::trace!(
                "next-core prepared {} text atlas runs and {} glyph atlas instances for pane {} revision {}",
                prepared.text_atlas.runs.len(),
                prepared.glyph_atlas.instances.len(),
                prepared.text_atlas.pane_id,
                prepared.text_atlas.revision
            );
        }
        let mut textured_glyph_upload = None;
        let shaped_glyph_atlas = font.as_ref().and_then(|font| {
            self.next_core_glyph_atlases
                .borrow_mut()
                .prepare_shaped_glyph_atlas_with_shaper(&prepared.text_atlas, font.id(), |text| {
                    match font.shape(
                        text,
                        || {},
                        |_| {},
                        None,
                        Direction::LeftToRight,
                        None,
                        None,
                    ) {
                        Ok(glyph_infos) => Some(shaper_glyphs_from_glyph_infos(glyph_infos)),
                        Err(err) => {
                            log::debug!("next-core font shaping skipped for {text:?}: {err:#}");
                            None
                        }
                    }
                })
        });
        let font_raster_source = font.map(NextCoreFontGlyphRasterSource::new);
        let glyph_upload = if let (Some(shaped_glyph_atlas), Some(font_raster_source)) =
            (shaped_glyph_atlas.as_ref(), font_raster_source.as_ref())
        {
            self.next_core_glyph_atlases
                .borrow_mut()
                .prepare_cached_upload_with_raster_source(
                    plan,
                    shaped_glyph_atlas,
                    viewport_width_px,
                    viewport_height_px,
                    font_raster_source,
                )
        } else {
            self.next_core_glyph_atlases
                .borrow_mut()
                .prepare_cached_upload(
                    plan,
                    &prepared.glyph_atlas,
                    viewport_width_px,
                    viewport_height_px,
                )
        };
        if let Some(glyph_upload) = glyph_upload {
            if !glyph_upload.texture_update.missing_key_indices.is_empty() {
                log::debug!(
                    "next-core glyph texture update missing {} keys for pane {} revision {}",
                    glyph_upload.texture_update.missing_key_indices.len(),
                    glyph_upload.pane_id,
                    glyph_upload.revision
                );
            }
            if font_raster_source.is_some() {
                log::trace!(
                    "next-core font raster source active for pane {} revision {}",
                    glyph_upload.pane_id,
                    glyph_upload.revision
                );
            }
            let texture_stats = match self
                .next_core_glyph_texture
                .upload_update(&glyph_upload.texture_update)
            {
                Ok(stats) => stats,
                Err(err) => {
                    log::warn!("next-core glyph texture upload failed: {err:#}");
                    NextCoreGlyphTextureUploadStats::default()
                }
            };
            let diagnostics = glyph_upload.diagnostics();
            let readiness_issue_count = diagnostics.readiness_issues().len();
            log::trace!(
                "next-core cached glyph atlas pane={} revision={} inserted={} overflow={} texture_regions={} texture_bytes={} layout_entries={} layout_missing={} vertices={} indices={} draw_ready={} readiness_issues={}",
                diagnostics.pane_id,
                diagnostics.revision,
                diagnostics.inserted_key_count,
                diagnostics.overflow_key_count,
                texture_stats.region_count,
                texture_stats.byte_count,
                diagnostics.layout_entry_count,
                diagnostics.layout_missing_key_count,
                diagnostics.vertex_count,
                diagnostics.index_count,
                diagnostics.draw_ready,
                readiness_issue_count
            );
            textured_glyph_upload = Some(glyph_upload.upload);
        }
        let encoded_solid =
            self.encode_next_core_upload(encoder, target, &prepared.upload, clear_color);
        let encoded_glyphs = textured_glyph_upload.as_ref().is_some_and(|upload| {
            self.encode_next_core_textured_glyph_upload(encoder, target, upload)
        });
        encoded_solid || encoded_glyphs
    }

    #[allow(unused_mut)]
    pub fn resize(&self, mut dims: Dimensions) {
        // During a live resize on Windows, the Dimensions that we're processing may be
        // lagging behind the true client size. We have to take the very latest value
        // from the window or else the underlying driver will raise an error about
        // the mismatch, so we need to sneakily read through the handle
        match self.handle.window {
            #[cfg(windows)]
            RawWindowHandle::Win32(h) => {
                let mut rect = unsafe { std::mem::zeroed() };
                unsafe { winapi::um::winuser::GetClientRect(h.hwnd.get() as _, &mut rect) };
                dims.pixel_width = (rect.right - rect.left) as usize;
                dims.pixel_height = (rect.bottom - rect.top) as usize;
            }
            _ => {}
        }

        if dims == *self.dimensions.borrow() {
            return;
        }
        *self.dimensions.borrow_mut() = dims;
        let mut config = self.config.borrow_mut();
        config.width = dims.pixel_width as u32;
        config.height = dims.pixel_height as u32;
        if config.width > 0 && config.height > 0 {
            // Avoid reconfiguring with a 0 sized surface, as webgpu will
            // panic in that case
            // <https://github.com/wezterm/wezterm/issues/2881>
            self.surface.configure(&self.device, &config);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{CellStyle, RenderRect, RenderTextRun};

    #[test]
    fn next_core_glyph_atlas_state_reuses_cached_placements() {
        let mut state = NextCoreGlyphAtlasState::new();
        let plan = buffer_plan_with_text(7, 1, "aba", 3, 24, 16);
        let glyphs = EngineWgpuRenderBackend::prepare_glyph_atlas(&plan);

        let first = state
            .prepare_cached_upload(&plan, &glyphs, 80.0, 40.0)
            .expect("first upload");
        assert_eq!(state.len(), 1);
        assert_eq!(first.update.inserted_key_indices, vec![0, 1]);
        assert!(first.update.overflow_key_indices.is_empty());
        assert_eq!(first.texture_update.regions.len(), 2);
        assert_eq!(first.upload.vertices.len(), 12);

        let repeat = state
            .prepare_cached_upload(&plan, &glyphs, 80.0, 40.0)
            .expect("repeat upload");
        assert!(repeat.update.inserted_key_indices.is_empty());
        assert!(repeat.update.overflow_key_indices.is_empty());
        assert!(repeat.texture_update.is_empty());
        assert_eq!(repeat.update.placements, first.update.placements);
    }

    #[test]
    fn next_core_cached_upload_reports_clean_layout_parity_for_repeat_upload() {
        let mut state = NextCoreGlyphAtlasState::new();
        let plan = buffer_plan_with_text(12, 1, "ab", 2, 16, 16);
        let glyphs = EngineWgpuRenderBackend::prepare_glyph_atlas(&plan);

        let first = state
            .prepare_cached_upload(&plan, &glyphs, 80.0, 40.0)
            .expect("first upload");
        let repeat = state
            .prepare_cached_upload(&plan, &glyphs, 80.0, 40.0)
            .expect("repeat upload");

        assert!(first.has_clean_layout_parity_with(&repeat));
        assert!(repeat.update.inserted_key_indices.is_empty());
    }

    #[test]
    fn next_core_cached_upload_diagnostics_include_layout_readiness() {
        let mut state = NextCoreGlyphAtlasState::new();
        let plan = buffer_plan_with_text(14, 1, "ab", 2, 16, 16);
        let glyphs = EngineWgpuRenderBackend::prepare_glyph_atlas(&plan);

        let upload = state
            .prepare_cached_upload(&plan, &glyphs, 80.0, 40.0)
            .expect("upload");
        let diagnostics = upload.diagnostics();

        assert_eq!(
            diagnostics,
            EngineRenderCachedGlyphUploadDiagnostics {
                pane_id: 14,
                submitted: true,
                revision: 1,
                cell_width_px: 8,
                cell_height_px: 16,
                inserted_key_count: 2,
                overflow_key_count: 0,
                texture_region_count: 2,
                texture_missing_key_count: 0,
                layout_entry_count: 2,
                layout_missing_key_count: 0,
                vertex_count: 8,
                index_count: 12,
                draw_ready: true,
            }
        );
        assert!(diagnostics.is_ready());
        assert!(diagnostics.readiness_issues().is_empty());
    }

    #[test]
    fn next_core_cached_upload_diagnostics_report_readiness_issues() {
        let mut state = NextCoreGlyphAtlasState::new();
        let plan = buffer_plan_with_text(15, 1, "ab", 2, 16, 16);
        let glyphs = EngineWgpuRenderBackend::prepare_glyph_atlas(&plan);

        let mut upload = state
            .prepare_cached_upload(&plan, &glyphs, 80.0, 40.0)
            .expect("upload");
        upload.update.overflow_key_indices.push(99);
        upload.texture_update.missing_key_indices.push(4);
        upload.upload.missing_key_indices.push(4);
        upload.upload.layout.missing_key_indices.push(4);
        upload.upload.submitted = false;
        upload.upload.vertices.clear();

        let diagnostics = upload.diagnostics();

        assert!(!diagnostics.is_ready());
        assert_eq!(
            diagnostics.readiness_issues(),
            vec![
                crate::engine::EngineRenderCachedGlyphUploadReadinessIssue::NotSubmitted,
                crate::engine::EngineRenderCachedGlyphUploadReadinessIssue::EmptyUpload,
                crate::engine::EngineRenderCachedGlyphUploadReadinessIssue::OverflowKeys,
                crate::engine::EngineRenderCachedGlyphUploadReadinessIssue::TextureMissingKeys,
                crate::engine::EngineRenderCachedGlyphUploadReadinessIssue::LayoutMissingKeys,
                crate::engine::EngineRenderCachedGlyphUploadReadinessIssue::NotDrawReady,
            ]
        );
    }

    #[test]
    fn next_core_cached_upload_layout_parity_reports_drift() {
        let mut state = NextCoreGlyphAtlasState::new();
        let expected_plan = buffer_plan_with_text(13, 1, "ab", 2, 16, 16);
        let mut actual_plan = buffer_plan_with_text(13, 2, "ab", 2, 16, 16);
        actual_plan.text_runs[0].col = 1;
        actual_plan.text_runs[0].rect.x = 8;
        let expected_glyphs = EngineWgpuRenderBackend::prepare_glyph_atlas(&expected_plan);
        let actual_glyphs = EngineWgpuRenderBackend::prepare_glyph_atlas(&actual_plan);

        let expected = state
            .prepare_cached_upload(&expected_plan, &expected_glyphs, 80.0, 40.0)
            .expect("expected upload");
        let actual = state
            .prepare_cached_upload(&actual_plan, &actual_glyphs, 80.0, 40.0)
            .expect("actual upload");
        let diff = expected.diff_layout_against(&actual);

        assert!(!diff.is_clean());
        assert_eq!(diff.expected_revision, 1);
        assert_eq!(diff.actual_revision, 2);
        assert_eq!(diff.missing_entries.len(), 2);
        assert_eq!(diff.unexpected_entries.len(), 2);
    }

    #[test]
    fn next_core_glyph_atlas_state_resets_when_cell_metrics_change() {
        let mut state = NextCoreGlyphAtlasState::new();
        let first_plan = buffer_plan_with_text(8, 1, "ab", 2, 16, 16);
        let resized_plan = buffer_plan_with_text(8, 2, "ab", 2, 20, 20);
        let first_glyphs = EngineWgpuRenderBackend::prepare_glyph_atlas(&first_plan);
        let resized_glyphs = EngineWgpuRenderBackend::prepare_glyph_atlas(&resized_plan);

        let first = state
            .prepare_cached_upload(&first_plan, &first_glyphs, 80.0, 40.0)
            .expect("first upload");
        let resized = state
            .prepare_cached_upload(&resized_plan, &resized_glyphs, 80.0, 40.0)
            .expect("resized upload");

        assert_eq!(state.len(), 1);
        assert_eq!(resized.cell_width_px, 10);
        assert_eq!(resized.cell_height_px, 20);
        assert_eq!(resized.update.inserted_key_indices, vec![0, 1]);
        assert_eq!(resized.texture_update.regions.len(), 2);
        assert_ne!(resized.update.placements, first.update.placements);
    }

    #[test]
    fn next_core_glyph_atlas_state_removes_pane_cache() {
        let mut state = NextCoreGlyphAtlasState::new();
        let plan = buffer_plan_with_text(9, 1, "x", 1, 8, 16);
        let glyphs = EngineWgpuRenderBackend::prepare_glyph_atlas(&plan);

        assert!(state
            .prepare_cached_upload(&plan, &glyphs, 80.0, 40.0)
            .is_some());
        assert_eq!(state.len(), 1);
        state.remove_pane(9);
        assert!(state.is_empty());
    }

    #[test]
    fn next_core_glyph_atlas_state_uses_external_raster_source() {
        struct SolidRasterSource;

        impl EngineRenderGlyphRasterSource for SolidRasterSource {
            fn rasterize_glyph_rgba(
                &self,
                _key: &crate::engine::EngineRenderGlyphAtlasKey,
                width_px: usize,
                height_px: usize,
            ) -> Option<Vec<u8>> {
                let mut bytes = Vec::with_capacity(width_px * height_px * 4);
                for _ in 0..width_px * height_px {
                    bytes.extend_from_slice(&[0x7a, 0x7b, 0x7c, 0x7d]);
                }
                Some(bytes)
            }
        }

        let mut state = NextCoreGlyphAtlasState::new();
        let plan = buffer_plan_with_text(10, 1, "q", 1, 8, 16);
        let glyphs = EngineWgpuRenderBackend::prepare_glyph_atlas(&plan);

        let upload = state
            .prepare_cached_upload_with_raster_source(
                &plan,
                &glyphs,
                80.0,
                40.0,
                &SolidRasterSource,
            )
            .expect("upload");

        assert_eq!(upload.texture_update.regions.len(), 1);
        assert_eq!(
            &upload.texture_update.regions[0].bytes_rgba[0..4],
            &[0x7a, 0x7b, 0x7c, 0x7d]
        );
    }

    #[test]
    fn next_core_shaped_glyph_atlas_reuses_cached_shape_for_same_font_revision() {
        let mut state = NextCoreGlyphAtlasState::new();
        let plan = buffer_plan_with_text(11, 3, "q", 1, 8, 16);
        let text_atlas = EngineWgpuRenderBackend::prepare_text_atlas(&plan);
        let mut shape_calls = 0usize;

        let first = state
            .prepare_shaped_glyph_atlas_with_shaper(&text_atlas, 700, |text| {
                shape_calls += 1;
                Some(vec![glyph_info(text, 0, 44, 8.0, 1)])
            })
            .expect("first shaped atlas");
        let repeat = state
            .prepare_shaped_glyph_atlas_with_shaper(&text_atlas, 700, |text| {
                shape_calls += 1;
                Some(vec![glyph_info(text, 0, 45, 8.0, 1)])
            })
            .expect("cached shaped atlas");
        let different_font = state
            .prepare_shaped_glyph_atlas_with_shaper(&text_atlas, 701, |text| {
                shape_calls += 1;
                Some(vec![glyph_info(text, 0, 46, 8.0, 1)])
            })
            .expect("reshaped atlas");

        assert_eq!(shape_calls, 2);
        assert_eq!(repeat, first);
        assert_ne!(different_font, first);
        assert_eq!(different_font.keys[0].raster_identity(), Some((0, 46)));
    }

    #[test]
    fn next_core_glyph_texture_region_validation_reports_stats() {
        let region = texture_region(0, 1, 2, 3, 4, 48);

        let stats =
            validate_next_core_glyph_texture_regions(16, 16, &[region]).expect("validate region");

        assert_eq!(
            stats,
            NextCoreGlyphTextureUploadStats {
                region_count: 1,
                byte_count: 48,
            }
        );
    }

    #[test]
    fn next_core_glyph_texture_region_validation_rejects_bad_byte_count() {
        let region = texture_region(0, 0, 0, 2, 2, 12);

        let err = validate_next_core_glyph_texture_regions(16, 16, &[region])
            .expect_err("bad byte count should fail");

        assert!(err.to_string().contains("expected 16"));
    }

    #[test]
    fn next_core_glyph_texture_region_validation_rejects_overflow() {
        let region = texture_region(0, 14, 14, 4, 4, 64);

        let err = validate_next_core_glyph_texture_regions(16, 16, &[region])
            .expect_err("overflow should fail");

        assert!(err.to_string().contains("overflows atlas"));
    }

    fn buffer_plan_with_text(
        pane_id: usize,
        revision: u64,
        text: &str,
        cells: usize,
        width: usize,
        height: usize,
    ) -> EngineRenderBufferPlan {
        EngineRenderBufferPlan {
            pane_id,
            submitted: true,
            revision,
            requires_full_repaint: true,
            damage_rects: Vec::new(),
            text_runs: vec![RenderTextRun {
                row: 0,
                col: 0,
                cells,
                text: text.to_string(),
                rect: RenderRect {
                    x: 0,
                    y: 0,
                    width,
                    height,
                },
                style: CellStyle::default(),
            }],
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    fn texture_region(
        key_index: usize,
        x: usize,
        y: usize,
        width_px: usize,
        height_px: usize,
        byte_count: usize,
    ) -> EngineRenderGlyphAtlasTextureRegion {
        EngineRenderGlyphAtlasTextureRegion {
            key_index,
            rect: RenderRect {
                x,
                y,
                width: width_px,
                height: height_px,
            },
            width_px,
            height_px,
            source_width_px: width_px,
            source_height_px: height_px,
            bearing_x_px: 0,
            bearing_y_px: 0,
            uses_raster_metrics: false,
            bytes_rgba: vec![0xff; byte_count],
        }
    }

    fn glyph_info(
        text: &str,
        font_idx: usize,
        glyph_pos: u32,
        x_advance: f64,
        num_cells: u8,
    ) -> EngineRenderShaperGlyph {
        EngineRenderShaperGlyph {
            text: text.to_string(),
            only_char: text.chars().next(),
            num_cells,
            font_idx,
            glyph_pos,
            x_advance_px: x_advance,
            x_offset_px: 0.0,
            y_offset_px: 0.0,
        }
    }
}
