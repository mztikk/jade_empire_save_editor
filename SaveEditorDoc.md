# Jade Empire Save Editor

We're building a production-grade Save Editor for the game Jade Empire, for Linux, Windows and macOS. This is professional-grade software that needs to be stable, fully-tested and real-world deployable. No shortcuts - solid as a rock.

## Dev Workflow
- Ask questions only when absolutely necessary - work autonomously
- Start by asking question to determine application name. Suggest a few
- Testing is of the utmost importance - use lots of unit tests and run automated testing
- Do things properly, this is prodution-grade software and not a hack/toy project
- Create AGENTS.md and relevant skills to support development, and contain concrete development guidelines
- Empty git repository with a default .gitignore for rust has been created [here](https://github.com/mztikk/jade_empire_save_editor). Commit and push everything to there, IMPORTANT: do a code review before committing and make sure all changes comply with code style, production-grade quality standards and have been properly tested and have unit tests that have passed where necessary.

## Tech stack
- Rust
- [Freya](https://freyaui.dev/) for GUI

## Basic Architecture
- Use a cargo workspace
- Build as modular as possible and create separate crates

## Environment
- All Game files and binaries can be found at `/media/Games/Heroic/Jade Empire/` if needed for examining save data algorithms or similar.
- Savegame files can be found at `/media/Games/Heroic/Jade Empire/Save/`