extern crate vulkano;
use vulkano::instance::Instance;
use vulkano::instance::InstanceExtensions;
use vulkano::instance::PhysicalDevice;
use vulkano::device::Device;
use vulkano::device::DeviceExtensions;
use vulkano::device::Features;
use vulkano::buffer::BufferUsage;
use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::CommandBuffer;
use vulkano::sync::GpuFuture;
use vulkano::pipeline::ComputePipeline;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;

use std::sync::Arc;

struct _Test {
	foo: u32,
	bar: bool,
}

fn main() {

	let instance = Instance::new(None, &InstanceExtensions::none(), None)
	.expect("failed to create instance");
	let physical = PhysicalDevice::enumerate(&instance).next().expect("no device available");

	for family in physical.queue_families() {
		println!("Found a queue family with {:?} queue(s)", family.queues_count());
	}

	let queue_family = physical.queue_families()
	.find(|&q| q.supports_graphics())
	.expect("couldn't find a graphical queue family");
	let (device, mut queues) = {
		Device::new(physical, &Features::none(), &DeviceExtensions::none(),
			[(queue_family, 0.5)].iter().cloned()).expect("failed to create device")
	};
	let queue = queues.next().unwrap();

	let source_content = 0 .. 64;
	let source = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), source_content)
	.expect("failed to create buffer");

	let dest_content = (0 .. 64).map(|_| 0);
	let dest = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), dest_content)
	.expect("failed to create buffer");

	let command_buffer_prev = AutoCommandBufferBuilder::new(device.clone(), queue.family()).unwrap()
	.copy_buffer(source.clone(), dest.clone()).unwrap()
	.build().unwrap();

	let finished_prev = command_buffer_prev.execute(queue.clone()).unwrap();
	finished_prev.then_signal_fence_and_flush().unwrap()
	.wait(None).unwrap();

	let src_content = source.read().unwrap();
	let dest_content = dest.read().unwrap();
	assert_eq!(&*src_content, &*dest_content);

	mod mult_shader {
		vulkano_shaders::shader!{
			ty: "compute",
			path: "shaders/mult.glsl"
		}
	}

	let shader = mult_shader::Shader::load(device.clone())
	.expect("failed to create shader module");

	let compute_pipeline = Arc::new(ComputePipeline::new(device.clone(), &shader.main_entry_point(), &())
	.expect("failed to create compute pipeline"));


	let set = Arc::new(PersistentDescriptorSet::start(compute_pipeline.clone(), 0)
	.add_buffer(dest.clone()).unwrap()
	.build().unwrap()
	);

	let command_buffer = AutoCommandBufferBuilder::new(device.clone(), queue.family()).unwrap()
	.dispatch([1024, 1, 1], compute_pipeline.clone(), set.clone(), ()).unwrap()
	.build().unwrap();

	let finished = command_buffer.execute(queue.clone()).unwrap();
	finished.then_signal_fence_and_flush().unwrap()
	.wait(None).unwrap();

	let content = dest.read().unwrap();
	for (n, val) in content.iter().enumerate() {
		assert_eq!(*val, n as u32 * 12);
	}

	println!("Everything succeeded!")
}
