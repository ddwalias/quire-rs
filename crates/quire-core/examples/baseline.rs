//! Small repeatable core baseline, not a cross-machine performance budget.
use quire_core::{RegionContent, Worksheet};
use std::{hint::black_box, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut worksheet = Worksheet::new("1000-region length chain");
    worksheet.add_region(RegionContent::Math("x := 25 cm".into()));
    for _ in 0..998 {
        worksheet.add_region(RegionContent::Math("x := x + 1 mm".into()));
    }
    worksheet.add_region(RegionContent::Math("x -> cm".into()));
    let start = Instant::now();
    let first = worksheet.calculate();
    let first_time = start.elapsed();
    let first_value = first.last().unwrap().as_ref().unwrap().as_ref().unwrap();
    assert!((first_value.quantity.si_value - 1.248).abs() < 1e-12);
    *worksheet.regions[0].content.source_mut() = "x := 50 cm".into();
    let start = Instant::now();
    let recalculated = worksheet.calculate();
    let recalculate_time = start.elapsed();
    assert!(
        (recalculated
            .last()
            .unwrap()
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .quantity
            .si_value
            - 1.498)
            .abs()
            < 1e-12
    );
    let start = Instant::now();
    for _ in 0..100 {
        black_box(worksheet.calculate());
    }
    let average = start.elapsed() / 100;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("baseline.quire.json");
    let start = Instant::now();
    worksheet.save(&path)?;
    let save_time = start.elapsed();
    let start = Instant::now();
    let reopened = Worksheet::open(&path)?;
    let reopen_time = start.elapsed();
    assert_eq!(reopened, worksheet);
    println!(
        "1000 regions: first={first_time:?}, edit/recalculate={recalculate_time:?}, mean of 100 recalculations={average:?}, save={save_time:?}, reopen={reopen_time:?}"
    );
    Ok(())
}
