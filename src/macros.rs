use enigo::agent::Token;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub(crate) struct Macro {
    pub(crate) name: String, /// displayed in GUI
    pub(crate) description: String, /// displayed in GUI, TODO: add the description display
    pub(crate) code: Vec<Instruction>,
}

impl Macro {
    pub(crate) fn new(name: String, description: String, code: Vec<Instruction>) -> Self {
        Self { name, description, code, }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Instruction {
    Token(Token),
    Wait(u64),
    Script(String)
}
