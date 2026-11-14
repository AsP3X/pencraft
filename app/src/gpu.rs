//! Runs a compiled shader on the GPU and reads the picture it drew back into memory.
//!
//! The app's window is drawn by eframe through wgpu; this borrows that same device. A shader is run once, over
//! a picture-sized target, and the result joins the other baked effect pictures, so the GPU isn't asked again
//! until the layer or the shader changes.

use std::borrow::Cow;
use std::collections::HashMap;
use std::future::Future;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use crate::app::lock;
use crate::shader::Program;

/// The triangle that covers the whole target; the shader file is the fragment half.
const VERTEX: &str = "
@vertex
fn main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
}
";
/// Pictures are premultiplied bytes that are blended as they are, like egui's own textures.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// Rows of a texture copied to a buffer start at multiples of this many bytes.
const ROW_ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
/// How long to wait for the GPU to hand a picture back.
const PATIENCE: Duration = Duration::from_secs(5);

/// The window's GPU device, with what has been built on it for shaders.
struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    vertex: wgpu::ShaderModule,
    /// A shader's pipeline and the layout of its inputs, by a hash of its source.
    pipelines: Mutex<HashMap<u64, Arc<Built>>>,
    /// Held while one picture is drawn and read back.
    busy: Mutex<()>,
}

struct Built {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

static GPU: OnceLock<Gpu> = OnceLock::new();

/// Keeps the window's device for running shaders. Called once at start-up; without it shaders aren't drawn.
pub fn install(device: wgpu::Device, queue: wgpu::Queue) {
    let vertex = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shader effect vertex"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(VERTEX)),
    });
    // A second call would only find the same device already kept.
    let _ = GPU.set(Gpu { device, queue, vertex, pipelines: Mutex::default(), busy: Mutex::default() });
}

/// Whether there is a GPU device to run shaders on.
#[must_use]
pub fn ready() -> bool {
    GPU.get().is_some()
}

/// Makes a GPU device of the test's own and keeps it for shaders, since a test has no window to borrow one
/// from. Returns whether there is a device now; a machine without any adapter has none.
// Human: `WGPU_BACKEND` and `WGPU_ADAPTER_NAME` pick the adapter, so a build server can run these tests on a
// software adapter (for example `WGPU_BACKEND=vulkan` with lavapipe, or `dx12` with WARP).
#[cfg(test)]
pub fn install_for_tests() -> bool {
    /// The value of a future that only has to be asked again until it is ready.
    fn wait<T>(future: impl Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        loop {
            if let Poll::Ready(value) = future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
                return value;
            }
            std::thread::yield_now();
        }
    }
    static TRIED: OnceLock<()> = OnceLock::new();
    TRIED.get_or_init(|| {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let Ok(adapter) = wait(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else { return };
        if let Ok((device, queue)) = wait(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
            install(device, queue);
        }
    });
    ready()
}

/// A picture a shader samples: premultiplied RGBA bytes, row by row from the top.
pub struct Input<'a> {
    /// The bytes, four a pixel.
    pub rgba: &'a [u8],
    /// Width in pixels.
    pub w: u32,
    /// Height in pixels.
    pub h: u32,
    /// Whether the picture mirrors past its edges; else its edge pixels repeat.
    pub mirror: bool,
}

/// The value of a future that is ready at once, as wgpu's are outside the browser.
fn at_once<T>(future: impl Future<Output = T>) -> Option<T> {
    let future = std::pin::pin!(future);
    match future.poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => Some(value),
        Poll::Pending => None,
    }
}

/// Runs `program` over a target of `size` pixels with the uniform bytes `block` and one input for each of its
/// samplers, in the order the shader declares them. Returns the target's premultiplied RGBA bytes.
///
/// # Errors
/// A sentence when there is no GPU, an input doesn't fit, or the GPU refuses the shader.
// Agent: BLOCKS the calling thread until the GPU has drawn (at most `PATIENCE`); MAIN-THREAD in practice.
pub fn run(program: &Program, size: (u32, u32), block: &[u8], inputs: &[Input<'_>]) -> Result<Vec<u8>, String> {
    let gpu = GPU.get().ok_or("this window has no GPU device to run shaders on")?;
    let limit = gpu.device.limits().max_texture_dimension_2d;
    if size.0 == 0 || size.1 == 0 || size.0 > limit || size.1 > limit {
        return Err(format!("a picture of {} × {} pixels doesn't fit the GPU", size.0, size.1));
    }
    // Human: without this scope a shader the GPU refuses would end the app; with it the refusal comes back
    // as an error and the layer is drawn unshaded.
    // Human: one picture at a time. Two threads waiting on the same device could each end the other's wait
    // before its own picture had come back, and the layer was then drawn without its shader.
    let _one_at_a_time = lock(&gpu.busy);
    let scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
    let drawn = gpu.draw(program, size, block, inputs);
    match at_once(scope.pop()) {
        Some(None) => drawn,
        Some(Some(error)) => Err(error.to_string()),
        None => Err("the GPU didn't say whether the shader ran".to_owned()),
    }
}

impl Gpu {
    /// The pipeline for `program`, built on first use.
    fn built(&self, program: &Program) -> Arc<Built> {
        let mut hasher = DefaultHasher::new();
        program.glsl.hash(&mut hasher);
        let key = hasher.finish();
        if let Some(built) = lock(&self.pipelines).get(&key) {
            return built.clone();
        }
        let samplers = program.uniforms.iter().filter(|u| u.ty == crate::shader::Type::Sampler).count();
        let mut entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        }];
        for index in 0..u32::try_from(samplers).unwrap_or(0) {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 1 + 2 * index,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 2 + 2 * index,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            });
        }
        let layout =
            self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("shader effect inputs"), entries: &entries });
        let fragment = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader effect"),
            source: wgpu::ShaderSource::Glsl { shader: Cow::Borrowed(&program.glsl), stage: wgpu::naga::ShaderStage::Fragment, defines: &[] },
        });
        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shader effect"),
            bind_group_layouts: &[Some(&layout)],
            ..Default::default()
        });
        let pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shader effect"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.vertex,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState { format: FORMAT, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let built = Arc::new(Built { pipeline, layout });
        lock(&self.pipelines).insert(key, built.clone());
        built
    }

    fn texture(&self, size: (u32, u32), usage: wgpu::TextureUsages) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shader effect picture"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage,
            view_formats: &[],
        })
    }

    fn draw(&self, program: &Program, size: (u32, u32), block: &[u8], inputs: &[Input<'_>]) -> Result<Vec<u8>, String> {
        let built = self.built(program);
        let uniforms = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shader effect uniforms"),
            size: u64::try_from(block.len().max(16)).map_err(|e| e.to_string())?,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&uniforms, 0, block);
        let mut views = Vec::new();
        let mut samplers = Vec::new();
        for input in inputs {
            let bytes = usize::try_from(u64::from(input.w) * u64::from(input.h) * 4).map_err(|e| e.to_string())?;
            if input.w == 0 || input.h == 0 || input.rgba.len() != bytes {
                return Err("a picture given to the shader has the wrong size".to_owned());
            }
            let texture = self.texture((input.w, input.h), wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                input.rgba,
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(input.w * 4), rows_per_image: Some(input.h) },
                wgpu::Extent3d { width: input.w, height: input.h, depth_or_array_layers: 1 },
            );
            views.push(texture.create_view(&wgpu::TextureViewDescriptor::default()));
            let address = if input.mirror { wgpu::AddressMode::MirrorRepeat } else { wgpu::AddressMode::ClampToEdge };
            samplers.push(self.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("shader effect sampler"),
                address_mode_u: address,
                address_mode_v: address,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }));
        }
        let mut entries = vec![wgpu::BindGroupEntry { binding: 0, resource: uniforms.as_entire_binding() }];
        for (index, (view, sampler)) in (0_u32..).zip(views.iter().zip(&samplers)) {
            entries.push(wgpu::BindGroupEntry { binding: 1 + 2 * index, resource: wgpu::BindingResource::TextureView(view) });
            entries.push(wgpu::BindGroupEntry { binding: 2 + 2 * index, resource: wgpu::BindingResource::Sampler(sampler) });
        }
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shader effect inputs"),
            layout: &built.layout,
            entries: &entries,
        });

        let target = self.texture(size, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC);
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let row = (size.0 * 4).next_multiple_of(ROW_ALIGN);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shader effect readback"),
            size: u64::from(row) * u64::from(size.1),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("shader effect") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shader effect"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&built.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &target, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(size.1) },
            },
            wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);

        let (sent, mapped) = std::sync::mpsc::channel();
        readback.map_async(wgpu::MapMode::Read, .., move |result| {
            // The receiver is only gone when this call has already given up waiting.
            let _ = sent.send(result);
        });
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(PATIENCE) }).map_err(|e| e.to_string())?;
        mapped.recv_timeout(PATIENCE).map_err(|_| "the GPU didn't hand the picture back")?.map_err(|e| e.to_string())?;
        let data = readback.get_mapped_range(..).map_err(|e| e.to_string())?;
        let (row, width) = (usize::try_from(row).map_err(|e| e.to_string())?, usize::try_from(size.0 * 4).map_err(|e| e.to_string())?);
        Ok(data.chunks(row).flat_map(|line| line.get(..width).unwrap_or(&[]).iter().copied()).collect())
    }
}
