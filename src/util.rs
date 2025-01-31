use std::process::Command;
use std::thread::{sleep, JoinHandle};
use cosmic::cosmic_config::{Config, ConfigGet};
use enigo::{Enigo, Keyboard, Mouse};
use enigo::agent::Token::{Button, Key, MoveMouse, Raw, Scroll, Text};
use enigo::Key as EnigoKey;
use enigo::Button as EnigoButton;
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

pub(crate) fn string_to_key(key_str: &str) -> Result<EnigoKey, &'static str> {
    match key_str {
        "F1" => Ok(EnigoKey::F1),
        "F2" => Ok(EnigoKey::F2),
        "F3" => Ok(EnigoKey::F3),
        "F4" => Ok(EnigoKey::F4),
        "F5" => Ok(EnigoKey::F5),
        "F6" => Ok(EnigoKey::F6),
        "F7" => Ok(EnigoKey::F7),
        "F8" => Ok(EnigoKey::F8),
        "F9" => Ok(EnigoKey::F9),
        "F10" => Ok(EnigoKey::F10),
        "F11" => Ok(EnigoKey::F11),
        "F12" => Ok(EnigoKey::F12),
        "F13" => Ok(EnigoKey::F13),
        "F14" => Ok(EnigoKey::F14),
        "F15" => Ok(EnigoKey::F15),
        "F16" => Ok(EnigoKey::F16),
        "F17" => Ok(EnigoKey::F17),
        "F18" => Ok(EnigoKey::F18),
        "F19" => Ok(EnigoKey::F19),
        "F20" => Ok(EnigoKey::F20),
        "F21" => Ok(EnigoKey::F21),
        "F22" => Ok(EnigoKey::F22),
        "F23" => Ok(EnigoKey::F23),
        "F24" => Ok(EnigoKey::F24),
        "Escape" => Ok(EnigoKey::Escape),
        "Space" => Ok(EnigoKey::Space),
        "Enter" => Ok(EnigoKey::Return),
        "Tab" => Ok(EnigoKey::Tab),
        "Backspace" => Ok(EnigoKey::Backspace),
        "CapsLock" => Ok(EnigoKey::CapsLock),
        "Shift" => Ok(EnigoKey::Shift),
        "Control" => Ok(EnigoKey::Control),
        "Alt" => Ok(EnigoKey::Alt),
        "Meta" => Ok(EnigoKey::Meta),
        "Super" => Ok(EnigoKey::Meta),
        "LeftArrow" => Ok(EnigoKey::LeftArrow),
        "RightArrow" => Ok(EnigoKey::RightArrow),
        "UpArrow" => Ok(EnigoKey::UpArrow),
        "DownArrow" => Ok(EnigoKey::DownArrow),
        "Insert" => Ok(EnigoKey::Insert),
        "Delete" => Ok(EnigoKey::Delete),
        "Home" => Ok(EnigoKey::Home),
        "End" => Ok(EnigoKey::End),
        "PageUp" => Ok(EnigoKey::PageUp),
        "PageDown" => Ok(EnigoKey::PageDown),
        "Numlock" => Ok(EnigoKey::Numlock),
        "ScrollLock" => Ok(EnigoKey::ScrollLock),
        "Pause" => Ok(EnigoKey::Pause),
        "PrintScr" => Ok(EnigoKey::PrintScr),
        "LMenu" => Ok(EnigoKey::LMenu),
        "LeftShift" => Ok(EnigoKey::LShift),
        "RightShift" => Ok(EnigoKey::RShift),
        "LeftControl" => Ok(EnigoKey::LControl),
        "RightControl" => Ok(EnigoKey::RControl),
        "LeftAlt" => Ok(EnigoKey::Option),
        #[cfg(target_os = "macos")]
        "RightCommand" => Ok(EnigoKey::RCommand),
        #[cfg(target_os = "windows")]
        "Play" => Ok(EnigoKey::Play),
        #[cfg(target_os = "windows")]
        "Snapshot" => Ok(EnigoKey::Snapshot),
        #[cfg(target_os = "windows")]
        "Processkey" => Ok(EnigoKey::Processkey),
        #[cfg(target_os = "windows")]
        "RButton" => Ok(EnigoKey::RButton),
        #[cfg(target_os = "windows")]
        "RWin" => Ok(EnigoKey::RWin),
        "Select" => Ok(EnigoKey::Select),
        #[cfg(target_os = "windows")]
        "Separator" => Ok(EnigoKey::Separator),
        #[cfg(target_os = "windows")]
        "Sleep" => Ok(EnigoKey::Sleep),
        "VolumeDown" => Ok(EnigoKey::VolumeDown),
        "VolumeMute" => Ok(EnigoKey::VolumeMute),
        "VolumeUp" => Ok(EnigoKey::VolumeUp),
        #[cfg(target_os = "windows")]
        "XButton1" => Ok(EnigoKey::XButton1),
        #[cfg(target_os = "windows")]
        "XButton2" => Ok(EnigoKey::XButton2),
        #[cfg(target_os = "windows")]
        "Zoom" => Ok(EnigoKey::Zoom),
        _ => {
            if key_str.len() == 1 {
                Ok(EnigoKey::Unicode(key_str.chars().next().unwrap()))
            } else {
                Err("Unknown key string")
            }
        }
    }
}

pub(crate) fn key_to_string(key: &EnigoKey) -> Result<&'static str, &'static str> {
    match key {
        EnigoKey::F1 => Ok("F1"),
        EnigoKey::F2 => Ok("F2"),
        EnigoKey::F3 => Ok("F3"),
        EnigoKey::F4 => Ok("F4"),
        EnigoKey::F5 => Ok("F5"),
        EnigoKey::F6 => Ok("F6"),
        EnigoKey::F7 => Ok("F7"),
        EnigoKey::F8 => Ok("F8"),
        EnigoKey::F9 => Ok("F9"),
        EnigoKey::F10 => Ok("F10"),
        EnigoKey::F11 => Ok("F11"),
        EnigoKey::F12 => Ok("F12"),
        EnigoKey::F13 => Ok("F13"),
        EnigoKey::F14 => Ok("F14"),
        EnigoKey::F15 => Ok("F15"),
        EnigoKey::F16 => Ok("F16"),
        EnigoKey::F17 => Ok("F17"),
        EnigoKey::F18 => Ok("F18"),
        EnigoKey::F19 => Ok("F19"),
        EnigoKey::F20 => Ok("F20"),
        EnigoKey::F21 => Ok("F21"),
        EnigoKey::F22 => Ok("F22"),
        EnigoKey::F23 => Ok("F23"),
        EnigoKey::F24 => Ok("F24"),
        EnigoKey::Escape => Ok("Escape"),
        EnigoKey::Space => Ok("Space"),
        EnigoKey::Return => Ok("Enter"),
        EnigoKey::Tab => Ok("Tab"),
        EnigoKey::Backspace => Ok("Backspace"),
        EnigoKey::CapsLock => Ok("CapsLock"),
        EnigoKey::Shift => Ok("Shift"),
        EnigoKey::Control => Ok("Control"),
        EnigoKey::Alt => Ok("Alt"),
        EnigoKey::Meta => Ok("Meta"),
        EnigoKey::LeftArrow => Ok("LeftArrow"),
        EnigoKey::RightArrow => Ok("RightArrow"),
        EnigoKey::UpArrow => Ok("UpArrow"),
        EnigoKey::DownArrow => Ok("DownArrow"),
        EnigoKey::Insert => Ok("Insert"),
        EnigoKey::Delete => Ok("Delete"),
        EnigoKey::Home => Ok("Home"),
        EnigoKey::End => Ok("End"),
        EnigoKey::PageUp => Ok("PageUp"),
        EnigoKey::PageDown => Ok("PageDown"),
        EnigoKey::Numlock => Ok("Numlock"),
        EnigoKey::ScrollLock => Ok("ScrollLock"),
        EnigoKey::Pause => Ok("Pause"),
        EnigoKey::PrintScr => Ok("PrintScr"),
        EnigoKey::LMenu => Ok("LMenu"),
        EnigoKey::LShift => Ok("LeftShift"),
        EnigoKey::RShift => Ok("RightShift"),
        EnigoKey::LControl => Ok("LeftControl"),
        EnigoKey::RControl => Ok("RightControl"),
        EnigoKey::Option => Ok("LeftAlt"),
        #[cfg(target_os = "macos")]
        EnigoKey::RCommand => Ok("RightCommand"),
        #[cfg(target_os = "windows")]
        EnigoKey::Play => Ok("Play"),
        #[cfg(target_os = "windows")]
        EnigoKey::Snapshot => Ok("Snapshot"),
        #[cfg(target_os = "windows")]
        EnigoKey::Processkey => Ok("Processkey"),
        #[cfg(target_os = "windows")]
        EnigoKey::RButton => Ok("RButton"),
        #[cfg(target_os = "windows")]
        EnigoKey::RWin => Ok("RWin"),
        EnigoKey::Select => Ok("Select"),
        #[cfg(target_os = "windows")]
        EnigoKey::Separator => Ok("Separator"),
        #[cfg(target_os = "windows")]
        EnigoKey::Sleep => Ok("Sleep"),
        EnigoKey::VolumeDown => Ok("VolumeDown"),
        EnigoKey::VolumeMute => Ok("VolumeMute"),
        EnigoKey::VolumeUp => Ok("VolumeUp"),
        #[cfg(target_os = "windows")]
        EnigoKey::XButton1 => Ok("XButton1"),
        #[cfg(target_os = "windows")]
        EnigoKey::XButton2 => Ok("XButton2"),
        #[cfg(target_os = "windows")]
        EnigoKey::Zoom => Ok("Zoom"),
        EnigoKey::Unicode(c) => {
            let mut s = String::new();
            s.push(*c);
            Ok(Box::leak(s.into_boxed_str()))
        }
        _ => Err("Unknown key"),
    }
}

pub(crate) fn string_to_button(button_str: &str) -> Result<EnigoButton, &'static str> {
    match button_str {
        "Left" => Ok(EnigoButton::Left),
        "Right" => Ok(EnigoButton::Right),
        "Middle" => Ok(EnigoButton::Middle),
        "Back" => Ok(EnigoButton::Back),
        "Forward" => Ok(EnigoButton::Forward),
        "ScrollUp" => Ok(EnigoButton::ScrollUp),
        "ScrollDown" => Ok(EnigoButton::ScrollDown),
        "ScrollLeft" => Ok(EnigoButton::ScrollLeft),
        "ScrollRight" => Ok(EnigoButton::ScrollRight),
        _ => Err("Unknown button string"),
    }
}

pub(crate) fn button_to_string(button: &EnigoButton) -> Result<&'static str, &'static str> {
    match button {
        EnigoButton::Left => Ok("Left"),
        EnigoButton::Right => Ok("Right"),
        EnigoButton::Middle => Ok("Middle"),
        EnigoButton::Back => Ok("Back"),
        EnigoButton::Forward => Ok("Forward"),
        EnigoButton::ScrollUp => Ok("ScrollUp"),
        EnigoButton::ScrollDown => Ok("ScrollDown"),
        EnigoButton::ScrollLeft => Ok("ScrollLeft"),
        EnigoButton::ScrollRight => Ok("ScrollRight"),
        _ => Err("Unknown button"),
    }
}