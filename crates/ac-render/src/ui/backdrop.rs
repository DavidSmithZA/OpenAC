//! Glass for the modern interface: a copy of the frame drawn so far,
//! blurred, for its windows to show behind their tint. The copy is halved
//! [`HALVINGS`] times and grown back to a quarter of the frame (the dual
//! filter: a few filtered taps a step, so a wide blur costs little), and
//! the interface samples that quarter where each of its pixels lies.
//!
//! The blur is taken where glass is drawn, so what the interface drew
//! before it (the HUD, a window below, the picture behind the character
//! screens) is blurred with the world. A frame blurs again at a later
//! glass only when more was drawn since, a few times at most
//! (`MAX_BLURS` in the UI renderer); past that the last blur serves.

/// How many times the copy is halved.
pub const HALVINGS: usize = 4;
/// The level the blur grows back to (0: half the frame, 1: a quarter).
pub const RESULT_LEVEL: usize = 1;

/// The size of level `level` (0 is half the frame), at least a pixel.
#[must_use]
pub fn level_size((w, h): (u32, u32), level: usize) -> (u32, u32) {
    let shift = level as u32 + 1;
    ((w >> shift).max(1), (h >> shift).max(1))
}

/// The steps of a blur, as (source, target) levels: `None` is the copy
/// of the frame. Down to the smallest, then back up to [`RESULT_LEVEL`].
#[must_use]
pub fn steps() -> Vec<(Option<usize>, usize, Step)> {
    let mut out = vec![(None, 0, Step::Down)];
    for level in 1..HALVINGS {
        out.push((Some(level - 1), level, Step::Down));
    }
    for level in (RESULT_LEVEL..HALVINGS - 1).rev() {
        out.push((Some(level + 1), level, Step::Up));
    }
    out
}

/// A halving or a doubling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// To half the source's size.
    Down,
    /// To twice the source's size.
    Up,
}

struct Targets {
    size: (u32, u32),
    copy: wgpu::Texture,
    /// The copy, bound as a source.
    copy_group: wgpu::BindGroup,
    /// Each level's view (a target) and binding (a source).
    levels: Vec<(wgpu::TextureView, wgpu::BindGroup)>,
}

/// The blur's pipelines and, once sized, its textures.
pub struct BackdropBlur {
    format: wgpu::TextureFormat,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    down: wgpu::RenderPipeline,
    up: wgpu::RenderPipeline,
    targets: Option<Targets>,
}

impl std::fmt::Debug for BackdropBlur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackdropBlur")
            .field("size", &self.targets.as_ref().map(|t| t.size))
            .finish_non_exhaustive()
    }
}

impl BackdropBlur {
    /// The pipelines for frames of `format`.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        use crate::renderer::pipelines::{sampler_entry, texture_entry};
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("backdrop blur"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/backdrop_blur.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("backdrop blur source"),
            entries: &[
                texture_entry(0, wgpu::TextureViewDimension::D2),
                sampler_entry(1),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("backdrop blur"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |label: &str, fragment: &str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fragment),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("backdrop blur"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            format,
            down: pipeline("backdrop blur down", "fs_down"),
            up: pipeline("backdrop blur up", "fs_up"),
            layout,
            sampler,
            targets: None,
        }
    }

    /// Size the textures for frames of `size` pixels; true when they were
    /// made anew (what binds the result must bind it again).
    pub fn ensure(&mut self, device: &wgpu::Device, size: (u32, u32)) -> bool {
        if self.targets.as_ref().is_some_and(|t| t.size == size) || size.0 == 0 || size.1 == 0 {
            return false;
        }
        let texture = |label: &str, (width, height): (u32, u32), usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | usage,
                view_formats: &[],
            })
        };
        let group = |view: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("backdrop blur source"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            })
        };
        let copy = texture("backdrop copy", size, wgpu::TextureUsages::COPY_DST);
        let copy_group = group(&copy.create_view(&wgpu::TextureViewDescriptor::default()));
        let levels = (0..HALVINGS)
            .map(|level| {
                let t = texture(
                    "backdrop level",
                    level_size(size, level),
                    wgpu::TextureUsages::RENDER_ATTACHMENT,
                );
                let view = t.create_view(&wgpu::TextureViewDescriptor::default());
                let g = group(&view);
                (view, g)
            })
            .collect();
        self.targets = Some(Targets {
            size,
            copy,
            copy_group,
            levels,
        });
        true
    }

    /// The blurred frame, once sized.
    #[must_use]
    pub fn result(&self) -> Option<&wgpu::TextureView> {
        self.targets.as_ref().map(|t| &t.levels[RESULT_LEVEL].0)
    }

    /// Copy `source` (the frame so far, the size the blur was made for) and
    /// blur it. False when it could not (not sized, or another size).
    pub fn run(&self, encoder: &mut wgpu::CommandEncoder, source: &wgpu::Texture) -> bool {
        let Some(t) = self.targets.as_ref() else {
            return false;
        };
        if (source.width(), source.height()) != t.size || source.format() != self.format {
            return false;
        }
        encoder.copy_texture_to_texture(
            source.as_image_copy(),
            t.copy.as_image_copy(),
            wgpu::Extent3d {
                width: t.size.0,
                height: t.size.1,
                depth_or_array_layers: 1,
            },
        );
        for (from, to, step) in steps() {
            let source = match from {
                None => &t.copy_group,
                Some(level) => &t.levels[level].1,
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("backdrop blur"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &t.levels[to].0,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(match step {
                Step::Down => &self.down,
                Step::Up => &self.up,
            });
            pass.set_bind_group(0, source, &[]);
            pass.draw(0..3, 0..1);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_blur_halves_down_and_grows_back_to_a_quarter() {
        assert_eq!(
            steps(),
            vec![
                (None, 0, Step::Down),
                (Some(0), 1, Step::Down),
                (Some(1), 2, Step::Down),
                (Some(2), 3, Step::Down),
                (Some(3), 2, Step::Up),
                (Some(2), 1, Step::Up),
            ]
        );
        assert_eq!(level_size((2880, 1800), RESULT_LEVEL), (720, 450));
        assert_eq!(level_size((2880, 1800), HALVINGS - 1), (180, 112));
        assert_eq!(level_size((10, 3), 3), (1, 1));
    }
}
