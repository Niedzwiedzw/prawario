use quicksilver::{
    geom::{Circle, Vector},
    graphics::Color,
    Graphics,
};
use crate::rendering::Render;
use itertools::Itertools;

use crate::game::{PlayerHandle, VectorDef};

pub type CollectibleHandle = PlayerHandle;

use serde::{Deserialize, Serialize};

macro_rules! min {
    ($x: expr) => ($x);
    ($x: expr, $($z: expr),+) => {{
        let y = min!($($z),*);
        if $x < y {
            $x
        } else {
            y
        }
    }}
}

macro_rules! max {
    ($x: expr) => ($x);
    ($x: expr, $($z: expr),+) => {{
        let y = min!($($z),*);
        if $x > y {
            $x
        } else {
            y
        }
    }}
}



#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum CollisionBetween {
    PlayerAndPlayer(PlayerHandle, PlayerHandle),
    PlayerAndCollectible(PlayerHandle, CollectibleHandle),
}

pub trait Obstacle {
    fn strength(&self) -> f32;
    fn radius(&self) -> f32;
    fn center(&self) -> Vector;
    fn collides(&self, other: &impl Obstacle) -> bool {
        self.center().distance(other.center()) < max!(self.radius(), other.radius())
    }

    fn can_kill(&self, other: &impl Obstacle) -> bool {
        self.strength() > other.strength()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Collectible {
    pub handle: CollectibleHandle,
    pub name: String,
    #[serde(with = "VectorDef")]
    pub position: Vector,
    pub speed: f32,
    #[serde(with = "VectorDef")]
    pub direction: Vector,
    pub size: f32,
}

impl Render for Collectible {
    fn render(&self, gfx: &mut Graphics) {
        gfx.fill_circle(&Circle::new(self.position, self.size as f32), Color::BLUE);
    }
}

impl Obstacle for Collectible {
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

impl PartialEq for Collectible {
    fn eq(&self, other: &Collectible) -> bool {
        self.handle == other.handle
    }
}

impl Eq for Collectible {}
