#[cfg(feature = "vulkan")]
extern crate gfx_backend_vulkan as back;
#[cfg(feature = "dx12")]
extern crate gfx_backend_dx12 as back;
#[cfg(feature = "metal")]
extern crate gfx_backend_metal as back;

extern crate gfx_hal as hal;

extern crate winit;

use hal::{
    Instance, Surface
};

fn main() {
    let mut events_loop = winit::EventsLoop::new();

    let wb = winit::WindowBuilder::new()
        .with_dimensions(
            winit::dpi::LogicalSize::from_physical(
                winit::dpi::PhysicalSize {
                    width: 1280.0,
                    height: 720.0,
                },
                1.0
            )
        )
        .with_title("voxel-renderer");
    
    let window = wb.build(&events_loop).unwrap();
    
    // Create instance
    let instance = back::Instance::create("voxel-renderer", 1);
    // Acquire surface
    let mut surface = instance.create_surface(&window);

    // Enumerate adapters and pick one that works for us
    let mut adapters = instance.enumerate_adapters();

    for adapter in &adapters {
        println!("{:?}", adapter.info);
    }

    let mut adapter = adapters.remove(0);

    let (mut device, mut queue_group) = adapter
        .open_with::<_, hal::Graphics>(1, |family| surface.supports_queue_family(family))
        .unwrap();
}
