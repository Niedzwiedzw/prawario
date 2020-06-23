use quicksilver::{
    Graphics
};

pub trait Render {
    fn render(&self, gfx: &mut Graphics);
}
