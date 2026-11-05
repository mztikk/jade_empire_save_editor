//! The windowed scenarios.
//!
//! Each scenario drives the real application window: the save list, tab
//! switching, real typing in the item filter and (on the KWin backend) a
//! compositor screenshot. They are deliberately small and independent; the
//! exhaustive layout and behaviour coverage stays in the fast headless suite.

use std::path::PathBuf;
use std::time::Duration;

use super::driver::{Driver, InputMode};
use super::probe::{NodeKind, Probe};

/// Default timeout for one UI transition.
pub const STEP: Duration = Duration::from_secs(10);

/// One named scenario.
pub type Scenario = fn(&mut Driver) -> Result<(), String>;

/// The scenarios the orchestrator runs, in order.
pub const SCENARIOS: &[(&str, Scenario)] = &[
    ("window_opens_on_wayland", window_opens_on_wayland),
    ("save_opens_and_tabs_switch", save_opens_and_tabs_switch),
    (
        "typing_filters_the_item_database",
        typing_filters_the_item_database,
    ),
    (
        "the_window_is_really_presented",
        the_window_is_really_presented,
    ),
];

/// Runs one scenario against a live window.
pub fn run(name: &str, probe: &Probe) -> Result<(), String> {
    let mode = match std::env::var("JADEFORGE_LIVE_INPUT").as_deref() {
        Ok("plugin") => Some(InputMode::Plugin),
        Ok("kwin") => Some(InputMode::KWin),
        _ => None,
    };
    let mut driver = Driver::attach_with_fallback(probe.clone(), mode, Duration::from_secs(20))?;
    println!("  input backend: {:?}", driver.input_mode());
    let (_, scenario) = SCENARIOS
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .ok_or_else(|| format!("unknown scenario {name:?}"))?;
    scenario(&mut driver)
}

fn window_opens_on_wayland(driver: &mut Driver) -> Result<(), String> {
    let window = driver.window();
    if window.title != jadeforge_gui::app::WINDOW_TITLE {
        return Err(format!("unexpected window title {:?}", window.title));
    }
    if !window.wayland {
        return Err("the window is not a Wayland window".to_owned());
    }
    if (window.width, window.height) != (1180, 760) {
        return Err(format!(
            "the production window is {}x{}, expected 1180x760",
            window.width, window.height
        ));
    }
    if window.frames == 0 {
        return Err("the window never rendered a frame".to_owned());
    }
    driver.wait_for_text(jadeforge_gui::app::WINDOW_TITLE, STEP)?;
    let save = driver.wait_for_text("Level ", STEP)?;
    println!(
        "  window {}x{} at scale {}, {} frame(s); first save row: {:?}",
        window.width, window.height, window.scale, window.frames, save.text
    );
    Ok(())
}

fn save_opens_and_tabs_switch(driver: &mut Driver) -> Result<(), String> {
    open_first_save(driver)?;
    for (tab, marker) in [
        ("Character", "Identity"),
        ("Styles", "Fighting styles"),
        ("Inventory", "ADD AN ITEM"),
        ("Gems", "Add a gem"),
        ("Companions", " known"),
        ("Overview", "At a glance"),
    ] {
        driver.click_exact(tab, STEP)?;
        let marker = driver.wait_for_text(marker, STEP).map_err(|error| {
            format!("the {tab} tab did not show {marker:?} after a real click: {error}")
        })?;
        println!("  {tab}: {:?}", marker.text);
    }
    Ok(())
}

fn typing_filters_the_item_database(driver: &mut Driver) -> Result<(), String> {
    open_first_save(driver)?;
    driver.click_exact("Inventory", STEP)?;
    driver.wait_for_text("ADD AN ITEM", STEP)?;
    let filter = driver.wait_for_text("Filter the item database", STEP)?;
    if filter.kind != NodeKind::Paragraph {
        return Err("the item filter is not a paragraph input".to_owned());
    }
    driver.click(&filter)?;
    driver.type_text("Viper")?;
    let result = driver
        .wait_for_text("weapons · #42", STEP)
        .map_err(|error| {
            format!("typing \"Viper\" into the real window did not filter the item list: {error}")
        })?;
    println!("  filtered result: {:?}", result.text);
    Ok(())
}

fn the_window_is_really_presented(driver: &mut Driver) -> Result<(), String> {
    if driver.input_mode() == InputMode::Plugin {
        println!("  skipped: compositor screenshots need the KWin input backend");
        return Ok(());
    }
    open_first_save(driver)?;
    driver.click_exact("Inventory", STEP)?;
    driver.wait_for_text("ADD AN ITEM", STEP)?;
    let path = artifact_path("presented.png");
    let (width, height) = driver.screenshot(&path)?;
    let window = driver.window();
    let expected = (window.width, window.height);
    if (width, height) != expected {
        return Err(format!(
            "the compositor screenshot is {width}x{height}, expected the {}x{} client area; wrote {}",
            expected.0,
            expected.1,
            path.display()
        ));
    }
    let image = image::open(&path)
        .map_err(|error| format!("cannot decode {}: {error}", path.display()))?
        .to_rgba8();
    let colors: std::collections::HashSet<[u8; 4]> =
        image.pixels().step_by(37).map(|pixel| pixel.0).collect();
    if colors.len() < 32 {
        return Err(format!(
            "the compositor screenshot looks blank ({} colours); wrote {}",
            colors.len(),
            path.display()
        ));
    }
    println!(
        "  captured the real window: {width}x{height}, {} colours, {}",
        colors.len(),
        path.display()
    );
    Ok(())
}

fn open_first_save(driver: &mut Driver) -> Result<(), String> {
    let row = driver.wait_for_text("Level ", STEP)?;
    driver.click(&row)?;
    driver.wait_for_text("At a glance", STEP).map_err(|error| {
        format!(
            "clicking the save row {:?} did not open the savegame: {error}",
            row.text
        )
    })?;
    Ok(())
}

fn artifact_path(name: &str) -> PathBuf {
    let root = std::env::var_os("JADEFORGE_LIVE_ARTIFACTS").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/live-ui"),
        PathBuf::from,
    );
    root.join(name)
}
