use tokio::sync::RwLock;
use std::sync::Arc;
use crate::rendering::Render;
use quicksilver::input::Key;
use quicksilver::Input;
use quicksilver::{
    geom::{Circle, Vector},
    graphics::Color,
    Graphics,
};

use serde_json::{from_str, to_string};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;
#[cfg(feature = "wee_alloc")]
use wasm_bindgen::JsValue;
#[cfg(feature = "wee_alloc")]
use web_sys::console;

use itertools::Itertools;

pub type PlayerHandle = usize;
pub type GameState = Arc<RwLock<Game>>;

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub enum UserInput {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "quicksilver::geom::Vector")]
struct VectorDef {
    x: f32,
    y: f32,
}

pub fn debug_log(string: String) {
    #[cfg(feature = "wee_alloc")]
    console::log_1(&JsValue::from_str(string.as_str()));
    #[cfg(feature = "backend")]
    println!("{}", string);
}

static INPUTS: [UserInput; 4] = [
    UserInput::Right,
    UserInput::Left,
    UserInput::Up,
    UserInput::Down,
];

pub fn pressed_keys(input: &mut Input) -> Vec<UserInput> {
    INPUTS
        .iter()
        .filter(|&&k| input.key_down(k.into()))
        .map(|e: &UserInput| e.clone())
        .collect()
}

impl TryFrom<Key> for UserInput {
    type Error = ();

    fn try_from(
        key: Key,
    ) -> std::result::Result<Self, <Self as std::convert::TryFrom<Key>>::Error> {
        match key {
            Key::W => Ok(Self::Up),
            Key::S => Ok(Self::Down),
            Key::A => Ok(Self::Left),
            Key::D => Ok(Self::Right),
            _ => Err(()),
        }
    }
}

impl Into<Key> for UserInput {
    fn into(self) -> Key {
        match self {
            UserInput::Up => Key::W,
            UserInput::Down => Key::S,
            UserInput::Left => Key::A,
            UserInput::Right => Key::D,
        }
    }
}

impl Into<Vector> for UserInput {
    fn into(self) -> Vector {
        match self {
            Self::Up => Vector { x: 0., y: -1. },
            Self::Down => Vector { x: 0., y: 1. },
            Self::Left => Vector { x: -1., y: 0. },
            Self::Right => Vector { x: 1., y: 0. },
        }
    }
}

pub type PlayerInput = (PlayerHandle, Vector);

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    #[serde(with = "VectorDef")]
    pub position: Vector,
    pub speed: f32,
    #[serde(with = "VectorDef")]
    pub direction: Vector,
    pub size: f32,
}

impl Player {
    pub fn new() -> Self {
        Self {
            size: crate::config::PLAYER_MIN_SIZE,
            speed: crate::config::PLAYER_DEFAULT_SPEED,
            ..Default::default()
        }
    }
}

impl Render for Player {
    fn render(&self, gfx: &mut Graphics) {
        gfx.fill_circle(&Circle::new(self.position, self.size as f32), Color::RED);
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Game {
    pub players: HashMap<PlayerHandle, Player>,
}

impl Render for Game {
    fn render(&self, gfx: &mut Graphics) {
        for player in self.players.values() {
            player.render(gfx)
        }
    }
}

impl Game {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(&mut self, key: PlayerHandle) -> usize {
        self.players.insert(key, Player::new());
        key
    }

    pub fn get_player_input(
        &mut self,
        mut input: &mut Input,
        player_handle: PlayerHandle,
    ) -> Vec<PlayerInput> {
        pressed_keys(&mut input)
            .into_iter()
            .map(|key| (player_handle, key.into()))
            .collect()
    }

    pub fn handle_inputs(&mut self, inputs: Vec<PlayerInput>) {
        for (handle, directions) in &inputs.iter().group_by(|(handle, direction)| handle) {
            if let Some(mut player) = self.players.get_mut(handle) {
                player.direction = directions.map(|player_input| player_input.1).sum();
            }
        }
    }

    pub fn handle_quicksilver_input(&mut self, mut input: &mut Input, player_handle: PlayerHandle) {
        let inputs = self.get_player_input(&mut input, player_handle);
        debug_log(format!("inputs: {:?}", inputs));
        self.handle_inputs(inputs);
    }

    pub fn step(&mut self) {
        for player in self.players.values_mut() {
            let direction = player.direction.clone();
            player.position += direction;
        }
    }

    pub fn state_dump(&self) -> String {
        to_string(self).expect(format!("was unable to dump {:#?}", self).as_str())
    }

    pub fn update_state(&mut self, new_state: String) {
        *self = from_str(new_state.as_str())
            .expect(format!("was unable to update  {:#?}\n with \n{:#?}", self, new_state).as_str())
    }
}


#[cfg(test)]
mod test_movement {
    use super::*;

    #[test]
    fn test_directions() {
        let mut game = Game::new();
        let player_handle = game.add(1);
        assert_eq!(game.players.get(&player_handle).unwrap().direction, Vector { x: 0., y: 0. });
        game.handle_inputs(vec![(player_handle, Vector::new(1., 0.))]);
        assert_eq!(game.players.get(&player_handle).unwrap().direction, Vector { x: 1., y: 0. });
        game.handle_inputs(vec![(1, Vector { x: 1.0, y: 0.0 }), (1, Vector { x: 0.0, y: -1.0 })]);
        assert_eq!(game.players.get(&player_handle).unwrap().direction, Vector { x: 1., y: -1. });
    }
}
