#[derive(Debug, Clone)]
pub struct TextDefinition {
    font_size: u16,
}

impl TextDefinition {
    /// Create a new rectangle definition with default values
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_font_size(&mut self, size: u16) {
        self.font_size = size;
    }

    pub fn font_size(&self) -> u16 {
        self.font_size
    }
}

impl Default for TextDefinition {
    fn default() -> Self {
        Self {
            font_size: 15, // Default font size
        }
    }
}
