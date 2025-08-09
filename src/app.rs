use crate::app::Message::*;
use crate::macros::{Instruction, Macro};
use crate::util::{add_macro, key_to_string, string_to_key, ThreadPool};
use crate::util::{get_macro, run_macro};
use crate::util::{get_mouse_button_names, mouse_button_to_index, index_to_mouse_button};
use crate::util::{config, thread, loop_control};
use tracing::warn;
use cosmic::app::{Core, Task};
use cosmic::cosmic_config::{Config, ConfigGet, ConfigSet};
use cosmic::iced::{Alignment, Length};
use cosmic::iced_widget::{button, column, row, scrollable, tooltip};
use cosmic::widget::button::text;
use cosmic::widget::{container, nav_bar};
use cosmic::{executor, widget, ApplicationExt, Apply, Element};
use enigo::agent::Token;
use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key};
use slotmap::{DefaultKey, SecondaryMap, SlotMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use cosmic::dialog::ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use cosmic::iced::futures::executor::block_on;
use cosmic::iced::futures::StreamExt;

// Constants for default values
const DEFAULT_WAIT_TIME: u64 = 1000;
const DEFAULT_SCROLL_AMOUNT: i32 = 4;

// Constants for icon paths
const ICON_ADD: &str = "/usr/share/icons/breeze-dark/actions/16/bqm-add.svg";
const ICON_REMOVE: &str = "/usr/share/icons/breeze-dark/actions/16/bqm-remove.svg";
const ICON_UP: &str = "/usr/share/icons/breeze-dark/actions/16/go-up.svg";
const ICON_DOWN: &str = "/usr/share/icons/breeze-dark/actions/16/go-down.svg";

#[derive(Clone, Copy)]
pub(crate) enum Page {
    Page1,
    //Page2,
    //Page3,
    //Page4,
}

impl Page {
    /// Page titles
    const fn as_str(self) -> &'static str {
        match self {
            Page::Page1 => "Macros",
            //Page::Page2 => "Page 2",
            //Page::Page3 => "Page 3",
            //Page::Page4 => "Page 4",
        }
    }
}

/// Messages that are used specifically by our [`App`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Message {
    SelectMacro(usize),
    RunMacro,
    ToggleLoopMode(bool),
    SetTitle(String),
    AddInstruction(usize, Instruction),
    EditInstruction(usize, Instruction),
    RemoveInstruction(isize),
    /// Reorder an instruction by moving it up or down
    /// Parameters: (index, direction) where direction is -1 for up, 1 for down
    ReorderInstruction(usize, isize),
    ClearInstructions,
    SaveMacro,
    NewMacro,
    RemoveMacro,
}

/// The [`App`] stores application-specific state.
pub(crate) struct App {
    /// COSMIC app settings
    core: Core,
    nav_model: nav_bar::Model,
    macro_selected: Option<usize>,
    current_macro: Option<Macro>,
    /// The application config
    pub(crate) config: Config,
    /// Enigo is an API for mouse and keyboard control
    enigo: Arc<Mutex<Enigo>>,
    thread_pool: ThreadPool,
    macros: SlotMap<DefaultKey, Macro>,
    macro_keys: SecondaryMap<DefaultKey, String>,
    macro_strs: Vec<String>,
    /// Flag to control looping execution
    is_looping: Arc<Mutex<bool>>,
    /// UI setting for whether to run in loop mode
    loop_mode_enabled: bool,
}

impl App {
    /// Updates the currently selected macro
    /// 
    /// # Arguments
    /// * `selected` - The index of the selected macro, or None to clear selection
    fn update_macro(&mut self, selected: Option<usize>) {
        self.macro_selected = selected;
        // Store the selected macro index in config so global shortcuts can access it
        if let Err(err) = self.config.set("selected_macro", selected) {
            warn!("Failed to save selected macro index: {}", err);
        }
        if let Some(selected) = selected {
            self.current_macro = Some(get_macro(&self.config, selected));
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
        for mac in macs {
            let key = self.macros.insert(mac);
            let mac = self.macros.get_mut(key).unwrap();
            self.macro_keys.insert(key, mac.name.clone());
            self.macro_strs.push(mac.name.clone());
        }
    }

    /// Automatically saves the current macro to config and updates the app state
    /// This should be called whenever the current macro is modified
    fn auto_save_current_macro(&mut self) {
        if let Some(selected) = self.macro_selected {
            if let Some(mac) = &self.current_macro {
                if let Err(err) = config::update_macro_at_index(&self.config, selected, mac) {
                    warn!("Failed to update macro: {}", err);
                } else {
                    self.update_macros();
                }
            }
        }
    }
}

fn add_default_config(config: &Config) {
    add_macro(config, Macro::new("New Macro".into(), "description".into(), vec![]));
}

/// Implement [`cosmic::Application`] to integrate with COSMIC.
impl cosmic::Application for App {
    /// Default async executor to use with the app.
    type Executor = executor::Default;

    /// Argument received [`cosmic::Application::new`].
    type Flags = Vec<(Page, String)>;

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
    fn init(core: Core, input: Self::Flags) -> (Self, Task<Self::Message>) {
        let mut nav_model = nav_bar::Model::default();

        for (title, content) in input {
            nav_model.insert().text(title.as_str()).data(content);
        }

        nav_model.activate_position(0);

        let mut app = App {
            core,
            nav_model,
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
        };

        let config = &app.config;
        let macros = config.get::<Vec<Macro>>("macros");

        // Add default config. Everything here is temporary until a later state of the project.
        if macros.is_err() {
            add_default_config(config);
        }

        app.update_macros();

        // Restore the loop mode setting from config
        if let Ok(loop_mode) = app.config.get::<bool>("loop_mode_enabled") {
            app.loop_mode_enabled = loop_mode;
        }

        // Restore the previously selected macro from config
        if let Ok(selected_index) = app.config.get::<Option<usize>>("selected_macro") {
            if let Some(index) = selected_index {
                // Validate that the index is still valid after loading macros
                if let Ok(macros) = app.config.get::<Vec<Macro>>("macros") {
                    if index < macros.len() {
                        app.update_macro(Some(index));
                    } else {
                        // Index is out of bounds, clear the selection
                        app.update_macro(None);
                    }
                }
            }
        }

        let command = app.update_title();

        let shortcuts = block_on(GlobalShortcuts::new()).unwrap();
        let session = block_on(shortcuts.create_session()).unwrap();

        // --- bind shortcuts ---
        let run_macro_sc = NewShortcut::new("run_macro", "Run Current Macro")
            .preferred_trigger(Some("<Ctrl><Alt>M"));
        let stop_loop_sc = NewShortcut::new("stop_loop", "Stop Macro Loop")
            .preferred_trigger(Some("<Ctrl><Alt>S"));

        block_on(shortcuts.bind_shortcuts(&session, &[run_macro_sc, stop_loop_sc], None)).unwrap();

        let mut activations = block_on(shortcuts.receive_activated()).unwrap();
        let enigo_clone = Arc::clone(&app.enigo);
        let config_clone = app.config.clone();
        let is_looping_clone = Arc::clone(&app.is_looping);

        tokio::spawn(async move {
            while let Some(evt) = activations.next().await {
                match evt.shortcut_id() {
                    "run_macro" => {
                        println!("Global shortcut activated: run_macro");

                        // Check if loop mode is enabled
                        let loop_mode_enabled = config_clone.get::<bool>("loop_mode_enabled").unwrap_or(false);

                        // Check if already looping
                        let currently_looping = if let Ok(is_looping) = is_looping_clone.lock() {
                            *is_looping
                        } else {
                            false
                        };

                        if loop_mode_enabled && currently_looping {
                            // Already looping, ignore the request
                            println!("Macro is already looping, ignoring run request");
                            continue;
                        }

                        // Get the selected macro index from config
                        if let Ok(selected_index) = config_clone.get::<Option<usize>>("selected_macro") {
                            if let Some(index) = selected_index {
                                if let Ok(macros) = config_clone.get::<Vec<Macro>>("macros") {
                                    if let Some(mac) = macros.get(index) {
                                        let enigo = Arc::clone(&enigo_clone);
                                        let mac = mac.clone();

                                        if loop_mode_enabled {
                                            // Start looping
                                            if let Ok(mut is_looping) = is_looping_clone.lock() {
                                                *is_looping = true;
                                            }

                                            let loop_flag = Arc::clone(&is_looping_clone);

                                            tokio::task::spawn_blocking(move || {
                                                println!("Starting macro loop via global shortcut: {}", mac.name);
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
                                                println!("Macro loop stopped via global shortcut.");
                                            });
                                        } else {
                                            // Run the macro once in a separate thread
                                            tokio::task::spawn_blocking(move || {
                                                println!("Running macro via global shortcut: {}", mac.name);
                                                run_macro(mac, enigo);
                                                println!("Macro complete.");
                                            });
                                        }
                                    } else {
                                        println!("No macro found at index {}", index);
                                    }
                                }
                            } else {
                                println!("No macro currently selected for global shortcut");
                            }
                        }
                    }
                    "stop_loop" => {
                        println!("Global shortcut activated: stop_loop");
                        // Set the looping flag to false to stop any running loops
                        if let Ok(mut is_looping) = is_looping_clone.lock() {
                            *is_looping = false;
                            println!("Loop stop requested via global shortcut.");
                        } else {
                            println!("Failed to access loop flag.");
                        }
                    }
                    _ => {}
                }
            }
        });

        (app, command)
    }

    /// Allows COSMIC to integrate with your application's [`nav_bar::Model`].
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav_model)
    }

    /// Called when a navigation item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<Self::Message> {
        self.nav_model.activate(id);
        self.update_title()
    }

    /// Handle application events here.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
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
            AddInstruction(index, instruction) => {
                if let Some(mac) = &mut self.current_macro {
                    mac.code.insert(index, instruction);
                    self.auto_save_current_macro();
                }
            }
            EditInstruction(index, instruction) => {
                if let Some(mac) = &mut self.current_macro {
                    if mac.code.len() > 0 {
                        mac.code[index] = instruction;
                        self.auto_save_current_macro();
                    }
                }
            }
            RemoveInstruction(index) => {
                if let Some(mac) = &mut self.current_macro {
                    if mac.code.len() > 0 && index >= 0 {
                        mac.code.remove(index as usize);
                        self.auto_save_current_macro();
                    }
                }
            }
            ReorderInstruction(index, direction) => {
                if let Some(mac) = &mut self.current_macro {
                    let len = mac.code.len();
                    if len > 1 && index < len {
                        let new_index = if direction < 0 {
                            // Move up (swap with previous)
                            if index > 0 { index - 1 } else { index }
                        } else {
                            // Move down (swap with next)
                            if index < len - 1 { index + 1 } else { index }
                        };

                        if new_index != index {
                            // Swap the instructions
                            mac.code.swap(index, new_index);
                            self.auto_save_current_macro();
                        }
                    }
                }
            }
            ClearInstructions => {
                if let Some(mac) = &mut self.current_macro {
                    mac.code.clear();
                    self.auto_save_current_macro();
                }
            }
            SaveMacro => {
                if let Some(selected) = self.macro_selected {
                    if let Some(mac) = &self.current_macro {
                        if let Err(err) = config::update_macro_at_index(&self.config, selected, mac) {
                            warn!("Failed to save macro: {}", err);
                        } else {
                            self.update_macros();
                        }
                    }
                }
            }
            NewMacro => {
                add_macro(&self.config, Macro::new("New Macro".into(), "New Macro".into(), vec![]));
                self.update_macros();
                self.update_macro(Some(self.macros.len() - 1));
            }
            RemoveMacro => {
                if let Some(selected) = self.macro_selected {
                    if let Err(err) = config::remove_macro_at_index(&self.config, selected) {
                        warn!("Failed to remove macro: {}", err);
                    } else {
                        self.update_macros();
                        self.current_macro = None;
                        self.macro_selected = None;
                    }
                }
            }
            ToggleLoopMode(enabled) => {
                self.loop_mode_enabled = enabled;
                // Store loop mode setting in config
                if let Err(err) = config::save_config_value(&self.config, "loop_mode_enabled", enabled) {
                    warn!("Failed to save loop mode setting: {}", err);
                }
            }
        }
        Task::none()
    }

    /// Creates a view after each update.
    fn view(&'_ self) -> Element<'_, Self::Message> {
        // The string associated with the page. Ex: "Manage macro"
        let page_content = self
            .nav_model
            .active_data::<String>()
            .map_or("No page selected", String::as_str);

        let page_text = cosmic::widget::text(page_content);

        let spacing = cosmic::theme::active().cosmic().spacing;

        let mut content = column![
            page_text
        ];

        content = content.push(row![
            column![
                cosmic::widget::text("Select macro"),
                cosmic::widget::dropdown(&self.macro_strs, self.macro_selected, |x: usize| SelectMacro(x))
            ],
            column![
                tooltip(
                    button("Run macro")
                        .on_press(RunMacro),
                    container("Runs the current macro once or starts looping if enabled"),
                    tooltip::Position::Right
                ),
                tooltip(
                    cosmic::widget::checkbox("Loop mode", self.loop_mode_enabled)
                        .on_toggle(ToggleLoopMode),
                    container("Enable to loop the macro continuously"),
                    tooltip::Position::Right
                )
            ],
            tooltip(
                widget::button::icon(widget::icon::from_path(PathBuf::from(ICON_ADD)))
                    .on_press(NewMacro),
                container("Add a new macro"),
                tooltip::Position::Right
            ),
            tooltip(
                widget::button::icon(widget::icon::from_path(PathBuf::from(ICON_REMOVE)))
                    .on_press(RemoveMacro),
                container("Remove the selected macro"),
                tooltip::Position::Right
            )
        ].spacing(50).padding([0, 0, 0, 0]));

        if let Some(mac) = &self.current_macro {
            content = content.push(column![
                text("Clear instructions")
                    .on_press(ClearInstructions),
                text("Save macro")
                    .on_press(SaveMacro),
            ]);

            let mut instructions: Vec<Element<Message>> = vec![];

            for (index, ins) in mac.code.iter().cloned().enumerate() {
                let instruction: Element<Message> = match ins {
                    Instruction::Token(token) => {
                        match token {
                            Token::Text(text) => {
                                row![
                                    widget::text::body("Text:".to_string()).align_y(Alignment::Center),
                                    widget::text_input("", text)
                                        .on_input(move |x| EditInstruction(index, Instruction::Token(Token::Text(x)))),
                                ].spacing(10).into()
                            }
                            Token::Key(key, direction) => {
                                row![
                                    widget::text::body("Key:".to_string()).align_y(Alignment::Center),
                                    widget::text_input("", key_to_string(&key).unwrap_or_default())
                                        .on_input(move |x| EditInstruction(index, Instruction::Token(Token::Key(string_to_key(x.as_str()).unwrap_or(key), direction.clone())))),
                                    widget::dropdown(&["Click", "Press", "Release"], Some(if direction == Direction::Click { 0usize } else if direction == Direction::Press { 1usize } else { 2usize }), move |x: usize| EditInstruction(index, Instruction::Token(Token::Key(key, if x == 0usize { Direction::Click } else if x == 1usize { Direction::Press } else { Direction::Release })))),
                                ].spacing(10).into()
                            }
                            Token::Raw(keycode, _) => {
                                widget::text::body(format!("Raw: {:?}", keycode)).into()
                            }
                            Token::Button(button, direction) => {
                                row![
                                    widget::text::body("Button:".to_string()).align_y(Alignment::Center),
                                    widget::dropdown(get_mouse_button_names(), Some(mouse_button_to_index(&button)), move |x: usize| EditInstruction(index, Instruction::Token(Token::Button(index_to_mouse_button(x), direction.clone())))),
                                    widget::dropdown(&["Click", "Press", "Release"], Some(if direction == Direction::Click { 0usize } else if direction == Direction::Press { 1usize } else { 2usize }), move |x: usize| EditInstruction(index, Instruction::Token(Token::Button(button, if x == 0 { Direction::Click } else if x == 1 { Direction::Press } else { Direction::Release })))),
                                ].spacing(10).width(Length::Fill).into()
                                //widget::text::body(format!("Button: {:?}", button)).into()
                            }
                            Token::MoveMouse(x, y, coordinate) => {
                                row![
                                    widget::text::body("Move mouse:".to_string()).align_y(Alignment::Center),
                                    widget::text_input("X", format!("{}", x))
                                        .on_input(move |new_x| EditInstruction(index, Instruction::Token(Token::MoveMouse(new_x.parse().unwrap_or(x), y, coordinate.clone())))),
                                    widget::text_input("Y", format!("{}", y))
                                        .on_input(move |new_y| EditInstruction(index, Instruction::Token(Token::MoveMouse(x, new_y.parse().unwrap_or(y), coordinate.clone())))),
                                    widget::dropdown(&["Absolute", "Relative"], Some(if coordinate == Coordinate::Abs { 0usize } else { 1usize }), move |coord: usize| EditInstruction(index, Instruction::Token(Token::MoveMouse(x, y, if coord == 0 { Coordinate::Abs } else { Coordinate::Rel })))),
                                ].spacing(10).into()
                            }
                            Token::Scroll(amount, axis) => {
                                row![
                                    widget::text::body("Scroll:".to_string()).align_y(Alignment::Center),
                                    widget::text_input("Amount", format!("{}", amount))
                                        .on_input(move |new_amount| EditInstruction(index, Instruction::Token(Token::Scroll(new_amount.parse().unwrap_or(amount), axis.clone())))),
                                    widget::dropdown(&["Vertical", "Horizontal"], Some(if axis == Axis::Vertical { 0 } else { 1 }), move |new_axis: usize| EditInstruction(index, Instruction::Token(Token::Scroll(amount, if new_axis == 0 { Axis::Vertical } else { Axis::Horizontal })))),
                                ].spacing(10).into()
                            }
                            _ => {
                                widget::text::body("Token not implemented").into()
                            }
                        }
                    }
                    Instruction::Wait(duration) => {
                        row![
                            widget::text::body("Wait:".to_string()).align_y(Alignment::Center),
                            widget::text_input("", duration.to_string())
                                .on_input(move |x| EditInstruction(index, Instruction::Wait(x.parse().unwrap_or(duration)))),
                        ].spacing(10).into()
                        //widget::text::body(format!("Wait: {}ms", duration)).into()
                    }
                    Instruction::Script(script) => {
                        row![
                            widget::text::body("Script:".to_string()).align_y(Alignment::Center),
                            widget::text_input("", script)
                                .on_input(move |x| EditInstruction(index, Instruction::Script(x))),
                        ].spacing(10).into()
                        //widget::text::body(format!("Script: {}", script)).into()
                    }
                };
                let instruction = row![
                    instruction,
                    // Up button
                    tooltip(
                        widget::button::icon(widget::icon::from_path(PathBuf::from(ICON_UP)))
                            .on_press(ReorderInstruction(index, -1)),
                        container("Move up"),
                        tooltip::Position::Top
                    ),
                    // Down button
                    tooltip(
                        widget::button::icon(widget::icon::from_path(PathBuf::from(ICON_DOWN)))
                            .on_press(ReorderInstruction(index, 1)),
                        container("Move down"),
                        tooltip::Position::Bottom
                    ),
                    widget::button::icon(widget::icon::from_path(PathBuf::from(ICON_REMOVE)))
                        .on_press(RemoveInstruction(index as isize)),
                    cosmic::widget::dropdown(
                        &[
                            "Wait",
                            "Text",
                            "Key",
                            "Mouse Button",
                            "Move Mouse",
                            "Scroll",
                            "Run Script",
                        ],
                        None,
                        move |selected| match selected {
                            0 => AddInstruction(index, Instruction::Wait(DEFAULT_WAIT_TIME)),
                            1 => AddInstruction(index, Instruction::Token(Token::Text("text".into()))),
                            2 => AddInstruction(index, Instruction::Token(Token::Key(Key::Unicode('a'.into()), Direction::Click))),
                            3 => AddInstruction(index, Instruction::Token(Token::Button(Button::Left, Direction::Click))),
                            4 => AddInstruction(index, Instruction::Token(Token::MoveMouse(0, 0, Coordinate::Rel))),
                            5 => AddInstruction(index, Instruction::Token(Token::Scroll(DEFAULT_SCROLL_AMOUNT, Axis::Vertical))),
                            6 => AddInstruction(index, Instruction::Script("script".into())),
                            _ => unreachable!(),
                        },
                    )
                ].into();

                instructions.push(instruction);
            }

            let len = mac.code.len();
            instructions.push(
                cosmic::widget::dropdown(
                    &[
                        "Wait",
                        "Text",
                        "Key",
                        "Mouse Button",
                        "Move Mouse",
                        "Scroll",
                        "Run Script",
                    ],
                    None,
                    move |selected| match selected {
                        0 => AddInstruction(len, Instruction::Wait(DEFAULT_WAIT_TIME)),
                        1 => AddInstruction(len, Instruction::Token(Token::Text("text".into()))),
                        2 => AddInstruction(len, Instruction::Token(Token::Key(Key::Unicode('a'.into()), Direction::Click))),
                        3 => AddInstruction(len, Instruction::Token(Token::Button(Button::Left, Direction::Click))),
                        4 => AddInstruction(len, Instruction::Token(Token::MoveMouse(0, 0, Coordinate::Rel))),
                        5 => AddInstruction(len, Instruction::Token(Token::Scroll(DEFAULT_SCROLL_AMOUNT, Axis::Vertical))),
                        6 => AddInstruction(len, Instruction::Script("script".into())),
                        _ => unreachable!(),
                    },
                ).into()
            );

            content = content.push(widget::settings::view_column(
                vec![
                widget::settings::section()
                    .add(
                        widget::column::with_children(vec![
                            widget::text::body("Title").into(),
                            widget::text_input("Macro", &mac.name)
                                .on_input(SetTitle)
                                .into()
                        ])
                            .spacing(spacing.space_xxs)
                            .padding([0, 15, 0, 15]),
                    )
                    .add(
                        widget::column::with_children(vec![
                            widget::text::body("Instructions").into(),
                            //widget::text_input("Description", &mac.description).into(),
                            widget::column::with_children(instructions).apply(scrollable).spacing(spacing.space_xs).into()
                        ])
                            .spacing(spacing.space_xxs)
                            .padding([0, 15, 0, 15]),
                    )
                    .into(),
            ])
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
    fn active_page_title(&mut self) -> &str {
        self.nav_model
            .text(self.nav_model.active())
            .unwrap_or("Unknown Page")
    }

    fn update_title(&mut self) -> Task<Message> {
        let header_title: String = format!("{} — Macros", self.active_page_title().to_owned());
        let window_title = header_title.clone();
        self.set_header_title(header_title);
        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }
}
