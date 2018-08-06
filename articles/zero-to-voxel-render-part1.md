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

# Setting Up

If you're on Windows and intend to use the Vulkan backend as a target, installing the [LunarG Vulkan SDK](https://vulkan.lunarg.com/) will be helpful for debugging later.

In addition, it's a good idea for any OS and backend to install [RenderDoc](https://renderdoc.org/), which is like a debugger for your graphics card, and is invaluable in trying to sort out weird bugs that will inevitably pop up when doing this sort of work.

The first thing we're going to do is get our workspace set up. This is fairly simple, assuming you have Rust and Cargo installed:

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

## Instance, Surface, Adapter

The next thing we'll do is create an `Instance`. This can be seen as the equivalent to a Vulkan Instance, and essentially represents backend-specific, per-application state. We won't be using it directly very much, but we do need to create it. Just below our window, we can add

```rust
let instance = back::Instance::create("voxel-renderer", 1);
```