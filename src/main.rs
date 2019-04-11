extern crate vulkano;
use vulkano::instance::Instance;
use vulkano::instance::InstanceExtensions;
use vulkano::instance::PhysicalDevice;
use vulkano::device::Device;
use vulkano::device::DeviceExtensions;
use vulkano::device::Features;
use vulkano::buffer::BufferUsage;
use vulkano::buffer::CpuAccessibleBuffer;

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
	let _queue = queues.next().unwrap();

	let data = 12;
	let _buffer = CpuAccessibleBuffer::from_data(device.clone(), BufferUsage::all(),
		data).expect("failed to create buffer");
}
