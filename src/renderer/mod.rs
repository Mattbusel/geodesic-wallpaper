//! wgpu render pipelines: surface wireframe and geodesic trail lines.

pub mod camera;

use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};
use crate::trail::TrailVertex;
use crate::error::GeodesicError;
use camera::Camera;
use windows::Win32::Foundation::HWND;
use raw_window_handle::{
    RawWindowHandle, RawDisplayHandle, Win32WindowHandle, WindowsDisplayHandle,
    HasWindowHandle, HasDisplayHandle,
};
use std::num::NonZeroIsize;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
}

/// wgpu renderer owning all GPU state for the geodesic wallpaper.
///
/// Owns two render pipelines: one for the surface wireframe and one for the
/// geodesic trail lines. All GPU buffers are pre-allocated at construction
/// time; no per-frame heap allocation occurs on the hot path.
pub struct Renderer {
    pub surface_config: wgpu::SurfaceConfiguration,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub wgpu_surface: wgpu::Surface<'static>,
    surface_pipeline: wgpu::RenderPipeline,
    surface_vbuf: wgpu::Buffer,
    surface_ibuf: wgpu::Buffer,
    surface_index_count: u32,
    trail_pipeline: wgpu::RenderPipeline,
    trail_vbuf: wgpu::Buffer,
    trail_vbuf_capacity: usize,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pub camera: Camera,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
}

const MAX_TRAIL_VERTS: usize = 100_000;

/// Minimal wrapper so wgpu can obtain a surface from a raw Win32 HWND.
struct RawHwnd(isize);

impl HasWindowHandle for RawHwnd {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let nz = NonZeroIsize::new(self.0)
            .ok_or(raw_window_handle::HandleError::Unavailable)?;
        let mut h = Win32WindowHandle::new(nz);
        h.hinstance = None;
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(RawWindowHandle::Win32(h)) })
    }
}

impl HasDisplayHandle for RawHwnd {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        Ok(unsafe { raw_window_handle::DisplayHandle::borrow_raw(RawDisplayHandle::Windows(WindowsDisplayHandle::new())) })
    }
}

impl Renderer {
    /// Create a new renderer targeting the given Win32 `hwnd`.
    ///
    /// # Errors
    ///
    /// Returns [`GeodesicError::RenderError`] if the wgpu instance, adapter,
    /// device, or surface cannot be created.
    pub async fn new(
        hwnd: HWND,
        width: u32,
        height: u32,
        mesh_verts: &[[f32; 3]],
        mesh_indices: &[u32],
    ) -> Result<Self, GeodesicError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let raw = RawHwnd(hwnd.0 as isize);
        let wgpu_surface = unsafe {
            instance
                .create_surface_unsafe(
                    wgpu::SurfaceTargetUnsafe::from_window(&raw)
                        .map_err(|e| GeodesicError::render(format!("surface target: {e}")))?,
                )
                .map_err(|e| GeodesicError::render(format!("create_surface: {e}")))?
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&wgpu_surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| GeodesicError::render("No compatible GPU adapter found"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| GeodesicError::render(format!("request_device: {e}")))?;

        let caps = wgpu_surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .or_else(|| caps.formats.first().copied())
            .ok_or_else(|| GeodesicError::render("No supported surface formats"))?;
        let alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .ok_or_else(|| GeodesicError::render("No supported alpha modes"))?;

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        wgpu_surface.configure(&device, &surface_config);

        let camera = Camera::new(width as f32 / height as f32);
        let uniforms = Uniforms { view_proj: camera.view_proj().to_cols_array_2d() };

        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniforms"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
            label: None,
        });

        // Surface pipeline
        let surface_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("surface"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/surface.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });
        let surface_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("surface_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &surface_shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: (3 * 4) as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &surface_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        // Build wireframe index buffer from triangles
        let mut line_indices: Vec<u32> = Vec::new();
        for tri in mesh_indices.chunks(3) {
            if tri.len() == 3 {
                line_indices.extend_from_slice(&[tri[0], tri[1], tri[1], tri[2], tri[2], tri[0]]);
            }
        }

        let surface_verts_bytes: Vec<u8> = mesh_verts.iter()
            .flat_map(|v| bytemuck::bytes_of(v).iter().copied())
            .collect();
        let surface_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surface_vbuf"),
            contents: &surface_verts_bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });
        let surface_index_count = line_indices.len() as u32;
        let surface_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surface_ibuf"),
            contents: bytemuck::cast_slice(&line_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Trail pipeline
        let trail_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("trail"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/trail.wgsl").into()),
        });
        let trail_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("trail_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &trail_shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TrailVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &trail_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineStrip,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        let trail_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("trail_vbuf"),
            size: (MAX_TRAIL_VERTS * std::mem::size_of::<TrailVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (depth_texture, depth_view) = Self::make_depth(&device, width, height);

        tracing::info!(width, height, "renderer initialised");
        Ok(Renderer {
            surface_config,
            device,
            queue,
            wgpu_surface,
            surface_pipeline,
            surface_vbuf,
            surface_ibuf,
            surface_index_count,
            trail_pipeline,
            trail_vbuf,
            trail_vbuf_capacity: MAX_TRAIL_VERTS,
            uniform_buf,
            bind_group,
            camera,
            depth_texture,
            depth_view,
        })
    }

    fn make_depth(device: &wgpu::Device, w: u32, h: u32) -> (wgpu::Texture, wgpu::TextureView) {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());
        (tex, view)
    }

    /// Resize the swap-chain and depth texture to the new pixel dimensions.
    ///
    /// Must be called whenever the Win32 window receives a `WM_SIZE` message.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.wgpu_surface.configure(&self.device, &self.surface_config);
        self.camera.aspect = width as f32 / height as f32;
        let (dt, dv) = Self::make_depth(&self.device, width, height);
        self.depth_texture = dt;
        self.depth_view = dv;
    }

    /// Record and submit a frame: surface wireframe + trail line strips.
    ///
    /// `trail_verts` is a flat concatenation of all trail vertex data;
    /// `trail_segment_lengths` gives the number of vertices belonging to each
    /// geodesic segment so the render pass can issue the correct draw calls.
    pub fn render(&mut self, trail_verts: &[TrailVertex], trail_segment_lengths: &[usize]) {
        let uniforms = Uniforms { view_proj: self.camera.view_proj().to_cols_array_2d() };
        self.queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&uniforms));

        if !trail_verts.is_empty() {
            let n = trail_verts.len().min(self.trail_vbuf_capacity);
            self.queue.write_buffer(&self.trail_vbuf, 0, bytemuck::cast_slice(&trail_verts[..n]));
        }

        let frame = match self.wgpu_surface.get_current_texture() {
            Ok(f) => f,
            Err(e) => { tracing::warn!("Surface err: {e}"); return; }
        };
        let view = frame.texture.create_view(&Default::default());
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.02, g: 0.02, b: 0.05, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            rp.set_pipeline(&self.surface_pipeline);
            rp.set_bind_group(0, &self.bind_group, &[]);
            rp.set_vertex_buffer(0, self.surface_vbuf.slice(..));
            rp.set_index_buffer(self.surface_ibuf.slice(..), wgpu::IndexFormat::Uint32);
            rp.draw_indexed(0..self.surface_index_count, 0, 0..1);

            rp.set_pipeline(&self.trail_pipeline);
            rp.set_bind_group(0, &self.bind_group, &[]);
            rp.set_vertex_buffer(0, self.trail_vbuf.slice(..));
            let mut offset = 0u32;
            for &len in trail_segment_lengths {
                if len >= 2 {
                    let end = (offset + len as u32).min(self.trail_vbuf_capacity as u32);
                    rp.draw(offset..end, 0..1);
                }
                offset += len as u32;
                if offset as usize >= self.trail_vbuf_capacity { break; }
            }
        }
        self.queue.submit(std::iter::once(enc.finish()));
        frame.present();
    }
}

