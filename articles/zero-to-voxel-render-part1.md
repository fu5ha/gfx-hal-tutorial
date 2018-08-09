---
layout: Post
title: "Zero to Voxel Renderer with gfx-hal: Part 1 - A Triangle"
date: 2018-08-05
featured: true
---

This series of blog posts is going to walk you through how to go from no knowledge of computer graphics programming to having a working voxel-style renderer in the vein of Minecraft (with maybe some extra bells and whistles) using `gfx-hal`. I won't assume any prior knowledge of OpenGL, Vulkan, etc., however I will assume that you have a working knowlege of the Rust programming language. If not, [go read the book](https://doc.rust-lang.org/book/2018-edition/index.html)! It is excellent, and Rust is an amazing language that everyone should know.

# Introduction

`gfx-hal` (which I'll now refer to as just `hal`) is a Rust crate (library) that is meant to allow the user to use the new, shiny, low-level, "explicit" graphics APIs in a cross-platform manner with low overhead. Its api is a Rustified version of the Vulkan API. It is an "abstraction layer" over the various low level graphics APIs. Since its api is so heavily based on Vulkan, its most frictionless backend target is Vulkan, but it also provides a low (as much as is possible) overhead translation to Metal and DirectX12, (also OpenGL and DirectX11, but these are primarily fallback targets).

*Side note: `gfx-hal` is not connected to the old `gfx-rs/gfx` crate that existed in the `pre-ll` days in anything other than semantics; it is a complete rewrite. So, knowledge from there won't carry directly over here and vice-versa.*

So, what does a low level graphics API do? Well, that has somewhat changed over the years as OpenGL has evolved and now given way to the next-generation graphics APIs. It used to be that OpenGL (or early versions of DirectX) were very much "fixed function" black boxes. The developer would pass them a set of vertices, information about the camera and lights, and tell it what style of shading they wanted to be applied, and the graphics API would do all the work of making that happen for them. However, that quickly became limiting as computer graphics became more and more complex and developers wanted more control over the details of how every aspect of the computations that were going on behind the scenes. Ultimately, that has left us with the next gen, "explicit" APIs (Vulkan, DirectX12, Metal), so called because they allow the developer unprecedented control over every little aspect of what the hardware is doing. This means that on the one hand, we can in theory squeak out every little oodle of performance from the hardware by specializing it specifically for our application's needs. On the other, though, it means that the developer needs to have knowledge of and understand every aspect of the device and API in order to take proper advantage of that control, and that the code is *very* wordy.

The goal of this first post is to be drawing a triangle, and we'll slowly build concepts and work up from there to create the rest of our renderer.

# Setting Up

If you're on Windows and intend to use the Vulkan backend as a target, installing the [LunarG Vulkan SDK](https://vulkan.lunarg.com/) will be helpful for debugging later.

In addition, it's a good idea for any OS and backend to install [RenderDoc](https://renderdoc.org/), which is like a debugger for your graphics card, and is invaluable in trying to sort out weird bugs that will inevitably pop up when doing this sort of work.

The first thing we're going to do is actually to download the `gfx` git repository and build the docs for `hal`. This is necessary because there is no published documentation on `docs.rs` or another public site. In the future, when you see a capitalized and code-formatted type like `Device` or `ChannelType`, you can look them up in the docs to find more information. To do this, we can first clone the `gfx` repo like so

```sh
git clone https://github.com/gfx-rs/gfx
```

Then we'll move into the `hal` directory and build the documentation like so (note that the `--open` flag should open the generated documentation automatically in your default web browser)

```sh
cd gfx/src/hal
cargo doc --open
```

The next thing we'll do is get our workspace set up. This is fairly simple, assuming you have Rust and Cargo installed:

```sh
cargo new --bin voxel-renderer
cd voxel renderer
```

Next we're going to edit our `Cargo.toml` to install the necessary dependencies and add some "features" that will allow us to choose which "actual" graphics backend to use at compile time.

```toml
[package]
name = "voxel-renderer"
version = "0.1.0"
publish = false

[features]
default = []
metal = ["gfx-backend-metal"]
dx12 = ["gfx-backend-dx12"]
vulkan = ["gfx-backend-vulkan"]

[dependencies]
winit = "0.16"
glsl-to-spirv = "0.1.4"
lazy_static = "1.1.0"
gfx-hal = { git = "https://github.com/gfx-rs/gfx", version = "0.1" }

[target.'cfg(not(target_os = "macos"))'.dependencies.gfx-backend-vulkan]
git = "https://github.com/gfx-rs/gfx"
version = "0.1"
optional = true

[target.'cfg(target_os = "macos")'.dependencies.gfx-backend-metal]
git = "https://github.com/gfx-rs/gfx"
version = "0.1"
optional = true

[target.'cfg(windows)'.dependencies.gfx-backend-dx12]
git = "https://github.com/gfx-rs/gfx"
version = "0.1"
optional = true
```

The first thing you'll notice is that we have three features, 'metal', 'dx12', and 'vulkan'. We can enable these at compile time with Cargo's `--features <features>` flag. We then pull in the `winit`, `lazy-static`, and `glsl-to-spirv` crates, which will be helpers to allow us to create a window in a cross platform manner and to convert glsl code to SPIR-V assembly dynamically. Next, we add `gfx-hal` via git, and finally pull in the dependencies for each of the backends based on the features we specified earlier and the platform being compiled on.

Now we can add the following to `src/main.rs`

```rust
#[cfg(feature = "vulkan")]
extern crate gfx_backend_vulkan as back;
#[cfg(feature = "dx12")]
extern crate gfx_backend_dx12 as back;
#[cfg(feature = "metal")]
extern crate gfx_backend_metal as back;

extern crate gfx_hal as hal;

extern crate winit;

fn main() {
}
```

Here we import our selected backend crate based on the feature that is enabled, and also import the `winit` and `gfx_hal` crates. Alright! Now we are ready to get started actually writing some code.

## Getting RLS to work with hal

Well... not quite. If you use the Rust Language Server (with VSCode), there's a little tweak we need to use to get proper error reporting. If you create a directory called `.vscode` in the root of your VSCode workspace and then add a `settings.json` file, you can change preferences in a workplace-specific context. For us, we'll add

```json
{
    "rust.features": ["vulkan"]
}
```

and substitute "vulkan" for whichever backend you're using. Yay! Now we're really ready.

# Creating a Window

In days past (and still today), creating a window in a cross platform manner in which to draw our graphics application was a big and painful ordeal. Thankfully, in Rust we have the excellent `winit` crate which makes this super easy.

```rust
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
}
```

First, we create an `EventsLoop`, which provides a way to retrieve events from the system and from the windows that were registered to the events loop. An `EventsLoop` can be seen more or less as a "context". Calling `EventsLoop::new()` initializes everything that will be required to create windows. For example on Linux creating an events loop opens a connection to the X or Wayland server.

Next, we create a "WindowBuilder" which allows us to change various parameters about the window we want to create in an ergonomic way. The only things we use are the `with_dimensions` and `with_title` methods. `with_dimensions` takes a `LogicalSize` struct, which represents the "logical" size of the window, or the number of pixels taking into account DPI scaling (for example, 1280.0 "logical" pixels on a Retina MBP screen would actually be 2275.5 "physical" pixels on the screen). Since we want to have a window with the same number of physical pixels no matter the dpi of the screen, we instead use a `PhysicalSize` struct an convert it to a `LogicalSize`.

Then we actually create the window using `wb.build` and passing in the `events_loop` we created earlier. Hey Presto, we have a window! Next, we need to start the process of actually being able to draw to it.

# Instance, Surface, Adapter

The next thing we'll do is create an `Instance`. This can be seen as the equivalent to a Vulkan Instance, and essentially represents backend-specific, per-application state. We won't be using it directly very much, but we do need to create it. Just below our window, we can add

```rust
let instance = back::Instance::create("voxel-renderer", 1);
```

Now that we have our instance and window, we can create a "surface" to draw on. Since `hal` does not interface directly with any specific underlying windowing system, it instead needs some sort of abstract type that will represent a "thing" that it can draw on. This surface will be backed by the window we just created.

```rust
let mut surface = instance.create_surface(&window);
```

Now we get to an interesting part. At this point we need to pick which of the compatible "adapters" (in Vulkan terminology, "physical devices"), which represent either pieces of hardware or software implementations of the backend spec which are present in the current system. To get a list of these, we ask the instance to enumerate its adapters. This will return a `Vec` of the compatible `Adapter`s, which we can then iterate over and pick from. If you wish, you can query this adapter to find its capabilities and make sure it supports the things that your application needs. In our case, we'll just print out the info for each device and then just choose the first one in the list.

First, though, we need to add a `use` statement before our `main` function. We'll bring the `hal::Instance` trait into scope, which will let us access the `enuerate_adapters` method on our instance.

```rust
use hal::{
    Instance,
};
```

Now we can add after our instance creation:

```rust
let mut adapters = instance.enumerate_adapters();

for adapter in &adapters {
    println!("{:?}", adapter.info);
}

let mut adapter = adapters.remove(0);
```

Alright! Now we have selected our adapter and are ready to start creating a "device."

# Device and Queues

In `hal`, a `Device` represents "logical" connections to an adapter. This `Device` is the primary interface for interacting with the physical adapter. In the future when we want to actually tell this device to do things for us, we will submit commands to queues provided by the device. These commands will be executed asynchronously, as the `Device` is able to process them. We can also affect the order and synchronization of these commands with various methods.

For now, the important thing to understand is how these queues are structured. A `Device` will expose one or more `QueueFamily`s, and each `QueueFamily` will expose one or more `CommandQueue`s, which is what we can then submit commands to. A `QueueFamily` is a group of queues that exhibit the same *capabilities*. The capabilities a family can support are `Graphics`, `Compute`, and `Transfer`. `Graphics` and `Compute` are quite self explanatory, supporting drawing and compute pipeline operations respectively, and `Transer` capability represents the ability to transfer memory from the host (CPU) to device (GPU) and around on the device itself.

The types of queues are `General`, `Graphics`, `Compute`, and `Transfer`. `General` queues can support all operations, `Graphics` queues can support `Graphics` and `Transfer` operations, `Compute` queues can support `Compute` and `Transfer` operations, and `Transfer` queues only support `Transfer` operations.

`hal` groups queues into `QueueGroup`s, which are like `QueueFamily`s, and in fact do represent a group of queues from a specific `QueueFamily`, however a `QueueGroup` is strongly typed and ensures that all the queues inside it support a specific set of `Capability`s and are from a specific `QueueFamily`.

Phew! Got all that? It's alright if you don't get everything yet, and don't be afraid to go back and review some, it's a lot to take in. One more thing to talk about before we jump back into the code is that in addition to checking if a `QueueFamily` supports certain capabilities, we can also check if some things support working with a certain `QueueFamily`. In this case, we want to check that our `QueueFamily` supports being able to present images to the `Surface` that we created earlier.

With `hal`, we can create a `Device` and `QueueGroup` at the same time by using the `open_with` method on our adapter. Technically this is sort of a shortcut for some intermediate steps, but it makes creating a `Device` and `QueueGroup` easy if we only need one queue family. We'll revisit the manual way later in the series. This method takes two type parameters, one of which is for the function that gets passed in and which we can tell `rustc` to figure out for us, and the other which we must define ourselves, and it represents the type of capability that we want our queue group to have. In our case, we want to use the `Graphics` capability (which includes both `Graphics` and `Transfer` capabilities). The arguments to `open_with` are first the number of queues we want in our `QueueGroup` and then a function (in this case a closure) that will act as a "filter" for queue families. The argument is a `QueueFamily` and it should return a `bool` meaning whether or not this family is acceptable. We will use this to make sure the family supports presenting to our surface.

Alright! Into the code. First we need to add to our `use` statement and add the `Surface` trait to the scope so that we can use the `supports_queue_family` method.

```rust
use hal::{
    Instance, Surface,
};
```

And now we create our device and queue group. We'll create a queue group with only one queue inside it for now.

```rust
let (mut device, mut queue_group) = adapter
    .open_with::<_, hal::Graphics>(1, |family| surface.supports_queue_family(family))
    .unwrap();
```

Well, for all that explanation, the code wasn't so bad at all!

# Swapchain

The next thing we'll do is create what's called a "swapchain". The reason we do this is to integrate drawing with the window and surface that we created earlier. In Vulkan (and `hal`), we could in theory create an application that displays nothing at all and simply saves its results out or does nothing at all with them. But, if we want to display something, we need to create a set of special images (which are behind the scenes simply buffers of memory), together called a "swapchain" into which we can render things if we want to actually display them to the screen. I'm going to take a detour at this point to explain the bigger picture of what we are doing as graphics programmers with these low level APIs.

## The Graphics Pipeline

Remember before how I said that early graphics APIs were basically fixed-function black boxes into which you would drive some input and get a result magically out the other side? Well, now we're going to jump into each of the steps involved, and see where and how we can change that magic box. Fundamentally, even the new, explicit, low-level graphics APIs are still essentially ways to set up the state of the hardware device (graphics card) and then tell it when and how to execute things.

The graphics pipeline is the sequence of operations that take the inputs into the black box and turn them all the way into the pixel colors of the output image. The basic goal of a modern graphics pipeline is to turn a series of **primitives** (almost always **triangles**) into a set of **fragments**, which can be thought of as analogous to **pixels** in an image, and then to compute an output color for each of those fragments. When we later make a "draw" call, what is happening is that we're telling the graphics card to take the currently bound state of the graphics pipeline and execute each stage to produce an output. We can run multiple draw calls on the same pipeline, say, one per object in our scene (this is an inefficient way to do it, but it does work), and slowly build up the final output image. With `hal`, we can also create multiple different ways to set up the pipeline, which can draw different kinds of objects in different ways. We can even assemble multiple runs (draw calls) of different pipelines into "render passes," and then use the output of one render pass as part of the input for another render pass. 

The inputs to the pipeline can include 

* **Vertices**, formatted into vertex buffers, which hold data about each vertex that can include position, color, texture coordinates, normals, and more.
* **Indices** into the vertex buffer, allowing the reuse of vertex information. Since the pipeline only understands how to work with triangles, rendering a square would actually require six vertices to be passed to the gpu, forcing you to repeat two of the vertices' data. Using index buffers, we can instead just re-reference the same data inside the vertex buffer using its index.
* **Data buffers**, which come in two basic types, Uniform and Storage, and can store data that is not unique to each vertex for use in different shaders
* **Images** (also called *textures*), which can either be preloaded with completely static data fed into them from the CPU, or images that were the *output of a previous run of the graphics pipeline*. By doing so we can apply "post processing" effects and many other interesting things like HDR, bloom, tonemapping, and deferred rendering.

The outputs of a pipeline can include

* A **Framebuffer object**, which is essentially a collection of one or more images to which data from each fragment will be written (this can include color information and depth information, as well as other information encoded into different channels of a color image)
* Some types of data buffers allow shaders to write data to them during execution

Here is a diagram that shows each of the steps of the pipeline. Green boxes signify "fixed function" pieces--these are not programmable, though you can often modify their behavior by setting different pieces of their state--and yellow boxes represent fully programmable pieces of the pipeline.

![graphics pipeline diagram](https://vulkan-tutorial.com/images/vulkan_simplified_pipeline.svg)

In order to program the yellow pieces of the pipeline, we write things called *shaders*, which are just pieces of code that are able to be executed on the graphics card instead of on your CPU. We'll explore these in much more detail as we go deeper into this series. Here are descriptions of each of the stages in the pipeline:

* The **input assembler** collects the raw vertex data from the buffers you specify and may also use an index buffer to repeat certain elements without having to duplicate the vertex data itself.
* The **vertex shader** is run for every vertex and generally applies transformations to turn vertex positions from "model space" to "screen space." This basically means translating the 3d position of each vertex from its point in the world all the way to its final "2d" position on the screen (this is a bit of a simplification, but we'll discuss that later). It also passes per-vertex data down the pipeline.
* The **tessellation shaders** allow you to subdivide geometry based on certain rules to increase the mesh quality. This is often used to make surfaces like brick walls and staircases look less flat when they are nearby.
* The **geometry shader** is run on every primitive (triangle, line, point) and can discard it or output more primitives than came in. This is similar to the tessellation shader, but much more flexible. However, it is not used much in today's applications because the performance is not that good on most graphics cards except for Intel's integrated GPUs.
* The **rasterization stage** discretizes the primitives into fragments. These are the pixel elements that fill their shape on the output image. Any fragments that fall outside the screen are discarded and the attributes outputted by the vertex shader are interpolated across the fragments, as shown in the figure (don't worry if you don't quite understand what this means yet). Usually the fragments that are behind other primitive fragments are also discarded here because of depth testing.
* The **fragment shader** is invoked for every fragment that survives and determines which output image(s), also called "render targets" and/or "framebuffers," the fragments are written to and with which color and depth values. It can do this using the interpolated data from the vertex shader, which can include things like texture coordinates and normals for lighting, and much more.
* The **color blending stage** applies operations to mix the output color of the fragment shader with the color that already exists in that pixel on the output image (framebuffer). Fragments can simply overwrite each other, add up or be mixed based upon transparency (alpha).

The two stages that we will be paying the most attention to are the **vertex shader** and the **fragment shader**, as these are the two shaders that are required to output a color image.

### Output Image, Framebuffer, Render Target

All of these terms are describing similar things, but there's some nuance to what exactly each one means. An output image is a single image, backed by some memory and formatted in a specific way, which can be written to for each fragment output. A `Framebuffer` is a collection of one or more output images, created by "attaching" those images to the framebuffer. Finally, a "render target" is the combination of a `Framebuffer` with a specific render pass, and describes the output of a render pass after all of its rendering tasks have completed.

## The Swapchain

So, now that we have a higher level overview of what's going to happen, I can better explain what the swapchain is and how it will slot into the higher level. As I said at the beginning of this section, the swapchain is a special set of images that are integrated into the OS's window management system, which are able to be "presented" on the Surface that we created earlier. These images can be bound as render targets to certain graphics pipelines: specifically, the final pipeline to be run which will then be displayed directly to the screen.

The reason we have multiple swapchain images is so that we can implement one of a few possible forms of multi-buffering. The problem this aims to solve is that we don't want the user to be able to see intermediate stages of an image being drawn before it is complete. Therefore, we don't draw directly into the image that is currently being drawn to the screen; instead, we draw into one of possibly multiple extra "backbuffers," which are not currently being displayed to the screen. Then, when we've determined that the image has been fully drawn into the backbuffer, we can ask the swapchain to "present" the newly drawn image to the screen and then get back the previously displayed image as part of our pool of backbuffers to draw on. There are a few possible ways to do this, and we'll explore them in more detail in a little bit below.

In order to create a swapchain, we'll need to determine three things which define its properties:

* The **format** of the images in the swapchain
* The **extent** (aka size) of the images in the swapchain
* The **presentation mode**, which is one of the methods of presenting that I alluded to earlier.

### Format

The first step for all of these is to query the surface to find out what features it is compatible with. We do this by using the `compatibility` method on our `surface` and pass in the `PhysicalDevice` which is contained in our `Adapter`. However before we can do so, we need to add some things to our scope.

```rust
let (capabilities, formats, presentation_modes) = surface.compatibility(&adapter.physical_device);
```

As we can see, we get a few things back in a tuple. The first is a `SurfaceCapabilities` object, then an `Option<Vec<Format>>`, which contains a list of supported image formats, and finally a `Vec<PresentMode>` which contains a list of supported presentation modes. For now what we care about is the `formats` list. What we're going to do is, if the `Option` contains a list, we'll iterate through that list and attempt to find a format with a `ChannelType` of `Srgb`. If we don't find one, then we'll just return the first format in the list, and if the `Option` is `None` then we'll just return a default format of `Rgba8Srgb`. First, though, we'll need to import a new module from `hal`, the `format` module, which we'll import as `f`... and while we're at it, we'll import what we're going to need for the rest of the swapchain creation as well.

```rust
use hal::{
    format as f, image as i,
    window::{ self, SwapchainConfig },
    Instance, Surface, Device,
};
```

Alright, now, let's see how finding a format works

```rust
let format = formats
    .map_or(f::Format::Rgba8Srgb, |formats| {
        formats
            .iter()
            .find(|format| format.base_format().1 == f::ChannelType::Srgb)
            .map(|format| *format)
            .unwrap_or(formats[0])
    });
```

### Extent

The next thing we'll do is find the size of the images (aka the extent) that we want to create. To do that, we'll either use the `Surface`'s current extent, or if it doesn't have one, we'll calculate one based on the size of the window as well as the `Surface`'s maximum and minimum supported extents.

```rust
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
```

### Presentation Mode

There are a few possible presentation modes that exist for us to choose from. Most of these modes were created to solve in one way or another the issue of screen tearing, which happens when you attempt to present an image to the screen out of sync with the monitor's refresh rate.

* `Immediate`: Present requests are applied immediately and tearing may be observed (depending on the frames per second). Internally the presentation engine doesn't use any queue for holding swap chain images.

![immediate mode diagram](https://software.intel.com/sites/default/files/managed/d0/01/api-vulkan-part-2-graphic-1.jpg)

* `Fifo`: The image is displayed (replaces currently displayed image) only on vertical blanking periods, so no tearing should be visible. Internally, the presentation engine uses FIFO queue with “numSwapchainImages – 1” elements. Present requests are appended to the end of this queue. During blanking periods, the image from the beginning of the queue replaces the currently displayed image, which may become available to application, which means frames are always displayed in the order they are submitted to the queue. If all images are in the queue, the application has to wait until v-sync releases the currently displayed image. Only after that does it becomes available to the application and program may render image into it. This mode must always be available.

* `Relaxed` (aka Fifo Relaxed): This mode is similar to Fifo, but when the image is displayed longer than one blanking period it may be released immediately without waiting for another v-sync signal (so if we are rendering frames with lower frequency than screen's refresh rate, tearing may be visible)

![fifo diagram](https://software.intel.com/sites/default/files/managed/63/b8/api-vulkan-part-2-graphic-2.jpg)

* `Mailbox`: The image is displayed only on vertical blanking periods and no tearing should be visible. But internally, the presentation engine uses a queue with only a single element. One image is displayed and one waits in the queue. If application wants to present another image it is not appended to the end of the queue but replaces the one that waits. So in the queue there is always the most recently generated image. This behavior is available if there are more than two images. For two images MAILBOX mode behaves similarly to FIFO (as we have to wait for the displayed image to be released, we don't have “spare” image which can be exchanged with the one that waits in the queue).

![mailbox diagram](https://software.intel.com/sites/default/files/managed/c3/d7/api-vulkan-part-2-graphic-3.jpg)

Which one to use depends on your application, but in my humble opinion, input lag is something that needs to be almost completely cut out in games when possible (especially if they are multiplayer/competitive), so `Immediate` would be a good choice. Alternatively, `Mailbox` would work well for slightly more input lag but no tearing, and less input lag than with `Fifo`.

To choose one, we'll look through the supported list we got back and see if we can find the one we want, and if not then we'll use `Fifo` since it is guaranteed.

```rust
let presentation_mode = presentation_modes
    .iter()
    .find(|&mode| *mode == window::PresentMode::Immediate)
    .map(|mode| *mode)
    .unwrap_or(window::PresentMode::Fifo);
```

### Swapchain and Backbuffer

There's two more things we need to decide: the number of images we want in our swapchain and the "usage" of those images. We choose the minimum number of images available for the swapchain since we want to use `Immediate` mode and fallback to `Fifo`. For our usage, we'll use the usage type `COLOR_ATTACHMENT`, which you'll just have to take my word on for now, but which we'll be going into greater detail on later. Now that we have all the properties chosen, we can bundle that information up into a `SwapchainConfig` like so

```rust
let swap_config = SwapchainConfig::new()
    .with_color(format)
    .with_image_count(capabilities.image_count.start)
    .with_image_usage(i::Usage::COLOR_ATTACHMENT)
    .with_presentation_mode(presentation_mode);
```

And, now that we have all our configuration set up, we can create our swapchain for real by calling the `create_swapchain` method on our `Device`.

```rust
let (swapchain, backbuffer) = device.create_swapchain(
    &mut surface
    swap_config,
    None,
    &extent,
);
```

You'll notice that we get two things back, the `Swapchain` itself, which is basically an interface for controlling which image to present to the screen and requesting images back from the chain to use for rendering, and something called a `Backbuffer`. In `hal`, this is an object that represents the actual backing image(s) of the swapchain. It can either be a collection of `Image`s, or a single `Framebuffer`... however the case of a single `Framebuffer` only applies for the OpenGL backend, which I'm not focusing on supporting right now anyway. Hopefully in the future the API will be consistent between all the backends and then we won't have to worry about that.

Hooray! We've made our swapchain.

#### Images in Slightly More Detail

So, images in `hal` are slightly more complex than they may first appear. Ultimately there are pieces of an image, which stack on top of each other, in `hal`: 

* A `Memory` object, which represents raw segment of memory. This is where the data for the image is ultimately stored, and it backs the next levels
* An `Image` object, which defines metadata about the kind of image (1, 2, or 3 dimensional), mipmap levels, the format of the image, how the image should tile, how the image will be used, and hints about how the image should be stored. This `Image` can then be bound to a specific `Memory` object for its backing.
* An `ImageView` object, which defines additional metadata about the image, including "swizzling" or component remapping and a "subresource range" which allows you to only look at certain aspects of an overall image.

When we created the swapchain, the images we got back were `Image`s, which means they were already backed by a piece of `Memory`, and we helped tell `hal` which `Format` and `Usage` we wanted those `Image`s to have. Before we use them, we'll need to create `ImageView`s to wrap them. We can then store these together with the `Image`s they're based on in a tuple.

```rust
let frame_images: Vec<_> = match backbuffer {
    window::Backbuffer::Images(images) => {
        images.into_iter()
            .map(|image| {
                let image_view = device.create_image_view(
                    &image,
                    i::ViewKind::D2,
                    format,
                    f::Swizzle::NO,
                    i::SubresourceRange {
                        aspects: f::Aspects::COLOR,
                        levels: 0..1,
                        layers: 0..1,
                    }
                );
                (image, image_view)
            })
            .collect()
    },
    _ => unimplemented!()
};
```

We're using the `ViewKind::D2`, which means that we want a 2 dimensional viw (which should match with the kind of the underlying image), the same format that we used before when creating the swapchain, and not performing any swizzling. Finally, we're using a `SubresourceRange` which lets us view the color part of the image (in this case, that is the only part), only the base mipmap level, and only the base layer, as that is the only layer that exists in the underlying image. (More layers are used for more advanced things like cubic and cube array images)

# Render Pass

The next big piece we'll be creating is a `RenderPass`. This is an object that represents the *description* of the "attachments", "subpasses", and "dependencies" of a render pass, where a render pass is the combination of possibly several runs of several different kinds of graphics pipelines that we talked about above to build one or more output images.

## Attachments

Recall our discussion about output images, framebuffers, and render targets. For now, we only have one output image: whichever of the swapchain images that we just created is not currently being displayed and to which we can render. Later, we will create `Framebuffer`s by using the corresponding swapchain image as an "attachment". For now, we are instead only going to be describing the form that this render pass is going to expect for each attachment. To do this, we'll create a set of `Attachment`s, which each describe one attachment that we will expect in the framebuffer. For us, we'll just create one, a color attachment for the swapchain image.

First, we'll import the `pass` module from `hal`:

```rust
use hal::{
    format as f, image as i,
    window::{ self, SwapchainConfig }, pass,
    Instance, Surface, Device,
};
```

Next, we'll make a new scope in which we'll create the pieces needed for the `RenderPass`, and then the `RenderPass` itself, and make our `color_attachment` inside it.

```rust
let render_pass = {
    let color_attachment = pass::Attachment {
        format: Some(format),
        samples: 1,
        ops: pass::AttachmentOps {
            load: pass::AttachmentLoadOp::Clear,
            store: pass::AttachmentStoreOp::Store,
        },
        stencil_ops: pass::AttachmentOps::DONT_CARE,
        layouts: i::Layout::Undefined..i::Layout::Present,
    };
    
    // more code will go here
};
```

Let's dissect a bit what exactly is going on here. First we provide a `Format`, for which we use the same format we used earlier for our swapchain images, since that is the same images we're trying to describe here. Next, we say that we're only going to be using one sample, which means we aren't using MSAA. Next, we provide a field called `ops`. These are the `AttachmentOps` that will be applied to main part of the attachment. Then we have `stencil_ops`, which is the same thing, but these ops will be applied to the stencil part of the attachment, if any. Since we don't have a stencil part of this attachment, we use `DONT_CARE`.

But, what are these `AttachmentOps`? Well, they let us define what we want to do with the attachment it is loaded at the beginning of this render pass, and what we want to do with it after the render pass is complete. For loading, we can choose between `Load`, which preserves the existing content of the attachment, `Clear`, which clears the content of the attachment, and `DontCare`, which will cause the content of the attachment to become undefined, but can let the driver do some optimization if we don't care about what happens. For storing, we can choose between `Store`, and `DontCare` which are very similar to the `Load` and `DontCare` options from before.

Finally, we define the layouts that the attachment will have before and after the render pass. The first layout is the layout that the attachment should have before the pass begins, and the second layout is the one that the attachment will automatically "transition" to after the render pass ends. But, what is a layout? `Image` objects in `hal` have a certain pixel format, however the layout of the pixels in memory can change based on what you're trying to do with said image. Some of the most common `Layout`s are

* `Layout::ColorAttachmentOptimal`: For use as a color attachment in a framebuffer
* `Layout::Present`: For use when presenting to the display
* `Layout::ShaderReadOnlyOptimal`: For use when the attachment is being read as part of a shader

In this case the first layout is `Layout::Undefined` which is used because we don't care what previous layout the image was in, as we'll be transitioning it to the layout we need it explicitly, and we don't care about keeping around the data that was in it previously as we'll be clearing it on load as we defined above. The final layout is `Layout::Present` as the output of this render pass is going to be presented directly to the screen.

## Subpasses

Render passes can be made up of multiple subpasses. This is mostly useful for taking advantage of tiled graphics processors, which are mostly found in mobile hardware. In this case, our render pass is just going to be made up of one subpass. To do so, we'll create a `SubpassDesc` which will describe the properties of a subpass.

*Still inside the scope we created before*
```rust
let subpass = pass::SubpassDesc {
    colors: &[(0, i::Layout::ColorAttachmentOptimal)],
    depth_stencil: None,
    inputs: &[],
    resolves: &[],
    preserves: &[],
};
```

Here's what each piece does:

* `colors`: Takes a slice of `AttachmentRef`s, which is a tuple of `(AttachmentId, AttachmentLayout)`. The attachment id refers to the order in which we insert the attachments when we create the render pass. This will make more sense in a little bit. The attachment layout is the `Layout` that we want the attachment to be automatically transitioned to when this subpass begins.
* `depth_stencil`: An optional `AttachmentRef` to be used as a depth or stencil buffer.
* `inputs`: A slice of `AttachmentRef`s to be used as "input attachments". These have a very specific use that I alluded to above, which is when you have multiple subpasses *within a single render pass*, you can use an output attachment from a previous subpass as an input to this subpass. It can have performance benefits over other ways of getting an image as input in a render pass, especially on mobile hardware, but also has limitations.
* `resolves`: We won't worry about this for now
* `preserves`: Attachments that are not used by this subpass directly but that must be preserved to be passed on to subsequent subpasses or passes.

## Subpass Dependencies
