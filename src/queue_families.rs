pub struct QueueFamilyIndice {
    pub graphics_family: Option<u32>,
}

impl QueueFamilyIndice {
    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some()
    }
}

impl Default for QueueFamilyIndice {
    fn default() -> Self {
        Self {
            graphics_family: None,
        }
    }
}