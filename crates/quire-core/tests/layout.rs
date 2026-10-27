use quire_core::{
    RegionContent, RegionLayout, Worksheet,
    layout::{CONTENT_BOTTOM, CONTENT_TOP, PAGE_WIDTH},
};

#[test]
fn version_one_migrates_without_losing_sources_order_ids_or_results() {
    let v1 = include_str!("../../../corpus/lengths.quire.json");
    let migrated = Worksheet::from_json(v1).unwrap();
    assert!(
        migrated
            .regions
            .iter()
            .all(|region| region.layout.unwrap().is_valid())
    );
    assert_eq!(
        migrated.regions[0].layout.unwrap(),
        RegionLayout::automatic(0)
    );
    assert_eq!(migrated.regions[1].id, 2);
    assert_eq!(migrated.regions[1].content.source(), "a := 25 cm");
    assert_eq!(
        migrated.calculate()[4]
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .display_value,
        125.0
    );
    let v2 = migrated.to_json().unwrap();
    assert!(v2.contains("\"version\": 2"));
    assert_eq!(Worksheet::from_json(&v2).unwrap(), migrated);
}

#[test]
fn moving_regions_changes_reading_order_and_persists_across_pages() {
    let mut sheet = Worksheet::default();
    sheet.add_region(RegionContent::Math("x := 2 m".into()));
    sheet.add_region(RegionContent::Math("x -> cm".into()));
    let original = sheet.calculate()[1].clone().unwrap().unwrap();
    assert_eq!(original.display_value, 200.0);
    sheet.regions[0].layout.as_mut().unwrap().page = 1;
    assert!(
        sheet.to_json().is_err(),
        "Do not persist an array order that contradicts the visual order"
    );
    sheet.sort_by_position();
    assert_eq!(sheet.regions[0].id, 2);
    assert!(sheet.calculate()[0].as_ref().unwrap().is_err());
    let reopened = Worksheet::from_json(&sheet.to_json().unwrap()).unwrap();
    assert_eq!(reopened, sheet);
    assert_eq!(reopened.calculate(), sheet.calculate());
}

#[test]
fn invalid_coordinates_unknown_layout_fields_and_missing_v2_layout_are_rejected() {
    let mut sheet = Worksheet::default();
    sheet.add_region(RegionContent::Text("Retain me".into()));
    let json = sheet.to_json().unwrap();
    for invalid in [
        json.replace("\"page\": 0", "\"page\": 1000"),
        json.replace("\"x\": 64", "\"x\": -1"),
        json.replace("\"y\": 156", "\"y\": 4294967295"),
        json.replace("\"width\": 666", "\"width\": 800"),
        json.replace("\"page\":", "\"unknown\": 1, \"page\":"),
        json.replace("\"version\": 2", "\"version\": 1"),
    ] {
        assert!(Worksheet::from_json(&invalid).is_err(), "{invalid}");
    }
    sheet.regions[0].layout = None;
    assert!(sheet.to_json().is_err());
}

#[test]
fn automatic_positions_stay_inside_pages_and_ties_preserve_order() {
    let mut sheet = Worksheet::default();
    for _ in 0..35 {
        sheet.add_region(RegionContent::Math("1".into()));
    }
    assert!(sheet.regions.iter().all(|region| {
        let layout = region.layout.unwrap();
        layout.y >= CONTENT_TOP
            && layout.y <= CONTENT_BOTTOM
            && layout.x + layout.width <= PAGE_WIDTH
            && layout.is_valid()
    }));
    assert!(sheet.regions[34].layout.unwrap().page > 0);
    let same = sheet.regions[0].layout;
    sheet.regions[1].layout = same;
    sheet.sort_by_position();
    assert_eq!(sheet.regions[0].id, 1);
    assert_eq!(sheet.regions[1].id, 2);
}
