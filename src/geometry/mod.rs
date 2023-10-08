use std::mem;

use ash::vk;

pub type Vec2 = nalgebra::Vector2<f32>;
pub type Vec3 = nalgebra::Vector3<f32>;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShaderVertex<'a> {
    position: &'a [f32],
    color: &'a [f32],
}

impl<'a> From<&'a Vertex> for ShaderVertex<'a> {
    fn from(vertex: &'a Vertex) -> Self {
        Self {
            position: vertex.position.as_slice(),
            color: vertex.color.as_slice(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    position: Vec2,
    color: Vec3,
}

impl Vertex {
    pub const STRIDE: usize = mem::size_of::<[f32; 2]>() + mem::size_of::<[f32; 3]>();

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
            offset: mem::size_of::<[f32; 2]>() as u32,
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
