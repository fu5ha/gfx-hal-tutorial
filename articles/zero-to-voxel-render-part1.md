---
layout: Post
title: "Zero to Voxel Renderer with gfx-hal - Part 1"
date: 2018-08-05
featured: true
---

This series of blog posts is going to walk you through how to go from no knowledge of computer graphics programming to having a working voxel-style renderer in the vein of Minecraft (with maybe some extra bells and whistles) using `gfx-hal`. I won't assume any prior knowledge of OpenGL, Vulkan, etc., however I will assume that you have a working knowlege of the Rust programming language. If not, [go read the book](https://doc.rust-lang.org/book/2018-edition/index.html)! It is excellent, and Rust is an amazing language that everyone should know.

# Introduction

`gfx-hal` (which I'll now refer to as just `hal`) is a Rust crate (library) that is meant to allow the user to use the new, shiny, low-level, "explicit" graphics APIs in a cross-platform manner with low overhead. Its api is a Rustified version of the Vulkan API. It is an "abstraction layer" over the various low level graphics APIs. Since its api is so heavily based on Vulkan, its most frictionless backend target is Vulkan, but it also provides a low (as much as is possible) overhead translation to Metal and DirectX12, (also OpenGL and DirectX11, but these are primarily fallback targets).

*Side note: `gfx-hal` is not connected to the old `gfx-rs/gfx` crate that existed in the `pre-ll` days in anything other than semantics; it is a complete rewrite. So, knowledge from there won't carry directly over here and vice-versa.*

So, what does a low level graphics API do? Well, that has somewhat changed over the years as OpenGL has evolved and now given way to the next-generation graphics APIs. It used to be that OpenGL (or early versions of DirectX) were very much "fixed function" black boxes. The developer would pass them a set of vertices, information about the camera and lights, and tell it what style of shading they wanted to be applied, and the graphics API would do all the work of making that happen for them. However, that quickly became limiting as computer graphics became more and more complex and developers wanted more control over the details of how every aspect of the computations that were going on behind the scenes. Ultimately, that has left us with the next gen, "explicit" APIs (Vulkan, DirectX12, Metal), so called because they allow the developer unprecedented control over every little aspect of what the hardware is doing. This means that on the one hand, we can in theory squeak out every little oodle of performance from the hardware by specializing it specifically for our application's needs. On the other, though, it means that the developer needs to have knowledge of and understand every aspect of the device and API in order to take proper advantage of that control, and that the code is *very* wordy.

So, our first goal is to be drawing a triangle, and we'll slowly build concepts and work up from there to create the rest of our renderer.

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

The graphics pipeline is the sequence of operations that take the inputs into the black box and turn them all the way into the pixel colors of the output image. The basic goal of a modern graphics pipeline is to turn a series of *primitives* (almost always *triangles*) into a set of *fragments*, which can be thought of as analogous to *pixels* in an image, and then to compute an output color for each of those fragments. When we later make a "draw" call, what is happening is that we're telling the graphics card to take the currently bound state of the graphics pipeline and execute each stage to produce an output. We can run multiple draw calls on the same pipeline, say, one per object in our scene (this is an inefficient way to do it, but it does work), and slowly build up the final output image. With `hal`, we can also create multiple different ways to set up the pipeline, which can draw different kinds of objects in different ways. We can even assemble multiple runs (draw calls) of different pipelines into "render passes," and then use the output of one render pass as part of the input for another render pass. The inputs to the pipeline can include 

* Vertices, formatted into vertex buffers, which hold data about each vertex that can include position, color, texture coordinates, normals, and more.
* Indices into the vertex buffer, allowing the reuse of vertex information. Since the pipeline only understands how to work with triangles, rendering a square would actually require six vertices to be passed to the gpu, forcing you to repeat two of the vertices' data. Using index buffers, we can instead just re-reference the same data inside the vertex buffer using its index.
* Data buffers, which come in two basic types, Uniform and Storage, and can store data that is not unique to each vertex for use in different shaders
* Images (also called *textures*), which can either be preloaded with completely static data fed into them from the CPU, or images that were the *output of a previous run of the graphics pipeline*. By doing so we can apply "post processing" effects and many other interesting things like HDR, bloom, tonemapping, and deferred rendering.

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

## Output Image, Framebuffer, Render Target

All of these terms are describing essentially the same thing--one of possibly multiple output images from one "run" of the graphics pipeline.
