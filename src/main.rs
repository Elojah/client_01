extern crate image;
extern crate vulkano;
extern crate vulkano_win;
extern crate winit;

use std::sync::Arc;
use image::ImageBuffer;
use image::Rgba;
use vulkano::buffer::BufferUsage;
use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::command_buffer::CommandBuffer;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::DynamicState;
use vulkano::device::Device;
use vulkano::device::DeviceExtensions;
use vulkano::device::Features;
use vulkano::format::Format;
use vulkano::framebuffer::Framebuffer;
use vulkano::framebuffer::Subpass;
use vulkano::image::Dimensions;
use vulkano::image::StorageImage;
use vulkano::instance::Instance;
use vulkano::instance::PhysicalDevice;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::viewport::Viewport;
use vulkano::swapchain::{Swapchain, SurfaceTransform, PresentMode};
use vulkano::sync::GpuFuture;
use vulkano_win::VkSurfaceBuild;
use winit::EventsLoop;
use winit::WindowBuilder;

#[derive(Copy, Clone)]
struct Vertex {
	position: [f32; 2],
}
vulkano::impl_vertex!(Vertex, position);

fn main() {

	// #Required init graphic card
	let instance = {
		let extensions = vulkano_win::required_extensions();
		Instance::new(None, &extensions, None).expect("failed to create Vulkan instance")
	};

	let physical = PhysicalDevice::enumerate(&instance).next().expect("no device available");

	for family in physical.queue_families() {
		println!("Found a queue family with {:?} queue(s)", family.queues_count());
	}

	let queue_family = physical.queue_families()
	.find(|&q| q.supports_graphics())
	.expect("couldn't find a graphical queue family");
	let (device, mut queues) = {
		let device_ext = vulkano::device::DeviceExtensions {
			khr_swapchain: true,
			.. vulkano::device::DeviceExtensions::none()
		};
		Device::new(physical, physical.supported_features(), &device_ext,
		[(queue_family, 0.5)].iter().cloned()).expect("failed to create device")
	};
	let queue = queues.next().unwrap();


	// #Window
	let mut events_loop = EventsLoop::new();
	let surface = WindowBuilder::new().build_vk_surface(&events_loop, instance.clone()).unwrap();

	// #Swapchain
	let caps = surface.capabilities(physical)
	.expect("failed to get surface capabilities");

	let dimensions = caps.current_extent.unwrap_or([1280, 1024]);
	let alpha = caps.supported_composite_alpha.iter().next().unwrap();
	let format = caps.supported_formats[0].0;

	let (swapchain, images) = Swapchain::new(device.clone(), surface.clone(),
	caps.min_image_count, format, dimensions, 1, caps.supported_usage_flags, &queue,
	SurfaceTransform::Identity, alpha, PresentMode::Fifo, true, None)
	.expect("failed to create swapchain");


	// #Shaders
	mod vs {
		vulkano_shaders::shader!{
			ty: "vertex",
			path: "shaders/vertex.glsl"
		}
	}
	let vs = vs::Shader::load(device.clone())
	.expect("failed to create shader module");

	mod fs {
		vulkano_shaders::shader!{
			ty: "fragment",
			path: "shaders/fragment.glsl"
		}
	}
	let fs = fs::Shader::load(device.clone())
	.expect("failed to create shader module");

	// #Graphic pipeline

	let render_pass = Arc::new(vulkano::single_pass_renderpass!(device.clone(),
		attachments: {
			color: {
				load: Clear,
				store: Store,
				format: Format::R8G8B8A8Unorm,
				samples: 1,
			}
		},
		pass: {
			color: [color],
			depth_stencil: {}
		}
	).unwrap());


	let pipeline = Arc::new(GraphicsPipeline::start()
	// Defines what kind of vertex input is expected.
	.vertex_input_single_buffer::<Vertex>()
	// The vertex shader.
	.vertex_shader(vs.main_entry_point(), ())
	// Defines the viewport (explanations below).
	.viewports_dynamic_scissors_irrelevant(1)
	// The fragment shader.
	.fragment_shader(fs.main_entry_point(), ())
	// This graphics pipeline object concerns the first pass of the render pass.
	.render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
	// Now that everything is specified, we call `build`.
	.build(device.clone())
	.unwrap());

	// #Viewport
	let buf = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(),
	(0 .. 1024 * 1024 * 4).map(|_| 0u8))
	.expect("failed to create buffer");

	let vertex1 = Vertex { position: [-0.5, -0.5] };
	let vertex2 = Vertex { position: [ 0.0,  0.5] };
	let vertex3 = Vertex { position: [ 0.5, -0.25] };
	let vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(),
	vec![vertex1, vertex2, vertex3].into_iter()).unwrap();

	let dynamic_state = DynamicState {
		viewports: Some(vec![Viewport {
			origin: [0.0, 0.0],
			dimensions: [1024.0, 1024.0],
			depth_range: 0.0 .. 1.0,
		}]),
		.. DynamicState::none()
	};

	let image = StorageImage::new(device.clone(), Dimensions::Dim2d { width: 1024, height: 1024 },
	Format::R8G8B8A8Unorm, Some(queue.family())).unwrap();

	let framebuffer = Arc::new(Framebuffer::start(render_pass.clone())
	.add(image.clone()).unwrap()
	.build().unwrap());

	let command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family()).unwrap()
	.begin_render_pass(framebuffer.clone(), false, vec![[0.0, 0.0, 1.0, 1.0].into()])
	.unwrap()
	.draw(pipeline.clone(), &dynamic_state, vertex_buffer.clone(), (), ())
	.unwrap()
	.end_render_pass()
	.unwrap()
	.copy_image_to_buffer(image.clone(), buf.clone())
	.unwrap()
	.build()
	.unwrap();

	let finished = command_buffer.execute(queue.clone()).unwrap();
	finished.then_signal_fence_and_flush().unwrap()
	.wait(None).unwrap();

	let buffer_content = buf.read().unwrap();
	let image = ImageBuffer::<Rgba<u8>, _>::from_raw(1024, 1024, &buffer_content[..]).unwrap();
	image.save("triangle.png").unwrap();

	// # Event loop
	events_loop.run_forever(|event| {
		match event {
			winit::Event::WindowEvent { event: winit::WindowEvent::CloseRequested, .. } => {
				winit::ControlFlow::Break
			},
			_ => winit::ControlFlow::Continue,
		}
	});

	println!("Everything succeeded!")
}
