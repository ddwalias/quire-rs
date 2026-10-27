//! Portable worksheet model, ordered calculation, and Quire-owned persistence.
mod calculation;
mod format;

pub use calculation::{Calculation, MathError, Quantity};
pub use format::FileError;
use serde::{Deserialize, Serialize};

/// A region's ID survives editing, reordering, and save/reopen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    pub id: u64,
    pub content: RegionContent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "source",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RegionContent {
    Text(String),
    Math(String),
}

impl RegionContent {
    pub fn source(&self) -> &str {
        match self {
            Self::Text(source) | Self::Math(source) => source,
        }
    }
    pub fn source_mut(&mut self) -> &mut String {
        match self {
            Self::Text(source) | Self::Math(source) => source,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Worksheet {
    pub title: String,
    pub regions: Vec<Region>,
}

impl Default for Worksheet {
    fn default() -> Self {
        Self::new("Untitled worksheet")
    }
}

impl Worksheet {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            regions: Vec::new(),
        }
    }

    pub fn add_region(&mut self, content: RegionContent) -> u64 {
        let mut id = self
            .regions
            .iter()
            .map(|r| r.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        if id >= u64::MAX - 1 {
            // A valid document can contain sparse IDs near the upper bound.
            // Reuse a free low ID instead of overflowing or adding an ID that
            // cannot be saved. A Vec cannot occupy this entire ID space.
            id = 1;
            while self.regions.iter().any(|region| region.id == id) {
                id += 1;
            }
        }
        self.regions.push(Region { id, content });
        id
    }

    /// Rebuild bindings from the top. Text has no effect; errors never leave
    /// a stale previous result or a previous definition of the same name.
    pub fn calculate(&self) -> Vec<Option<Result<Calculation, MathError>>> {
        calculation::calculate(self)
    }
}
