use crate::obstacles::CollectibleHandle;
use crate::rendering::Render;
use itertools::Itertools;
use quicksilver::input::Key;
use quicksilver::Input;
use quicksilver::{
    geom::{Circle, Vector},
    graphics::Color,
    Graphics,
};

use crate::communication::ClientMessage;
use crate::obstacles::{Collectible, CollisionBetween, Obstacle};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use std::collections::HashMap;
use std::convert::TryFrom;
#[cfg(feature = "wee_alloc")]
use wasm_bindgen::JsValue;
#[cfg(feature = "wee_alloc")]
use web_sys::console;

pub type PlayerHandle = usize;

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub enum UserInput {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub enum GameStateMutation {
    KillPlayer(PlayerHandle),
    HealPlayer(PlayerHandle, f32),
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "quicksilver::geom::Vector")]
pub struct VectorDef {
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

pub type PlayerInput = (PlayerHandle, UserInput);

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Player {
    pub handle: PlayerHandle,
    pub name: String,
    #[serde(with = "VectorDef")]
    pub position: Vector,
    pub speed: f32,
    #[serde(with = "VectorDef")]
    pub direction: Vector,
    pub size: f32,
}

impl Player {
    pub fn new(player_handle: PlayerHandle) -> Self {
        Self {
            size: crate::config::PLAYER_MIN_SIZE,
            speed: crate::config::PLAYER_DEFAULT_SPEED,
            handle: player_handle,
            ..Default::default()
        }
    }
}

impl PartialEq for Player {
    fn eq(&self, other: &Player) -> bool {
        self.handle == other.handle
    }
}

impl Eq for Player {}

impl Obstacle for Player {
    fn radius(&self) -> f32 {
        self.size
    }
    fn center(&self) -> quicksilver::geom::Vector {
        self.position
    }

    fn strength(&self) -> f32 {
        self.size
    }

}

impl Render for Player {
    fn render(&self, gfx: &mut Graphics) {
        gfx.fill_circle(&Circle::new(self.position, self.size as f32), Color::RED);
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Game {
    pub players: HashMap<PlayerHandle, Player>,
    pub collectibles: HashMap<CollectibleHandle, Collectible>,
    pub active_player: Option<PlayerHandle>, // for frontend
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

    pub fn add(&mut self, key: PlayerHandle) -> PlayerHandle {
        self.players.insert(key, Player::new(key));
        key
    }

    pub fn remove(&mut self, key: &PlayerHandle) {
        self.players.remove(&key);
    }

    pub fn get_player_input(&mut self, mut input: &mut Input) -> Vec<PlayerInput> {
        if let Some(player_handle) = self.active_player {
            return pressed_keys(&mut input)
                .into_iter()
                .map(|key| (player_handle, key.into()))
                .collect();
        }
        vec![]
    }

    pub fn to_client_message(&self, inputs: &Vec<PlayerInput>) -> Option<ClientMessage> {
        if inputs.is_empty() {
            return None;
        }
        Some(ClientMessage {
            inputs: inputs.clone(),
            player_handle: self.active_player?,
        })
    }

    pub fn handle_inputs(&mut self, inputs: Vec<PlayerInput>) {
        for (handle, directions) in &inputs.iter().group_by(|(handle, _direction)| handle) {
            if let Some(mut player) = self.players.get_mut(handle) {
                player.direction = directions.map(|player_input| player_input.1.into()).sum();
            }
        }
    }

    pub fn handle_client_message(&mut self, message: &ClientMessage) {
        self.handle_inputs(message.inputs.clone());
    }

    pub fn handle_quicksilver_input(&mut self, mut input: &mut Input) {
        if let Some(_player_handle) = self.active_player {
            let inputs = self.get_player_input(&mut input);
            // debug_log(format!("inputs: {:?}", inputs));
            self.handle_inputs(inputs);
        }
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
        if let Ok(state) = from_str(new_state.as_str()) {
            let active_player = self.active_player.clone();
            *self = Self {
                active_player,
                ..state
            }
        }
    }

    pub fn player_collisions(&self) -> Vec<CollisionBetween> {
        self.players
            .values()
            .combinations(2)
            .map(|pair| (pair[0], pair[1]))
            .filter(|(one, other)| one.collides(*other))
            .map(|(one, other)| CollisionBetween::PlayerAndPlayer(one.handle, other.handle))
            .collect()
    }

    pub fn collectible_collisions(&self) -> Vec<CollisionBetween> {
        let mut collisions = vec![];
        for player in self.players.values() {
            for collectible in self.collectibles.values() {
                if player.collides(collectible) {
                    collisions.push(CollisionBetween::PlayerAndCollectible(
                        player.handle,
                        collectible.handle,
                    ))
                }
            }
        }
        collisions
    }
    pub fn to_mutation(&self, collision: &CollisionBetween) -> Option<Vec<GameStateMutation>> {
        match collision {
            CollisionBetween::PlayerAndPlayer(one, other) => {
                let (one, other) = (self.players.get(&one)?, self.players.get(&other)?);
                if one.can_kill(other) {
                    return Some(vec![
                        GameStateMutation::KillPlayer(other.handle),
                        GameStateMutation::HealPlayer(one.handle, other.size),
                    ]);
                } else if other.can_kill(one) {
                    return Some(vec![
                        GameStateMutation::KillPlayer(one.handle),
                        GameStateMutation::HealPlayer(other.handle, one.size),
                    ]);
                } else { return None }
            },
            CollisionBetween::PlayerAndCollectible(player_handle, collectible_handle) => {
                let (player, collectible) = (self.players.get(player_handle)?, self.collectibles.get(collectible_handle)?);
                if player.can_kill(collectible) {
                    return Some(vec![GameStateMutation::HealPlayer(player.handle, collectible.strength())]);
                } else { return None }
            }
        }
    }

    pub fn mutations(&self) -> Vec<GameStateMutation> {
        self.player_collisions()
            .iter()
            .chain(self.collectible_collisions().iter())
            .filter_map(|c| self.to_mutation(c))
            .flatten()
            .collect()
    }
}

#[cfg(test)]
mod test_movement {
    use super::*;

    #[test]
    fn test_directions() {
        let mut game = Game::new();
        let player_handle = game.add(1);
        assert_eq!(
            game.players.get(&player_handle).unwrap().direction,
            Vector { x: 0., y: 0. }
        );
        game.handle_inputs(vec![(player_handle, UserInput::Right)]);
        assert_eq!(
            game.players.get(&player_handle).unwrap().direction,
            Vector { x: 1., y: 0. }
        );
        game.handle_inputs(vec![(1, UserInput::Right), (1, UserInput::Up)]);
        assert_eq!(
            game.players.get(&player_handle).unwrap().direction,
            Vector { x: 1., y: -1. }
        );
    }
}
