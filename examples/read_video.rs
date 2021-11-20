//! Example that reads a video and displays it to a window.

use std::{
    fs::File,
    io::BufReader,
    iter,
    num::NonZeroU32,
    thread,
    time::{Duration, Instant},
};

use pollster::block_on;
use rodio::{Decoder, OutputStream, Source};
use vp9::{ivf::IvfDemuxer, Frame, Vp9Decoder};
use wgpu::*;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() -> anyhow::Result<()> {
    let file = File::open("assets/video.ivf")?;
    let mut demuxer = IvfDemuxer::new(BufReader::new(file))?;
    let mut decoder = Vp9Decoder::new();
    let mut video_frame = Frame::new(demuxer.header().width, demuxer.header().height);

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("VP9 Example")
        .with_inner_size(PhysicalSize::new(
            demuxer.header().width,
            demuxer.header().height,
        ))
        .with_resizable(false)
        .build(&event_loop)?;

    let instance = Instance::new(Backends::all());
    let surface = unsafe { instance.create_surface(&window) };
    let adapter = block_on(instance.request_adapter(&RequestAdapterOptions {
        compatible_surface: Some(&surface),
        power_preference: PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .unwrap();
    let (device, queue) =
        block_on(adapter.request_device(&DeviceDescriptor::default(), None)).unwrap();
    let target_format = TextureFormat::Bgra8Unorm;

    let bg_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler {
                    filtering: true,
                    comparison: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    multisampled: false,
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    multisampled: false,
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    multisampled: false,
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bg_layout],
        push_constant_ranges: &[],
    });

    let shader = device.create_shader_module(&include_wgsl!("shader.wgsl"));

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[ColorTargetState {
                format: target_format,
                blend: None,
                write_mask: ColorWrites::all(),
            }],
        }),
    });

    let width = demuxer.header().width;
    let height = demuxer.header().height;
    let y_texture = device.create_texture(&TextureDescriptor {
        label: None,
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::R8Unorm,
        usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
    });
    let u_texture = device.create_texture(&TextureDescriptor {
        label: None,
        size: Extent3d {
            width: width / 2,
            height: height / 2,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::R8Unorm,
        usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
    });
    let v_texture = device.create_texture(&TextureDescriptor {
        label: None,
        size: Extent3d {
            width: width / 2,
            height: height / 2,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::R8Unorm,
        usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
    });

    let sampler = device.create_sampler(&SamplerDescriptor {
        label: None,
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: FilterMode::Nearest,
        lod_min_clamp: 0.,
        lod_max_clamp: 100.,
        compare: None,
        anisotropy_clamp: None,
        border_color: None,
    });

    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &bg_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::Sampler(&sampler),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::TextureView(&y_texture.create_view(&Default::default())),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::TextureView(&u_texture.create_view(&Default::default())),
            },
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::TextureView(&v_texture.create_view(&Default::default())),
            },
        ],
    });

    surface.configure(
        &device,
        &SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: target_format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: PresentMode::Fifo,
        },
    );

    // Start playing sound as we start the video.
    // TODO: synchronize sound with the video if
    // we lag behind
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let audio = BufReader::new(File::open("assets/audio.ogg")?);
    let source = Decoder::new(audio)?;
    stream_handle.play_raw(source.convert_samples())?;

    let start = Instant::now();
    let time_base = demuxer.header().time_base_num as f64 / demuxer.header().time_base_denom as f64;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }

        let target = surface.get_current_frame().unwrap();

        let data_frame = match demuxer.next_frame().unwrap() {
            Some(f) => f,
            None => {
                *control_flow = ControlFlow::Exit;
                return;
            }
        };

        let target_time = data_frame.timestamp as f64 * time_base;
        let current_time = start.elapsed().as_secs_f64();
        if current_time < target_time {
            thread::sleep(Duration::from_secs_f64(target_time - current_time));
        }

        decoder.decode(data_frame.data).unwrap();

        while decoder.next_frame(&mut video_frame).unwrap() {}

        // Write frame data to the GPU
        queue.write_texture(
            ImageCopyTexture {
                texture: &y_texture,
                mip_level: 0,
                origin: Default::default(),
                aspect: TextureAspect::All,
            },
            video_frame.y_plane(),
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(NonZeroU32::new(width).unwrap()),
                rows_per_image: Some(NonZeroU32::new(height).unwrap()),
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.write_texture(
            ImageCopyTexture {
                texture: &u_texture,
                mip_level: 0,
                origin: Default::default(),
                aspect: TextureAspect::All,
            },
            video_frame.u_plane(),
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(NonZeroU32::new(width / 2).unwrap()),
                rows_per_image: Some(NonZeroU32::new(height / 2).unwrap()),
            },
            Extent3d {
                width: width / 2,
                height: height / 2,
                depth_or_array_layers: 1,
            },
        );
        queue.write_texture(
            ImageCopyTexture {
                texture: &v_texture,
                mip_level: 0,
                origin: Default::default(),
                aspect: TextureAspect::All,
            },
            video_frame.v_plane(),
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(NonZeroU32::new(width / 2).unwrap()),
                rows_per_image: Some(NonZeroU32::new(height / 2).unwrap()),
            },
            Extent3d {
                width: width / 2,
                height: height / 2,
                depth_or_array_layers: 1,
            },
        );

        let mut encoder = device.create_command_encoder(&Default::default());

        let view = target.output.texture.create_view(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit(iter::once(encoder.finish()));
    });
}
