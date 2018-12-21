use imgui::{ImFontConfig, ImGui, Ui};
use imgui_gfx_renderer::{Renderer, Shaders};
use imgui_winit_support;
use std::time::Instant;

use crate::gui::Scene;

pub fn run(title: String, clear_color: [f32; 4], mut scene: Scene) {
    use gfx::{self, Device};
    use gfx_window_glutin;
    use glutin;

    type ColorFormat = gfx::format::Rgba8;
    type DepthFormat = gfx::format::DepthStencil;

    let mut events_loop = glutin::EventsLoop::new();
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let window = glutin::WindowBuilder::new()
        .with_title(title)
        .with_dimensions(glutin::dpi::LogicalSize::new(1024f64, 768f64));

    let (window, mut device, mut factory, mut main_color, mut main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(window, context, &events_loop)
            .expect("Unable to init gfx_window_glutin");

    let (ww, wh): (f64, f64) = window.get_outer_size().unwrap().into();
    let (dw, dh): (f64, f64) = window.get_primary_monitor().get_dimensions().into();
    window.set_position(((dw - ww) / 2.0, (dh - wh) / 2.0).into());

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let shaders = {
        let version = device.get_info().shading_language;
        if version.is_embedded {
            if version.major >= 3 {
                Shaders::GlSlEs300
            } else {
                Shaders::GlSlEs100
            }
        } else if version.major >= 4 {
            Shaders::GlSl400
        } else if version.major >= 3 {
            if version.minor >= 2 {
                Shaders::GlSl150
            } else {
                Shaders::GlSl130
            }
        } else {
            Shaders::GlSl110
        }
    };

    let mut imgui = ImGui::init();
    unsafe {
        device.with_gl(|gl| {
            gl.Disable(gfx_gl::FRAMEBUFFER_SRGB);
        });
    }
    imgui.set_ini_filename(None);

    let hidpi_factor = window.get_hidpi_factor().round();

    let font_size = (13.0 * hidpi_factor) as f32;

    imgui.fonts().add_default_font_with_config(
        ImFontConfig::new()
            .oversample_h(1)
            .pixel_snap_h(true)
            .size_pixels(font_size),
    );

    imgui.set_font_global_scale((1.0 / hidpi_factor) as f32);

    let mut renderer = Renderer::init(&mut imgui, &mut factory, shaders, main_color.clone())
        .expect("Failed to initialize renderer");

    imgui_winit_support::configure_keys(&mut imgui);

    let mut last_frame = Instant::now();
    let mut quit = false;

    loop {
        events_loop.poll_events(|event| {
            use glutin::{
                Event,
                WindowEvent::{CloseRequested, Resized},
            };

            imgui_winit_support::handle_event(
                &mut imgui,
                &event,
                window.get_hidpi_factor(),
                hidpi_factor,
            );

            if let Event::WindowEvent { event, .. } = event {
                match event {
                    Resized(_) => {
                        gfx_window_glutin::update_views(&window, &mut main_color, &mut main_depth);
                        renderer.update_render_target(main_color.clone());
                    }
                    CloseRequested => quit = true,
                    _ => (),
                }
            }
        });
        if quit {
            break;
        }

        let now = Instant::now();
        let delta = now - last_frame;
        let delta_s = delta.as_secs() as f32 + delta.subsec_nanos() as f32 / 1_000_000_000.0;
        last_frame = now;

        imgui_winit_support::update_mouse_cursor(&imgui, &window);

        let frame_size = imgui_winit_support::get_frame_size(&window, hidpi_factor).unwrap();

        let ui = imgui.frame(frame_size, delta_s);
        if !scene.run_ui(&ui) {
            break;
        }

        encoder.clear(&main_color, clear_color);
        renderer
            .render(ui, &mut factory, &mut encoder)
            .expect("Rendering failed");
        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();
    }
}
