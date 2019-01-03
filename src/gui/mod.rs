use imgui::{im_str, ImGuiCond, Ui};

use crate::bits::DiagnosticStatus;
use std::sync::{Arc, Mutex};
use std::thread;

mod support;
//mod memory_editor;

//use self::memory_editor::MemoryEditor;

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
//    memory_editor: MemoryEditor,
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
//            memory_editor: MemoryEditor::default(),
        }
    }

    fn run_ui(&mut self, ui: &Ui) -> bool {
        ui.show_metrics_window(&mut true);
        //self.memory_editor.draw_contents(ui);
        self.real_run_ui(ui)
    }

    fn real_run_ui(&mut self, ui: &Ui) -> bool {
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