use std::mem;

use ash::vk;

pub type Point2 = cgmath::Point2<f32>;
pub type Point3 = cgmath::Point3<f32>;

pub type Vec2 = cgmath::Vector2<f32>;
pub type Vec3 = cgmath::Vector3<f32>;

pub type Mat4 = cgmath::Matrix4<f32>;

#[repr(C)]
#[derive(Debug, Clone, PartialEq)]
pub struct ModelViewProj {
    model: Mat4,
    view: Mat4,
    proj: Mat4,
}

impl ModelViewProj {
    pub fn new(model: Mat4, view: Mat4, proj: Mat4) -> Self {
        Self { model, view, proj }
    }
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq)]
pub struct Vertex {
    position: Vec2,
    color: Vec3,
}

impl Vertex {
    pub const STRIDE: usize = mem::size_of::<Self>();

    pub const BINDING_DESCRIPTIONS: &[vk::VertexInputBindingDescription] =
        &[vk::VertexInputBindingDescription {
            binding: 0,
            stride: Self::STRIDE as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }];
    pub const ATTRIBUTE_DESCRIPTIONS: &[vk::VertexInputAttributeDescription] = &[
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 0,
            format: vk::Format::R32G32_SFLOAT,
            offset: 0,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 1,
            format: vk::Format::R32G32B32_SFLOAT,
            offset: mem::size_of::<Vec2>() as u32,
        },
    ];

    pub const fn new(position: Vec2, color: Vec3) -> Self {
        Self { position, color }
    }

    #[allow(dead_code)]
    pub const fn zero() -> Self {
        Self::new(Vec2::new(0.0, 0.0), Vec3::new(0.0, 0.0, 0.0))
    }
}
