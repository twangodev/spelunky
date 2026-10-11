//! Batched sprites drawn at the actual surface resolution.
//!
//! Logical positions stay fractional all the way to rasterization. The viewport
//! uses a uniform scale and expands the visible world to fill the surface.

use crate::viewport::Viewport;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use winit::window::Window;

#[cfg(test)]
const LOGICAL_WIDTH: f32 = 320.0;
#[cfg(test)]
const LOGICAL_HEIGHT: f32 = 240.0;
const INITIAL_VERTEX_CAPACITY: usize = 4096;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QuadBlend {
    #[default]
    Alpha,
    Multiply,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DrawQuad {
    /// Logical x, y, width, height. Fractional coordinates are preserved.
    pub rect: [f32; 4],
    /// Atlas u, v, width, height in normalized coordinates.
    /// `[0.0; 4]` samples the atlas's white first texel for solid colors.
    pub uv: [f32; 4],
    pub color: [f32; 4],
    /// Clockwise radians in the screen's downward-positive coordinate system.
    pub rotation: f32,
    /// Rotation origin in logical pixels relative to the rectangle's top left.
    pub pivot: [f32; 2],
    pub blend: QuadBlend,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub struct Renderer {
    // The surface also owns an Arc, keeping its window alive for its full lifetime.
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    present_modes: Vec<wgpu::PresentMode>,
    pipeline: wgpu::RenderPipeline,
    multiply_pipeline: wgpu::RenderPipeline,
    render_format: wgpu::TextureFormat,
    atlas_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    vertices: Vec<Vertex>,
    suspended: bool,
}

impl Renderer {
    pub async fn new(
        window: Arc<Window>,
        atlas_rgba: &[u8],
        atlas_width: u32,
        atlas_height: u32,
    ) -> Result<Self, String> {
        let expected_bytes = (atlas_width as usize)
            .checked_mul(atlas_height as usize)
            .and_then(|pixels| pixels.checked_mul(4));
        if atlas_width == 0 || atlas_height == 0 || expected_bytes != Some(atlas_rgba.len()) {
            return Err("Atlas dimensions must match a nonempty RGBA image".into());
        }

        let size = window.inner_size();
        // Browser builds deliberately request WebGL 2 rather than relying on
        // WebGPU availability. Native builds use wgpu's available backends.
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: if cfg!(target_arch = "wasm32") {
                wgpu::Backends::GL
            } else {
                wgpu::Backends::all()
            },
            ..Default::default()
        });
        let surface = instance.create_surface(window).map_err(|e| e.to_string())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("No compatible graphics adapter; WebGL 2 is required in browsers")?;
        let adapter_limits = adapter.limits();
        if atlas_width > adapter_limits.max_texture_dimension_2d
            || atlas_height > adapter_limits.max_texture_dimension_2d
        {
            return Err("Sprite atlas exceeds the adapter's texture size limit".into());
        }
        let limits = wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter_limits);
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Spelunky graphics device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: limits,
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None,
            )
            .await
            .map_err(|e| e.to_string())?;
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| !format.is_srgb())
            .or_else(|| capabilities.formats.first().copied())
            .ok_or("Graphics surface has no supported color formats")?;
        let alpha_mode = capabilities
            .alpha_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::CompositeAlphaMode::Opaque)
            .or_else(|| capabilities.alpha_modes.first().copied())
            .ok_or("Graphics surface has no supported alpha modes")?;
        // GM8 blends the original RGB bytes directly. A linear UNORM render
        // view avoids sRGB conversion making dark masks substantially brighter.
        let render_format = format.remove_srgb_suffix();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: if render_format == format {
                vec![]
            } else {
                vec![render_format]
            },
        };
        surface.configure(&device, &config);

        let atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Classic sprite atlas"),
            size: wgpu::Extent3d {
                width: atlas_width,
                height: atlas_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            atlas_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(atlas_width * 4),
                rows_per_image: Some(atlas_height),
            },
            atlas.size(),
        );
        let atlas_view = atlas.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Nearest sprite sampling"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let atlas_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sprite atlas layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
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
        });
        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sprite atlas"),
            layout: &atlas_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sprite pipeline layout"),
            bind_group_layouts: &[&atlas_layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sprite shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let pipeline = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            render_format,
            QuadBlend::Alpha,
        );
        let multiply_pipeline = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            render_format,
            QuadBlend::Multiply,
        );
        let vertex_buffer = create_vertex_buffer(&device, INITIAL_VERTEX_CAPACITY);
        Ok(Self {
            surface,
            device,
            queue,
            config,
            present_modes: capabilities.present_modes,
            pipeline,
            multiply_pipeline,
            render_format,
            atlas_bind_group,
            vertex_buffer,
            vertex_capacity: INITIAL_VERTEX_CAPACITY,
            vertices: Vec::with_capacity(INITIAL_VERTEX_CAPACITY),
            suspended: size.width == 0 || size.height == 0,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.suspended = width == 0 || height == 0;
        if self.suspended {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn set_vsync(&mut self, enabled: bool) {
        self.config.present_mode = if enabled {
            wgpu::PresentMode::Fifo
        } else {
            [wgpu::PresentMode::Immediate, wgpu::PresentMode::Mailbox]
                .into_iter()
                .find(|mode| self.present_modes.contains(mode))
                .unwrap_or(wgpu::PresentMode::Fifo)
        };
        self.config.desired_maximum_frame_latency = if enabled { 2 } else { 1 };
        if !self.suspended {
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn viewport(&self) -> Viewport {
        Viewport::from_surface(self.config.width, self.config.height)
    }

    pub fn render(
        &mut self,
        quads: &[DrawQuad],
        clear: [f64; 4],
    ) -> Result<(), wgpu::SurfaceError> {
        if self.suspended {
            return Ok(());
        }
        let frame = self.surface.get_current_texture()?;
        self.vertices.clear();
        let view = self.viewport();
        for quad in quads {
            append_quad_in_view(&mut self.vertices, quad, view);
        }
        if self.vertices.len() > self.vertex_capacity {
            self.vertex_capacity = self.vertices.len().next_power_of_two();
            self.vertex_buffer = create_vertex_buffer(&self.device, self.vertex_capacity);
        }
        if !self.vertices.is_empty() {
            self.queue
                .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        }
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.render_format),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Sprite frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Sprite pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear[0],
                            g: clear[1],
                            b: clear[2],
                            a: clear[3],
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let [x, y, width, height] = viewport(self.config.width, self.config.height);
            pass.set_viewport(x, y, width, height, 0.0, 1.0);
            // Rasterize every physical pixel; the logical view expands uniformly.
            let left = x.ceil() as u32;
            let top = y.ceil() as u32;
            let right = (x + width).floor() as u32;
            let bottom = (y + height).floor() as u32;
            if right > left && bottom > top {
                pass.set_scissor_rect(left, top, right - left, bottom - top);
                pass.set_bind_group(0, &self.atlas_bind_group, &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                for (blend, range) in draw_batches(quads) {
                    pass.set_pipeline(match blend {
                        QuadBlend::Alpha => &self.pipeline,
                        QuadBlend::Multiply => &self.multiply_pipeline,
                    });
                    pass.draw(range, 0..1);
                }
            }
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    blend: QuadBlend,
) -> wgpu::RenderPipeline {
    let blend = match blend {
        QuadBlend::Alpha => wgpu::BlendState::ALPHA_BLENDING,
        QuadBlend::Multiply => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Dst,
                dst_factor: wgpu::BlendFactor::Zero,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        },
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Classic sprite/mask batch"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vertex_main"),
            buffers: &[Vertex::layout()],
            compilation_options: Default::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fragment_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(blend),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        multiview: None,
        cache: None,
    })
}

fn draw_batches(quads: &[DrawQuad]) -> Vec<(QuadBlend, std::ops::Range<u32>)> {
    let mut batches: Vec<(QuadBlend, std::ops::Range<u32>)> = Vec::with_capacity(3);
    for (index, quad) in quads.iter().enumerate() {
        let end = (index as u32 + 1) * 6;
        if let Some((_, range)) = batches.last_mut().filter(|(blend, _)| *blend == quad.blend) {
            range.end = end;
        } else {
            batches.push((quad.blend, index as u32 * 6..end));
        }
    }
    batches
}

fn create_vertex_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Reusable sprite vertices"),
        size: (capacity * std::mem::size_of::<Vertex>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn viewport(width: u32, height: u32) -> [f32; 4] {
    [0.0, 0.0, width as f32, height as f32]
}

#[cfg(test)]
fn append_quad(vertices: &mut Vec<Vertex>, quad: &DrawQuad) {
    append_quad_in_view(vertices, quad, Viewport::canonical());
}

fn append_quad_in_view(vertices: &mut Vec<Vertex>, quad: &DrawQuad, view: Viewport) {
    let [x, y, width, height] = quad.rect;
    let [u, v, uv_width, uv_height] = quad.uv;
    let [pivot_x, pivot_y] = quad.pivot;
    let (sin, cos) = quad.rotation.sin_cos();
    let vertex = |dx: f32, dy: f32| {
        let local_x = width * dx - pivot_x;
        let local_y = height * dy - pivot_y;
        let world_x = x + pivot_x + local_x * cos - local_y * sin;
        let world_y = y + pivot_y + local_x * sin + local_y * cos;
        Vertex {
            position: [
                world_x * 2.0 / view.width - 1.0,
                1.0 - world_y * 2.0 / view.height,
            ],
            uv: [u + uv_width * dx, v + uv_height * dy],
            color: quad.color,
        }
    };
    vertices.extend_from_slice(&[
        vertex(0.0, 0.0),
        vertex(0.0, 1.0),
        vertex(1.0, 0.0),
        vertex(1.0, 0.0),
        vertex(0.0, 1.0),
        vertex(1.0, 1.0),
    ]);
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Executes the production shader, vertex assembly, blend pipelines and
    /// batch ordering. The second target format exercises the linear-view
    /// fallback used when a surface only advertises an sRGB format.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn gpu_pixels(quads: &[DrawQuad], srgb_texture: bool) -> Vec<u8> {
        gpu_pixels_in_view(quads, srgb_texture, Viewport::canonical())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn gpu_pixels_in_view(
        quads: &[DrawQuad],
        srgb_texture: bool,
        viewport: Viewport,
    ) -> Vec<u8> {
        let width = viewport.surface_width;
        let height = viewport.surface_height;
        let stride = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        pollster::block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    force_fallback_adapter: true,
                    ..Default::default()
                })
                .await
                .expect("GPU pixel regression requires a software or native adapter");
            eprintln!("GPU_ACCEPTANCE adapter={:?}", adapter.get_info());
            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("source mask pixel regression"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                        memory_hints: wgpu::MemoryHints::MemoryUsage,
                    },
                    None,
                )
                .await
                .unwrap();
            let atlas = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("white atlas texel"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                atlas.as_image_copy(),
                &[255; 4],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4),
                    rows_per_image: Some(1),
                },
                atlas.size(),
            );
            let atlas_view = atlas.create_view(&Default::default());
            let sampler = device.create_sampler(&Default::default());
            let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
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
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&atlas_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bind_layout],
                push_constant_ranges: &[],
            });
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            });
            let format = wgpu::TextureFormat::Rgba8Unorm;
            let alpha = create_pipeline(&device, &layout, &shader, format, QuadBlend::Alpha);
            let multiply = create_pipeline(&device, &layout, &shader, format, QuadBlend::Multiply);
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("mask readback"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: if srgb_texture {
                    wgpu::TextureFormat::Rgba8UnormSrgb
                } else {
                    format
                },
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[format],
            });
            let view = target.create_view(&wgpu::TextureViewDescriptor {
                format: Some(format),
                ..Default::default()
            });
            let mut vertices = Vec::new();
            for quad in quads {
                append_quad_in_view(&mut vertices, quad, viewport);
            }
            let buffer = create_vertex_buffer(&device, vertices.len().max(1));
            queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&vertices));
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_vertex_buffer(0, buffer.slice(..));
                for (blend, range) in draw_batches(quads) {
                    pass.set_pipeline(if blend == QuadBlend::Multiply {
                        &multiply
                    } else {
                        &alpha
                    });
                    pass.draw(range, 0..1);
                }
            }
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: u64::from(stride) * u64::from(height),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_texture_to_buffer(
                target.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(stride),
                        rows_per_image: Some(height),
                    },
                },
                target.size(),
            );
            queue.submit(Some(encoder.finish()));
            let (sender, receiver) = std::sync::mpsc::channel();
            readback
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    sender.send(result).unwrap();
                });
            device.poll(wgpu::Maintain::Wait);
            receiver
                .recv_timeout(std::time::Duration::from_secs(30))
                .unwrap()
                .unwrap();
            let bytes = readback.slice(..).get_mapped_range();
            bytes
                .chunks(stride as usize)
                .flat_map(|row| row[..width as usize * 4].iter().copied())
                .collect()
        })
    }

    #[test]
    fn viewport_covers_the_entire_surface() {
        assert_eq!(viewport(1920, 1080), [0.0, 0.0, 1920.0, 1080.0]);
        assert_eq!(viewport(800, 800), [0.0, 0.0, 800.0, 800.0]);
    }

    #[test]
    fn expanded_projection_keeps_square_pixels_at_all_surface_shapes() {
        let quad = DrawQuad {
            rect: [64.0, 64.0, 16.0, 16.0],
            color: [1.; 4],
            ..Default::default()
        };
        for (width, height) in [(1920, 1080), (800, 800), (600, 1200), (3440, 1440)] {
            let view = Viewport::from_surface(width, height);
            let mut vertices = Vec::new();
            append_quad_in_view(&mut vertices, &quad, view);
            let dx = (vertices[2].position[0] - vertices[0].position[0]) * width as f32 / 2.0;
            let dy = (vertices[0].position[1] - vertices[1].position[1]) * height as f32 / 2.0;
            assert!((dx - dy).abs() < 0.001);
            assert!((f64::from(dx) - 16.0 * view.scale).abs() < 0.001);
        }
    }

    #[test]
    fn intermediate_frame_positions_are_not_rounded() {
        let mut vertices = Vec::new();
        let mut quad = DrawQuad {
            rect: [10.0, 20.0, 16.0, 16.0],
            uv: [0.0; 4],
            color: [1.0; 4],
            ..DrawQuad::default()
        };
        append_quad(&mut vertices, &quad);
        let first_x = vertices[0].position[0];
        vertices.clear();
        quad.rect[0] += 0.25;
        append_quad(&mut vertices, &quad);
        let movement = vertices[0].position[0] - first_x;
        assert!((movement - 0.25 * 2.0 / LOGICAL_WIDTH).abs() < 1e-6);
    }

    #[test]
    fn quarter_turn_rotates_corners_around_local_pivot() {
        let quad = DrawQuad {
            rect: [10.25, 20.5, 16.0, 8.0],
            pivot: [8.0, 4.0],
            rotation: std::f32::consts::FRAC_PI_2,
            uv: [0.2, 0.3, 0.1, 0.15],
            color: [0.5, 0.75, 1.0, 0.25],
            blend: QuadBlend::Alpha,
        };
        let mut vertices = Vec::new();
        append_quad(&mut vertices, &quad);
        let expected = [
            [22.25, 16.5],
            [14.25, 16.5],
            [22.25, 32.5],
            [22.25, 32.5],
            [14.25, 16.5],
            [14.25, 32.5],
        ];
        for (vertex, [x, y]) in vertices.iter().zip(expected) {
            let actual_x = (vertex.position[0] + 1.0) * LOGICAL_WIDTH * 0.5;
            let actual_y = (1.0 - vertex.position[1]) * LOGICAL_HEIGHT * 0.5;
            assert!((actual_x - x).abs() < 1e-5);
            assert!((actual_y - y).abs() < 1e-5);
            assert_eq!(vertex.color, quad.color);
        }
        // Rotation moves geometry without changing atlas sampling or opacity.
        assert_eq!(vertices[0].uv, [0.2, 0.3]);
        assert!((vertices[5].uv[0] - 0.3).abs() < 1e-6);
        assert!((vertices[5].uv[1] - 0.45).abs() < 1e-6);
    }
}
