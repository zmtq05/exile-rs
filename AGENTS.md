# AGENTS.md - Exile-rs Development Guide

## Project Overview
Tauri v2 desktop app for managing Path of Exile tools (PoB manager, timers, etc.)
- **Frontend**: TypeScript, Svelte 5 (Runes), TailwindCSS 4, shadcn-svelte
- **Backend**: Rust (stable, edition 2024) with tauri-specta for type-safe bindings
- **Architecture**: SPA mode using @sveltejs/adapter-static

## Build & Development

```bash
# Development
pnpm tauri dev              # Full app with hot reload

# Frontend only
pnpm run dev                # SvelteKit dev server
pnpm run check              # svelte-check (type check)

# Rust (from src-tauri/)
cargo build                 # Debug build
cargo clippy                # Lint (MUST pass before commit)
cargo fmt                   # Format (MUST run before commit)
cargo test                  # Run all tests
cargo test <name>           # Single test: cargo test test_parse_version
cargo test <name> -- --exact  # Exact match
```

## CI/CD (GitHub Actions)

### CI (`.github/workflows/ci.yml`)
Triggers on PR/push to main:
- `cargo fmt --check` + `cargo clippy` (Rust lint)
- `pnpm run check` (svelte-check)
- `pnpm tauri build` (full build verification)

### Release (`.github/workflows/release.yml`)
Triggers on `v*` tag push:
1. **Version check**: Tag must match `Cargo.toml`, `package.json`, `tauri.conf.json`
2. Build Windows installer (.msi, .exe)
3. Create draft GitHub release

```bash
# Release workflow
# 1. Update versions in all 3 files
# 2. Commit and tag
git tag v0.2.0 && git push origin v0.2.0
```

## Code Style

### TypeScript/Svelte

**Imports** (order: external → internal → types):
```typescript
import { onMount } from "svelte";
import { commands, type ErrorKind } from "@/bindings";
import { Button } from "@/components/ui/button";
```

**Svelte 5 Runes**:
```svelte
<script lang="ts">
  let value = $state("");
  let computed = $derived(value.toUpperCase());
  let isValid = $derived(value.length > 0);
</script>
```

**Error Handling** (commands return `{ status: "ok" | "error", data?, error? }`):
```typescript
const result = await commands.installPob(fileData);
if (result.status === "error") {
  if (result.error.kind === "cancelled") return; // Silent ignore
  error = { kind: result.error.kind, message: result.error.message };
}
```

**shadcn-svelte**:
- ALWAYS use from `@/components/ui/*` - search with MCP tools first
- Install: `pnpm dlx shadcn-svelte@latest add <component>`
- Use `cn()` from `@/utils` for className merging
- **WARNING**: Svelte, NOT React - no `asChild` prop

### Rust

**Tauri Commands** (MUST have both attributes):
```rust
#[tauri::command]
#[specta::specta]
pub async fn my_command(manager: State<'_, PobManager>) -> Result<T, ErrorKind> {
    Ok(manager.do_something().await?)
}
```

**Error Types** (two-layer system):
```rust
// Internal domain errors (pob/error.rs) - use thiserror
#[derive(Debug, thiserror::Error)]
pub enum PobError {
    #[error("User cancelled")]
    Cancelled,
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}

// IPC errors (errors.rs) - user-facing categories
#[derive(Serialize, Type)]
#[serde(tag = "kind", content = "message")]
pub enum ErrorKind {
    Cancelled,           // No message, UI ignores
    Network(String),     // Retry may help
    Io(String),          // Filesystem issues
    NotFound(String),    // Resource missing
    Conflict(String),    // e.g., PoB is running
    Domain(String),      // Other errors
}
```

**Tracing** (structured logging):
```rust
tracing::info!(
    phase = "download",
    operation = "start",
    file = %file_name,
    "Starting download"
);
```

**Registration**: Add commands to `collect_commands![]` in `lib.rs`

## Project Structure

```
exile-rs/
├── src/                          # Frontend
│   ├── routes/                   # SvelteKit pages
│   ├── lib/
│   │   ├── bindings.ts          # Auto-generated (DO NOT EDIT)
│   │   ├── components/ui/       # shadcn-svelte
│   │   └── utils.ts             # cn() helper
│   ├── app.css                  # Tailwind + theme
│   └── app.html                 # Root HTML (dark mode enabled)
├── src-tauri/                    # Backend
│   ├── src/
│   │   ├── lib.rs               # App setup, command registration
│   │   ├── commands.rs          # Tauri IPC commands (thin adapter)
│   │   ├── errors.rs            # ErrorKind for frontend
│   │   └── pob/                 # PoB domain module
│   │       ├── manager.rs       # PobManager (install/uninstall logic)
│   │       ├── error.rs         # PobError (domain errors)
│   │       ├── progress.rs      # ProgressSink trait, InstallReporter
│   │       └── version.rs       # Version parsing
│   └── tauri.conf.json          # Tauri config
├── .github/workflows/           # CI/CD
│   ├── ci.yml                   # Lint + build on PR
│   └── release.yml              # Release on v* tag
└── AGENTS.md                    # This file
```

## MCP Tools for Agents

**Svelte** (`@sveltejs/opencode`):
- `svelte_svelte-autofixer` - MUST validate code before sending to user
- `svelte-file-editor` subagent - MUST USE for .svelte files

**shadcn-svelte**:
- `shadcn-svelte_shadcnSvelteGetTool` - Get component docs
- `shadcn-svelte_shadcnSvelteSearchTool` - Search components

## Important Rules

**DO NOT**:
- Edit `src/lib/bindings.ts` (auto-generated by specta)
- Use `as any`, `@ts-ignore`, `@ts-expect-error`
- Commit without `cargo fmt` + `cargo clippy`
- Create custom UI components when shadcn equivalent exists

**DO**:
- Run `pnpm tauri dev` after Rust changes (regenerates bindings)
- Use `@/*` alias for imports
- Follow two-layer error pattern (PobError → ErrorKind)
- Add structured fields to tracing logs

## Adding Features

**Tauri Command**:
1. Define in `commands.rs` with `#[tauri::command]` + `#[specta::specta]`
2. Add to `collect_commands![]` in `lib.rs`
3. Run `pnpm tauri dev` to regenerate bindings

**UI Component**:
1. Search shadcn-svelte MCP first
2. Install: `pnpm dlx shadcn-svelte@latest add <name>`
3. If custom needed, use `svelte-file-editor` subagent
