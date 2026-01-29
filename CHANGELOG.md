# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-01-29

### Added
- Initial release
- **PoB Manager**: Install and manage Korean Path of Building
  - Automatic version detection from DC Inside gallery
  - Download progress tracking
  - Process conflict detection
  - Uninstallation support
- Dark mode UI with shadcn-svelte components
- Auto-update support via Tauri updater
- Windows installer (.msi, .exe)

### Technical
- Tauri v2 desktop application
- Frontend: TypeScript, Svelte 5 (Runes), TailwindCSS 4
- Backend: Rust with type-safe IPC via tauri-specta
- CI/CD: Automated lint, test, and release workflows

[0.1.0]: https://github.com/zmtq05/exile-rs/releases/tag/v0.1.0
