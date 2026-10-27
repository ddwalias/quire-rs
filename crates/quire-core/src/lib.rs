//! UI-independent worksheet data.
//!
//! This initial model supports text regions only. File formats, math and
//! calculation are intentionally not implemented yet.

/// A worksheet held in memory by the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worksheet {
    title: String,
    text_regions: Vec<String>,
}

impl Default for Worksheet {
    fn default() -> Self {
        Self::new("Untitled worksheet")
    }
}

impl Worksheet {
    /// Creates an empty worksheet with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            text_regions: Vec::new(),
        }
    }

    /// Returns the worksheet's display title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Adds an empty text region to the end of the worksheet.
    pub fn add_text_region(&mut self) {
        self.text_regions.push(String::new());
    }

    /// Returns the text regions in worksheet order.
    pub fn text_regions(&self) -> &[String] {
        &self.text_regions
    }

    /// Returns the text regions for editing.
    pub fn text_regions_mut(&mut self) -> &mut [String] {
        &mut self.text_regions
    }
}

#[cfg(test)]
mod tests {
    use super::Worksheet;

    #[test]
    fn new_worksheet_starts_empty() {
        let worksheet = Worksheet::new("Engineering notes");

        assert_eq!(worksheet.title(), "Engineering notes");
        assert!(worksheet.text_regions().is_empty());
    }

    #[test]
    fn adding_and_editing_text_preserves_region_order() {
        let mut worksheet = Worksheet::default();
        worksheet.add_text_region();
        worksheet.add_text_region();
        worksheet.text_regions_mut()[0].push_str("First");
        worksheet.text_regions_mut()[1].push_str("Second");

        assert_eq!(worksheet.text_regions(), ["First", "Second"]);
    }
}
