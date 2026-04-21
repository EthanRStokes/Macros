use std::process::Command;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, JoinHandle};
use cosmic::cosmic_config::{Config, ConfigGet, ConfigSet};
use enigo::{Enigo, Keyboard, Mouse};
use enigo::agent::Token::{Button, Key, MoveMouse, Raw, Scroll, Text};
use enigo::Key as EnigoKey;
use enigo::Button as EnigoButton;
use tracing::warn;
use crate::macros::{Instruction, Macro};

pub(crate) fn add_macro(config: &Config, mut mac: Macro) -> Result<(), String> {
    mac.ensure_id();
    config::save_macro(config, &mac)
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

    const APP_ID: &str = "com.treetrain1.Macros";
    const MACROS_DIR_NAME: &str = "macros";
    const MACRO_FILE_EXTENSION: &str = "json";
    const SELECTED_MACRO_ID_KEY: &str = "selected_macro_id";
    const LEGACY_SELECTED_MACRO_KEY: &str = "selected_macro";
    const LEGACY_MACROS_KEY: &str = "macros";

    fn app_config_dir() -> Result<PathBuf, String> {
        let mut config_dir = dirs::config_dir().ok_or_else(|| "Unable to resolve config directory".to_string())?;
        config_dir.push(APP_ID);
        Ok(config_dir)
    }

    fn ensure_macros_dir() -> Result<PathBuf, String> {
        let mut path = app_config_dir()?;
        path.push(MACROS_DIR_NAME);
        fs::create_dir_all(&path).map_err(|err| format!("Failed to create macros directory '{}': {}", path.display(), err))?;
        Ok(path)
    }

    fn macro_file_path(id: &str) -> Result<PathBuf, String> {
        if id.trim().is_empty() {
            return Err("Cannot build macro path with empty id".to_string());
        }

        let mut path = ensure_macros_dir()?;
        path.push(format!("{}.{}", id, MACRO_FILE_EXTENSION));
        Ok(path)
    }

    fn read_macro_file(path: &Path) -> Result<Macro, String> {
        let contents = fs::read_to_string(path)
            .map_err(|err| format!("Failed to read macro file '{}': {}", path.display(), err))?;
        let mut mac: Macro = serde_json::from_str(&contents)
            .map_err(|err| format!("Failed to parse macro file '{}': {}", path.display(), err))?;
        mac.ensure_id();
        Ok(mac)
    }

    fn write_macro_file(path: &Path, mac: &Macro) -> Result<(), String> {
        let serialized = serde_json::to_string_pretty(mac)
            .map_err(|err| format!("Failed to serialize macro '{}': {}", mac.name, err))?;

        // Write to a temporary file first and rename to reduce corruption risk.
        let mut temp_path = path.to_path_buf();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("Invalid macro file name '{}'", path.display()))?;
        temp_path.set_file_name(format!("{}.tmp", file_name));

        fs::write(&temp_path, serialized)
            .map_err(|err| format!("Failed to write temporary macro file '{}': {}", temp_path.display(), err))?;

        if path.exists() {
            fs::remove_file(path)
                .map_err(|err| format!("Failed to replace macro file '{}': {}", path.display(), err))?;
        }

        fs::rename(&temp_path, path)
            .map_err(|err| format!("Failed to finalize macro file '{}': {}", path.display(), err))?;

        Ok(())
    }

    fn list_macro_file_paths() -> Result<Vec<PathBuf>, String> {
        let dir = ensure_macros_dir()?;
        let entries = fs::read_dir(&dir)
            .map_err(|err| format!("Failed to scan macros directory '{}': {}", dir.display(), err))?;

        let mut paths = entries
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.is_file())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some(MACRO_FILE_EXTENSION))
            .collect::<Vec<_>>();
        paths.sort();
        Ok(paths)
    }

    fn clear_legacy_keys(config: &Config) {
        if let Err(err) = config.set(LEGACY_MACROS_KEY, Vec::<Macro>::new()) {
            warn!("Failed to clear legacy macros key: {}", err);
        }
        if let Err(err) = config.set(LEGACY_SELECTED_MACRO_KEY, Option::<usize>::None) {
            warn!("Failed to clear legacy selected macro key: {}", err);
        }
    }

    pub(crate) fn set_selected_macro_id(config: &Config, macro_id: Option<&str>) -> Result<(), String> {
        config
            .set(SELECTED_MACRO_ID_KEY, macro_id.map(|id| id.to_string()))
            .map_err(|err| {
                let error_msg = format!("Failed to save selected macro id: {}", err);
                warn!("{}", error_msg);
                error_msg
            })
    }

    pub(crate) fn get_selected_macro_id(config: &Config) -> Option<String> {
        config
            .get::<Option<String>>(SELECTED_MACRO_ID_KEY)
            .ok()
            .flatten()
    }

    pub(crate) fn save_macro(_config: &Config, mac: &Macro) -> Result<(), String> {
        let path = macro_file_path(&mac.id)?;
        write_macro_file(&path, mac)
    }

    pub(crate) fn remove_macro_by_id(_config: &Config, macro_id: &str) -> Result<(), String> {
        let path = macro_file_path(macro_id)?;
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|err| format!("Failed to remove macro file '{}': {}", path.display(), err))?;
        }
        Ok(())
    }

    pub(crate) fn get_macro_by_id(config: &Config, macro_id: &str) -> Option<Macro> {
        get_macros_from_config(config)
            .into_iter()
            .find(|mac| mac.id == macro_id)
    }

    pub(crate) fn migrate_legacy_macros_to_files(config: &Config) -> Result<(), String> {
        let mut legacy_macros = match config.get::<Vec<Macro>>(LEGACY_MACROS_KEY) {
            Ok(macros) => macros,
            Err(_) => return Ok(()),
        };

        let existing_paths = list_macro_file_paths()?;

        if existing_paths.is_empty() {
            for mac in &mut legacy_macros {
                mac.ensure_id();
                save_macro(config, mac)?;
            }
        }

        if let Ok(Some(selected_index)) = config.get::<Option<usize>>(LEGACY_SELECTED_MACRO_KEY) {
            if let Some(selected_macro) = legacy_macros.get(selected_index) {
                set_selected_macro_id(config, Some(&selected_macro.id))?;
            }
        }

        clear_legacy_keys(config);
        Ok(())
    }

    pub(crate) fn get_macros_from_config(config: &Config) -> Vec<Macro> {
        if let Err(err) = migrate_legacy_macros_to_files(config) {
            warn!("Failed to migrate legacy macro config: {}", err);
        }

        let mut macros = Vec::new();
        let paths = match list_macro_file_paths() {
            Ok(paths) => paths,
            Err(err) => {
                warn!("Failed to scan macro files: {}", err);
                return macros;
            }
        };

        for path in paths {
            match read_macro_file(&path) {
                Ok(mac) => macros.push(mac),
                Err(err) => warn!("{}", err),
            }
        }

        macros.sort_by(|left, right| {
            let name_order = left.name.to_lowercase().cmp(&right.name.to_lowercase());
            if name_order.is_eq() {
                left.id.cmp(&right.id)
            } else {
                name_order
            }
        });

        macros
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
