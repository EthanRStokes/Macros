use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread::{sleep, JoinHandle};
use cosmic::cosmic_config::{Config, ConfigGet, ConfigSet};
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

pub(crate) fn add_macro(config: &Config, mac: Macro) {
    let tx = config.transaction();
    let mut macros = config.get::<Vec<Macro>>("macros");

    if macros.is_err() {
        tx.set("macros", vec![mac]).expect("Error setting config");
    } else {
        macros.as_mut().unwrap().push(mac);
        tx.set("macros", macros.unwrap()).expect("Error unwrapping macro");
    }

    println!("Commit transaction: {:?}", tx.commit());
}

pub(crate) fn run_macro(mac: Macro, enigo: Arc<Mutex<Enigo>>) {
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
                // Lock the mutex to get access to the enigo instance
                let mut enigo = match enigo.lock() {
                    Ok(guard) => guard,
                    Err(err) => {
                        warn!("Failed to lock enigo mutex: {}", err);
                        return; // Exit if we can't get the lock
                    }
                };

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
                warn!("Instruction not implemented.");
            }
        }
    }
}

pub fn make_enigo() -> Enigo<'static> {
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

    /// Cleans up completed threads from the pool
    /// 
    /// This method checks each thread in the pool and removes those that have completed.
    /// It should be called periodically to prevent the pool from growing indefinitely.
    pub(crate) fn cleanup_completed_threads(&mut self) {
        // Keep only threads that are still running
        self.workers.retain(|worker| !worker.is_finished());
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
        #[cfg(all(unix, not(target_os = "macos")))]
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
        #[cfg(all(unix, not(target_os = "macos")))]
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

pub(crate) fn get_mouse_button_names() -> &'static [&'static str] {
    &[
        "Left",
        "Right",
        "Middle",
        "Back",
        "Forward",
        "ScrollUp",
        "ScrollDown",
        "ScrollLeft",
        "ScrollRight",
    ]
}

pub(crate) fn mouse_button_to_index(button: &EnigoButton) -> usize {
    match button {
        EnigoButton::Left => 0,
        EnigoButton::Right => 1,
        EnigoButton::Middle => 2,
        EnigoButton::Back => 3,
        EnigoButton::Forward => 4,
        EnigoButton::ScrollUp => 5,
        EnigoButton::ScrollDown => 6,
        EnigoButton::ScrollLeft => 7,
        EnigoButton::ScrollRight => 8,
        _ => 0, // Default to Left
    }
}

pub(crate) fn index_to_mouse_button(index: usize) -> EnigoButton {
    match index {
        0 => EnigoButton::Left,
        1 => EnigoButton::Right,
        2 => EnigoButton::Middle,
        3 => EnigoButton::Back,
        4 => EnigoButton::Forward,
        5 => EnigoButton::ScrollUp,
        6 => EnigoButton::ScrollDown,
        7 => EnigoButton::ScrollLeft,
        8 => EnigoButton::ScrollRight,
        _ => EnigoButton::Left, // Default fallback
    }
}

/// Config utility functions
pub(crate) mod config {
    use super::*;
    use tracing::warn;

    pub(crate) fn get_macros_from_config(config: &Config) -> Vec<Macro> {
        config.get::<Vec<Macro>>("macros").unwrap_or_else(|err| {
            warn!("Failed to get macros config: {}", err);
            Vec::new()
        })
    }

    pub(crate) fn set_macros_in_config(config: &Config, macros: Vec<Macro>) -> Result<(), String> {
        config.set("macros", macros).map_err(|err| {
            let error_msg = format!("Failed to set macros config: {}", err);
            warn!("{}", error_msg);
            error_msg
        })
    }

    pub(crate) fn update_macro_at_index(config: &Config, index: usize, new_macro: &Macro) -> Result<(), String> {
        let mut macros = get_macros_from_config(config);
        if index < macros.len() {
            macros[index] = new_macro.clone();
            set_macros_in_config(config, macros)
        } else {
            Err(format!("Macro index {} out of bounds", index))
        }
    }

    pub(crate) fn remove_macro_at_index(config: &Config, index: usize) -> Result<(), String> {
        let mut macros = get_macros_from_config(config);
        if index < macros.len() {
            macros.remove(index);
            set_macros_in_config(config, macros)
        } else {
            Err(format!("Macro index {} out of bounds", index))
        }
    }

    pub(crate) fn save_config_value<T: serde::Serialize>(config: &Config, key: &str, value: T) -> Result<(), String> {
        config.set(key, value).map_err(|err| {
            let error_msg = format!("Failed to save {} to config: {}", key, err);
            warn!("{}", error_msg);
            error_msg
        })
    }

    pub(crate) fn get_config_value<T: serde::de::DeserializeOwned>(config: &Config, key: &str) -> Option<T> {
        config.get::<T>(key).ok()
    }
}

/// Thread management utilities
pub(crate) mod thread {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread::{self};
    use tracing::warn;

    pub(crate) fn spawn_macro_thread<F>(
        thread_pool: &mut ThreadPool,
        name: String,
        task: F,
    ) -> Result<(), String>
    where
        F: FnOnce() + Send + 'static,
    {
        let thread_num = thread_pool.workers.len();
        let thread_name = format!("macro_thread_{}: {}", thread_num, name);

        match thread::Builder::new().name(thread_name).spawn(task) {
            Ok(thread) => {
                thread_pool.add_worker(thread);
                thread_pool.cleanup_completed_threads();
                Ok(())
            }
            Err(err) => {
                let error_msg = format!("Failed to spawn thread '{}': {}", name, err);
                warn!("{}", error_msg);
                Err(error_msg)
            }
        }
    }

    pub(crate) fn create_loop_task(
        mac: Macro,
        enigo: Arc<Mutex<Enigo<'static>>>,
        loop_flag: Arc<Mutex<bool>>,
    ) -> impl FnOnce() + Send + 'static {
        move || {
            println!("Starting macro loop: {}", mac.name);
            loop {
                // Check if we should stop looping
                if let Ok(should_continue) = loop_flag.lock() {
                    if !*should_continue {
                        break;
                    }
                } else {
                    warn!("Failed to lock loop flag, stopping loop");
                    break;
                }

                run_macro(mac.clone(), Arc::clone(&enigo));

                // Small delay between iterations to prevent overwhelming the system
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            println!("Macro loop stopped.");
        }
    }

    pub(crate) fn create_single_run_task(
        mac: Macro,
        enigo: Arc<Mutex<Enigo<'static>>>,
    ) -> impl FnOnce() + Send + 'static {
        move || {
            println!("Running macro: {}", mac.name);
            run_macro(mac, enigo);
            println!("Macro complete.");
        }
    }
}

/// Instruction creation utilities
pub(crate) mod instruction_utils {
    use super::*;
    use enigo::agent::Token;
    use enigo::{Axis, Button, Coordinate, Direction, Key};

    pub(crate) fn create_default_instruction(instruction_type: usize) -> Option<Instruction> {
        match instruction_type {
            0 => Some(Instruction::Wait(1000)), // Default wait time
            1 => Some(Instruction::Token(Token::Text("text".into()))),
            2 => Some(Instruction::Token(Token::Key(Key::Unicode('a'), Direction::Click))),
            3 => Some(Instruction::Token(Token::Button(Button::Left, Direction::Click))),
            4 => Some(Instruction::Token(Token::MoveMouse(0, 0, Coordinate::Rel))),
            5 => Some(Instruction::Token(Token::Scroll(4, Axis::Vertical))), // Default scroll amount
            6 => Some(Instruction::Script("script".into())),
            _ => None,
        }
    }

    pub(crate) fn get_instruction_type_names() -> &'static [&'static str] {
        &[
            "Wait",
            "Text",
            "Key",
            "Mouse Button",
            "Move Mouse",
            "Scroll",
            "Run Script",
        ]
    }
}

/// UI helper utilities
pub(crate) mod ui_utils {
    use enigo::{Axis, Coordinate, Direction};

    pub(crate) fn direction_to_index(direction: &Direction) -> usize {
        match direction {
            Direction::Click => 0,
            Direction::Press => 1,
            Direction::Release => 2,
        }
    }

    pub(crate) fn index_to_direction(index: usize) -> Direction {
        match index {
            0 => Direction::Click,
            1 => Direction::Press,
            2 => Direction::Release,
            _ => Direction::Click, // Default fallback
        }
    }

    pub(crate) fn coordinate_to_index(coordinate: &Coordinate) -> usize {
        match coordinate {
            Coordinate::Abs => 0,
            Coordinate::Rel => 1,
        }
    }

    pub(crate) fn index_to_coordinate(index: usize) -> Coordinate {
        match index {
            0 => Coordinate::Abs,
            1 => Coordinate::Rel,
            _ => Coordinate::Abs, // Default fallback
        }
    }

    pub(crate) fn axis_to_index(axis: &Axis) -> usize {
        match axis {
            Axis::Vertical => 0,
            Axis::Horizontal => 1,
        }
    }

    pub(crate) fn index_to_axis(index: usize) -> Axis {
        match index {
            0 => Axis::Vertical,
            1 => Axis::Horizontal,
            _ => Axis::Vertical, // Default fallback
        }
    }

    pub(crate) fn get_direction_names() -> &'static [&'static str] {
        &["Click", "Press", "Release"]
    }

    pub(crate) fn get_coordinate_names() -> &'static [&'static str] {
        &["Absolute", "Relative"]
    }

    pub(crate) fn get_axis_names() -> &'static [&'static str] {
        &["Vertical", "Horizontal"]
    }
}

/// Loop control utilities
pub(crate) mod loop_control {
    use std::sync::{Arc, Mutex};
    use tracing::warn;

    pub(crate) fn set_loop_state(loop_flag: &Arc<Mutex<bool>>, state: bool) -> Result<(), String> {
        match loop_flag.lock() {
            Ok(mut flag) => {
                *flag = state;
                Ok(())
            }
            Err(err) => {
                let error_msg = format!("Failed to set loop state: {}", err);
                warn!("{}", error_msg);
                Err(error_msg)
            }
        }
    }

    pub(crate) fn get_loop_state(loop_flag: &Arc<Mutex<bool>>) -> bool {
        loop_flag.lock().map(|flag| *flag).unwrap_or(false)
    }

    pub(crate) fn stop_loop(loop_flag: &Arc<Mutex<bool>>) -> Result<(), String> {
        set_loop_state(loop_flag, false)
    }

    pub(crate) fn start_loop(loop_flag: &Arc<Mutex<bool>>) -> Result<(), String> {
        set_loop_state(loop_flag, true)
    }
}
