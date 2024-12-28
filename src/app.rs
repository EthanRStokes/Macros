use crate::app::NavMenuAction::SelectMacro;
use crate::macros::{Instruction, Macro};
use crate::util::ThreadPool;
use crate::util::{get_macro, run_macro};
use cosmic::app::{Core, Task};
use cosmic::cosmic_config::{Config, ConfigGet, ConfigSet};
use cosmic::iced::{Alignment, Length};
use cosmic::iced_widget::{column, row};
use cosmic::widget::button::text;
use cosmic::widget::nav_bar;
use cosmic::{executor, ApplicationExt, Element};
use enigo::agent::Token;
use enigo::{Axis, Coordinate, Direction, Enigo, Key};
use std::ops::DerefMut;
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
#[derive(Clone, Debug)]
pub(crate) enum Message {
    NavMenuAction(NavMenuAction),
    RunMacro(Option<usize>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum NavMenuAction {
    SelectMacro(usize),
    RunMacro,
    AddInstruction(Instruction),
    RemoveInstruction(isize),
    ClearInstructions,
    SaveMacro,
}

/// The [`App`] stores application-specific state.
pub(crate) struct App {
    /// COSMIC app settings
    core: Core,
    nav_model: nav_bar::Model,
    macro_selected: Option<usize>,
    current_macro: Option<Macro>,
    /// The application config
    config: Config,
    /// Enigo is an API for mouse and keyboard control
    enigo: Arc<Mutex<Enigo>>,
    thread_pool: ThreadPool,
    macros: Option<Vec<String>>,
}

impl App {
    fn update_macro(&mut self, selected: Option<usize>) {
        self.macro_selected = selected;
        if let Some(selected) = selected {
            self.current_macro = Some(get_macro(&self.config, selected));
        }
    }
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
            macros: None,
        };

        let config = &app.config;
        let tx = config.transaction();
        let mut macros = config.get::<Vec<Macro>>("macros");

        // Add default config. Everything here is temporary until a later state of the project.
        if macros.is_err() {
            tx.set("macros", vec![
                Macro::new("macro".into(), "description".into(), vec![
                    Instruction::Wait(1000),
                    Instruction::Token(Token::MoveMouse(100, 100, Coordinate::Rel)),
                    Instruction::Token(Token::Key(Key::Unicode('a'.into()), Direction::Press)),
                    Instruction::Token(Token::Key(Key::Unicode('a'.into()), Direction::Release)),
                    Instruction::Token(Token::Key(Key::Unicode('a'.into()), Direction::Press)),
                    Instruction::Token(Token::Key(Key::Unicode('a'.into()), Direction::Release)),
                    Instruction::Wait(1000),
                    Instruction::Token(Token::Key(Key::Unicode('b'.into()), Direction::Press)),
                    Instruction::Token(Token::Key(Key::Unicode('b'.into()), Direction::Release)),
                    Instruction::Token(Token::Text("All of this text just got pasted".into())),
                    Instruction::Wait(500),
                    Instruction::Token(Token::Scroll(4, Axis::Vertical)),
                ]),
                Macro::new("macro2".into(), "description".into(), vec![
                    Instruction::Wait(1000),
                    Instruction::Token(Token::Text("NJOPFPDSFSODPFJODSIFJOPSDPFJ SPAM".into())),
                    Instruction::Wait(500),
                    Instruction::Token(Token::Scroll(4, Axis::Vertical)),
                ]),
                Macro::new("rustrustrust".into(), "awesome macro".into(), vec![
                    Instruction::Wait(1000),
                    Instruction::Token(Token::Text("Rust Rust Rust Rust Rust".into())),
                ]),
            ]).expect("TODO: panic message");
            macros = config.get::<Vec<Macro>>("macros");
        }
        println!("Commit transaction: {:?}", tx.commit());

        let macros = macros.unwrap();
        app.macros = Some(macros.iter().map(|x| x.name.clone()).collect::<Vec<String>>());

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
            Message::NavMenuAction(message) => match message {
                SelectMacro(selected) => {
                    self.update_macro(Some(selected));
                }
                NavMenuAction::RunMacro => {
                    if let Some(mac) = self.current_macro.clone() {
                        run_macro(mac.clone(), &mut *self.enigo.lock().unwrap());
                    }
                }
                NavMenuAction::AddInstruction(instruction) => {
                    if let Some(mut mac) = self.current_macro.clone() {
                        mac.code.push(instruction);
                        self.current_macro = Some(mac);
                    }
                }
                NavMenuAction::RemoveInstruction(index) => {
                    if let Some(mut mac) = self.current_macro.clone() {
                        if mac.code.len() > 0 && index >= 0 {
                            mac.code.remove(index as usize);
                            self.current_macro = Some(mac);
                        }
                    }
                }
                NavMenuAction::ClearInstructions => {
                    if let Some(mut mac) = self.current_macro.clone() {
                        mac.code.clear();
                        self.current_macro = Some(mac);
                    }
                }
                NavMenuAction::SaveMacro => {
                    if let Some(selected) = self.macro_selected {
                        if let Some(mac) = self.current_macro.clone() {
                            let mut macros = self.config.get::<Vec<Macro>>("macros").expect("Macros config missing?");
                            macros[selected] = mac;
                            self.config.set("macros", macros).expect("Couldn't set macros config?");
                        }
                    }
                }
            }
            Message::RunMacro(selected) => {
                if selected.is_none() {
                    return Task::none();
                }
                let selected = selected.unwrap();
                let pool = &mut self.thread_pool;
                let thread_num = pool.workers.len();
                let enigo = (&self.enigo).clone();
                let config = self.config.clone();

                let thread = thread::Builder::new().name(format!("macro_thread: {thread_num}")).spawn(move || {
                    println!("Running macro...");
                    let mac = get_macro(&config, selected);
                    let mut enigo = enigo.lock().unwrap();
                    run_macro(mac, enigo.deref_mut());
                    println!("Macro complete.");
                }).expect("Macro thread failed to spawn");

                // TODO: remove from the pool once the thread has completed.
                pool.add_worker(thread);
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

        let mut content = column![
            page_text
        ];

        if let Some(macs) = &self.macros {
            content = content.push(row![
                column![
                    cosmic::widget::text("Select macro"),
                    cosmic::widget::dropdown(macs, self.macro_selected, |x: usize| Message::NavMenuAction(SelectMacro(x)))
                ],
                text("Run macro")
                    .on_press(Message::RunMacro(self.macro_selected.clone()))
            ]);
        }

        if let Some(mac) = &self.current_macro {
            // TODO: make actual buttons with arguments. Thinking fancy boxes with lines, like a flowchart
            content = content.push(row![
                text("Add wait")
                    .on_press(Message::NavMenuAction(NavMenuAction::AddInstruction(Instruction::Wait(1000)))),
                text("Add text")
                    .on_press(Message::NavMenuAction(NavMenuAction::AddInstruction(Instruction::Token(Token::Text("text".into()))))),
                text("Remove instruction")
                    .on_press(Message::NavMenuAction(NavMenuAction::RemoveInstruction(mac.code.len() as isize - 1))),
                text("Clear instructions")
                    .on_press(Message::NavMenuAction(NavMenuAction::ClearInstructions)),
                text("Run macro")
                    .on_press(Message::NavMenuAction(NavMenuAction::RunMacro)),
            ]);
            content = content.push(text("Save macro").on_press(Message::NavMenuAction(NavMenuAction::SaveMacro)));
        }

        // Centers all the content and makes it look nice
        let centered = cosmic::widget::container(content)
            .width(Length::Fill)
            .height(Length::Shrink)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center);

        Element::from(centered)
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