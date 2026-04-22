use crate::app::Message::*;
use crate::macros::{Instruction, Macro};
use crate::util::{add_macro, key_to_string, string_to_key, ThreadPool};
use crate::util::run_macro;
use crate::util::{get_mouse_button_names, mouse_button_to_index, index_to_mouse_button};
use crate::util::{config, thread, loop_control};
use tracing::warn;
use cosmic::app::{Core, Task};
use cosmic::cosmic_config::{Config, ConfigGet};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::iced::{Color};
use cosmic::iced::border::Border;
use cosmic::iced::keyboard;
use cosmic::widget::{column, container, mouse_area, row, scrollable, tooltip};
use cosmic::{executor, widget, ApplicationExt, Apply, Element};
use enigo::agent::Token;
use enigo::{Axis, Coordinate, Direction, Enigo, Key};
use slotmap::{DefaultKey, SecondaryMap, SlotMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
#[cfg(target_os = "linux")]
use cosmic::dialog::ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use cosmic::iced::futures::executor::block_on;
#[cfg(not(target_os = "linux"))]
use cosmic::iced::futures::{SinkExt, Stream};
use cosmic::iced::futures::StreamExt;
#[cfg(not(target_os = "linux"))]
use cosmic::iced::futures::channel::mpsc::Sender;
#[cfg(not(target_os = "linux"))]
use cosmic::iced::stream::channel;
use cosmic::iced::widget::button;
#[cfg(not(target_os = "linux"))]
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager,
    HotKeyState,
    hotkey::{Code, HotKey, Modifiers},
};

// Constants for default values
const CLEAR_CONFIRM_TIMEOUT_SECS: u64 = 5;
const LOOP_ITERATION_DELAY_MS: u64 = 1;

// Constants for bundled icon paths
const ICON_REMOVE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/res/icons/remove.svg");
const ICON_UP: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/res/icons/up.svg");
const ICON_DOWN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/res/icons/down.svg");

/// Messages that are used specifically by our [`App`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Message {
    SelectMacro(usize),
    RunMacro,
    ToggleLoopMode(bool),
    SetTitle(String),
    AddInstruction(Vec<usize>, usize, Instruction),
    EditInstruction(Vec<usize>, Instruction),
    StartKeyCapture(Vec<usize>),
    KeyCaptureEvent(keyboard::Event),
    RemoveInstruction(Vec<usize>),
    ReorderInstruction(Vec<usize>, isize),
    MoveInstructionOut(Vec<usize>),
    HoverInstruction(Vec<usize>),
    UnhoverInstruction(Vec<usize>),
    StartInstructionDrag(Vec<usize>),
    HoverBodyDropEnd(Vec<usize>),
    UnhoverBodyDropEnd(Vec<usize>),
    DropDraggedInstructionOn(Vec<usize>),
    DropDraggedInstructionAtEnd(Vec<usize>),
    CancelInstructionDrag,
    ClearInstructions,
    ClearInstructionsTimeout(u64),
    SaveMacro,
    NewMacro,
    RemoveMacro,
    #[cfg(not(target_os = "linux"))]
    GlobalHotkeyEvent(GlobalHotKeyEvent)
}

/// The [`App`] stores application-specific state.
pub(crate) struct App {
    /// COSMIC app settings
    core: Core,
    macro_selected: Option<usize>,
    current_macro: Option<Macro>,
    /// The application config
    pub(crate) config: Config,
    /// Enigo is an API for mouse and keyboard control
    enigo: Arc<Mutex<Enigo<'static>>>,
    thread_pool: ThreadPool,
    macros: SlotMap<DefaultKey, Macro>,
    macro_keys: SecondaryMap<DefaultKey, String>,
    macro_strs: Vec<String>,
    /// Flag to control looping execution
    is_looping: Arc<Mutex<bool>>,
    /// UI setting for whether to run in loop mode
    loop_mode_enabled: bool,
    /// Whether the remove-macro action is armed for confirmation
    confirm_remove_macro: bool,
    /// Whether the clear-instructions action is armed for confirmation
    confirm_clear_instructions: bool,
    /// Generation counter used to ignore stale clear-confirm timeout tasks
    clear_confirm_generation: u64,
    /// Which instruction index is currently waiting for key capture
    key_capture_path: Option<Vec<usize>>,
    /// Path currently hovered to reveal the drag handle
    hovered_instruction_path: Option<Vec<usize>>,
    /// Path currently being dragged from the handle
    dragging_instruction_path: Option<Vec<usize>>,
    /// Current drag-and-drop target
    drag_drop_target: Option<DragDropTarget>,
    #[cfg(not(target_os = "linux"))]
    manager: GlobalHotKeyManager,
    #[cfg(not(target_os = "linux"))]
    run_macro_id: u32,
    #[cfg(not(target_os = "linux"))]
    stop_loop_id: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DragDropTarget {
    Instruction(Vec<usize>),
    EndOfBody(Vec<usize>),
}

impl App {
    /// Updates the currently selected macro
    ///
    /// # Arguments
    /// * `selected` - The index of the selected macro, or None to clear selection
    fn update_macro(&mut self, selected: Option<usize>) {
        self.macro_selected = selected;
        self.confirm_remove_macro = false;
        self.confirm_clear_instructions = false;
        self.key_capture_path = None;
        self.hovered_instruction_path = None;
        self.dragging_instruction_path = None;
        self.drag_drop_target = None;
        let macros = config::get_macros_from_config(&self.config);

        if let Some(index) = selected {
            if let Some(mac) = macros.get(index).cloned() {
                if let Err(err) = config::set_selected_macro_id(&self.config, Some(&mac.id)) {
                    warn!("Failed to save selected macro id: {}", err);
                }
                self.current_macro = Some(mac);
                return;
            }
        }

        self.macro_selected = None;
        self.current_macro = None;
        self.key_capture_path = None;
        self.hovered_instruction_path = None;
        self.dragging_instruction_path = None;
        self.drag_drop_target = None;
        if let Err(err) = config::set_selected_macro_id(&self.config, None) {
            warn!("Failed to clear selected macro id: {}", err);
        }
    }

    /// Refreshes the list of macros from the config
    /// 
    /// This method retrieves all macros from the config and updates the internal
    /// data structures (macros, macro_keys, macro_strs) accordingly.
    fn update_macros(&mut self) {
        let macs = config::get_macros_from_config(&self.config);
        self.macros.clear();
        self.macro_keys.clear();
        self.macro_strs.clear();
        for mac in &macs {
            let key = self.macros.insert(mac.clone());
            let mac = self.macros.get_mut(key).unwrap();
            self.macro_keys.insert(key, mac.name.clone());
            self.macro_strs.push(mac.name.clone());
        }

        if let Some(selected_id) = config::get_selected_macro_id(&self.config) {
            if let Some((index, mac)) = macs
                .iter()
                .enumerate()
                .find(|(_, mac)| mac.id == selected_id)
            {
                self.macro_selected = Some(index);
                self.current_macro = Some(mac.clone());
                return;
            }
        }

        self.macro_selected = None;
        self.current_macro = None;
        self.key_capture_path = None;
        self.hovered_instruction_path = None;
        self.dragging_instruction_path = None;
        self.drag_drop_target = None;
    }

    /// Automatically saves the current macro to config and updates the app state
    /// This should be called whenever the current macro is modified
    fn auto_save_current_macro(&mut self) {
        if let Some(mac) = &self.current_macro {
            if let Err(err) = config::save_macro(&self.config, mac) {
                warn!("Failed to update macro: {}", err);
            } else {
                if let Err(err) = config::set_selected_macro_id(&self.config, Some(&mac.id)) {
                    warn!("Failed to save selected macro id: {}", err);
                }
                self.update_macros();
            }
        }
    }
}

fn map_iced_key_to_enigo_key(key: keyboard::Key<&str>) -> Option<Key> {
    match key {
        keyboard::Key::Character(text) => {
            let mut chars = text.chars();
            let c = chars.next()?;
            if chars.next().is_none() {
                Some(Key::Unicode(c))
            } else {
                None
            }
        }
        keyboard::Key::Named(named) => {
            let named_text = format!("{:?}", named);
            string_to_key(&named_text).ok()
        }
        keyboard::Key::Unidentified => None,
    }
}

fn add_default_config(config: &Config) {
    if let Err(err) = add_macro(config, Macro::new("New Macro".into(), "description".into(), vec![])) {
        warn!("Failed to add default macro: {}", err);
    }
}

fn clamp_loop_count(count: u32) -> u32 {
    count.clamp(1, 1000)
}

fn get_instruction<'a>(instructions: &'a [Instruction], path: &[usize]) -> Option<&'a Instruction> {
    let (first, rest) = path.split_first()?;
    let instruction = instructions.get(*first)?;

    if rest.is_empty() {
        Some(instruction)
    } else {
        match instruction {
            Instruction::Loop { body, .. } => get_instruction(body, rest),
            _ => None,
        }
    }
}

fn get_instruction_mut<'a>(instructions: &'a mut [Instruction], path: &[usize]) -> Option<&'a mut Instruction> {
    let (first, rest) = path.split_first()?;
    let instruction = instructions.get_mut(*first)?;

    if rest.is_empty() {
        Some(instruction)
    } else {
        match instruction {
            Instruction::Loop { body, .. } => get_instruction_mut(body, rest),
            _ => None,
        }
    }
}

fn get_body_mut<'a>(instructions: &'a mut Vec<Instruction>, path: &[usize]) -> Option<&'a mut Vec<Instruction>> {
    if path.is_empty() {
        return Some(instructions);
    }

    let (first, rest) = path.split_first()?;
    let instruction = instructions.get_mut(*first)?;

    match instruction {
        Instruction::Loop { body, .. } => {
            if rest.is_empty() {
                Some(body)
            } else {
                get_body_mut(body, rest)
            }
        }
        _ => None,
    }
}

fn get_body<'a>(instructions: &'a [Instruction], path: &[usize]) -> Option<&'a [Instruction]> {
    if path.is_empty() {
        return Some(instructions);
    }

    let (first, rest) = path.split_first()?;
    let instruction = instructions.get(*first)?;

    match instruction {
        Instruction::Loop { body, .. } => {
            if rest.is_empty() {
                Some(body.as_slice())
            } else {
                get_body(body, rest)
            }
        }
        _ => None,
    }
}

fn parent_path(path: &[usize]) -> Option<(&[usize], usize)> {
    let (last, parent) = path.split_last()?;
    Some((parent, *last))
}

fn replace_instruction_at_path(instructions: &mut Vec<Instruction>, path: &[usize], new_instruction: Instruction) -> bool {
    if let Some(instruction) = get_instruction_mut(instructions.as_mut_slice(), path) {
        *instruction = new_instruction;
        true
    } else {
        false
    }
}

fn remove_instruction_at_path(instructions: &mut Vec<Instruction>, path: &[usize]) -> bool {
    let Some((parent, index)) = parent_path(path) else {
        return false;
    };

    let Some(body) = get_body_mut(instructions, parent) else {
        return false;
    };

    if index < body.len() {
        body.remove(index);
        true
    } else {
        false
    }
}

fn reorder_instruction_at_path(instructions: &mut Vec<Instruction>, path: &[usize], direction: isize) -> bool {
    let Some((parent, index)) = parent_path(path) else {
        return false;
    };

    let Some(body) = get_body_mut(instructions, parent) else {
        return false;
    };

    let len = body.len();
    if len <= 1 || index >= len {
        return false;
    }

    let new_index = if direction < 0 {
        if index > 0 { index - 1 } else { index }
    } else if index < len - 1 {
        index + 1
    } else {
        index
    };

    if new_index != index {
        body.swap(index, new_index);
        true
    } else {
        false
    }
}

fn insert_instruction_at_path(instructions: &mut Vec<Instruction>, body_path: &[usize], index: usize, instruction: Instruction) -> bool {
    let Some(body) = get_body_mut(instructions, body_path) else {
        return false;
    };

    if index <= body.len() {
        body.insert(index, instruction);
        true
    } else {
        false
    }
}

fn can_move_instruction_out(path: &[usize]) -> bool {
    path.len() >= 2
}

fn path_is_prefix(prefix: &[usize], path: &[usize]) -> bool {
    prefix.len() <= path.len() && prefix.iter().zip(path.iter()).all(|(a, b)| a == b)
}

fn can_move_instruction_to_body(instructions: &[Instruction], source: &[usize], dest_body_path: &[usize], dest_index: usize) -> bool {
    let Some((src_parent, src_index)) = parent_path(source) else {
        return false;
    };

    if path_is_prefix(source, dest_body_path) {
        return false;
    }

    let Some(dest_body) = get_body(instructions, dest_body_path) else {
        return false;
    };

    if dest_index > dest_body.len() {
        return false;
    }

    let mut adjusted = dest_index;
    if src_parent == dest_body_path && src_index < adjusted {
        adjusted -= 1;
    }

    !(src_parent == dest_body_path && src_index == adjusted)
}

fn move_instruction_to_body(instructions: &mut Vec<Instruction>, source: &[usize], dest_body_path: &[usize], dest_index: usize) -> bool {
    if !can_move_instruction_to_body(instructions, source, dest_body_path, dest_index) {
        return false;
    }

    let Some((src_parent, src_index)) = parent_path(source) else {
        return false;
    };

    let mut adjusted = dest_index;
    if src_parent == dest_body_path && src_index < adjusted {
        adjusted -= 1;
    }

    let Some(moved) = ({
        let Some((parent, index)) = parent_path(source) else {
            return false;
        };
        let Some(body) = get_body_mut(instructions, parent) else {
            return false;
        };
        if index >= body.len() {
            return false;
        }
        Some(body.remove(index))
    }) else {
        return false;
    };

    insert_instruction_at_path(instructions, dest_body_path, adjusted, moved)
}

fn can_drop_instruction_on_target(instructions: &[Instruction], source: &[usize], target: &[usize]) -> bool {
    if source == target || path_is_prefix(source, target) {
        return false;
    }

    let Some(target_instruction) = get_instruction(instructions, target) else {
        return false;
    };

    let Some((target_parent, target_index)) = parent_path(target) else {
        return false;
    };

    let _ = target_instruction;
    can_move_instruction_to_body(instructions, source, target_parent, target_index)
}

fn drop_instruction_on_target(instructions: &mut Vec<Instruction>, source: &[usize], target: &[usize]) -> bool {
    let Some(target_instruction) = get_instruction(instructions, target).cloned() else {
        return false;
    };

    let Some((target_parent, target_index)) = parent_path(target) else {
        return false;
    };

    let _ = target_instruction;
    move_instruction_to_body(instructions, source, target_parent, target_index)
}

fn can_drop_instruction_at_end(instructions: &[Instruction], source: &[usize], body_path: &[usize]) -> bool {
    let Some(body) = get_body(instructions, body_path) else {
        return false;
    };
    can_move_instruction_to_body(instructions, source, body_path, body.len())
}

fn drop_instruction_at_end(instructions: &mut Vec<Instruction>, source: &[usize], body_path: &[usize]) -> bool {
    let Some(body) = get_body(instructions, body_path) else {
        return false;
    };
    move_instruction_to_body(instructions, source, body_path, body.len())
}

fn move_instruction_out_of_loop(instructions: &mut Vec<Instruction>, path: &[usize]) -> bool {
    let Some((loop_body_path, child_index)) = parent_path(path) else {
        return false;
    };
    let Some((grand_parent_path, loop_index)) = parent_path(loop_body_path) else {
        return false;
    };

    let moved = {
        let Some(loop_body) = get_body_mut(instructions, loop_body_path) else {
            return false;
        };
        if child_index >= loop_body.len() {
            return false;
        }
        loop_body.remove(child_index)
    };

    let Some(grand_body) = get_body_mut(instructions, grand_parent_path) else {
        return false;
    };

    if loop_index <= grand_body.len() {
        grand_body.insert(loop_index + 1, moved);
        true
    } else {
        false
    }
}

fn instruction_from_selection(selected: usize) -> Option<Instruction> {
    crate::util::instruction_utils::create_default_instruction(selected)
}

fn render_instruction_add_dropdown(body_path: Vec<usize>, insert_index: usize) -> Element<'static, Message> {
    cosmic::widget::dropdown(
        crate::util::instruction_utils::get_instruction_type_names(),
        None,
        move |selected| match instruction_from_selection(selected) {
            Some(instruction) => AddInstruction(body_path.clone(), insert_index, instruction),
            None => unreachable!(),
        },
    )
    .into()
}

fn render_instruction_list(app: &App, instructions: &[Instruction], body_path: Vec<usize>) -> Element<'static, Message> {
    let spacing = cosmic::theme::active().cosmic().spacing;
    let mut children: Vec<Element<Message>> = Vec::new();

    for (index, ins) in instructions.iter().cloned().enumerate() {
        let mut full_path = body_path.clone();
        full_path.push(index);

        let instruction_source = ins.clone();
        let instruction: Element<Message> = match instruction_source {
            Instruction::Token(token) => match token {
                Token::Text(text) => row![
                    widget::text::body("Text:".to_string()).align_y(Alignment::Center),
                    widget::text_input("", text)
                        .on_input({
                            let path = full_path.clone();
                            move |x| EditInstruction(path.clone(), Instruction::Token(Token::Text(x)))
                        }),
                ]
                .spacing(10)
                .into(),
                Token::Key(key, direction) => {
                    let key_label = if app.key_capture_path.as_ref() == Some(&full_path) {
                        "Press any key...".to_string()
                    } else {
                        format!("{}", key_to_string(&key).unwrap_or("Unknown"))
                    };

                    row![
                        widget::text::body("Key:".to_string()).align_y(Alignment::Center),
                        button(cosmic::widget::text(key_label))
                            .on_press(StartKeyCapture(full_path.clone()))
                            .width(Length::Fill),
                        widget::dropdown(
                            &["Click", "Press", "Release"],
                            Some(if direction == Direction::Click { 0usize } else if direction == Direction::Press { 1usize } else { 2usize }),
                            {
                                let path = full_path.clone();
                                move |x: usize| {
                                    EditInstruction(
                                        path.clone(),
                                        Instruction::Token(Token::Key(
                                            key.clone(),
                                            if x == 0usize { Direction::Click } else if x == 1usize { Direction::Press } else { Direction::Release },
                                        )),
                                    )
                                }
                            },
                        ),
                    ]
                    .spacing(10)
                    .width(Length::Fill)
                    .into()
                }
                Token::Raw(keycode, _) => widget::text::body(format!("Raw: {:?}", keycode)).into(),
                Token::Button(button, direction) => row![
                    widget::text::body("Mouse:".to_string()).align_y(Alignment::Center),
                    widget::dropdown(
                        get_mouse_button_names(),
                        Some(mouse_button_to_index(&button)),
                        {
                            let path = full_path.clone();
                            move |x: usize| {
                                EditInstruction(path.clone(), Instruction::Token(Token::Button(index_to_mouse_button(x), direction.clone())))
                            }
                        },
                    ),
                    widget::dropdown(
                        &["Click", "Press", "Release"],
                        Some(if direction == Direction::Click { 0usize } else if direction == Direction::Press { 1usize } else { 2usize }),
                        {
                            let path = full_path.clone();
                            move |x: usize| {
                                EditInstruction(
                                    path.clone(),
                                    Instruction::Token(Token::Button(
                                        button,
                                        if x == 0 { Direction::Click } else if x == 1 { Direction::Press } else { Direction::Release },
                                    )),
                                )
                            }
                        },
                    ),
                ]
                .spacing(10)
                .width(Length::Fill)
                .into(),
                Token::MoveMouse(x, y, coordinate) => row![
                    widget::text::body("Move mouse:".to_string()).align_y(Alignment::Center),
                    widget::text_input("X", format!("{}", x))
                        .on_input({
                            let path = full_path.clone();
                            move |new_x| EditInstruction(path.clone(), Instruction::Token(Token::MoveMouse(new_x.parse().unwrap_or(x), y, coordinate.clone())))
                        }),
                    widget::text_input("Y", format!("{}", y))
                        .on_input({
                            let path = full_path.clone();
                            move |new_y| EditInstruction(path.clone(), Instruction::Token(Token::MoveMouse(x, new_y.parse().unwrap_or(y), coordinate.clone())))
                        }),
                    widget::dropdown(
                        &["Absolute", "Relative"],
                        Some(if coordinate == Coordinate::Abs { 0usize } else { 1usize }),
                        {
                            let path = full_path.clone();
                            move |coord: usize| EditInstruction(path.clone(), Instruction::Token(Token::MoveMouse(x, y, if coord == 0 { Coordinate::Abs } else { Coordinate::Rel })))
                        },
                    ),
                ]
                .spacing(10)
                .into(),
                Token::Scroll(amount, axis) => row![
                    widget::text::body("Scroll:".to_string()).align_y(Alignment::Center),
                    widget::text_input("Amount", format!("{}", amount))
                        .on_input({
                            let path = full_path.clone();
                            move |new_amount| EditInstruction(path.clone(), Instruction::Token(Token::Scroll(new_amount.parse().unwrap_or(amount), axis.clone())))
                        }),
                    widget::dropdown(
                        &["Vertical", "Horizontal"],
                        Some(if axis == Axis::Vertical { 0 } else { 1 }),
                        {
                            let path = full_path.clone();
                            move |new_axis: usize| EditInstruction(path.clone(), Instruction::Token(Token::Scroll(amount, if new_axis == 0 { Axis::Vertical } else { Axis::Horizontal })))
                        },
                    ),
                ]
                .spacing(10)
                .into(),
                _ => widget::text::body("Token not implemented").into(),
            },
            Instruction::Wait(duration) => row![
                widget::text::body("Wait (ms):".to_string()).align_y(Alignment::Center),
                widget::text_input("", duration.to_string())
                    .on_input({
                        let path = full_path.clone();
                        move |x| EditInstruction(path.clone(), Instruction::Wait(x.parse().unwrap_or(duration)))
                    }),
            ]
            .spacing(10)
            .into(),
            Instruction::Script(script) => row![
                widget::text::body("Script:".to_string()).align_y(Alignment::Center),
                widget::text_input("", script)
                    .on_input({
                        let path = full_path.clone();
                        move |x| EditInstruction(path.clone(), Instruction::Script(x))
                    }),
            ]
            .spacing(10)
            .into(),
            Instruction::Loop { count, body } => {
                let clamped_count = clamp_loop_count(count);
                let nested = render_instruction_list(app, &body, full_path.clone());

                column![
                    row![
                        widget::text::body("Loop:".to_string()).align_y(Alignment::Center),
                        widget::text_input("Count", clamped_count.to_string())
                            .on_input({
                                let path = full_path.clone();
                                move |x| {
                                    let parsed = x.parse::<u32>().unwrap_or(clamped_count);
                                    let next_count = clamp_loop_count(parsed);
                                    EditInstruction(
                                        path.clone(),
                                        Instruction::Loop {
                                            count: next_count,
                                            body: body.clone(),
                                        },
                                    )
                                }
                            }),
                        tooltip(
                            cosmic::widget::text("Loop section"),
                            container("Repeat the nested instructions a fixed number of times"),
                            tooltip::Position::Top,
                        ),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    container(
                        column![
                            widget::text::body("Loop body"),
                            nested,
                        ]
                        .spacing(spacing.space_xs)
                        .padding([0, 12, 0, 12]),
                    )
                    .width(Length::Fill),
                ]
                .spacing(spacing.space_xxs)
                .into()
            }
        };

        let show_handle = app.hovered_instruction_path.as_ref() == Some(&full_path)
            || app.dragging_instruction_path.as_ref() == Some(&full_path);
        let handle: Element<Message> = if show_handle {
            mouse_area(button(cosmic::widget::text("⋮⋮")).padding([6, 8]))
                .on_drag(StartInstructionDrag(full_path.clone()))
                .on_release(CancelInstructionDrag)
                .into()
        } else {
            container(cosmic::widget::text("  "))
                .width(Length::Fixed(24.0))
                .into()
        };

        let controls = container(
            row![
                tooltip(
                    widget::button::icon(widget::icon::from_path(PathBuf::from(ICON_UP))).padding(8).on_press(ReorderInstruction(full_path.clone(), -1)),
                    container("Move up"),
                    tooltip::Position::Top,
                ),
                tooltip(
                    widget::button::icon(widget::icon::from_path(PathBuf::from(ICON_DOWN))).padding(8).on_press(ReorderInstruction(full_path.clone(), 1)),
                    container("Move down"),
                    tooltip::Position::Bottom,
                ),
                tooltip(
                    widget::button::icon(widget::icon::from_path(PathBuf::from(ICON_REMOVE))).padding(8).on_press(RemoveInstruction(full_path.clone())),
                    container("Remove instruction"),
                    tooltip::Position::Left,
                ),
                render_instruction_add_dropdown(body_path.clone(), index),
            ]
            .spacing(spacing.space_xs)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Alignment::Center);

        let row_content = row![
            container(handle).width(Length::Fixed(28.0)).align_x(Alignment::Center),
            instruction,
            controls
        ]
        .spacing(spacing.space_xs)
        .width(Length::Fill);

        let header = mouse_area(row_content)
            .on_enter(HoverInstruction(full_path.clone()))
            .on_exit(UnhoverInstruction(full_path.clone()))
            .on_release(DropDraggedInstructionOn(full_path.clone()));

        if let Instruction::Loop { body, .. } = ins.clone() {
            let end_is_target = matches!(app.drag_drop_target, Some(DragDropTarget::EndOfBody(ref p)) if p == &full_path);
            let drop_zone = mouse_area(
                container(widget::text::body(""))
                    .width(Length::Fill)
                    .padding([8, 12])
                    .style(move |_theme| {
                        let mut border = Border::default();
                        border.width = 2.0;
                        border.color = if end_is_target {
                            Color::from_rgb8(76, 154, 255)
                        } else {
                            Color::TRANSPARENT
                        };

                        cosmic::widget::container::Style {
                            border,
                            ..Default::default()
                        }
                    }),
            )
            .on_enter(HoverBodyDropEnd(full_path.clone()))
            .on_exit(UnhoverBodyDropEnd(full_path.clone()))
            .on_release(DropDraggedInstructionAtEnd(full_path.clone()));

            children.push(
                column![
                    header,
                    drop_zone,
                    container(render_instruction_list(app, &body, full_path.clone())).padding([0, 0, 0, 18]),
                ]
                .spacing(spacing.space_xs)
                .into(),
            );
        } else {
            children.push(header.into());
        }
    }

    children.push(render_instruction_add_dropdown(body_path, instructions.len()));

    widget::column::with_children(children)
        .spacing(spacing.space_xs)
        .into()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GlobalShortcutAction {
    RunMacro,
    StopLoop,
}

fn spawn_global_shortcut_action(
    action: GlobalShortcutAction,
    config: Config,
    enigo: Arc<Mutex<Enigo<'static>>>,
    is_looping: Arc<Mutex<bool>>,
) {
    tokio::spawn(async move {
        execute_global_shortcut_action(action, &config, &enigo, &is_looping);
    });
}

fn execute_global_shortcut_action(
    action: GlobalShortcutAction,
    config: &Config,
    enigo: &Arc<Mutex<Enigo<'static>>>,
    is_looping: &Arc<Mutex<bool>>,
) {
    match action {
        GlobalShortcutAction::RunMacro => run_selected_macro_from_shortcut(config, enigo, is_looping),
        GlobalShortcutAction::StopLoop => stop_macro_loop_from_shortcut(is_looping),
    }
}

fn run_selected_macro_from_shortcut(
    config: &Config,
    enigo: &Arc<Mutex<Enigo<'static>>>,
    is_looping: &Arc<Mutex<bool>>,
) {
    println!("Global shortcut activated: run_macro");

    let loop_mode_enabled = config.get::<bool>("loop_mode_enabled").unwrap_or(false);
    let currently_looping = is_looping.lock().map(|state| *state).unwrap_or(false);

    if loop_mode_enabled && currently_looping {
        println!("Macro is already looping, ignoring run request");
        return;
    }
    let selected_macro_id = match config::get_selected_macro_id(config) {
        Some(id) => id,
        None => {
            println!("No macro currently selected for global shortcut");
            return;
        }
    };

    let Some(mac) = config::get_macro_by_id(config, &selected_macro_id) else {
        println!("No macro found with id {}", selected_macro_id);
        return;
    };

    let enigo = Arc::clone(enigo);

    if loop_mode_enabled {
        if let Ok(mut state) = is_looping.lock() {
            *state = true;
        }

        let loop_flag = Arc::clone(is_looping);
        tokio::task::spawn_blocking(move || {
            println!("Starting macro loop via global shortcut: {}", mac.name);
            loop {
                if let Ok(should_continue) = loop_flag.lock() {
                    if !*should_continue {
                        break;
                    }
                } else {
                    warn!("Failed to lock loop flag, stopping loop");
                    break;
                }

                run_macro(mac.clone(), Arc::clone(&enigo));
                std::thread::sleep(std::time::Duration::from_millis(LOOP_ITERATION_DELAY_MS));
            }
            println!("Macro loop stopped via global shortcut.");
        });
    } else {
        tokio::task::spawn_blocking(move || {
            println!("Running macro via global shortcut: {}", mac.name);
            run_macro(mac, enigo);
            println!("Macro complete.");
        });
    }
}

fn stop_macro_loop_from_shortcut(is_looping: &Arc<Mutex<bool>>) {
    println!("Global shortcut activated: stop_loop");
    if let Ok(mut state) = is_looping.lock() {
        *state = false;
        println!("Loop stop requested via global shortcut.");
    } else {
        println!("Failed to access loop flag.");
    }
}

#[cfg(not(target_os = "linux"))]
fn hotkey_sub() -> impl Stream<Item = Message> {
    channel(32, |mut sender: Sender<Message>| async move {
        let receiver = GlobalHotKeyEvent::receiver();
        // Poll for global hotkey events every 50ms and emit only key press events.
        loop {
            if let Ok(event) = receiver.try_recv() {
                if event.state == HotKeyState::Pressed {
                    sender
                        .send(Message::GlobalHotkeyEvent(event))
                        .await
                        .unwrap();
                }
            }
            async_std::task::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
}

/// Implement [`cosmic::Application`] to integrate with COSMIC.
impl cosmic::Application for App {
    /// Default async executor to use with the app.
    type Executor = executor::Default;

    /// Argument received [`cosmic::Application::new`].
    type Flags = ();

    /// Message type specific to our [`App`].
    type Message = Message;

    /// The unique application ID to supply to the window manager.
    const APP_ID: &'static str = "com.treetrain1.Macros";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    /// Creates the application, and optionally emits command on initialize.
    fn init(core: Core, _input: Self::Flags) -> (Self, Task<Self::Message>) {
        #[cfg(not(target_os = "linux"))]
        let (manager, run_macro_id, stop_loop_id) = {
            let manager = GlobalHotKeyManager::new().unwrap();
            let run_macro_hotkey = HotKey::new(Some(Modifiers::CONTROL), Code::F6);
            let stop_loop_hotkey = HotKey::new(Some(Modifiers::CONTROL), Code::F7);
            let run_macro_id = run_macro_hotkey.id();
            let stop_loop_id = stop_loop_hotkey.id();

            manager.register(run_macro_hotkey).expect("Failed to register 'start' keybind");
            manager.register(stop_loop_hotkey).expect("Failed to register 'stop' keybind");
            (manager, run_macro_id, stop_loop_id)
        };

        let mut app = App {
            core,
            macro_selected: None,
            current_macro: None,
            config: Config::new(Self::APP_ID, 1).unwrap(),
            enigo: Arc::new(Mutex::from(crate::util::make_enigo())),
            thread_pool: ThreadPool::new(),
            macros: SlotMap::new(),
            macro_keys: SecondaryMap::new(),
            macro_strs: vec![],
            is_looping: Arc::new(Mutex::new(false)),
            loop_mode_enabled: false,
            confirm_remove_macro: false,
            confirm_clear_instructions: false,
            clear_confirm_generation: 0,
            key_capture_path: None,
            hovered_instruction_path: None,
            dragging_instruction_path: None,
            drag_drop_target: None,
            #[cfg(not(target_os = "linux"))]
            manager,
            #[cfg(not(target_os = "linux"))]
            run_macro_id,
            #[cfg(not(target_os = "linux"))]
            stop_loop_id,
        };

        let config = &app.config;

        if let Err(err) = config::migrate_legacy_macros_to_files(config) {
            warn!("Failed to migrate legacy macros: {}", err);
        }

        let macros = config::get_macros_from_config(config);
        if macros.is_empty() {
            add_default_config(config);
        }

        app.update_macros();

        // Restore the loop mode setting from config
        if let Ok(loop_mode) = app.config.get::<bool>("loop_mode_enabled") {
            app.loop_mode_enabled = loop_mode;
        }

        let command = app.update_title();

        #[cfg(target_os = "linux")]
        {
            if let Ok(shortcuts) = block_on(GlobalShortcuts::new()) {
                if let Ok(session) = block_on(shortcuts.create_session()) {
                    // --- bind shortcuts ---
                    let run_macro_sc = NewShortcut::new("run_macro", "Run Current Macro")
                        .preferred_trigger(Some("<Ctrl><Alt>M"));
                    let stop_loop_sc = NewShortcut::new("stop_loop", "Stop Macro Loop")
                        .preferred_trigger(Some("<Ctrl><Alt>S"));

                    if block_on(shortcuts.bind_shortcuts(&session, &[run_macro_sc, stop_loop_sc], None)).is_ok() {
                        if let Ok(mut activations) = block_on(shortcuts.receive_activated()) {
                            let enigo_clone = Arc::clone(&app.enigo);
                            let config_clone = app.config.clone();
                            let is_looping_clone = Arc::clone(&app.is_looping);

                            tokio::spawn(async move {
                                while let Some(evt) = activations.next().await {
                                    match evt.shortcut_id() {
                                        "run_macro" => {
                                            spawn_global_shortcut_action(
                                                GlobalShortcutAction::RunMacro,
                                                config_clone.clone(),
                                                Arc::clone(&enigo_clone),
                                                Arc::clone(&is_looping_clone),
                                            );
                                        }
                                        "stop_loop" => {
                                            spawn_global_shortcut_action(
                                                GlobalShortcutAction::StopLoop,
                                                config_clone.clone(),
                                                Arc::clone(&enigo_clone),
                                                Arc::clone(&is_looping_clone),
                                            );
                                        }
                                        _ => {}
                                    }
                                }
                            });
                        } else {
                            warn!("Global shortcuts unavailable: failed to receive activations stream");
                        }
                    } else {
                        warn!("Global shortcuts unavailable: failed to bind shortcuts");
                    }
                } else {
                    warn!("Global shortcuts unavailable: failed to create session");
                }
            } else {
                warn!("Global shortcuts unavailable: failed to initialize shortcuts");
            }
        }

        (app, command)
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let key_capture_subscription = if self.key_capture_path.is_some() {
            keyboard::listen().map(Message::KeyCaptureEvent)
        } else {
            Subscription::none()
        };

        #[cfg(not(target_os = "linux"))]
        {
            Subscription::batch(vec![
                Subscription::run(hotkey_sub),
                key_capture_subscription,
            ])
        }

        #[cfg(target_os = "linux")]
        {
            key_capture_subscription
        }
    }

    /// Handle application events here.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        if !matches!(message, RemoveMacro) {
            self.confirm_remove_macro = false;
        }
        if !matches!(message, ClearInstructions | ClearInstructionsTimeout(_)) {
            self.confirm_clear_instructions = false;
        }

        match message {
            SetTitle(title) => {
                if let Some(mac) = &mut self.current_macro {
                    mac.name = title;
                    self.auto_save_current_macro();
                }
            }
            SelectMacro(selected) => {
                self.update_macro(Some(selected));
            }
            RunMacro => {
                if let Some(mac) = self.current_macro.clone() {
                    if self.loop_mode_enabled {
                        // Start loop mode execution
                        if let Err(err) = loop_control::start_loop(&self.is_looping) {
                            warn!("Failed to start loop: {}", err);
                            return Task::none();
                        }

                        let loop_task = thread::create_loop_task(
                            mac.clone(),
                            Arc::clone(&self.enigo),
                            Arc::clone(&self.is_looping)
                        );

                        if let Err(err) = thread::spawn_macro_thread(
                            &mut self.thread_pool,
                            format!("loop_{}", mac.name),
                            loop_task
                        ) {
                            warn!("Failed to spawn loop thread: {}", err);
                            // Reset loop state if thread spawn fails
                            let _ = loop_control::stop_loop(&self.is_looping);
                        }
                    } else {
                        // Single execution mode
                        let single_task = thread::create_single_run_task(
                            mac.clone(),
                            Arc::clone(&self.enigo)
                        );

                        if let Err(err) = thread::spawn_macro_thread(
                            &mut self.thread_pool,
                            format!("single_{}", mac.name),
                            single_task
                        ) {
                            warn!("Failed to spawn single run thread: {}", err);
                        }
                    }
                }
            }
            AddInstruction(body_path, index, instruction) => {
                if let Some(mac) = &mut self.current_macro {
                    self.key_capture_path = None;
                    self.dragging_instruction_path = None;
                    self.drag_drop_target = None;
                    let _ = insert_instruction_at_path(&mut mac.code, &body_path, index, instruction);
                    self.auto_save_current_macro();
                }
            }
            EditInstruction(path, instruction) => {
                if let Some(mac) = &mut self.current_macro {
                    if replace_instruction_at_path(&mut mac.code, &path, instruction) {
                        self.auto_save_current_macro();
                    }
                }
            }
            StartKeyCapture(path) => {
                self.key_capture_path = Some(path);
            }
            KeyCaptureEvent(event) => {
                if let keyboard::Event::KeyPressed { key, .. } = event {
                    let Some(path) = self.key_capture_path.clone() else {
                        return Task::none();
                    };

                    if let Some(mac) = &mut self.current_macro {
                        if let Some(Instruction::Token(Token::Key(_, direction))) = get_instruction(&mac.code, &path).cloned() {
                            if let Some(captured_key) = map_iced_key_to_enigo_key(key.as_ref()) {
                                let _ = replace_instruction_at_path(
                                    &mut mac.code,
                                    &path,
                                    Instruction::Token(Token::Key(captured_key, direction)),
                                );
                                self.auto_save_current_macro();
                            }
                        }
                    }

                    // Stop listening after the first observed key press.
                    self.key_capture_path = None;
                }
            }
            RemoveInstruction(path) => {
                if let Some(mac) = &mut self.current_macro {
                    self.key_capture_path = None;
                    self.dragging_instruction_path = None;
                    self.drag_drop_target = None;
                    if remove_instruction_at_path(&mut mac.code, &path) {
                        self.auto_save_current_macro();
                    }
                }
            }
            ReorderInstruction(path, direction) => {
                if let Some(mac) = &mut self.current_macro {
                    self.key_capture_path = None;
                    self.dragging_instruction_path = None;
                    self.drag_drop_target = None;
                    if reorder_instruction_at_path(&mut mac.code, &path, direction) {
                        self.auto_save_current_macro();
                    }
                }
            }
            MoveInstructionOut(path) => {
                if let Some(mac) = &mut self.current_macro {
                    self.key_capture_path = None;
                    self.dragging_instruction_path = None;
                    self.drag_drop_target = None;
                    if move_instruction_out_of_loop(&mut mac.code, &path) {
                        self.auto_save_current_macro();
                    }
                }
            }
            HoverInstruction(path) => {
                self.hovered_instruction_path = Some(path.clone());
                if let (Some(source), Some(mac)) = (self.dragging_instruction_path.as_ref(), self.current_macro.as_ref()) {
                    if can_drop_instruction_on_target(&mac.code, source, &path) {
                        self.drag_drop_target = Some(DragDropTarget::Instruction(path));
                    } else if matches!(self.drag_drop_target, Some(DragDropTarget::Instruction(_))) {
                        self.drag_drop_target = None;
                    }
                }
            }
            UnhoverInstruction(path) => {
                if self.hovered_instruction_path.as_ref() == Some(&path) {
                    self.hovered_instruction_path = None;
                }
                if self.drag_drop_target == Some(DragDropTarget::Instruction(path)) {
                    self.drag_drop_target = None;
                }
            }
            StartInstructionDrag(path) => {
                self.key_capture_path = None;
                if let Some(mac) = &self.current_macro
                    && get_instruction(&mac.code, &path).is_some()
                {
                    self.dragging_instruction_path = Some(path);
                    self.drag_drop_target = None;
                }
            }
            HoverBodyDropEnd(body_path) => {
                if let (Some(source), Some(mac)) = (self.dragging_instruction_path.as_ref(), self.current_macro.as_ref()) {
                    if can_drop_instruction_at_end(&mac.code, source, &body_path) {
                        self.drag_drop_target = Some(DragDropTarget::EndOfBody(body_path));
                    }
                }
            }
            UnhoverBodyDropEnd(body_path) => {
                if self.drag_drop_target == Some(DragDropTarget::EndOfBody(body_path)) {
                    self.drag_drop_target = None;
                }
            }
            DropDraggedInstructionOn(target_path) => {
                let source = self.dragging_instruction_path.clone();
                self.dragging_instruction_path = None;
                self.drag_drop_target = None;

                if let (Some(source), Some(mac)) = (source, &mut self.current_macro)
                    && drop_instruction_on_target(&mut mac.code, &source, &target_path)
                {
                    self.auto_save_current_macro();
                }
            }
            DropDraggedInstructionAtEnd(body_path) => {
                let source = self.dragging_instruction_path.clone();
                self.dragging_instruction_path = None;
                self.drag_drop_target = None;

                if let (Some(source), Some(mac)) = (source, &mut self.current_macro)
                    && drop_instruction_at_end(&mut mac.code, &source, &body_path)
                {
                    self.auto_save_current_macro();
                }
            }
            CancelInstructionDrag => {
                self.dragging_instruction_path = None;
                self.drag_drop_target = None;
            }
            ClearInstructions => {
                if !self.confirm_clear_instructions {
                    self.confirm_clear_instructions = true;
                    self.clear_confirm_generation = self.clear_confirm_generation.wrapping_add(1);
                    let generation = self.clear_confirm_generation;
                    return Task::perform(
                        async move {
                            tokio::time::sleep(std::time::Duration::from_secs(CLEAR_CONFIRM_TIMEOUT_SECS)).await;
                            generation
                        },
                        |generation| ClearInstructionsTimeout(generation).into(),
                    );
                } else if let Some(mac) = &mut self.current_macro {
                    mac.code.clear();
                    self.key_capture_path = None;
                    self.dragging_instruction_path = None;
                    self.drag_drop_target = None;
                    self.auto_save_current_macro();
                    self.confirm_clear_instructions = false;
                }
            }
            ClearInstructionsTimeout(generation) => {
                if generation == self.clear_confirm_generation {
                    self.confirm_clear_instructions = false;
                }
            }
            SaveMacro => {
                if let Some(mac) = &self.current_macro {
                    if let Err(err) = config::save_macro(&self.config, mac) {
                        warn!("Failed to save macro: {}", err);
                    } else {
                        self.update_macros();
                    }
                }
            }
            NewMacro => {
                let new_macro = Macro::new("New Macro".into(), "New Macro".into(), vec![]);
                let new_id = new_macro.id.clone();
                if let Err(err) = add_macro(&self.config, new_macro) {
                    warn!("Failed to create macro: {}", err);
                }
                self.key_capture_path = None;
                self.dragging_instruction_path = None;
                self.drag_drop_target = None;
                self.update_macros();
                if let Some((index, _)) = config::get_macros_from_config(&self.config)
                    .iter()
                    .enumerate()
                    .find(|(_, mac)| mac.id == new_id)
                {
                    self.update_macro(Some(index));
                }
            }
            RemoveMacro => {
                if !self.confirm_remove_macro {
                    self.confirm_remove_macro = true;
                } else {
                    if let Some(mac) = self.current_macro.clone() {
                        if let Err(err) = config::remove_macro_by_id(&self.config, &mac.id) {
                            warn!("Failed to remove macro: {}", err);
                        } else {
                            self.key_capture_path = None;
                            self.dragging_instruction_path = None;
                            self.drag_drop_target = None;
                            self.update_macros();
                            self.update_macro(None);
                        }
                    }
                    self.confirm_remove_macro = false;
                }
            }
            ToggleLoopMode(enabled) => {
                self.loop_mode_enabled = enabled;
                // Store loop mode setting in config
                if let Err(err) = config::save_config_value(&self.config, "loop_mode_enabled", enabled) {
                    warn!("Failed to save loop mode setting: {}", err);
                }
            }
            #[cfg(not(target_os = "linux"))]
            GlobalHotkeyEvent(event) => {
                // Guard against duplicate release events.
                if event.state != HotKeyState::Pressed {
                    return Task::none();
                }

                println!("{:?}", event);
                let action = if event.id == self.run_macro_id {
                    Some(GlobalShortcutAction::RunMacro)
                } else if event.id == self.stop_loop_id {
                    Some(GlobalShortcutAction::StopLoop)
                } else {
                    None
                };

                if let Some(action) = action {
                    spawn_global_shortcut_action(
                        action,
                        self.config.clone(),
                        Arc::clone(&self.enigo),
                        Arc::clone(&self.is_looping),
                    );
                }
            }
        }
        Task::none()
    }

    /// Creates a view after each update.
    fn view(&'_ self) -> Element<'_, Self::Message> {
        let spacing = cosmic::theme::active().cosmic().spacing;
        let has_selected_macro = self.current_macro.is_some();

        let pill_button = |label: &'static str| {
            button(cosmic::widget::text(label)).padding([10, 18])
        };

        let symbol_label_button = |symbol: &'static str, label: &'static str| {
            button(
                row![
                    cosmic::widget::text(symbol),
                    cosmic::widget::text(label),
                ]
                .spacing(spacing.space_xs)
                .align_y(Alignment::Center),
            )
            .padding([10, 14])
        };

        let compact_icon_button = |icon_path: &'static str| {
            widget::button::icon(widget::icon::from_path(PathBuf::from(icon_path))).padding(8)
        };

        let run_macro_label = if self.loop_mode_enabled {
            "⟲ Start loop"
        } else {
            "▶ Run macro"
        };

        let run_macro_button = if has_selected_macro {
            pill_button(run_macro_label).on_press(RunMacro)
        } else {
            pill_button(run_macro_label)
        };

        let new_macro_button = symbol_label_button("＋", "New macro").on_press(NewMacro);

        let remove_macro_button = if has_selected_macro {
            compact_icon_button(ICON_REMOVE).on_press(RemoveMacro)
        } else {
            compact_icon_button(ICON_REMOVE)
        };

        let mut content = column![];

        // Top control section: Left (Select macro) and Right (New/Delete)
        content = content.push(
            row![
                // Left third: Select macro controls
                container(
                    column![
                        cosmic::widget::text("Select macro"),
                        cosmic::widget::dropdown(&self.macro_strs, self.macro_selected, |x: usize| SelectMacro(x))
                    ]
                    .spacing(spacing.space_xxs)
                    .align_x(Alignment::Center)
                )
                .width(Length::Fill)
                .align_x(Alignment::Center),
                // Spacer
                container(cosmic::widget::text("")).width(Length::Fill),
                // Right third: New/Delete controls
                container(
                    column![
                        tooltip(
                            new_macro_button,
                            container("Add a new macro"),
                            tooltip::Position::Left
                        ),
                        tooltip(
                            remove_macro_button,
                            container(if self.confirm_remove_macro {
                                "Click again to permanently delete the selected macro"
                            } else {
                                "Arms deletion for the selected macro"
                            }),
                            tooltip::Position::Left
                        ),
                    ]
                    .spacing(12)
                    .align_x(Alignment::Center)
                )
                .width(Length::Fill)
                .align_x(Alignment::Center),
            ]
            .spacing(spacing.space_s)
            .width(Length::Fill)
        );

        #[cfg(target_os = "linux")]
        let loop_mode = cosmic::widget::checkbox(self.loop_mode_enabled)
            .name("Loop mode")
            .on_toggle(ToggleLoopMode);
        #[cfg(not(target_os = "linux"))]
        let loop_mode = cosmic::widget::checkbox(self.loop_mode_enabled)
            .on_toggle(ToggleLoopMode);

        // Bottom control section: Left (Run macro + Loop mode) and Right (placeholder)
        content = content.push(
            row![
                // Left third: Run macro and loop mode
                container(
                    row![
                        tooltip(
                            run_macro_button,
                            container("Runs the current macro once or starts looping if enabled"),
                            tooltip::Position::Top
                        ),
                        tooltip(
                            container(
                                row![
                                    cosmic::widget::text("Loop mode"),
                                    loop_mode,
                                ]
                                .spacing(8)
                                .align_y(Alignment::Center)
                            )
                            .padding([8, 12]),
                            container("Enable to loop the macro continuously"),
                            tooltip::Position::Top
                        )
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center)
                )
                .width(Length::Fill)
                .align_x(Alignment::Center),
                // Spacer
                container(cosmic::widget::text("")).width(Length::Fill),
                // Right third: placeholder
                container(cosmic::widget::text("")).width(Length::Fill).align_x(Alignment::Center),
            ]
            .spacing(spacing.space_s)
            .width(Length::Fill)
        );

        if let Some(mac) = &self.current_macro {
            let clear_instructions_label = if self.confirm_clear_instructions {
                "⚠ Confirm clear (5s)"
            } else {
                "⚠ Clear instructions"
            };

            let instructions = render_instruction_list(self, &mac.code, vec![]);

            content = content.push(widget::settings::view_column(
                vec![
                    widget::settings::section()
                        .add(
                            widget::column::with_children(vec![
                                widget::text::body("Title").into(),
                                widget::text_input("Macro", &mac.name)
                                    .on_input(SetTitle)
                                    .into(),
                            ])
                            .spacing(spacing.space_xxs)
                            .padding([0, 15, 0, 15]),
                        )
                        .add(
                            widget::column::with_children(vec![
                                widget::text::body("Instructions").into(),
                                instructions,
                                container(
                                    row![
                                        tooltip(
                                            pill_button(clear_instructions_label).on_press(ClearInstructions),
                                            container(if self.confirm_clear_instructions {
                                                "Click again within 5 seconds to remove every instruction in this macro"
                                            } else {
                                                "Arms removal for every instruction in this macro"
                                            }),
                                            tooltip::Position::Top,
                                        ),
                                        tooltip(
                                            pill_button("💾 Save macro").on_press(SaveMacro),
                                            container("Persist the current macro to your config"),
                                            tooltip::Position::Top,
                                        ),
                                    ]
                                    .spacing(12)
                                    .align_y(Alignment::Center),
                                )
                                .width(Length::Fill)
                                .align_x(Alignment::Center)
                                .into(),
                            ])
                            .spacing(spacing.space_xxs)
                            .padding([0, 15, 0, 15])
                            .apply(scrollable),
                        )
                        .into(),
                ],
            )
            .padding(10))
            .width(Length::Fill)
            .height(Length::Shrink)
            .align_x(Alignment::Center);
        }

        // Centers all the content and makes it look nice
        let centered = cosmic::widget::container(content)
            .width(Length::Fill)
            .height(Length::Shrink)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center).into();

        centered
    }
}

impl App where Self: cosmic::Application, {
    fn update_title(&mut self) -> Task<Message> {
        let header_title: String = "Macros".to_string();
        let window_title = header_title.clone();
        self.set_header_title(header_title);
        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }
}
