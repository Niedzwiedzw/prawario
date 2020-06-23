use crate::game::UserInput;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClientMessage {
    pub input: UserInput,
}


impl ClientMessage {
    pub fn new(input: UserInput) -> Self {
        Self { input }
    }
}
