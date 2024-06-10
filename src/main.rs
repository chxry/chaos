use std::{mem, slice};
use std::time::Instant;
use std::f32::consts::PI;
use winit::event_loop::EventLoop;
use winit::event::{Event, WindowEvent, DeviceEvent, MouseButton, MouseScrollDelta};
use winit::window::WindowBuilder;
use wgpu::util::DeviceExt;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::filter::LevelFilter;
use glam::{Vec2, Vec3, Vec4, Mat4};
use rand::Rng;
use shared::{PARTICLES, TRAIL_LENGTH, Uniform};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result {
  tracing_subscriber::registry()
    .with(tracing_subscriber::fmt::layer().with_filter(LevelFilter::INFO))
    .init();
  let event_loop = EventLoop::new()?;
  let window = &WindowBuilder::new().build(&event_loop)?;

  let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
  let surface = instance.create_surface(window)?;
  let adapter = instance
    .request_adapter(&wgpu::RequestAdapterOptions {
      power_preference: wgpu::PowerPreference::HighPerformance,
      compatible_surface: Some(&surface),
      force_fallback_adapter: false,
    })
    .await
    .unwrap();
  let (device, queue) = adapter
    .request_device(
      &wgpu::DeviceDescriptor {
        required_limits: wgpu::Limits::default(),
        required_features: wgpu::Features::VERTEX_WRITABLE_STORAGE,
        label: None,
      },
      None,
    )
    .await?;

  let shader = device.create_shader_module(wgpu::include_spirv!(env!("shaders.spv")));

  let particles_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    entries: &[wgpu::BindGroupLayoutEntry {
      binding: 0,
      visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX,
      ty: wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Storage { read_only: false },
        has_dynamic_offset: false,
        min_binding_size: None,
      },
      count: None,
    }],
    label: None,
  });
  let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    entries: &[wgpu::BindGroupLayoutEntry {
      binding: 0,
      visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX,
      ty: wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: None,
      },
      count: None,
    }],
    label: None,
  });
  let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    bind_group_layouts: &[&particles_layout, &uniform_layout],
    push_constant_ranges: &[],
    label: None,
  });

  let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
    layout: Some(&pipeline_layout),
    module: &shader,
    entry_point: "compute",
    compilation_options: wgpu::PipelineCompilationOptions::default(),
    label: None,
  });

  let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    layout: Some(&pipeline_layout),
    vertex: wgpu::VertexState {
      module: &shader,
      entry_point: "vert",
      buffers: &[],
      compilation_options: wgpu::PipelineCompilationOptions::default(),
    },
    fragment: Some(wgpu::FragmentState {
      module: &shader,
      entry_point: "frag",
      targets: &[Some(wgpu::ColorTargetState {
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
      })],
      compilation_options: wgpu::PipelineCompilationOptions::default(),
    }),
    primitive: wgpu::PrimitiveState {
      topology: wgpu::PrimitiveTopology::PointList,
      ..Default::default()
    },
    depth_stencil: None,
    multisample: wgpu::MultisampleState::default(),
    multiview: None,
    label: None,
  });

  let mut rng = rand::thread_rng();
  let mut particles = vec![];
  for _ in 0..PARTICLES {
    let pos = Vec4::new(
      rng.gen_range(-0.01..0.01),
      rng.gen_range(-0.01..0.01),
      rng.gen_range(-0.01..0.01),
      0.0,
    );
    for _ in 0..TRAIL_LENGTH {
      particles.push(pos);
    }
  }

  let particles_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    contents: cast_slice(&particles),
    usage: wgpu::BufferUsages::STORAGE,
    label: None,
  });
  let particles_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    layout: &particles_layout,
    entries: &[wgpu::BindGroupEntry {
      binding: 0,
      resource: particles_buf.as_entire_binding(),
    }],
    label: None,
  });

  let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
    size: mem::size_of::<Uniform>() as _,
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
    label: None,
  });
  let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    layout: &uniform_layout,
    entries: &[wgpu::BindGroupEntry {
      binding: 0,
      resource: uniform_buf.as_entire_binding(),
    }],
    label: None,
  });

  let mut camera = Camera::new();
  let mut last_frame = Instant::now();

  event_loop.run(move |event, elwt| match event {
    Event::WindowEvent { event, .. } => match event {
      WindowEvent::Resized(size) => {
        surface.configure(
          &device,
          &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
          },
        );
      }
      WindowEvent::RedrawRequested => {
        let delta_time = last_frame.elapsed().as_secs_f32();
        last_frame = Instant::now();
        let surface = surface.get_current_texture().unwrap();
        let surface_view = surface
          .texture
          .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        queue.write_buffer(
          &uniform_buf,
          0,
          cast(&Uniform {
            cam: camera.get_matrix(
              surface.texture.width() as f32 / surface.texture.height() as f32,
              delta_time,
            ),
            delta_time,
          }),
        );

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
          timestamp_writes: None,
          label: None,
        });
        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &particles_bind_group, &[]);
        compute_pass.set_bind_group(1, &uniform_bind_group, &[]);
        for _ in 0..10 {
          compute_pass.dispatch_workgroups(PARTICLES, 1, 1);
        }
        drop(compute_pass);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
          color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &surface_view,
            resolve_target: None,
            ops: wgpu::Operations {
              load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
              store: wgpu::StoreOp::Store,
            },
          })],
          depth_stencil_attachment: None,
          timestamp_writes: None,
          occlusion_query_set: None,
          label: None,
        });
        render_pass.set_pipeline(&render_pipeline);
        render_pass.set_bind_group(0, &particles_bind_group, &[]);
        render_pass.set_bind_group(1, &uniform_bind_group, &[]);
        render_pass.draw(0..(PARTICLES * TRAIL_LENGTH), 0..1);
        drop(render_pass);

        queue.submit([encoder.finish()]);
        surface.present();
      }
      WindowEvent::MouseInput {
        button: MouseButton::Left,
        state,
        ..
      } => camera.dragging = state.is_pressed(),
      WindowEvent::MouseWheel { delta, .. } => {
        camera.zoom_vel = -match delta {
          MouseScrollDelta::LineDelta(_, y) => y,
          MouseScrollDelta::PixelDelta(p) => p.y as f32,
        } * 200.0;
      }
      WindowEvent::CloseRequested => elwt.exit(),
      _ => {}
    },
    Event::DeviceEvent { event, .. } => match event {
      DeviceEvent::MouseMotion { delta } if camera.dragging => {
        camera.angles_vel = Vec2::new(delta.0 as _, -delta.1 as _) * 2.0;
      }
      _ => {}
    },
    Event::AboutToWait => window.request_redraw(),
    _ => {}
  })?;
  Ok(())
}

struct Camera {
  angles: Vec2,
  angles_vel: Vec2,
  zoom: f32,
  zoom_vel: f32,
  dragging: bool,
}

impl Camera {
  fn new() -> Self {
    Camera {
      angles: Vec2::ZERO,
      angles_vel: Vec2::ZERO,
      zoom: 25.0,
      zoom_vel: 0.0,
      dragging: false,
    }
  }

  fn get_matrix(&mut self, aspect_ratio: f32, delta_time: f32) -> Mat4 {
    self.angles += self.angles_vel * delta_time;
    self.angles.y = self.angles.y.clamp(-PI / 2.0 + 0.1, PI / 2.0 - 0.1);
    self.angles_vel *= 0.9;
    self.zoom += self.zoom_vel * delta_time;
    self.zoom = self.zoom.max(0.0);
    self.zoom_vel *= 0.9;

    let focus = Vec3::new(0.0, 0.0, 20.0);
    Mat4::perspective_infinite_lh(1.4, aspect_ratio, 0.01)
      * Mat4::look_at_lh(
        focus
          + self.zoom
            * Vec3::new(
              self.angles.x.cos() * self.angles.y.cos(),
              self.angles.y.sin(),
              self.angles.x.sin() * self.angles.y.cos(),
            ),
        focus,
        Vec3::Y,
      )
  }
}

fn cast_slice<T>(t: &[T]) -> &[u8] {
  // safety: u8 is always valid
  unsafe { slice::from_raw_parts(t.as_ptr() as _, mem::size_of_val(t)) }
}

fn cast<T>(t: &T) -> &[u8] {
  cast_slice(slice::from_ref(t))
}
