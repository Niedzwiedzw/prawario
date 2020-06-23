use crate::game::Game;
use crate::game::PlayerHandle;
use crate::game::PlayerInput;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientMessage {
    pub inputs: Vec<PlayerInput>,
    pub player_handle: PlayerHandle,
}

impl ClientMessage {
    pub fn new(inputs: Vec<PlayerInput>, player_handle: PlayerHandle) -> Self {
        Self {
            inputs,
            player_handle,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    HelloPlayer(PlayerHandle, Game),
}
