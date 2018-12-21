#[macro_use]
extern crate log;
#[macro_use]
extern crate enum_map;

mod bits;
mod curses_ui;
mod disasm;
mod vm;

#[cfg(feature = "gui")]
mod gui;

use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct Environment {
    pub diagnostics_mutex: Arc<Mutex<bits::DiagnosticStatus>>,
}

fn main() {
    let env = Environment::default();
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    #[cfg(feature = "gui")]
    gui::run(env.diagnostics_mutex.clone());
    //curses_ui::start(env.diagnostics_mutex.clone());
    let mut vm = vm::VirtualMachine::with_memory(u16::max_value() as usize);
    vm.read_image_file(&mut ::std::fs::File::open("./res/2048.obj").unwrap());
    vm.add_diagnostic_mutex(env.diagnostics_mutex.clone());
    //vm.memory_dump();
    vm.run();
}
