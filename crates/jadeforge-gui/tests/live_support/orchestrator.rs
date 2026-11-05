//! The scenario runner for the windowed UI suite.
//!
//! One process cannot safely create a second winit event loop (and on macOS
//! the loop must own the main thread), so the suite runs one scenario per child
//! process. This binary is its own orchestrator: with `JADEFORGE_LIVE=1` it
//! builds a synthetic savegame and game installation, then re-executes itself
//! once per scenario with `JADEFORGE_LIVE_SCENARIO` set. Without the opt-in it
//! is a no-op, which keeps `cargo nextest run --workspace` from opening windows
//! on a developer's desktop.

use std::path::PathBuf;
use std::process::{Command, ExitCode, ExitStatus};
use std::time::{Duration, Instant};

use jadeforge_gui::app::{app_with_directory, window_config};

use super::live_app_id;
use super::probe::Probe;
use super::scenarios::{SCENARIOS, run as run_scenario};

/// Timeout for one scenario child, including the initial render.
const SCENARIO_TIMEOUT: Duration = Duration::from_secs(120);

/// Entry point for the `live` test target.
pub fn run() -> ExitCode {
    if std::env::args().any(|argument| argument == "--list") {
        // Nextest may enumerate harness-less targets. One scenario runner is
        // one test binary; the individual names are reported by the runner.
        println!("windowed: test");
        return ExitCode::SUCCESS;
    }
    if std::env::var_os("JADEFORGE_LIVE_SCENARIO").is_some() {
        return run_child();
    }
    if !flag("JADEFORGE_LIVE") {
        println!(
            "skipping the windowed UI tests; run `just live` (or set JADEFORGE_LIVE=1) to open real windows"
        );
        return ExitCode::SUCCESS;
    }
    let display = std::env::var_os("WAYLAND_DISPLAY").filter(|value| !value.is_empty());
    if display.is_none() {
        if flag("JADEFORGE_LIVE_REQUIRE") {
            eprintln!(
                "the windowed UI tests are required (JADEFORGE_LIVE_REQUIRE=1) but WAYLAND_DISPLAY is not set"
            );
            return ExitCode::FAILURE;
        }
        println!("skipping the windowed UI tests: WAYLAND_DISPLAY is not set");
        return ExitCode::SUCCESS;
    }
    orchestrate()
}

fn run_child() -> ExitCode {
    let Some(name) = std::env::var_os("JADEFORGE_LIVE_SCENARIO") else {
        eprintln!("JADEFORGE_LIVE_SCENARIO is missing");
        return ExitCode::FAILURE;
    };
    let name = name.to_string_lossy().into_owned();
    let Some(directory) = std::env::var_os("JADEFORGE_SAVE_DIR") else {
        eprintln!("JADEFORGE_SAVE_DIR is missing for scenario {name}");
        return ExitCode::FAILURE;
    };
    let directory = PathBuf::from(directory);
    let probe = Probe::new();
    let plugin = probe.plugin();
    let test_probe = probe.clone();
    let scenario = name.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_scenario(&scenario, &test_probe)
        }));
        let outcome = match outcome {
            Ok(result) => result,
            Err(payload) => Err(panic_message(payload.as_ref())),
        };
        // Always close the window, even when the scenario asserted. This
        // bypasses the unsaved-changes dialog on purpose.
        test_probe.close();
        let _ = sender.send(outcome);
    });

    let start = directory;
    let config = freya::prelude::LaunchConfig::new()
        .with_window(
            window_config(move || app_with_directory(start.clone())).with_app_id(live_app_id()),
        )
        .with_plugin(plugin);
    freya_winit::launch(config);

    let outcome = receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap_or_else(|_| Err("the window closed before the scenario finished".to_owned()));
    let _ = worker.join();
    match outcome {
        Ok(()) => {
            println!("ok {name}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("FAILED {name}: {error}");
            ExitCode::FAILURE
        }
    }
}

fn orchestrate() -> ExitCode {
    let root = unique_root();
    let Some((_, game)) = crate::support::synthetic(&root) else {
        eprintln!(
            "cannot build the synthetic savegame installation under {}",
            root.display()
        );
        return ExitCode::FAILURE;
    };
    let saves = game.join("Save");
    let artifacts = workspace_target().join("live-ui");
    let _ = std::fs::create_dir_all(&artifacts);
    let executable = match std::env::current_exe() {
        Ok(executable) => executable,
        Err(error) => {
            eprintln!("cannot locate the test executable: {error}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "windowed UI tests on Wayland ({}), {} scenario(s), artifacts in {}",
        std::env::var("WAYLAND_DISPLAY").unwrap_or_default(),
        SCENARIOS.len(),
        artifacts.display()
    );
    let mut failures = Vec::new();
    for (name, _) in SCENARIOS {
        println!("-- {name}");
        let config = root.join("config").join(name);
        let mut command = Command::new(&executable);
        command
            .env("JADEFORGE_LIVE", "1")
            .env("JADEFORGE_LIVE_SCENARIO", name)
            .env("JADEFORGE_SAVE_DIR", &saves)
            .env("JADEFORGE_GAME_DIR", &game)
            .env("JADEFORGE_CONFIG_DIR", &config)
            .env("JADEFORGE_LIVE_ARTIFACTS", &artifacts);
        match spawn_and_wait(&mut command, SCENARIO_TIMEOUT) {
            Ok(status) if status.success() => {}
            Ok(status) => failures.push((*name, format!("child exited with {status}"))),
            Err(error) => failures.push((*name, error)),
        }
    }
    let _ = std::fs::remove_dir_all(&root);
    if failures.is_empty() {
        println!("all {} windowed scenarios passed", SCENARIOS.len());
        ExitCode::SUCCESS
    } else {
        for (name, error) in &failures {
            eprintln!("FAILED {name}: {error}");
        }
        ExitCode::FAILURE
    }
}

fn spawn_and_wait(command: &mut Command, timeout: Duration) -> Result<ExitStatus, String> {
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot start the scenario child: {error}"))?;
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "timed out after {} s; the window did not close",
                    timeout.as_secs()
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("cannot wait for the scenario child: {error}"));
            }
        }
    }
}

fn unique_root() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    std::env::temp_dir().join(format!("jadeforge-windowed-{}-{nanos}", std::process::id()))
}

fn workspace_target() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target")
}

fn flag(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| {
        matches!(
            value.to_string_lossy().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes"
        )
    })
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "scenario panicked".to_owned()
    }
}
