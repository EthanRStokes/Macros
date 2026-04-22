use enigo::agent::Token;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn default_macro_id() -> String {
    Uuid::new_v4().simple().to_string()
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub(crate) struct Macro {
    #[serde(default = "default_macro_id")]
    pub(crate) id: String,
    pub(crate) name: String, /// displayed in GUI
    pub(crate) description: String, /// displayed in GUI, TODO: add the description display
    pub(crate) code: Vec<Instruction>,
}

impl Macro {
    pub(crate) fn new(name: String, description: String, code: Vec<Instruction>) -> Self {
        Self {
            id: default_macro_id(),
            name,
            description,
            code,
        }
    }

    pub(crate) fn ensure_id(&mut self) {
        if self.id.trim().is_empty() {
            self.id = default_macro_id();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Instruction {
    Token(Token),
    Wait(u64),
    Script(String),
    Loop {
        count: u32,
        body: Vec<Instruction>,
    },
}
