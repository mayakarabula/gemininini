#![feature(array_chunks, iter_array_chunks, slice_flatten, iter_intersperse)]

use gemini_fetch::Page;
use anyhow::Result;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
mod font;
mod config;
mod state;

use url::Url;
use config::configure;
use state::State;

use pixels::wgpu::BlendState;

use pixels::{PixelsBuilder, SurfaceTexture};
use winit_input_helper::WinitInputHelper;

const GEMINI_ADDRESS: &str = "gemini://mayaks.eu/";

async fn get_gemini_page (address: &Url) -> Result<String> {
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

#[tokio::main]
async fn main() -> Result<(), pixels::Error> {
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

    // Create a window with a title and size
    let window = WindowBuilder::new()
        .with_title("Gemini client with uf2")
        .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
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

        let gemini_body = (get_gemini_page(&gemini_url).await).expect("Error fetching Gemini page");

        let mut state = State::new(
            font,
            config.foreground,
            config.background,
            String::from(GEMINI_ADDRESS),
            gemini_body,
            800,
        );

        let mut pixels = {
            let window_size = window.inner_size();
            let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
            PixelsBuilder::new(800, 600, surface_texture)
                .clear_color({
                    let [r, g, b, a] = config.background.map(|v| v as f64 / 255.0);
                    pixels::wgpu::Color { r, g, b, a }
                })
                .blend_state(BlendState::REPLACE) // TODO: Investigate rendering weirdness.
                .build()?
        };

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
                    .for_each(|px| *px = {
                        config.background
                    });
               
                state.draw(&mut pixels);

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

            // Resize the window.
            if let Some(size) = input.window_resized() {
                state.resize(window.inner_size().width);
                
            }
        }
    });
}
