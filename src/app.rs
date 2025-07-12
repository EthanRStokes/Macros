use crate::app::Message::*;
use crate::macros::{Instruction, Macro};
use crate::util::{add_macro, button_to_string, key_to_string, string_to_button, string_to_key, ThreadPool};
use crate::util::{get_macro, run_macro};
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
use std::ops::DerefMut;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

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
    SetTitle(String),
    AddInstruction(usize, Instruction),
    EditInstruction(usize, Instruction),
    RemoveInstruction(isize),
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
}

impl App {
    fn update_macro(&mut self, selected: Option<usize>) {
        self.macro_selected = selected;
        if let Some(selected) = selected {
            self.current_macro = Some(get_macro(&self.config, selected));
        }
    }

    fn update_macros(&mut self) {
        let macs = self.config.get::<Vec<Macro>>("macros").expect("Macros file not found");
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
}

fn add_default_config(config: &Config) {
    add_macro(config, Macro::new("macro".into(), "description".into(), vec![]));
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
        };

        let config = &app.config;
        let macros = config.get::<Vec<Macro>>("macros");

        // Add default config. Everything here is temporary until a later state of the project.
        if macros.is_err() {
            add_default_config(config);
        }

        app.update_macros();
        //app.macros = Some(macros.iter().map(|x| x.name.clone()).collect::<Vec<String>>());

        let command = app.update_title();

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
                }
            }
            SelectMacro(selected) => {
                self.update_macro(Some(selected));
            }
            RunMacro => {
                if let Some(mac) = self.current_macro.clone() {
                    let pool = &mut self.thread_pool;
                    let thread_num = pool.workers.len();
                    let enigo = (&self.enigo).clone();

                    let thread = thread::Builder::new().name(format!("macro_thread: {thread_num}")).spawn(move || {
                        println!("Running macro...");
                        let mac = mac;
                        let mut enigo = enigo.lock().unwrap();
                        run_macro(mac, enigo.deref_mut());
                        println!("Macro complete.");
                    }).expect("Macro thread failed to spawn");

                    // TODO: remove from the pool once the thread has completed.
                    pool.add_worker(thread);
                }
            }
            AddInstruction(index, instruction) => {
                if let Some(mac) = &mut self.current_macro {
                    mac.code.insert(index, instruction);
                }
            }
            EditInstruction(index, instruction) => {
                if let Some(mac) = &mut self.current_macro {
                    if mac.code.len() > 0 {
                        mac.code[index] = instruction;
                    }
                }
            }
            RemoveInstruction(index) => {
                if let Some(mac) = &mut self.current_macro {
                    if mac.code.len() > 0 && index >= 0 {
                        mac.code.remove(index as usize);
                    }
                }
            }
            ClearInstructions => {
                if let Some(mac) = &mut self.current_macro {
                    mac.code.clear();
                }
            }
            SaveMacro => {
                if let Some(selected) = self.macro_selected {
                    if let Some(mac) = &self.current_macro {
                        match self.config.get::<Vec<Macro>>("macros") {
                            Ok(mut macros) => {
                                if selected < macros.len() {
                                    macros[selected] = mac.clone(); // Clone only when needed for saving
                                    match self.config.set("macros", macros) {
                                        Ok(_) => self.update_macros(),
                                        Err(err) => warn!("Couldn't set macros config: {}", err)
                                    }
                                } else {
                                    warn!("Selected macro index out of bounds: {}", selected);
                                }
                            },
                            Err(err) => warn!("Failed to get macros config: {}", err)
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
                    match self.config.get::<Vec<Macro>>("macros") {
                        Ok(mut macros) => {
                            if selected < macros.len() {
                                macros.remove(selected);
                                match self.config.set("macros", macros) {
                                    Ok(_) => {},
                                    Err(err) => warn!("Couldn't set macros config: {}", err)
                                }
                            } else {
                                warn!("Selected macro index out of bounds: {}", selected);
                            }
                        },
                        Err(err) => warn!("Failed to get macros config: {}", err)
                    }
                    self.update_macros();
                    self.current_macro = None;
                    self.macro_selected = None;
                }
            }
        }
        Task::none()
    }

    /// Creates a view after each update.
    fn view(&self) -> Element<Self::Message> {
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
            tooltip(
                button("Run macro")
                    .on_press(RunMacro),
                container("Runs the current macro"),
                tooltip::Position::Right
            ),
            tooltip(
                widget::button::icon(widget::icon::from_path(PathBuf::from("/usr/share/icons/breeze-dark/actions/16/bqm-add.svg")))
                    .on_press(NewMacro),
                container("Add a new macro"),
                tooltip::Position::Right
            ),
            tooltip(
                widget::button::icon(widget::icon::from_path(PathBuf::from("/usr/share/icons/breeze-dark/actions/16/bqm-remove.svg")))
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
                                    widget::button::icon(widget::icon::from_path(PathBuf::from("/usr/share/icons/breeze-dark/actions/16/albumfolder-user-trash.svg")))
                                        .on_press(RemoveInstruction(index as isize))
                                ].spacing(10).into()
                            }
                            Token::Key(key, direction) => {
                                row![
                                    widget::text::body("Key:".to_string()).align_y(Alignment::Center),
                                    widget::text_input("", key_to_string(&key).unwrap_or_default())
                                        .on_input(move |x| EditInstruction(index, Instruction::Token(Token::Key(string_to_key(x.as_str()).unwrap_or(key), direction.clone())))),
                                    widget::dropdown(&["Click", "Press", "Release"], Some(if direction == Direction::Click { 0usize } else if direction == Direction::Press { 1usize } else { 2usize }), move |x: usize| EditInstruction(index, Instruction::Token(Token::Key(key, if x == 0usize { Direction::Click } else if x == 1usize { Direction::Press } else { Direction::Release })))),
                                    widget::button::icon(widget::icon::from_path(PathBuf::from("/usr/share/icons/breeze-dark/actions/16/albumfolder-user-trash.svg")))
                                        .on_press(RemoveInstruction(index as isize))
                                ].spacing(10).into()
                            }
                            Token::Raw(keycode, _) => {
                                widget::text::body(format!("Raw: {:?}", keycode)).into()
                            }
                            Token::Button(button, direction) => {
                                let button_str = button_to_string(&button).unwrap_or_default();
                                row![
                                    widget::text::body("Button:".to_string()).align_y(Alignment::Center),
                                    widget::text_input("", button_str)
                                        .on_input(move |x| EditInstruction(index, Instruction::Token(Token::Button(string_to_button(x.as_str()).unwrap_or(button), direction.clone())))),
                                    widget::dropdown(&["Click", "Press", "Release"], Some(if direction == Direction::Click { 0usize } else if direction == Direction::Press { 1usize } else { 2usize }), move |x: usize| EditInstruction(index, Instruction::Token(Token::Button(button, if x == 0 { Direction::Click } else if x == 1 { Direction::Press } else { Direction::Release })))),
                                    widget::button::icon(widget::icon::from_path(PathBuf::from("/usr/share/icons/breeze-dark/actions/16/albumfolder-user-trash.svg")))
                                        .on_press(RemoveInstruction(index as isize))
                                ].spacing(10).into()
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
                                    widget::button::icon(widget::icon::from_path(PathBuf::from("/usr/share/icons/breeze-dark/actions/16/albumfolder-user-trash.svg")))
                                        .on_press(RemoveInstruction(index as isize))
                                ].spacing(10).into()
                            }
                            Token::Scroll(amount, axis) => {
                                row![
                                    widget::text::body("Scroll:".to_string()).align_y(Alignment::Center),
                                    widget::text_input("Amount", format!("{}", amount))
                                        .on_input(move |new_amount| EditInstruction(index, Instruction::Token(Token::Scroll(new_amount.parse().unwrap_or(amount), axis.clone())))),
                                    widget::dropdown(&["Vertical", "Horizontal"], Some(if axis == Axis::Vertical { 0 } else { 1 }), move |new_axis: usize| EditInstruction(index, Instruction::Token(Token::Scroll(amount, if new_axis == 0 { Axis::Vertical } else { Axis::Horizontal })))),
                                    widget::button::icon(widget::icon::from_path(PathBuf::from("/usr/share/icons/breeze-dark/actions/16/albumfolder-user-trash.svg")))
                                        .on_press(RemoveInstruction(index as isize))
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
                            widget::button::icon(widget::icon::from_path(PathBuf::from("/usr/share/icons/breeze-dark/actions/16/albumfolder-user-trash.svg")))
                                .on_press(RemoveInstruction(index as isize))
                        ].spacing(10).into()
                        //widget::text::body(format!("Wait: {}ms", duration)).into()
                    }
                    Instruction::Script(script) => {
                        row![
                            widget::text::body("Script:".to_string()).align_y(Alignment::Center),
                            widget::text_input("", script)
                                .on_input(move |x| EditInstruction(index, Instruction::Script(x))),
                            widget::button::icon(widget::icon::from_path(PathBuf::from("/usr/share/icons/breeze-dark/actions/16/albumfolder-user-trash.svg")))
                                        .on_press(RemoveInstruction(index as isize))
                        ].spacing(10).into()
                        //widget::text::body(format!("Script: {}", script)).into()
                    }
                };
                let instruction = row![
                    instruction,
                    cosmic::widget::dropdown(
                        &[
                            "Wait",
                            "Text",
                            "Key",
                            "Mouse Button",
                            "Move Move",
                            "Scroll",
                            "Run Script",
                        ],
                        None,
                        move |selected| match selected {
                            0 => AddInstruction(index, Instruction::Wait(1000)),
                            1 => AddInstruction(index, Instruction::Token(Token::Text("text".into()))),
                            2 => AddInstruction(index, Instruction::Token(Token::Key(Key::Unicode('a'.into()), Direction::Click))),
                            3 => AddInstruction(index, Instruction::Token(Token::Button(Button::Left, Direction::Click))),
                            4 => AddInstruction(index, Instruction::Token(Token::MoveMouse(100, 100, Coordinate::Rel))),
                            5 => AddInstruction(index, Instruction::Token(Token::Scroll(4, Axis::Vertical))),
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
                        "Move Move",
                        "Scroll",
                        "Run Script",
                    ],
                    None,
                    move |selected| match selected {
                        0 => AddInstruction(len, Instruction::Wait(1000)),
                        1 => AddInstruction(len, Instruction::Token(Token::Text("text".into()))),
                        2 => AddInstruction(len, Instruction::Token(Token::Key(Key::Unicode('a'.into()), Direction::Click))),
                        3 => AddInstruction(len, Instruction::Token(Token::Button(Button::Left, Direction::Click))),
                        4 => AddInstruction(len, Instruction::Token(Token::MoveMouse(100, 100, Coordinate::Rel))),
                        5 => AddInstruction(len, Instruction::Token(Token::Scroll(4, Axis::Vertical))),
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
