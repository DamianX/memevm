use imgui::{im_str, ImGuiCond, Ui};

use crate::bits::DiagnosticStatus;
use std::sync::{Arc, Mutex};
use std::thread;

mod support;

extern crate gfx_gl as gl;

#[derive(Debug, PartialEq)]
enum DisplayFormat {
    Decimal,
    Hexadecimal,
    Octal,
    Binary,
}

impl DisplayFormat {
    fn format(&self, input: u16) -> String {
        match self {
            DisplayFormat::Decimal => format!("{}", input),
            DisplayFormat::Hexadecimal => format!("{:04X}", input),
            DisplayFormat::Octal => format!("{:o}", input),
            DisplayFormat::Binary => format!("{:016b}", input),
        }
    }
}

pub struct Scene {
    diagnostics_mutex: Arc<Mutex<DiagnosticStatus>>,
    registers_window_enabled: bool,
    memory_window_enabled: bool,
    console_window_enabled: bool,
    latest_diagnostics: DiagnosticStatus,
    current_display_format: DisplayFormat,
}
impl Scene {
    fn new(diagnostics_mutex: Arc<Mutex<DiagnosticStatus>>) -> Self {
        Scene {
            diagnostics_mutex,
            registers_window_enabled: true,
            memory_window_enabled: true,
            console_window_enabled: true,
            latest_diagnostics: DiagnosticStatus::default(),
            current_display_format: DisplayFormat::Hexadecimal,
        }
    }

    fn run_ui(&mut self, ui: &Ui) -> bool {
        ui.main_menu_bar(|| {
            ui.menu(im_str!("File")).build(|| {
                if ui.menu_item(im_str!("Open")).build() {
                    println!("Open")
                };
            });
        });
        //ui.show_metrics_window(&mut true);
        self.try_update_diagnostics();
        self.show_registers_window(ui);
        self.show_memory_window(ui);
        self.show_console_window(ui);
        true
    }

    fn try_update_diagnostics(&mut self) {
        if let Ok(mut diagnostics_lock) = self.diagnostics_mutex.try_lock() {
            diagnostics_lock.memory_view_range = self.latest_diagnostics.memory_view_range;
            self.latest_diagnostics = diagnostics_lock.clone();
        }
    }

    fn show_registers_window(&mut self, ui: &Ui) {
        if !self.registers_window_enabled {
            return;
        }
        let mut opened = self.registers_window_enabled;
        ui.window(im_str!("Registers"))
            .opened(&mut opened)
            .position((10.0, 30.0), ImGuiCond::FirstUseEver)
            .size((170.0, 310.0), ImGuiCond::FirstUseEver)
            .build(|| {
                let register_name_max_length = 4;
                for (register, contents) in self.latest_diagnostics.registers {
                    let register_name = format!("{:?}", register);
                    let additional_spaces = " ".repeat(register_name_max_length - register_name.len());
                    ui.text(im_str!("{}:{}{}", register_name, additional_spaces, self.current_display_format.format(contents)));
                }
                if ui.radio_button_bool(im_str!("Hex"), self.current_display_format == DisplayFormat::Hexadecimal) {
                    self.current_display_format = DisplayFormat::Hexadecimal;
                }
                if ui.radio_button_bool(im_str!("Dec"), self.current_display_format == DisplayFormat::Decimal) {
                    self.current_display_format = DisplayFormat::Decimal;
                }
                if ui.radio_button_bool(im_str!("Oct"), self.current_display_format == DisplayFormat::Octal) {
                    self.current_display_format = DisplayFormat::Octal;
                }
                if ui.radio_button_bool(im_str!("Bin"), self.current_display_format == DisplayFormat::Binary) {
                    self.current_display_format = DisplayFormat::Binary;
                }
            });
        self.registers_window_enabled = opened;
    }

    fn show_memory_window(&mut self, ui: &Ui) {
        if !self.memory_window_enabled {
            return;
        }
        let mut opened = self.memory_window_enabled;
        ui.window(im_str!("Memory"))
            .opened(&mut opened)
            .position((100.0, 30.0), ImGuiCond::FirstUseEver)
            .size((720.0, 350.0), ImGuiCond::FirstUseEver)
            .build(|| {
                self.latest_diagnostics.memory_view_range = (0x3000, 256);
                let meme = unsafe {
                    std::slice::from_raw_parts(
                        self.latest_diagnostics.memory_view.as_ptr() as *const u8,
                        self.latest_diagnostics.memory_view.len() * std::mem::size_of::<u16>(),
                    )
                };
                ui.text(im_str!("len: {}", meme.len()));
                for line in hexdump::hexdump_iter(&meme) {
                    ui.text(im_str!("{}", line));
                }
            });
        self.memory_window_enabled = opened;
    }

    fn show_console_window(&mut self, _ui: &Ui) {}
}

pub fn run(diagnostics_mutex: Arc<Mutex<DiagnosticStatus>>) {
    thread::spawn(move || {
        let scene = Scene::new(diagnostics_mutex);
        support::run("lc3vm".to_owned(), [0.25, 0.25, 0.5, 1.0], scene);
    });
}

/*fn actual_run(diagnostics_mutex: Arc<Mutex<DiagnosticStatus>>) {
    use gfx::{self, Device};
    use gfx_window_glutin;
    use glutin::{self, GlContext};

    let mut events_loop = glutin::EventsLoop::new();
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let window = glutin::WindowBuilder::new()
        .with_title("memevm")
        .with_min_dimensions(glutin::dpi::LogicalSize::new(600.0, 800.0))
        .with_dimensions(glutin::dpi::LogicalSize::new(800.0, 800.0));
    let (window, mut device, mut factory, mut main_color, mut main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(window, context, &events_loop)
            .expect("Failed to create window");

    unsafe {
        device.with_gl(|gl| {
            gl.Disable(gl::FRAMEBUFFER_SRGB);
        });
    }

    let (ww, wh): (f64, f64) = window.get_outer_size().unwrap().into();
    let (dw, dh): (f64, f64) = window.get_primary_monitor().get_dimensions().into();
    window.set_position(((dw - ww) / 2.0, (dh - wh) / 2.0).into());

    let mut encoder: Encoder = factory.create_command_buffer().into();

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
            Shaders::GlSl130
        } else {
            Shaders::GlSl110
        }
    };

    let mut imgui = ImGui::init();
    imgui.style_mut().colors.clone_from(&dark_theme());
    imgui.set_ini_filename(None);

    let mut window_hidpi_factor = window.get_hidpi_factor();
    let mut hidpi_factor = window_hidpi_factor.round();

    let mut frame_size = FrameSize {
        logical_size: window
            .get_inner_size()
            .unwrap()
            .to_physical(window_hidpi_factor)
            .to_logical(hidpi_factor)
            .into(),
        hidpi_factor,
    };

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

    configure_keys(&mut imgui);

    let mut scene = Scene::new(&mut factory, &main_color, &main_depth, diagnostics_mutex);

    let mut last_frame = Instant::now();
    let mut mouse_state = MouseState::default();
    let mut quit = false;
    let mut mouse_captured = false;
    let mut kbd_captured = false;

    loop {
        events_loop.poll_events(|event| {
            use glutin::ElementState::Pressed;
            use glutin::WindowEvent::*;
            use glutin::{Event, MouseButton, MouseScrollDelta, TouchPhase};

            if let Event::WindowEvent { event, .. } = event {
                match event {
                    CloseRequested => quit = true,
                    Resized(new_logical_size) => {
                        gfx_window_glutin::update_views(&window, &mut main_color, &mut main_depth);
                        window.resize(new_logical_size.to_physical(hidpi_factor));
                        renderer.update_render_target(main_color.clone());
                        frame_size.logical_size = new_logical_size
                            .to_physical(window_hidpi_factor)
                            .to_logical(hidpi_factor)
                            .into();
                    }
                    HiDpiFactorChanged(new_factor) => {
                        window_hidpi_factor = new_factor;
                        hidpi_factor = window_hidpi_factor.round();
                        frame_size.hidpi_factor = hidpi_factor;
                        frame_size.logical_size = window
                            .get_inner_size()
                            .unwrap()
                            .to_physical(window_hidpi_factor)
                            .to_logical(hidpi_factor)
                            .into();
                    }
                    Focused(false) => {
                        // If the window is unfocused, unset modifiers, or
                        // Alt-Tab will set it permanently & cause trouble. No,
                        // I don't know why this doesn't just work.
                        imgui.set_key_ctrl(false);
                        imgui.set_key_alt(false);
                        imgui.set_key_shift(false);
                        imgui.set_key_super(false);
                    }
                    KeyboardInput { input, .. } => {
                        use glutin::VirtualKeyCode as Key;

                        let pressed = input.state == Pressed;
                        match input.virtual_keycode {
                            Some(Key::Tab) => imgui.set_key(0, pressed),
                            _ => {}
                        }
                    }
                    CursorMoved { position, .. } => {
                        // Rescale position from glutin logical coordinates to our logical
                        // coordinates
                        let pos = position
                            .to_physical(window_hidpi_factor)
                            .to_logical(hidpi_factor)
                            .into();
                        mouse_state.pos = pos;
                    }
                    MouseInput { state, button, .. } => match button {
                        MouseButton::Left => mouse_state.pressed[0] = state == Pressed,
                        MouseButton::Right => mouse_state.pressed[1] = state == Pressed,
                        MouseButton::Middle => mouse_state.pressed[2] = state == Pressed,
                        MouseButton::Other(i) => {
                            if let Some(b) = mouse_state.pressed.get_mut(2 + i as usize) {
                                *b = state == Pressed;
                            }
                        }
                    },
                    MouseWheel {
                        delta: MouseScrollDelta::LineDelta(x, y),
                        phase: TouchPhase::Moved,
                        ..
                    } => {
                        mouse_state.wheel = y;
                    }
                    MouseWheel {
                        delta: MouseScrollDelta::PixelDelta(pos),
                        phase: TouchPhase::Moved,
                        ..
                    } => {
                        // Rescale pixel delta from glutin logical coordinates to our logical
                        // coordinates
                        let diff = pos
                            .to_physical(window_hidpi_factor)
                            .to_logical(hidpi_factor);
                        mouse_state.wheel = diff.y as f32;
                    }
                    ReceivedCharacter(c) => imgui.add_input_character(c),
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

        update_mouse(&mut imgui, &mut mouse_state);

        let mouse_cursor = imgui.mouse_cursor();
        if imgui.mouse_draw_cursor() || mouse_cursor == ImGuiMouseCursor::None {
            // Hide OS cursor
            window.hide_cursor(true);
        } else {
            // Set OS cursor
            window.hide_cursor(false);
            window.set_cursor(match mouse_cursor {
                ImGuiMouseCursor::None => unreachable!("mouse_cursor was None!"),
                ImGuiMouseCursor::Arrow => glutin::MouseCursor::Arrow,
                ImGuiMouseCursor::TextInput => glutin::MouseCursor::Text,
                ImGuiMouseCursor::Move => glutin::MouseCursor::Move,
                ImGuiMouseCursor::ResizeNS => glutin::MouseCursor::NsResize,
                ImGuiMouseCursor::ResizeEW => glutin::MouseCursor::EwResize,
                ImGuiMouseCursor::ResizeNESW => glutin::MouseCursor::NeswResize,
                ImGuiMouseCursor::ResizeNWSE => glutin::MouseCursor::NwseResize,
            });
        }

        // Workaround: imgui-gfx-renderer will not call ui.render() under this
        // condition, which occurs when minimized, and imgui will assert
        // because of missing either a Render() or EndFrame() call.
        let logical_size = frame_size.logical_size;
        if logical_size.0 > 0.0 && logical_size.1 > 0.0 {
            let ui = imgui.frame(frame_size, delta_s);
            if !scene.run_ui(&ui) {
                break;
            }

            mouse_captured = ui.want_capture_mouse();
            kbd_captured = ui.want_capture_keyboard();

            encoder.clear(&main_color, clear_color);
            renderer
                .render(ui, &mut factory, &mut encoder)
                .expect("Rendering failed!");
            encoder.flush(&mut device);
            window.swap_buffers().unwrap();
            device.cleanup();
        }
    }
}

*/
