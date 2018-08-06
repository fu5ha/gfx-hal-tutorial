#[cfg(feature = "vulkan")]
extern crate gfx_backend_vulkan as back;
#[cfg(feature = "dx12")]
extern crate gfx_backend_dx12 as back;
#[cfg(feature = "metal")]
extern crate gfx_backend_metal as back;

extern crate gfx_hal as hal;

extern crate winit;

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
    
    let instance = back::Instance::create("voxel-renderer", 1);
}
