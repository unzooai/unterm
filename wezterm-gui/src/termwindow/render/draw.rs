use crate::colorease::ColorEaseUniform;
use crate::termwindow::webgpu::ShaderUniform;
use crate::termwindow::RenderFrame;
use crate::uniforms::UniformBuilder;
use ::window::glium;
use ::window::glium::uniforms::{
    MagnifySamplerFilter, MinifySamplerFilter, Sampler, SamplerWrapFunction,
};
use ::window::glium::{BlendingFunction, LinearBlendingFactor, Surface};
use config::FreeTypeLoadTarget;

const INDICES_PER_QUAD: usize = 6;

impl crate::TermWindow {
    pub fn call_draw(&mut self, frame: &mut RenderFrame) -> anyhow::Result<()> {
        match frame {
            RenderFrame::Glium(ref mut frame) => self.call_draw_glium(frame),
            RenderFrame::WebGpu => self.call_draw_webgpu(),
        }
    }

    fn call_draw_webgpu(&mut self) -> anyhow::Result<()> {
        use crate::termwindow::webgpu::WebGpuTexture;

        let webgpu = self.webgpu.as_ref().unwrap().clone();

        let output = webgpu.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = webgpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let next_core_mode = next_core_webgpu_pane_mode();
        let next_core_pane_frame = if next_core_mode.is_enabled() {
            if let Some(pane) = self.get_active_pane_no_overlay() {
                match self.prepare_next_core_webgpu_pane_frame(
                    pane.pane_id(),
                    next_core_mode == NextCoreWebGpuPaneMode::Replace,
                ) {
                    Ok(frame) => Some(frame),
                    Err(err) => {
                        log::debug!("next-core WebGPU pane render skipped: {err:#}");
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };
        let next_core_replace_diagnostics = match (
            next_core_mode == NextCoreWebGpuPaneMode::Replace,
            next_core_pane_frame.as_ref(),
        ) {
            (true, Some(frame)) => Some(frame.replace_diagnostics),
            (true, None) => Some(webgpu.next_core_pane_replace_diagnostics(&None, None)),
            (false, _) => None,
        };
        let replace_legacy_pane =
            should_replace_legacy_pane(next_core_mode, next_core_replace_diagnostics.as_ref());
        {
            let render_state = self.render_state.as_ref().unwrap();
            let tex = render_state.glyph_cache.borrow().atlas.texture();
            let tex = tex.downcast_ref::<WebGpuTexture>().unwrap();
            let texture_view = tex.create_view(&wgpu::TextureViewDescriptor::default());

            let texture_linear_bind_group =
                webgpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &webgpu.texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(
                                &webgpu.texture_linear_sampler,
                            ),
                        },
                    ],
                    label: Some("linear bind group"),
                });

            let texture_nearest_bind_group =
                webgpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &webgpu.texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(
                                &webgpu.texture_nearest_sampler,
                            ),
                        },
                    ],
                    label: Some("nearest bind group"),
                });

            let mut cleared = false;
            let foreground_text_hsb = self.config.foreground_text_hsb;
            let foreground_text_hsb = [
                foreground_text_hsb.hue,
                foreground_text_hsb.saturation,
                foreground_text_hsb.brightness,
            ];

            let milliseconds = self.created.elapsed().as_millis() as u32;
            let projection = euclid::Transform3D::<f32, f32, f32>::ortho(
                -(self.dimensions.pixel_width as f32) / 2.0,
                self.dimensions.pixel_width as f32 / 2.0,
                self.dimensions.pixel_height as f32 / 2.0,
                -(self.dimensions.pixel_height as f32) / 2.0,
                -1.0,
                1.0,
            )
            .to_arrays_transposed();

            for layer in render_state.layers.borrow().iter() {
                for idx in 0..3 {
                    let vb = &layer.vb.borrow()[idx];
                    let (vertex_count, index_count) = vb.vertex_index_count();
                    let vertex_buffer;
                    let uniforms;
                    if vertex_count > 0 {
                        let mut vertices = vb.current_vb_mut();
                        let mut render_pass =
                            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("Render Pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: if cleared {
                                            wgpu::LoadOp::Load
                                        } else {
                                            wgpu::LoadOp::Clear(wgpu::Color {
                                                r: 0.,
                                                g: 0.,
                                                b: 0.,
                                                a: 1.,
                                            })
                                        },
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: None,
                                occlusion_query_set: None,
                                timestamp_writes: None,
                            });
                        cleared = true;

                        uniforms = webgpu.create_uniform(ShaderUniform {
                            foreground_text_hsb,
                            milliseconds,
                            projection,
                        });

                        render_pass.set_pipeline(&webgpu.render_pipeline);
                        render_pass.set_bind_group(0, &uniforms, &[]);
                        render_pass.set_bind_group(1, &texture_linear_bind_group, &[]);
                        render_pass.set_bind_group(2, &texture_nearest_bind_group, &[]);
                        vertex_buffer = vertices.webgpu_mut().recreate();
                        vertex_buffer.unmap();
                        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                        render_pass.set_index_buffer(
                            vb.indices.webgpu().slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        if replace_legacy_pane {
                            draw_non_pane_webgpu_ranges(
                                &mut render_pass,
                                index_count,
                                &layer.pane_quad_ranges(idx),
                            );
                        } else {
                            render_pass.draw_indexed(0..index_count as _, 0, 0..1);
                        }
                    }

                    vb.next_index();
                }
            }
        }

        if next_core_mode.is_enabled() {
            if let Some(frame) = next_core_pane_frame {
                let encoded = webgpu.encode_next_core_pane_frame(&mut encoder, &view, frame, None);
                if !encoded {
                    log::debug!("next-core WebGPU pane render skipped: empty buffer plan");
                }
            }
        }

        // submit will accept anything that implements IntoIter
        webgpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn call_draw_glium(&mut self, frame: &mut glium::Frame) -> anyhow::Result<()> {
        use window::glium::texture::SrgbTexture2d;

        let gl_state = self.render_state.as_ref().unwrap();
        let tex = gl_state.glyph_cache.borrow().atlas.texture();
        let tex = tex.downcast_ref::<SrgbTexture2d>().unwrap();

        frame.clear_color(0., 0., 0., 1.);

        let projection = euclid::Transform3D::<f32, f32, f32>::ortho(
            -(self.dimensions.pixel_width as f32) / 2.0,
            self.dimensions.pixel_width as f32 / 2.0,
            self.dimensions.pixel_height as f32 / 2.0,
            -(self.dimensions.pixel_height as f32) / 2.0,
            -1.0,
            1.0,
        )
        .to_arrays_transposed();

        let use_subpixel = match self
            .config
            .freetype_render_target
            .unwrap_or(self.config.freetype_load_target)
        {
            FreeTypeLoadTarget::HorizontalLcd | FreeTypeLoadTarget::VerticalLcd => true,
            _ => false,
        };

        let dual_source_blending = glium::DrawParameters {
            blend: glium::Blend {
                color: BlendingFunction::Addition {
                    source: LinearBlendingFactor::SourceOneColor,
                    destination: LinearBlendingFactor::OneMinusSourceOneColor,
                },
                alpha: BlendingFunction::Addition {
                    source: LinearBlendingFactor::SourceOneColor,
                    destination: LinearBlendingFactor::OneMinusSourceOneColor,
                },
                constant_value: (0.0, 0.0, 0.0, 0.0),
            },

            ..Default::default()
        };

        let alpha_blending = glium::DrawParameters {
            blend: glium::Blend {
                color: BlendingFunction::Addition {
                    source: LinearBlendingFactor::SourceAlpha,
                    destination: LinearBlendingFactor::OneMinusSourceAlpha,
                },
                alpha: BlendingFunction::Addition {
                    source: LinearBlendingFactor::One,
                    destination: LinearBlendingFactor::OneMinusSourceAlpha,
                },
                constant_value: (0.0, 0.0, 0.0, 0.0),
            },
            ..Default::default()
        };

        // Clamp and use the nearest texel rather than interpolate.
        // This prevents things like the box cursor outlines from
        // being randomly doubled in width or height
        let atlas_nearest_sampler = Sampler::new(&*tex)
            .wrap_function(SamplerWrapFunction::Clamp)
            .magnify_filter(MagnifySamplerFilter::Nearest)
            .minify_filter(MinifySamplerFilter::Nearest);

        let atlas_linear_sampler = Sampler::new(&*tex)
            .wrap_function(SamplerWrapFunction::Clamp)
            .magnify_filter(MagnifySamplerFilter::Linear)
            .minify_filter(MinifySamplerFilter::Linear);

        let foreground_text_hsb = self.config.foreground_text_hsb;
        let foreground_text_hsb = (
            foreground_text_hsb.hue,
            foreground_text_hsb.saturation,
            foreground_text_hsb.brightness,
        );

        let milliseconds = self.created.elapsed().as_millis() as u32;

        let cursor_blink: ColorEaseUniform = (*self.cursor_blink_state.borrow()).into();
        let blink: ColorEaseUniform = (*self.blink_state.borrow()).into();
        let rapid_blink: ColorEaseUniform = (*self.rapid_blink_state.borrow()).into();

        for layer in gl_state.layers.borrow().iter() {
            for idx in 0..3 {
                let vb = &layer.vb.borrow()[idx];
                let (vertex_count, index_count) = vb.vertex_index_count();
                if vertex_count > 0 {
                    let vertices = vb.current_vb_mut();
                    let subpixel_aa = use_subpixel && idx == 1;

                    let mut uniforms = UniformBuilder::default();

                    uniforms.add("projection", &projection);
                    uniforms.add("atlas_nearest_sampler", &atlas_nearest_sampler);
                    uniforms.add("atlas_linear_sampler", &atlas_linear_sampler);
                    uniforms.add("foreground_text_hsb", &foreground_text_hsb);
                    uniforms.add("subpixel_aa", &subpixel_aa);
                    uniforms.add("milliseconds", &milliseconds);
                    uniforms.add_struct("cursor_blink", &cursor_blink);
                    uniforms.add_struct("blink", &blink);
                    uniforms.add_struct("rapid_blink", &rapid_blink);

                    frame.draw(
                        vertices.glium().slice(0..vertex_count).unwrap(),
                        vb.indices.glium().slice(0..index_count).unwrap(),
                        gl_state.glyph_prog.as_ref().unwrap(),
                        &uniforms,
                        if subpixel_aa {
                            &dual_source_blending
                        } else {
                            &alpha_blending
                        },
                    )?;
                }

                vb.next_index();
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NextCoreWebGpuPaneMode {
    Disabled,
    Append,
    Replace,
}

impl NextCoreWebGpuPaneMode {
    fn is_enabled(self) -> bool {
        self != Self::Disabled
    }
}

fn next_core_webgpu_pane_mode() -> NextCoreWebGpuPaneMode {
    let Some(raw) = std::env::var("UNTERM_NEXT_CORE_WEBGPU_PANE").ok() else {
        return NextCoreWebGpuPaneMode::Disabled;
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "replace" | "replace-pane" | "exclusive" => NextCoreWebGpuPaneMode::Replace,
        "1" | "true" | "yes" | "on" | "append" => NextCoreWebGpuPaneMode::Append,
        _ => NextCoreWebGpuPaneMode::Disabled,
    }
}

fn should_replace_legacy_pane(
    mode: NextCoreWebGpuPaneMode,
    diagnostics: Option<&crate::engine::EngineRenderPaneReplaceDiagnostics>,
) -> bool {
    let Some(diagnostics) = diagnostics else {
        return false;
    };
    if !diagnostics.replace_ready && mode == NextCoreWebGpuPaneMode::Replace {
        log::trace!(
            "next-core WebGPU replace fallback pane={:?} revision={:?} issues={} batch_ready={} batch_readiness_issues={} prepared_frame_ready={} prepared_frame_matches_batch={} prepared_frame_readiness_issues={} cached_glyph_required={} cached_glyph_ready={} cached_glyph_matches_batch={} cached_glyph_readiness_issues={}",
            diagnostics.pane_id,
            diagnostics.revision,
            diagnostics.readiness_issues().len(),
            diagnostics.batch_ready,
            diagnostics.batch_readiness_issue_count,
            diagnostics.prepared_frame_ready,
            diagnostics.prepared_frame_matches_batch,
            diagnostics.prepared_frame_readiness_issue_count,
            diagnostics.cached_glyph_upload_required,
            diagnostics.cached_glyph_upload_ready,
            diagnostics.cached_glyph_upload_matches_batch,
            diagnostics.cached_glyph_upload_readiness_issue_count
        );
    }
    diagnostics.replace_ready
}

#[cfg(test)]
fn should_replace_legacy_pane_from_parts(
    mode: NextCoreWebGpuPaneMode,
    batch: &Option<crate::engine::EngineRenderBufferBatch>,
    prepared_frame: Option<&crate::engine::render_backend::EngineWgpuPreparedFrameDiagnostics>,
    cached_glyph_upload: Option<&crate::engine::EngineRenderCachedGlyphUploadDiagnostics>,
) -> bool {
    let diagnostics =
        next_core_webgpu_replace_diagnostics(mode, batch, prepared_frame, cached_glyph_upload);
    should_replace_legacy_pane(mode, Some(&diagnostics))
}

#[cfg(test)]
fn next_core_webgpu_replace_diagnostics(
    mode: NextCoreWebGpuPaneMode,
    batch: &Option<crate::engine::EngineRenderBufferBatch>,
    prepared_frame: Option<&crate::engine::render_backend::EngineWgpuPreparedFrameDiagnostics>,
    cached_glyph_upload: Option<&crate::engine::EngineRenderCachedGlyphUploadDiagnostics>,
) -> crate::engine::EngineRenderPaneReplaceDiagnostics {
    crate::engine::EngineRenderPaneReplaceDiagnostics::from_parts(
        mode == NextCoreWebGpuPaneMode::Replace,
        batch.as_ref(),
        prepared_frame,
        cached_glyph_upload,
    )
}

fn draw_non_pane_webgpu_ranges(
    render_pass: &mut wgpu::RenderPass<'_>,
    index_count: usize,
    pane_quad_ranges: &[std::ops::Range<usize>],
) {
    let total_quads = index_count / INDICES_PER_QUAD;
    for range in non_pane_quad_ranges(total_quads, pane_quad_ranges) {
        draw_quad_range(render_pass, range.start, range.end);
    }
}

fn non_pane_quad_ranges(
    total_quads: usize,
    pane_quad_ranges: &[std::ops::Range<usize>],
) -> Vec<std::ops::Range<usize>> {
    let mut ranges = vec![];
    let mut next_quad = 0;

    for range in pane_quad_ranges {
        let skip_start = range.start.min(total_quads);
        let skip_end = range.end.min(total_quads);
        if next_quad < skip_start {
            ranges.push(next_quad..skip_start);
        }
        next_quad = next_quad.max(skip_end);
    }

    if next_quad < total_quads {
        ranges.push(next_quad..total_quads);
    }

    ranges
}

fn draw_quad_range(render_pass: &mut wgpu::RenderPass<'_>, start_quad: usize, end_quad: usize) {
    let start_index = (start_quad * INDICES_PER_QUAD) as u32;
    let end_index = (end_quad * INDICES_PER_QUAD) as u32;
    if start_index < end_index {
        render_pass.draw_indexed(start_index..end_index, 0, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        next_core_webgpu_replace_diagnostics, non_pane_quad_ranges,
        should_replace_legacy_pane_from_parts, NextCoreWebGpuPaneMode,
    };
    use crate::engine::render_backend::{EngineRenderVertex, EngineWgpuPreparedFrameDiagnostics};
    use crate::engine::EngineRenderVertexLayer;
    use crate::engine::{
        EngineRenderBufferBatch, EngineRenderBufferPlan, EngineRenderCachedGlyphUploadDiagnostics,
        EngineRenderCommitStats, EngineRenderPaneReplaceReadinessIssue,
    };

    #[test]
    fn non_pane_quad_ranges_skip_middle() {
        assert_eq!(non_pane_quad_ranges(10, &[3..7]), vec![0..3, 7..10]);
    }

    #[test]
    fn non_pane_quad_ranges_clips_and_merges_overlaps() {
        assert_eq!(non_pane_quad_ranges(10, &[0..3, 2..6, 8..20]), vec![6..8]);
    }

    #[test]
    fn non_pane_quad_ranges_empty_when_all_pane() {
        assert!(non_pane_quad_ranges(5, &[0..5]).is_empty());
    }

    #[test]
    fn next_core_replace_requires_draw_ready_batch() {
        assert!(should_replace_legacy_pane_from_parts(
            NextCoreWebGpuPaneMode::Replace,
            &Some(buffer_batch(true)),
            Some(&prepared_frame_diagnostics(true)),
            None
        ));
        let diagnostics = next_core_webgpu_replace_diagnostics(
            NextCoreWebGpuPaneMode::Replace,
            &Some(buffer_batch(true)),
            Some(&prepared_frame_diagnostics(true)),
            None,
        );
        assert!(diagnostics.replace_ready);
        assert!(diagnostics.readiness_issues().is_empty());
    }

    #[test]
    fn next_core_replace_keeps_legacy_pane_for_empty_repeat_batch() {
        assert!(!should_replace_legacy_pane_from_parts(
            NextCoreWebGpuPaneMode::Replace,
            &Some(buffer_batch(false)),
            Some(&prepared_frame_diagnostics(true)),
            None
        ));
        assert!(!should_replace_legacy_pane_from_parts(
            NextCoreWebGpuPaneMode::Append,
            &Some(buffer_batch(true)),
            Some(&prepared_frame_diagnostics(true)),
            None
        ));
        assert!(!should_replace_legacy_pane_from_parts(
            NextCoreWebGpuPaneMode::Replace,
            &None,
            None,
            None
        ));
        let append_diagnostics = next_core_webgpu_replace_diagnostics(
            NextCoreWebGpuPaneMode::Append,
            &Some(buffer_batch(true)),
            Some(&prepared_frame_diagnostics(true)),
            None,
        );
        assert_eq!(
            append_diagnostics.readiness_issues(),
            vec![EngineRenderPaneReplaceReadinessIssue::NotRequested]
        );
        let missing_diagnostics = next_core_webgpu_replace_diagnostics(
            NextCoreWebGpuPaneMode::Replace,
            &None,
            None,
            None,
        );
        assert_eq!(
            missing_diagnostics.readiness_issues(),
            vec![
                EngineRenderPaneReplaceReadinessIssue::MissingBufferBatch,
                EngineRenderPaneReplaceReadinessIssue::MissingPreparedFrame,
            ]
        );
    }

    #[test]
    fn next_core_replace_keeps_legacy_pane_when_prepared_frame_is_not_ready() {
        assert!(!should_replace_legacy_pane_from_parts(
            NextCoreWebGpuPaneMode::Replace,
            &Some(buffer_batch(true)),
            Some(&prepared_frame_diagnostics(false)),
            None
        ));
        assert!(!should_replace_legacy_pane_from_parts(
            NextCoreWebGpuPaneMode::Replace,
            &Some(buffer_batch(true)),
            None,
            None
        ));
        let diagnostics = next_core_webgpu_replace_diagnostics(
            NextCoreWebGpuPaneMode::Replace,
            &Some(buffer_batch(true)),
            Some(&prepared_frame_diagnostics(false)),
            None,
        );
        assert_eq!(
            diagnostics.readiness_issues(),
            vec![EngineRenderPaneReplaceReadinessIssue::PreparedFrameNotReady]
        );
        let missing_frame = next_core_webgpu_replace_diagnostics(
            NextCoreWebGpuPaneMode::Replace,
            &Some(buffer_batch(true)),
            None,
            None,
        );
        assert_eq!(
            missing_frame.readiness_issues(),
            vec![EngineRenderPaneReplaceReadinessIssue::MissingPreparedFrame]
        );
    }

    #[test]
    fn next_core_replace_requires_matching_batch_diagnostics() {
        assert!(!should_replace_legacy_pane_from_parts(
            NextCoreWebGpuPaneMode::Replace,
            &Some(buffer_batch(true)),
            Some(&prepared_frame_diagnostics_for_revision(true, 4)),
            None
        ));
        let stale_frame = next_core_webgpu_replace_diagnostics(
            NextCoreWebGpuPaneMode::Replace,
            &Some(buffer_batch(true)),
            Some(&prepared_frame_diagnostics_for_revision(true, 4)),
            None,
        );
        assert_eq!(
            stale_frame.readiness_issues(),
            vec![EngineRenderPaneReplaceReadinessIssue::PreparedFrameBatchMismatch]
        );

        assert!(!should_replace_legacy_pane_from_parts(
            NextCoreWebGpuPaneMode::Replace,
            &Some(buffer_batch(true)),
            Some(&prepared_text_frame_diagnostics(true)),
            Some(&cached_glyph_upload_diagnostics_for_revision(true, 4))
        ));
        let stale_glyph_upload = next_core_webgpu_replace_diagnostics(
            NextCoreWebGpuPaneMode::Replace,
            &Some(buffer_batch(true)),
            Some(&prepared_text_frame_diagnostics(true)),
            Some(&cached_glyph_upload_diagnostics_for_revision(true, 4)),
        );
        assert_eq!(
            stale_glyph_upload.readiness_issues(),
            vec![EngineRenderPaneReplaceReadinessIssue::CachedGlyphUploadBatchMismatch]
        );
    }

    #[test]
    fn next_core_replace_requires_cached_glyph_upload_for_text_frames() {
        assert!(!should_replace_legacy_pane_from_parts(
            NextCoreWebGpuPaneMode::Replace,
            &Some(buffer_batch(true)),
            Some(&prepared_text_frame_diagnostics(true)),
            None
        ));
        let missing_upload = next_core_webgpu_replace_diagnostics(
            NextCoreWebGpuPaneMode::Replace,
            &Some(buffer_batch(true)),
            Some(&prepared_text_frame_diagnostics(true)),
            None,
        );
        assert_eq!(
            missing_upload.readiness_issues(),
            vec![EngineRenderPaneReplaceReadinessIssue::MissingCachedGlyphUpload]
        );

        assert!(!should_replace_legacy_pane_from_parts(
            NextCoreWebGpuPaneMode::Replace,
            &Some(buffer_batch(true)),
            Some(&prepared_text_frame_diagnostics(true)),
            Some(&cached_glyph_upload_diagnostics(false))
        ));
        let incomplete_upload = next_core_webgpu_replace_diagnostics(
            NextCoreWebGpuPaneMode::Replace,
            &Some(buffer_batch(true)),
            Some(&prepared_text_frame_diagnostics(true)),
            Some(&cached_glyph_upload_diagnostics(false)),
        );
        assert_eq!(
            incomplete_upload.readiness_issues(),
            vec![EngineRenderPaneReplaceReadinessIssue::CachedGlyphUploadNotReady]
        );

        assert!(should_replace_legacy_pane_from_parts(
            NextCoreWebGpuPaneMode::Replace,
            &Some(buffer_batch(true)),
            Some(&prepared_text_frame_diagnostics(true)),
            Some(&cached_glyph_upload_diagnostics(true))
        ));
    }

    fn buffer_batch(draw_ready: bool) -> EngineRenderBufferBatch {
        EngineRenderBufferBatch {
            pane_id: 7,
            stats: EngineRenderCommitStats {
                submit: draw_ready,
                previous_revision: None,
                revision: 3,
                skipped_revisions: 0,
                requires_full_repaint: draw_ready,
                full: draw_ready,
                viewport: None,
                damage_rect_count: usize::from(draw_ready),
                background_quad_count: usize::from(draw_ready),
                text_run_count: 0,
                cursor_visible: false,
            },
            buffer_plan: EngineRenderBufferPlan {
                pane_id: 7,
                submitted: draw_ready,
                revision: 3,
                requires_full_repaint: draw_ready,
                damage_rects: Vec::new(),
                text_runs: Vec::new(),
                vertices: if draw_ready {
                    vec![EngineRenderVertex {
                        position: [0.0, 0.0],
                        color: [1.0, 1.0, 1.0, 1.0],
                        layer: EngineRenderVertexLayer::Background,
                        command_index: 0,
                    }]
                } else {
                    Vec::new()
                },
                indices: if draw_ready { vec![0] } else { Vec::new() },
            },
        }
    }

    fn prepared_frame_diagnostics(replace_ready: bool) -> EngineWgpuPreparedFrameDiagnostics {
        prepared_frame_diagnostics_for_revision(replace_ready, 3)
    }

    fn prepared_frame_diagnostics_for_revision(
        replace_ready: bool,
        revision: u64,
    ) -> EngineWgpuPreparedFrameDiagnostics {
        EngineWgpuPreparedFrameDiagnostics {
            pane_id: 7,
            submitted: replace_ready,
            revision,
            solid_vertex_count: usize::from(replace_ready),
            solid_index_count: usize::from(replace_ready),
            text_run_count: 0,
            glyph_key_count: 0,
            glyph_instance_count: 0,
            replace_ready,
        }
    }

    fn prepared_text_frame_diagnostics(replace_ready: bool) -> EngineWgpuPreparedFrameDiagnostics {
        EngineWgpuPreparedFrameDiagnostics {
            text_run_count: 1,
            glyph_key_count: 1,
            glyph_instance_count: 1,
            ..prepared_frame_diagnostics(replace_ready)
        }
    }

    fn cached_glyph_upload_diagnostics(ready: bool) -> EngineRenderCachedGlyphUploadDiagnostics {
        cached_glyph_upload_diagnostics_for_revision(ready, 3)
    }

    fn cached_glyph_upload_diagnostics_for_revision(
        ready: bool,
        revision: u64,
    ) -> EngineRenderCachedGlyphUploadDiagnostics {
        EngineRenderCachedGlyphUploadDiagnostics {
            pane_id: 7,
            submitted: ready,
            revision,
            cell_width_px: 8,
            cell_height_px: 16,
            inserted_key_count: usize::from(ready),
            overflow_key_count: 0,
            texture_region_count: usize::from(ready),
            texture_missing_key_count: 0,
            layout_entry_count: usize::from(ready),
            layout_missing_key_count: 0,
            vertex_count: if ready { 4 } else { 0 },
            index_count: if ready { 6 } else { 0 },
            draw_ready: ready,
        }
    }
}
