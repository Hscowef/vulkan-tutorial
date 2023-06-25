use std::collections::HashSet;

#[derive(Clone, Copy, Debug)]
pub struct QueueFamilyIndice {
    pub graphics_family: Option<u32>,
    pub present_family: Option<u32>,
}

impl QueueFamilyIndice {
    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }

    pub fn get_unique_families(&self) -> HashSet<u32> {
        let mut uniques = HashSet::new();
        if let Some(value) = self.graphics_family {
            uniques.insert(value);
        }

        if let Some(value) = self.present_family {
            uniques.insert(value);
        }

        uniques
    }
}

impl Default for QueueFamilyIndice {
    fn default() -> Self {
        Self {
            graphics_family: None,
            present_family: None,
        }
    }
}
