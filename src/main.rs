#![feature(array_chunks, iter_array_chunks, slice_flatten, iter_intersperse)]

use std::str::FromStr;

use anyhow::Result;
use gemini_fetch::Page;
use tokio::runtime::Runtime;
use winit::{
    dpi::{PhysicalSize, LogicalSize},
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder, Window},
};
mod config;
mod font;
mod state;

use config::configure;
use state::State;
use url::Url;

use pixels::wgpu::BlendState;

use pixels::{PixelsBuilder, SurfaceTexture};
use winit_input_helper::{TextChar, WinitInputHelper};

const GEMINI_ADDRESS: &str = "gemini://mayaks.eu/";

async fn get_gemini_page(address: &Url) -> Result<String> {
    match Page::fetch(address, None).await {
        Ok(page) => {
            // Handle the fetched Gemini page
            println!("URL: {}", page.url);
            println!("Status: {:?}", page.header.status);
            println!("Meta: {}", page.header.meta);
            if let Some(body) = page.body {
                Ok(body)
            } else {
                Ok("No body found in the Gemini page".to_string())
            }
        }
        Err(err) => {
            // Handle errors
            eprintln!("Error: {}", err);
            Ok("Error fetching Gemini page".to_string())
        }
    }
}

fn get_gemini_page_blocking(address: &Url) -> Result<String> {
    Runtime::new().unwrap().block_on(get_gemini_page(address))
}

fn main() -> Result<(), pixels::Error> {
    let config = match configure() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("ERROR: {err}");
            eprintln!("Run with --help for usage information.");
            std::process::exit(1);
        }
    };

    let gemini_url = Url::parse(GEMINI_ADDRESS).expect("Invalid URL");

    // Create an event loop
    let event_loop = EventLoop::new();

    let mut input = WinitInputHelper::new();

    let scale_factor = {
        const DEFAULT_SCALE_FACTOR: f64 = 1.0;
        let env_scale_factor = std::env::var("GEM_SCALE_FACTOR")
            .ok()
            .and_then(|v| v.parse::<f64>().ok().map(|f| u32::max(1, f.round() as u32)));
        let wm_scale_factor = || {
            let Ok(dummy) = Window::new(&event_loop) else {
                eprintln!(
                    "INFO:  Could not construct dummy window to measure scale factor,\
                    assuming a factor of {DEFAULT_SCALE_FACTOR}"
                );
                return DEFAULT_SCALE_FACTOR;
            };

            dummy.scale_factor()
        };
        env_scale_factor.unwrap_or(wm_scale_factor().round() as u32)
    };

    let size = PhysicalSize::new(800 * scale_factor, 600 * scale_factor);

    // Create a window with a title and size
    let window = WindowBuilder::new()
        .with_title("Gemini client with uf2")
        .with_inner_size(size)
        .build(&event_loop)
        .expect("Failed to create window");

    let font_path = config.font_path;
    let font = match font::load_font(&font_path) {
        Ok(font) => font,
        Err(err) => {
            eprintln!("ERROR: Failed to load font from {font_path:?}: {err}");
            std::process::exit(1);
        }
    };

    let gemini_body = get_gemini_page_blocking(&gemini_url).expect("Error fetching Gemini page");

    let mut state = State::new(
        font,
        config.foreground,
        config.background,
        String::from(GEMINI_ADDRESS),
        gemini_body,
        window.inner_size().width / scale_factor,
        window.inner_size().height / scale_factor,
    );

    state.prepare_lines();

    let size = PhysicalSize::new(state.window_width, state.window_height);

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        PixelsBuilder::new(size.width, size.height, surface_texture)
            .clear_color({
                let [r, g, b, a] = config.background.map(|v| v as f64 / 255.0);
                pixels::wgpu::Color { r, g, b, a }
            })
            .blend_state(BlendState::REPLACE) // TODO: Investigate rendering weirdness.
            .build()?
    };

    let mut address = String::new();

    // Main event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,

            Event::RedrawRequested(_) => {
                // Clear the screen before drawing.
                pixels
                    .frame_mut()
                    .array_chunks_mut()
                    .for_each(|px| *px = config.background);

                let start = std::time::Instant::now();
                state.draw(&mut pixels);
                let end = std::time::Instant::now();
                let delta = (end - start).as_secs_f32();
                // eprintln!("drawing took: {delta:.6}");

                // Try to render.
                if let Err(err) = pixels.render() {
                    eprintln!("ERROR: {err}");
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }

            _ => (),
        }

        if input.update(&event) {
            // Close events.
            if input.close_requested() {
                eprintln!("INFO:  Close requested. Bye :)");
                *control_flow = ControlFlow::Exit;
                return;
            }
            let text: Vec<TextChar> = input.text();
            if !text.is_empty() {
                for ch in input.text() {
                    match ch {
                        TextChar::Char(ch) => address.push(ch),
                        TextChar::Back => {
                            address.pop();
                        }
                    }
                }
                eprintln!("current address: {address}");
                state.set_address(address.clone());
            }

            if input.key_pressed(VirtualKeyCode::Up) {
                if state.starting_line > 0 {
                    state.set_starting_line(state.starting_line - 1);
                }
            }

            if input.key_pressed(VirtualKeyCode::Down) {
                if state.starting_line < state.content_lines.len() {
                    state.set_starting_line(state.starting_line + 1);
                }
            }

            if input.key_pressed(VirtualKeyCode::Return) {
                eprintln!("fetching address: {address}");
                let content = get_gemini_page_blocking(&Url::from_str(&address).unwrap()).unwrap();
                eprintln!("content: {content}");
                state.update(address.clone(), content);
                address.clear();
            }

            // Resize the window.
            if let Some(size) = input.window_resized() {
                eprintln!("resizing");
                let PhysicalSize { width, height } = size;
                pixels.resize_surface(width * scale_factor, height * scale_factor).unwrap();
                pixels.resize_buffer(width, height).unwrap();
                window.set_inner_size(PhysicalSize::new(width, height));

                state.resize(width / scale_factor, height / scale_factor);
            }

            state.prepare_lines();
            window.request_redraw();
        }
    });
}
