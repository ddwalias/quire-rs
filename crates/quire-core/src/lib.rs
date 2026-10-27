//! Portable worksheet model, ordered calculation, and Quire-owned persistence.
mod calculation;
mod format;
pub mod layout;
pub mod syntax;

pub use calculation::{Calculation, MathError, Quantity};
pub use format::FileError;
pub use layout::RegionLayout;
use serde::{Deserialize, Serialize};

/// A region's ID survives editing, reordering, and save/reopen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    pub id: u64,
    pub content: RegionContent,
    /// Absent only while decoding a version 1 document, before migration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<RegionLayout>,
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
        let layout = self
            .regions
            .iter()
            .filter_map(|region| region.layout)
            .max_by_key(|layout| (layout.page, layout.y))
            .map(|last| {
                let next_y = last.y + 84;
                if next_y > layout::CONTENT_BOTTOM {
                    RegionLayout {
                        page: last.page + 1,
                        ..RegionLayout::automatic(0)
                    }
                } else {
                    RegionLayout {
                        page: last.page,
                        y: next_y,
                        ..RegionLayout::automatic(0)
                    }
                }
            })
            .unwrap_or_else(|| RegionLayout::automatic(0));
        self.regions.push(Region {
            id,
            content,
            layout: Some(layout),
        });
        id
    }

    /// Visual reading order is page, then top to bottom, then left to right.
    /// Exact ties retain their previous calculation order.
    pub fn sort_by_position(&mut self) {
        self.regions.sort_by_key(|region| {
            region
                .layout
                .map(|layout| (layout.page, layout.y, layout.x))
        });
    }

    /// Rebuild bindings from the top. Text has no effect; errors never leave
    /// a stale previous result or a previous definition of the same name.
    pub fn calculate(&self) -> Vec<Option<Result<Calculation, MathError>>> {
        calculation::calculate(self)
    }
}
