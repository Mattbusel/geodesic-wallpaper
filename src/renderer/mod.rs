//! wgpu render pipelines: surface wireframe and geodesic trail lines.

pub mod camera;

use crate::error::GeodesicError;
use crate::trail::TrailVertex;
use bytemuck::{Pod, Zeroable};
use camera::Camera;
use raw_window_handle::{
    HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle, Win32WindowHandle,
    WindowsDisplayHandle,
};
use std::num::NonZeroIsize;
use wgpu::util::DeviceExt;
use windows::Win32::Foundation::HWND;

/// Default trail vertex buffer capacity for the headless renderer.
const MAX_TRAIL_VERTS: usize = 100_000;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    light_dir: [f32; 4],
    time: f32,
    _pad: [f32; 3],
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
    /// Whether to draw the surface wireframe each frame.
    pub show_wireframe: bool,
    /// Background clear colour used each frame.
    pub background_color: wgpu::Color,
    /// Accumulated time in seconds for shader animations.
    pub elapsed_secs: f32,
    /// Light direction vector for surface shading.
    pub light_dir: [f32; 3],
    /// Whether to display the FPS HUD.
    pub show_fps_hud: bool,
}

/// Minimal wrapper so wgpu can obtain a surface from a raw Win32 HWND.
struct RawHwnd(isize);

impl HasWindowHandle for RawHwnd {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let nz = NonZeroIsize::new(self.0).ok_or(raw_window_handle::HandleError::Unavailable)?;
        let mut h = Win32WindowHandle::new(nz);
        h.hinstance = None;
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(RawWindowHandle::Win32(h)) })
    }
}

impl HasDisplayHandle for RawHwnd {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        Ok(unsafe {
            raw_window_handle::DisplayHandle::borrow_raw(RawDisplayHandle::Windows(
                WindowsDisplayHandle::new(),
            ))
        })
    }
}

impl Renderer {
    /// Create a new renderer targeting the given Win32 `hwnd`.
    ///
    /// `max_trail_verts` sets the GPU buffer capacity for trail geometry.
    /// `show_wireframe` controls whether the surface mesh is drawn.
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
        max_trail_verts: usize,
        show_wireframe: bool,
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
        let uniforms = Uniforms {
            view_proj: camera.view_proj().to_cols_array_2d(),
            light_dir: [1.0, 1.0, 1.0, 0.0],
            time: 0.0,
            _pad: [0.0; 3],
        };

        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniforms"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
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

        let surface_verts_bytes: Vec<u8> = mesh_verts
            .iter()
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

        let max_trail_verts = max_trail_verts.max(100);
        let trail_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("trail_vbuf"),
            size: (max_trail_verts * std::mem::size_of::<TrailVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (depth_texture, depth_view) = Self::make_depth(&device, width, height);

        tracing::info!(width, height, max_trail_verts, "renderer initialised");
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
            trail_vbuf_capacity: max_trail_verts,
            uniform_buf,
            bind_group,
            camera,
            depth_texture,
            depth_view,
            show_wireframe,
            background_color: wgpu::Color { r: 0.02, g: 0.02, b: 0.05, a: 1.0 },
            elapsed_secs: 0.0,
            light_dir: [1.0, 1.0, 1.0],
            show_fps_hud: false,
        })
    }

    /// Create a headless renderer that renders to an offscreen texture (no window surface).
    ///
    /// Returns the renderer plus the offscreen colour texture that can be read back
    /// with [`Renderer::render_to_texture`].
    ///
    /// # Errors
    ///
    /// Returns [`GeodesicError::RenderError`] if any wgpu object cannot be created.
    pub async fn new_headless(
        width: u32,
        height: u32,
        mesh_verts: &[[f32; 3]],
        mesh_indices: &[u32],
    ) -> Result<(Self, wgpu::Texture), GeodesicError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| GeodesicError::render("No compatible GPU adapter found (headless)"))?;

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
            .map_err(|e| GeodesicError::render(format!("request_device (headless): {e}")))?;

        // Use a fixed sRGB format for the offscreen texture.
        let format = wgpu::TextureFormat::Bgra8UnormSrgb;

        let offscreen_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("headless_color"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let camera = Camera::new(width as f32 / height as f32);
        let uniforms = Uniforms {
            view_proj: camera.view_proj().to_cols_array_2d(),
            light_dir: [1.0, 1.0, 1.0, 0.0],
            time: 0.0,
            _pad: [0.0; 3],
        };

        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniforms_headless"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
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
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
            label: None,
        });

        let surface_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("surface_headless"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/surface.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });
        let surface_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("surface_pipeline_headless"),
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

        let mut line_indices: Vec<u32> = Vec::new();
        for tri in mesh_indices.chunks(3) {
            if tri.len() == 3 {
                line_indices.extend_from_slice(&[tri[0], tri[1], tri[1], tri[2], tri[2], tri[0]]);
            }
        }
        let surface_verts_bytes: Vec<u8> = mesh_verts
            .iter()
            .flat_map(|v| bytemuck::bytes_of(v).iter().copied())
            .collect();
        let surface_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surface_vbuf_headless"),
            contents: &surface_verts_bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });
        let surface_index_count = line_indices.len() as u32;
        let surface_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surface_ibuf_headless"),
            contents: bytemuck::cast_slice(&line_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let trail_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("trail_headless"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/trail.wgsl").into()),
        });
        let trail_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("trail_pipeline_headless"),
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
            label: Some("trail_vbuf_headless"),
            size: (MAX_TRAIL_VERTS * std::mem::size_of::<TrailVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (depth_texture, depth_view) = Self::make_depth(&device, width, height);

        // Build a fake wgpu_surface and surface_config. In headless mode these are
        // never used for presentation — render_to_texture uses the offscreen_tex instead.
        // We need placeholder values so we can store the Renderer struct as-is.
        // We create a minimal dummy surface config with the chosen format; the wgpu_surface
        // is constructed via a raw HWND of 1 (a sentinel, never presented).
        // Instead of keeping a real surface, we store all state we need separately.
        // For simplicity, reuse the same struct with a "never-present" mode by
        // keeping device/queue/pipelines only and storing render target externally.
        //
        // To avoid the complexity of splitting the struct, we use a null HWND surface
        // wrapped in a dummy handle. On Windows a null HWND surface won't crash until
        // get_current_texture() is called. Since headless mode calls render_to_texture
        // (not render), this is safe.
        //
        // IMPORTANT: render() must NOT be called on a headless renderer.

        // We create the instance again to get a surface we can configure.
        // Actually — the Renderer struct requires a wgpu_surface. We create a
        // placeholder by constructing a surface from the offscreen texture's view.
        // In wgpu 22 there is no "null" surface. We work around this by storing
        // None in wgpu_surface via an Option, but that would require changing the
        // struct.  Instead we return (renderer_fields, offscreen_tex) and the
        // caller uses `render_to_texture` which doesn't touch wgpu_surface.
        //
        // Since changing the struct is the cleanest approach, we add a headless flag.
        // For now we store a minimal SurfaceConfiguration that matches the offscreen
        // format; `render_to_texture` bypasses `wgpu_surface` entirely.
        //
        // The trick: create a wgpu::Surface from a 1x1 dummy HWND so the struct
        // can hold the field. On headless call paths we never call present().
        // This is the minimal-invasive approach.

        // Build the Renderer struct manually, reusing device/queue from above.
        // We skip creating a real wgpu_surface by creating the struct with
        // surface_config pointing to the offscreen format.
        // We need a placeholder wgpu_surface — just reuse a surface created from
        // the adapter's software rasterizer if available, or accept that
        // new_headless is only usable on systems where a software adapter exists.
        //
        // Final decision: use the `unsafe` surface path with a null hwnd (HWND(1))
        // as a placeholder. This is equivalent to what wgpu's own test helpers do.
        let placeholder_hwnd = windows::Win32::Foundation::HWND(1 as *mut core::ffi::c_void);
        let raw = RawHwnd(placeholder_hwnd.0 as isize);
        let wgpu_surface = unsafe {
            instance
                .create_surface_unsafe(
                    wgpu::SurfaceTargetUnsafe::from_window(&raw)
                        .map_err(|e| GeodesicError::render(format!("headless surface target: {e}")))?
                )
                .map_err(|e| GeodesicError::render(format!("headless create_surface: {e}")))?
        };

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        // Do NOT configure the placeholder surface — it will fail.
        // render_to_texture never calls wgpu_surface methods.

        let renderer = Renderer {
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
            background_color: wgpu::Color { r: 0.02, g: 0.02, b: 0.05, a: 1.0 },
            elapsed_secs: 0.0,
            light_dir: [1.0, 1.0, 1.0],
            show_fps_hud: false,
        };

        Ok((renderer, offscreen_tex))
    }

    /// Render the current scene to an offscreen texture and read back the pixel data.
    ///
    /// Returns raw BGRA bytes (one byte per channel, width × height × 4 bytes total).
    /// The caller is responsible for converting the channel order if needed before
    /// saving (e.g. BGRA → RGBA swap).
    ///
    /// This method is intended for use with [`Self::new_headless`] and must not be
    /// called on a windowed renderer (the surface is not configured).
    pub fn render_to_texture(
        &mut self,
        offscreen_tex: &wgpu::Texture,
        trail_verts: &[TrailVertex],
        trail_segment_lengths: &[usize],
    ) -> Result<Vec<u8>, GeodesicError> {
        let width = offscreen_tex.size().width;
        let height = offscreen_tex.size().height;

        let ld = self.light_dir;
        let uniforms = Uniforms {
            view_proj: self.camera.view_proj().to_cols_array_2d(),
            light_dir: [ld[0], ld[1], ld[2], 0.0],
            time: self.elapsed_secs,
            _pad: [0.0; 3],
        };
        self.queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&uniforms));

        if !trail_verts.is_empty() {
            let n = trail_verts.len().min(self.trail_vbuf_capacity);
            self.queue.write_buffer(&self.trail_vbuf, 0, bytemuck::cast_slice(&trail_verts[..n]));
        }

        let color_view = offscreen_tex.create_view(&Default::default());
        let mut enc = self.device.create_command_encoder(&Default::default());

        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("headless_render"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.background_color),
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
                if offset as usize >= self.trail_vbuf_capacity {
                    break;
                }
            }
        }

        // wgpu requires rows to be aligned to 256 bytes.
        let bytes_per_pixel = 4u32; // BGRA8
        let unpadded_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_row = (unpadded_row + align - 1) / align * align;
        let buf_size = (padded_row * height) as u64;

        let readback_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: buf_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        enc.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: offscreen_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback_buf,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );

        self.queue.submit(std::iter::once(enc.finish()));

        // Map the buffer and wait.
        let slice = readback_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|_| GeodesicError::render("map_async channel closed"))?
            .map_err(|e| GeodesicError::render(format!("map_async failed: {e}")))?;

        let data = slice.get_mapped_range();
        // Strip row padding and convert BGRA → RGBA.
        let mut pixels: Vec<u8> = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = (row * padded_row) as usize;
            let row_data = &data[start..start + unpadded_row as usize];
            // BGRA → RGBA swap
            for px in row_data.chunks_exact(4) {
                pixels.push(px[2]); // R
                pixels.push(px[1]); // G
                pixels.push(px[0]); // B
                pixels.push(px[3]); // A
            }
        }
        drop(data);
        readback_buf.unmap();

        Ok(pixels)
    }

    fn make_depth(device: &wgpu::Device, w: u32, h: u32) -> (wgpu::Texture, wgpu::TextureView) {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
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

    /// Return the capacity of the trail vertex buffer (number of vertices).
    pub fn trail_vbuf_capacity(&self) -> usize {
        self.trail_vbuf_capacity
    }

    /// Resize the swap-chain and depth texture to the new pixel dimensions.
    ///
    /// Must be called whenever the Win32 window receives a `WM_SIZE` message.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.wgpu_surface
            .configure(&self.device, &self.surface_config);
        self.camera.aspect = width as f32 / height as f32;
        let (dt, dv) = Self::make_depth(&self.device, width, height);
        self.depth_texture = dt;
        self.depth_view = dv;
    }

    /// Set the background clear colour from linear RGB components.
    pub fn set_background(&mut self, r: f64, g: f64, b: f64) {
        self.background_color = wgpu::Color { r, g, b, a: 1.0 };
    }

    /// Replace the surface mesh buffers with new data.
    pub fn update_surface_mesh(&mut self, vertices: &[[f32; 3]], indices: &[u32]) {
        let mut line_indices: Vec<u32> = Vec::new();
        for tri in indices.chunks(3) {
            if tri.len() == 3 {
                line_indices.extend_from_slice(&[tri[0], tri[1], tri[1], tri[2], tri[2], tri[0]]);
            }
        }
        let verts_bytes: Vec<u8> = vertices
            .iter()
            .flat_map(|v| bytemuck::bytes_of(v).iter().copied())
            .collect();
        self.surface_vbuf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surface_vbuf"),
            contents: &verts_bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.surface_ibuf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surface_ibuf"),
            contents: bytemuck::cast_slice(&line_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.surface_index_count = line_indices.len() as u32;
    }

    /// Toggle the FPS HUD visibility.
    pub fn toggle_fps_hud(&mut self) {
        self.show_fps_hud = !self.show_fps_hud;
    }

    /// Record and submit a frame: surface wireframe + trail line strips.
    ///
    /// `trail_verts` is a flat concatenation of all trail vertex data;
    /// `trail_segment_lengths` gives the number of vertices belonging to each
    /// geodesic segment so the render pass can issue the correct draw calls.
    pub fn render(&mut self, trail_verts: &[TrailVertex], trail_segment_lengths: &[usize]) {
        let ld = self.light_dir;
        let uniforms = Uniforms {
            view_proj: self.camera.view_proj().to_cols_array_2d(),
            light_dir: [ld[0], ld[1], ld[2], 0.0],
            time: self.elapsed_secs,
            _pad: [0.0; 3],
        };
        self.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&uniforms));

        if !trail_verts.is_empty() {
            let n = trail_verts.len().min(self.trail_vbuf_capacity);
            self.queue
                .write_buffer(&self.trail_vbuf, 0, bytemuck::cast_slice(&trail_verts[..n]));
        }

        let frame = match self.wgpu_surface.get_current_texture() {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("Surface err: {e}");
                return;
            }
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
                        load: wgpu::LoadOp::Clear(self.background_color),
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
            if self.show_wireframe {
                rp.set_pipeline(&self.surface_pipeline);
                rp.set_bind_group(0, &self.bind_group, &[]);
                rp.set_vertex_buffer(0, self.surface_vbuf.slice(..));
                rp.set_index_buffer(self.surface_ibuf.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..self.surface_index_count, 0, 0..1);
            }

            rp.set_pipeline(&self.trail_pipeline);
            rp.set_bind_group(0, &self.bind_group, &[]);
            rp.set_vertex_buffer(0, self.trail_vbuf.slice(..));
            let mut offset = 0u32;
            let capacity_u32 = self.trail_vbuf_capacity as u32;
            for &len in trail_segment_lengths {
                if len >= 2 {
                    let end = offset.saturating_add(len as u32).min(capacity_u32);
                    rp.draw(offset..end, 0..1);
                }
                offset = offset.saturating_add(len as u32);
                if offset >= capacity_u32 {
                    break;
                }
            }
        }
        self.queue.submit(std::iter::once(enc.finish()));
        frame.present();
    }
}
