//! Portable page coordinates, independent of screen pixels and UI toolkits.
use serde::{Deserialize, Serialize};

pub const PAGE_WIDTH: u32 = 794;
pub const PAGE_HEIGHT: u32 = 1123;
pub const PAGE_MARGIN: u32 = 64;
pub const CONTENT_TOP: u32 = 156;
pub const CONTENT_BOTTOM: u32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionLayout {
    pub page: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
}

impl RegionLayout {
    pub fn automatic(index: usize) -> Self {
        let rows = ((CONTENT_BOTTOM - CONTENT_TOP) / 84 + 1) as usize;
        Self {
            page: (index / rows) as u32,
            x: PAGE_MARGIN,
            y: CONTENT_TOP + (index % rows) as u32 * 84,
            width: PAGE_WIDTH - 2 * PAGE_MARGIN,
        }
    }

    pub fn is_valid(self) -> bool {
        self.page < 1000
            && self.x >= 24
            && self.y >= CONTENT_TOP
            && self.y <= CONTENT_BOTTOM
            && self.width >= 96
            && self
                .x
                .checked_add(self.width)
                .is_some_and(|right| right <= PAGE_WIDTH - 24)
    }
}
