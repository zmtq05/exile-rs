# AGENTS.md - Exile-rs Development Guide

## Project Overview
Tauri v2 desktop application with SvelteKit 5 frontend and Rust backend.
- **Frontend**: TypeScript, Svelte 5 (Runes), TailwindCSS, shadcn-svelte
- **Backend**: Rust with tauri-specta for type-safe TypeScript bindings
- **Architecture**: SPA mode using @sveltejs/adapter-static

## Build Commands

### Development
```bash
pnpm run dev              # Start SvelteKit dev server only
pnpm tauri dev            # Start full Tauri app with hot reload
```

### Production Build
```bash
pnpm run build            # Build frontend assets
pnpm tauri build          # Build Tauri application bundle
```

### Type Checking
```bash
pnpm run check            # Run svelte-check once
pnpm run check:watch      # Run svelte-check in watch mode
```

### Rust (run from src-tauri/)
```bash
cargo build               # Build Rust backend
cargo build --release     # Build optimized release
cargo clippy              # Lint Rust code
cargo clippy --fix        # Auto-fix Rust lints
cargo fmt                 # Format Rust code
cargo test                # Run Rust tests (none exist yet)
cargo test <test_name>    # Run single test by name
```

## Testing

**Current State**: No tests configured yet
- **Rust**: Use `#[cfg(test)]` modules or `/tests` directory
- **Frontend**: Consider Vitest or Playwright

**Run Single Test** (when exists): `cargo test <test_name>` or `cargo test <test_name> -- --exact`

## MCP Tools & Subagents

### Svelte MCP (@sveltejs/opencode)
- `svelte_list-sections`, `svelte_get-documentation` - Browse Svelte 5/SvelteKit docs
- `svelte_svelte-autofixer` - Validate Svelte code (USE before sending to user)
- `svelte_playground-link` - Generate playground links
- **Subagent**: `svelte-file-editor` - MUST USE when creating/editing .svelte files

### shadcn-svelte MCP
- `shadcn-svelte_shadcnSvelteSearchTool`, `shadcn-svelte_shadcnSvelteGetTool` - Search/get components
- `shadcn-svelte_shadcnSvelteIconsTool`, `shadcn-svelte_shadcnSvelteListTool` - Icons/list
- `shadcn-svelte_bitsUiGetTool` - Bits UI details (underlying library)

## Code Style

### TypeScript/Svelte

#### Imports
- Use `@/*` alias for lib imports: `import { commands } from "@/bindings"`
- Auto-generated bindings are at `@/bindings` (DO NOT manually edit)
- Prefer named imports over default imports
- Group imports: external packages → internal modules → types

#### Formatting
- TypeScript strict mode enabled (`strict: true`)
- Use Svelte 5 runes: `$state`, `$derived`, `$effect`
- Prefer `const` over `let` unless reassignment needed
- Use `type` for type definitions, `interface` for extensible contracts

#### Component Pattern
- Use Svelte 5 runes: `let value = $state("")`, `let computed = $derived(...)`
- TypeScript: `<script lang="ts">` with async/await for Tauri commands
- **CRITICAL**: Use `svelte-file-editor` subagent or `svelte_svelte-autofixer` before sending code

#### UI Components (shadcn-svelte)
- **ALWAYS use** shadcn-svelte from `@/components/ui/*` - use MCP tools to search first
- Install: `pnpm dlx shadcn-svelte@latest add <component-name>`
- Use `cn()` from `@/utils` for className merging
- **WARNING**: This is Svelte, NOT React - don't use `asChild` or React patterns
- DO NOT create custom Button, Input, Dialog when shadcn equivalent exists

### Rust

- Standard conventions: `cargo fmt`, `cargo clippy` before commits
- **Tauri Commands**: Use `#[tauri::command]` + `#[specta::specta]` (required for type generation)
- **Registration**: Add to `collect_commands![]` in `lib.rs` specta_builder
- Bindings auto-generate to `src/lib/bindings.ts` in debug mode

### Naming & Error Handling

**Naming**: Rust (snake_case/PascalCase), TypeScript (camelCase/PascalCase), Files (kebab-case .svelte)

**Errors**: 
- TypeScript: `try/catch` for Tauri commands, explicit error types
- Rust: `Result<T, E>` with `?` operator, custom error types over `String`

## Project Structure

```
exile-rs/
├── src/                          # Frontend source
│   ├── routes/                   # SvelteKit routes
│   │   ├── +page.svelte         # Main page
│   │   └── +layout.svelte       # Root layout
│   ├── lib/                      # Shared frontend code
│   │   ├── bindings.ts          # Auto-generated (DO NOT EDIT)
│   │   ├── utils.ts             # Utility functions
│   │   ├── components/ui/       # shadcn-svelte components
│   │   └── hooks/               # Custom Svelte hooks
│   ├── app.css                  # Global styles (Tailwind)
│   └── app.html                 # HTML template
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── lib.rs               # Main library (commands, setup)
│   │   └── main.rs              # Binary entry point
│   ├── Cargo.toml               # Rust dependencies
│   └── tauri.conf.json          # Tauri configuration
├── package.json                  # Frontend dependencies & scripts
├── svelte.config.js             # SvelteKit configuration
├── vite.config.js               # Vite bundler config
└── tsconfig.json                # TypeScript configuration
```

## Important Rules

**DO NOT**: Edit `src/lib/bindings.ts` (auto-generated), use `as any`, commit without `cargo fmt`/`clippy`

**DO**: Run `pnpm tauri dev` after Rust changes, use `@/*` alias, export TS types from Rust via specta

## Adding Features

**Tauri Command**: Define with `#[tauri::command]` + `#[specta::specta]` in `lib.rs`, add to `collect_commands![]`, run `pnpm tauri dev`

**UI Component**: Use shadcn-svelte MCP to search → install via `pnpm dlx shadcn-svelte@latest add <name>` → if custom needed, use `svelte-file-editor` subagent

**Route**: Create `+page.svelte` in `src/routes/<name>/`, optional `+page.ts` for load functions
