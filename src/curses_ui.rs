use enum_map::EnumMap;
use pancurses;

use crate::bits::{DiagnosticStatus, Register};

use std::sync::{Arc, Mutex};
use std::thread;

pub fn start(diagnostics_mutex: Arc<Mutex<DiagnosticStatus>>) {
    thread::spawn(move || {
        actual_start(diagnostics_mutex);
    });
}

fn actual_start(diagnostics_mutex: Arc<Mutex<DiagnosticStatus>>) {
    let window = pancurses::initscr();
    pancurses::resize_term(40, 120);
    window.printw("Type things, press delete to quit\n");
    window.refresh();
    window.keypad(true);
    pancurses::mousemask(
        pancurses::ALL_MOUSE_EVENTS | pancurses::REPORT_MOUSE_POSITION,
        std::ptr::null_mut(),
    );
    pancurses::noecho();
    pancurses::half_delay(1);
    let mut diagnostics = DiagnosticStatus::default();
    let subwin = window.subwin(1, 1, 10, 1).unwrap();
    println!("ATTRGET: {:?}", window.attrget());
    loop {
        if let Ok(ref diagnostic_lock) = diagnostics_mutex.try_lock() {
            use std::ops::Deref;
            diagnostics = diagnostic_lock.deref().clone();
        }
        let registers = diagnostics.registers;
        print_registers(&window, &registers);
        use pancurses::Input;
        match window.getch() {
            Some(Input::Character(c)) => {
                window.addch(c);
            }
            Some(Input::KeyDC) => break,
            Some(Input::KeyMouse) => {
                if let Ok(mouse_event) = pancurses::getmouse() {
                    window.mvprintw(
                        1,
                        1,
                        &format!("Mouse at {}, {}", mouse_event.x, mouse_event.y),
                    );
                }
            }
            Some(Input::KeyResize) => {
                pancurses::resize_term(0, 0);
            }
            Some(input) => {
                window.addstr(&format!("{:?}", input));
            }
            None => (),
        }
        window.draw_box('|', '-');
    }
    pancurses::endwin();
}

fn print_registers(window: &pancurses::Window, registers: &EnumMap<Register, u16>) {
    let line1 = format!(
        "R0: {:04x}\tR1: {:04x}\tR2: {:04x}\tR3: {:04x}\n",
        registers[Register::R0],
        registers[Register::R1],
        registers[Register::R2],
        registers[Register::R3],
    );
    window.mvprintw(1, 20, &line1);
    let line2 = format!(
        "R4: {:04x}\tR5: {:04x}\tR6: {:04x}\tR7: {:04x}\n",
        registers[Register::R4],
        registers[Register::R5],
        registers[Register::R6],
        registers[Register::R7],
    );
    window.mvprintw(2, 20, &line2);
    let line3 = format!(
        "PC: {:04x}\tPSR: {:04x}\tCC: {:04x}\n",
        registers[Register::PC],
        registers[Register::PSR],
        registers[Register::COND],
    );
    window.mvprintw(3, 20, &line3);
}
