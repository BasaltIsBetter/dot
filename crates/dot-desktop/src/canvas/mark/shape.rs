use crate::canvas::line::Ink;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rectangle {
    pub(crate) left: f32,
    pub(crate) top: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) ink: Ink,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Ring {
    pub(crate) centre_x: f32,
    pub(crate) centre_y: f32,
    pub(crate) radius: f32,
    pub(crate) thickness: f32,
    pub(crate) ink: Ink,
}
