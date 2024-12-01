use std::process::Command;
use std::thread::sleep;
use cosmic::cosmic_config::{Config, ConfigGet};
use enigo::agent::Token;
use enigo::{Enigo, Keyboard, Mouse};
use tracing::warn;
use crate::macros::{Instruction, Macro};

pub(crate) fn get_macro(config: &Config, mac: usize) -> Macro {
    let macs = config.get::<Vec<Macro>>("macros").expect("TODO: panic message");
    macs[mac].clone()
}

pub(crate) fn run_macro(mac: Macro, enigo: &mut Enigo) {
    for ins in mac.code {
        #[allow(unreachable_patterns)] match ins {
            Instruction::Wait(duration) => {
                sleep(std::time::Duration::from_millis(duration));
            }
            Instruction::Script(script) => {
                println!("Running script: {}", script);
                Command::new("bash")
                    .arg(script)
                    .output()
                    .expect("TODO: panic message");
            }
            Instruction::Token(token) => {
                match token {
                    Token::Text(text) => {
                        enigo.text(&text).expect("TODO: panic message");
                    }
                    Token::Key(key, direction) => {
                        enigo.key(key, direction).expect("TODO: panic message");
                    }
                    Token::Raw(keycode, direction) => {
                        enigo.raw(keycode, direction).expect("TODO: panic message");
                    }
                    Token::Button(button, direction) => {
                        enigo.button(button, direction).expect("TODO: panic message");
                    }
                    Token::MoveMouse(x, y, coord) => {
                        enigo.move_mouse(x, y, coord).expect("TODO: panic message");
                    }
                    Token::Scroll(amount, axis) => {
                        enigo.scroll(amount, axis).expect("TODO: panic message");
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