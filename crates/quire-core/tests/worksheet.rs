use quire_core::{Calculation, RegionContent, Worksheet};

fn sheet(lines: &[&str]) -> Worksheet {
    let mut sheet = Worksheet::new("Test worksheet");
    for line in lines {
        sheet.add_region(RegionContent::Math((*line).into()));
    }
    sheet
}
fn result(sheet: &Worksheet, index: usize) -> Calculation {
    sheet.calculate()[index]
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .clone()
}
fn failure(source: &str) -> String {
    sheet(&[source]).calculate()[0]
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap_err()
        .to_string()
}

#[test]
fn arithmetic_precedence_parentheses_unary_and_scientific_literals() {
    for (source, expected) in [
        ("2 + 3 * 4", 14.0),
        ("(2 + 3) * 4", 20.0),
        ("20 / 5 / 2", 2.0),
        ("10 - 3 - 2", 5.0),
        ("-2^2", -4.0),
        ("(-2)^2", 4.0),
        ("2^3^2", 512.0),
        ("2^-2", 0.25),
        ("--2 + +3", 5.0),
        ("1.2e3 + .5E-1", 1200.05),
    ] {
        let value = result(&sheet(&[source]), 0);
        assert_eq!(value.quantity.si_value, expected, "{source}");
        assert_eq!(value.quantity.length_power, 0, "{source}");
    }
}

#[test]
fn ordered_bindings_redefinitions_and_recalculation() {
    let mut worksheet = sheet(&["a", "a := 2", "b := a * 3", "a := a + 1", "b + a"]);
    assert!(worksheet.calculate()[0].as_ref().unwrap().is_err());
    assert_eq!(result(&worksheet, 4).quantity.si_value, 9.0);
    *worksheet.regions[1].content.source_mut() = "a := 4".into();
    assert_eq!(result(&worksheet, 2).quantity.si_value, 12.0);
    assert_eq!(result(&worksheet, 4).quantity.si_value, 17.0);
    worksheet.regions.swap(1, 2);
    assert!(worksheet.calculate()[1].as_ref().unwrap().is_err());
    assert!(worksheet.calculate()[4].as_ref().unwrap().is_err());
}

#[test]
fn invalid_definition_blocks_stale_binding_and_recovers_after_edit() {
    let mut worksheet = sheet(&[
        "x := 3",
        "x := 1/0",
        "x + 1",
        "unrelated := 42",
        "unrelated",
    ]);
    let calculated = worksheet.calculate();
    assert!(calculated[1].as_ref().unwrap().is_err());
    assert!(
        calculated[2]
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("invalid")
    );
    assert_eq!(result(&worksheet, 4).quantity.si_value, 42.0);
    *worksheet.regions[1].content.source_mut() = "x := 5".into();
    assert_eq!(result(&worksheet, 2).quantity.si_value, 6.0);
    *worksheet.regions[1].content.source_mut() = format!("x := {}", "1".repeat(20_000));
    assert!(worksheet.calculate()[2].as_ref().unwrap().is_err());
}

#[test]
fn length_conversion_dimensions_and_upstream_edit() {
    let mut worksheet = sheet(&["a := 25 cm", "b := a + 1 m", "b -> m", "b -> cm"]);
    assert_eq!(result(&worksheet, 2).quantity.si_value, 1.25);
    assert_eq!(result(&worksheet, 2).quantity.length_power, 1);
    assert_eq!(result(&worksheet, 3).display_value, 125.0);
    assert_eq!(result(&worksheet, 3).display_unit, "cm");
    *worksheet.regions[0].content.source_mut() = "a := 50 cm".into();
    assert_eq!(result(&worksheet, 2).quantity.si_value, 1.5);
    assert_eq!(result(&worksheet, 3).display_value, 150.0);
    // Conversion through metres introduces binary rounding at this scale;
    // allow 2e-10 mm (less than two ULPs around one million millimetres).
    assert!((result(&sheet(&["1 km + 2 mm -> mm"]), 0).display_value - 1_000_002.0).abs() <= 2e-10);
    let area = result(&sheet(&["(2 m)^2"]), 0);
    assert_eq!(area.quantity.length_power, 2);
    assert_eq!(area.display_unit, "m^2");
    assert_eq!(result(&sheet(&["(2 m)/(50 cm)"]), 0).quantity.si_value, 4.0);
    assert_eq!(result(&sheet(&["m^-2"]), 0).quantity.length_power, -2);
}

#[test]
fn invalid_inputs_are_errors_not_partial_results() {
    for source in [
        "",
        " ",
        "1 +",
        "(1+2",
        "1)",
        "1 2",
        "sin(1)",
        "π",
        "2%3",
        "a = 2",
        "x :=",
        "2x := 3",
        "x := y := 1",
        "m := 3",
        "1.2.3",
        "1e+",
        "1e999",
        "1e308 * 1e308",
        "0/0",
        "1/-0",
        "0^0",
        "0^-1",
        "2^0.5",
        "2^17",
        "2^m",
        "((m^16)^16)",
        "1 -> cm",
        "m^2 -> m",
        "1 m + 2",
        "1 m -> s",
        "1 m -> cm -> m",
    ] {
        assert!(!failure(source).is_empty(), "{source}");
    }
    assert!(failure("1 m + 2").contains("Dimensional"));
    assert!(failure("1/0").contains("zero"));
    assert!(failure("unknown").contains("Undefined variable"));
}

#[test]
fn adversarial_inputs_have_bounded_depth_and_size() {
    for source in [
        format!("{}1{}", "(".repeat(200), ")".repeat(200)),
        format!("{}1", "-".repeat(300)),
        "1+".repeat(1100),
        "1".repeat(20_000),
    ] {
        assert!(!failure(&source).is_empty());
    }
}

#[test]
fn unicode_text_invalid_math_order_ids_and_title_survive_roundtrip() {
    let mut worksheet = Worksheet::new("Length – 長さ");
    worksheet.add_region(RegionContent::Text("Notes\nμm and 日本語 🦀".into()));
    worksheet.add_region(RegionContent::Math("a := 25 cm".into()));
    worksheet.add_region(RegionContent::Text(String::new()));
    worksheet.add_region(RegionContent::Math("a + 1 m -> cm".into()));
    worksheet.add_region(RegionContent::Math("future_syntax(α)".into()));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("長さ.quire.json");
    worksheet.save(&path).unwrap();
    let reopened = Worksheet::open(&path).unwrap();
    assert_eq!(reopened, worksheet);
    assert_eq!(reopened.calculate(), worksheet.calculate());
    assert_eq!(result(&reopened, 3).display_value, 125.0);
    assert!(reopened.calculate()[0].is_none());
    let mut edited = reopened;
    *edited.regions[1].content.source_mut() = "a := 50 cm".into();
    edited.save(&path).unwrap();
    assert_eq!(
        result(&Worksheet::open(path).unwrap(), 3).display_value,
        150.0
    );
}

#[test]
fn unknown_fields_versions_regions_duplicates_and_truncation_are_rejected() {
    let json = sheet(&["2 + 3"]).to_json().unwrap();
    for invalid in [
        json.replace("\"version\": 2", "\"version\": 99"),
        json.replace("quire-worksheet", "other-format"),
        json.replace("\"math\"", "\"plot\""),
        json.replace("\"title\":", "\"unexpected\": 1, \"title\":"),
        json.replace("\"id\": 1", "\"id\": 0"),
        json[..json.len() - 1].into(),
        format!("{{\"extra\": 1, {}", &json[1..]),
    ] {
        assert!(Worksheet::from_json(&invalid).is_err(), "{invalid}");
    }
    let mut duplicate = sheet(&["1", "2"]);
    duplicate.regions[1].id = 1;
    assert!(duplicate.to_json().is_err());
    assert!(Worksheet::from_json(&" ".repeat(8 * 1024 * 1024 + 1)).is_err());
}

#[test]
fn failed_saves_and_mathcad_paths_leave_existing_bytes_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("protected.quire.json");
    let original = sheet(&["3"]);
    original.save(&path).unwrap();
    let before = std::fs::read(&path).unwrap();
    let mut invalid = sheet(&["1", "2"]);
    invalid.regions[1].id = 1;
    assert!(invalid.save(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let mut oversized = Worksheet::default();
    oversized.add_region(RegionContent::Text("a".repeat(8 * 1024 * 1024)));
    assert!(oversized.save(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(
        original
            .save(dir.path().join("missing/file.quire.json"))
            .is_err()
    );
    let mcdx = dir.path().join("reference.mcdx");
    std::fs::write(&mcdx, b"opaque Mathcad sentinel").unwrap();
    assert!(Worksheet::open(&mcdx).is_err());
    assert!(original.save(&mcdx).is_err());
    assert_eq!(std::fs::read(mcdx).unwrap(), b"opaque Mathcad sentinel");
}

#[test]
fn synthetic_corpus_length_example_matches_recorded_semantic_observation() {
    // See corpus/README.md for the reference version, settings, and scope.
    // The synthetic JSON contains no original Mathcad document or raw output.
    let worksheet =
        Worksheet::from_json(include_str!("../../../corpus/lengths.quire.json")).unwrap();
    let metres = result(&worksheet, 3);
    assert_eq!(metres.quantity.si_value, 1.25);
    assert_eq!(metres.quantity.length_power, 1);
    assert_eq!(metres.display_value, 1.25);
    assert_eq!(metres.display_unit, "m");
    assert_eq!(result(&worksheet, 4).display_value, 125.0);
}

#[test]
fn unknown_content_metadata_and_duplicate_json_keys_are_rejected() {
    let json = sheet(&["2"]).to_json().unwrap();
    for invalid in [
        json.replace("\"source\":", "\"extra\": true, \"source\":"),
        json.replace("\"id\": 1", "\"extra\": true, \"id\": 1"),
        json.replace("\"version\": 2", "\"version\": 2, \"version\": 2"),
        json.replace(
            "\"kind\": \"math\"",
            "\"kind\": \"math\", \"kind\": \"math\"",
        ),
        json.replace("\"id\": 1", "\"id\": 18446744073709551615"),
    ] {
        assert!(Worksheet::from_json(&invalid).is_err(), "{invalid}");
    }
}

#[test]
fn read_only_file_and_invalid_utf8_are_handled_without_overwriting() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("read-only.quire.json");
    std::fs::write(&path, b"original").unwrap();
    let original_permissions = std::fs::metadata(&path).unwrap().permissions();
    let mut read_only = original_permissions.clone();
    read_only.set_readonly(true);
    std::fs::set_permissions(&path, read_only).unwrap();
    let save = Worksheet::default().save(&path);
    std::fs::set_permissions(&path, original_permissions).unwrap();
    assert!(save.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    std::fs::write(&path, [0xff, 0xfe]).unwrap();
    assert!(Worksheet::open(&path).is_err());
}

#[test]
fn sparse_high_region_ids_still_allow_adding_and_saving() {
    let json = sheet(&["1"])
        .to_json()
        .unwrap()
        .replace("\"id\": 1", "\"id\": 18446744073709551613");
    let mut worksheet = Worksheet::from_json(&json).unwrap();
    assert_eq!(worksheet.add_region(RegionContent::Text("New".into())), 1);
    assert_eq!(worksheet.add_region(RegionContent::Math("2".into())), 2);
    let reopened = Worksheet::from_json(&worksheet.to_json().unwrap()).unwrap();
    assert_eq!(reopened, worksheet);
}

#[test]
fn long_invalid_dependency_chain_has_clear_bounded_errors_and_recovers() {
    let mut worksheet = sheet(&["x := 1/0"]);
    for _ in 0..1000 {
        worksheet.add_region(RegionContent::Math("x := x + 1".into()));
    }
    for result in worksheet.calculate() {
        let error = result.unwrap().unwrap_err();
        assert!(error.0.len() < 128);
    }
    *worksheet.regions[0].content.source_mut() = "x := 0".into();
    assert_eq!(result(&worksheet, 1000).quantity.si_value, 1000.0);
}
