#[cfg(feature = "vulkan")]
extern crate gfx_backend_vulkan as back;
#[cfg(feature = "dx12")]
extern crate gfx_backend_dx12 as back;
#[cfg(feature = "metal")]
extern crate gfx_backend_metal as back;

extern crate gfx_hal as hal;

extern crate winit;

use hal::{
    format as f, image as i,
    window::{ self, SwapchainConfig },
    Instance, Surface, Device,
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

    let (capabilities, formats, presentation_modes) = surface.compatibility(&adapter.physical_device);

    let format = formats
        .map_or(f::Format::Rgba8Srgb, |formats| {
            formats
                .iter()
                .find(|format| format.base_format().1 == f::ChannelType::Srgb)
                .map(|format| *format)
                .unwrap_or(formats[0])
        });
    
    let extent = match capabilities.current_extent {
        Some(extent) => extent,
        None => {
            let window_size = window.get_inner_size().unwrap().to_physical(window.get_hidpi_factor());
            let mut extent = hal::window::Extent2D { width: window_size.width as _, height: window_size.height as _ };

            extent.width = extent.width
                .max(capabilities.extents.start.width)
                .min(capabilities.extents.end.width);
            extent.height = extent.height
                .max(capabilities.extents.start.height)
                .min(capabilities.extents.end.height);
            
            extent
        }
    };

    let presentation_mode = presentation_modes
        .iter()
        .find(|&mode| *mode == window::PresentMode::Immediate)
        .map(|mode| *mode)
        .unwrap_or(window::PresentMode::Fifo);

    let swap_config = SwapchainConfig::new()
        .with_color(format)
        .with_image_count(capabilities.image_count.start)
        .with_image_usage(i::Usage::COLOR_ATTACHMENT);

    let (swapchain, backbuffer) = device.create_swapchain(
        &mut surface,
        swap_config,
        None,
        &extent,
    );
}
