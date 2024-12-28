use std::process::Command;
use std::thread::{sleep, JoinHandle};
use cosmic::cosmic_config::{Config, ConfigGet};
use enigo::{Enigo, Keyboard, Mouse};
use enigo::agent::Token::{Button, Key, MoveMouse, Raw, Scroll, Text};
use tracing::warn;
use crate::macros::{Instruction, Macro};

pub(crate) fn get_macro(config: &Config, mac: usize) -> Macro {
    let macs = config.get::<Vec<Macro>>("macros").expect("Macros file not found");
    macs[mac].clone()
}

pub(crate) fn run_macro(mac: Macro, enigo: &mut Enigo) {
    for ins in mac.code {
        #[allow(unreachable_patterns)] match ins {
            Instruction::Wait(duration) => {
                sleep(std::time::Duration::from_millis(duration));
            }
            Instruction::Script(script) => {
                println!("Running script: {script}");
                Command::new("bash")
                    .arg(&script)
                    .output()
                    .expect(&format!("Failed to run script: {script}"));
            }
            Instruction::Token(token) => {
                match token {
                    Text(text) => {
                        enigo.text(&text).expect(&format!("Failed to type text: {text}"));
                    }
                    Key(key, direction) => {
                        enigo.key(key, direction).expect("Failed to type key");
                    }
                    Raw(keycode, direction) => {
                        enigo.raw(keycode, direction).expect(&format!("Failed to type raw keycode: {keycode}"));
                    }
                    Button(button, direction) => {
                        enigo.button(button, direction).expect("Failed to click mouse button");
                    }
                    MoveMouse(x, y, coord) => {
                        enigo.move_mouse(x, y, coord).expect(&format!("Failed to move mouse to: ({x}, {y})"));
                    }
                    Scroll(amount, axis) => {
                        enigo.scroll(amount, axis).expect(&format!("Failed to scroll by: {amount}"));
                    }
                    _ => {
                        warn!("Token not implemented.");
                    }
                }
            }
            _ => {
                println!("Instruction not implemented.");
            }
        }
    }
}

pub fn make_enigo() -> Enigo {
    Enigo::new(&enigo::Settings::default()).unwrap()
}

pub(crate) struct ThreadPool {
    pub(crate) workers: Vec<JoinHandle<()>>,
}

impl ThreadPool {
    pub(crate) fn new() -> Self {
        ThreadPool { workers: Vec::new() }
    }

    pub(crate) fn add_worker(&mut self, worker: JoinHandle<()>) {
        self.workers.push(worker);
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for worker in self.workers.drain(..) {
            worker.join().expect("Failed to join worker thread");
        }
    }
}