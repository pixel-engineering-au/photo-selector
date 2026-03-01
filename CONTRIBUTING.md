# Contributing to Photo Selector

Welcome. This document covers everything you need to understand
the codebase, make changes confidently, and get them merged.

---

## Table of Contents

1. [Project overview](#1-project-overview)
2. [Architecture](#2-architecture)
3. [Layer by layer](#3-layer-by-layer)
4. [How the layers communicate](#4-how-the-layers-communicate)
5. [Data flow walkthrough](#5-data-flow-walkthrough)
6. [Development setup](#6-development-setup)
7. [Running tests](#7-running-tests)
8. [How to add a new feature](#8-how-to-add-a-new-feature)
9. [How to fix a bug](#9-how-to-fix-a-bug)
10. [Code conventions](#10-code-conventions)
11. [What not to do](#11-what-not-to-do)
12. [Glossary](#12-glossary)

---

## 1. Project overview

Photo Selector is a desktop app for fast photo culling. The user
opens a folder, navigates through images, and marks each one as
kept or rejected. Files are physically moved into `selected/` or
`rejected/` subfolders. Nothing is deleted.

The app is built with:
- **Rust** for all business logic (the `core` crate)
- **Tauri v2** as the desktop shell (the `src-tauri` crate)
- **React + TypeScript** for the user interface (`src/`)
- **Zustand** for frontend state management

```
photo-selector/
  core/          Pure Rust business logic — no Tauri dependency
  src-tauri/     Tauri backend — bridges core to the WebView
  src/           React frontend
  cli/           Optional command-line interface (uses core directly)
```

---

## 2. Architecture

The architecture has three strict layers. **Each layer only talks
to the layer directly adjacent to it.** This is the most important
rule in the codebase.

```
┌─────────────────────────────────────┐
│            React Frontend           │
│   (components, Zustand store)       │
│                                     │
│  reads from store                   │
│  calls invoke() to send commands    │
└──────────────┬──────────────────────┘
               │  Tauri IPC bridge
               │  (invoke / emit)
┌──────────────▼──────────────────────┐
│          Tauri Backend              │
│   (commands.rs, state.rs, lib.rs)   │
│                                     │
│  receives commands from frontend    │
│  calls AppState methods             │
│  forwards AppEvents to frontend     │
└──────────────┬──────────────────────┘
               │  Rust function calls
┌──────────────▼──────────────────────┐
│          Core Library               │
│   (AppState, ImageIndex, etc.)      │
│                                     │
│  all business logic lives here      │
│  returns Vec<AppEvent> from methods │
│  knows nothing about Tauri or React │
└─────────────────────────────────────┘
```

### Why this matters

- **Core is independently testable.** You can run `cargo test -p
  photo-selector-core` without a WebView, without Node, without
  Tauri. All business logic tests live here.

- **Tauri is a thin bridge.** Commands lock the state, call core,
  and emit events. They contain no business logic themselves.

- **Frontend is purely reactive.** It never modifies state directly.
  It calls commands and waits for events. The Zustand store is
  updated entirely by incoming events from core.

---

## 3. Layer by layer

### 3.1 Core (`core/src/`)

The core library is a standard Rust crate with no Tauri dependency.
It can be used from the CLI, from tests, or from any future
interface without modification.

| File | Responsibility |
|------|---------------|
| `app_state.rs` | Main state machine. The single entry point for all operations. |
| `events.rs` | All `AppEvent` variants. Every state change is expressed here. |
| `image_index.rs` | Sorted list of discovered images. Handles scanning and sort orders. |
| `image_cache.rs` | Per-image metadata cache. Tracks `file_size`, `dimensions`, `load_state`. |
| `navigation.rs` | Page cursor. Tracks `current_index` and `view_count`, computes ranges. |
| `undo.rs` | Bounded LIFO stack of `UndoEntry` records. |
| `stats.rs` | Counts remaining/selected/rejected by reading subdirectory contents. |
| `file_ops.rs` | Filesystem moves. The only place that touches the actual files. |

**`AppState` is the only public interface you need to call.** All
other structs are internal implementation details that `AppState`
coordinates. External callers (Tauri commands, CLI) only call
`AppState` methods and receive `Vec<AppEvent>` back.

```rust
// Every AppState method follows this pattern:
pub fn next(&mut self) -> Vec<AppEvent> { ... }
pub fn load_dir(&mut self, dir: &Path) -> Vec<AppEvent> { ... }
pub fn act_on_current_at(&mut self, action: Action, view_index: usize)
    -> Result<Vec<AppEvent>, String> { ... }
```

### 3.2 Tauri backend (`src-tauri/src/`)

| File | Responsibility |
|------|---------------|
| `lib.rs` | App entry point. Registers state, plugins, and all commands. |
| `state.rs` | `TauriAppState` — wraps `AppState` in a `Mutex` for thread-safe sharing. |
| `commands.rs` | One `#[tauri::command]` function per `AppState` method. |

Every command follows the same three-step pattern without exception:

```rust
#[tauri::command]
pub fn next_page(
    state: State<'_, TauriAppState>,
    handle: AppHandle,
) -> Result<(), String> {
    // 1. Lock the state
    let events = state.0.lock()
        .map_err(|e| e.to_string())?
    // 2. Call core
        .next();
    // 3. Emit all resulting events to the frontend
    emit_all(&handle, events);
    Ok(())
}
```

No business logic lives in commands. If you find yourself writing
an `if` or a `match` inside a command (other than error handling),
that logic belongs in core instead.

### 3.3 Frontend (`src/`)

| Path | Responsibility |
|------|---------------|
| `main.tsx` | Mounts React. Starts the core event listener once at boot. |
| `store/events.ts` | TypeScript mirror of all Rust types that cross the bridge. |
| `store/appStore.ts` | Zustand store. Single source of truth. Updated only by events. |
| `hooks/useKeyboard.ts` | Keyboard shortcut bindings. Calls `invoke()` on key press. |
| `components/App.tsx` | Root layout. Sidebar + MainView split. Renders ScanOverlay. |
| `components/Sidebar.tsx` | Controls: open folder, stats, sort, view count, undo. |
| `components/MainView.tsx` | Routes between welcome / empty / image grid states. |
| `components/ImageGrid.tsx` | Renders the image tile grid. Handles hover and actions. |
| `components/NavBar.tsx` | Page counter and prev/next buttons. |
| `components/ScanOverlay.tsx` | Full-screen overlay shown during folder scanning. |

---

## 4. How the layers communicate

### Frontend → Tauri (commands)

The frontend calls Tauri commands using `invoke()`:

```typescript
import { invoke } from '@tauri-apps/api/core';

// No return value needed — results come back as events
await invoke('next_page');
await invoke('select_image', { viewIndex: 0 });
await invoke('set_sort_order', { order: { type: 'NameAsc' } });
```

Commands are fire-and-forget from the frontend's perspective.
Results always come back as events, never as return values
(except `get_metadata` which is a pure query with no side effects).

### Tauri → Frontend (events)

Core emits `AppEvent` values. Tauri serialises them and sends
them to the WebView on the `"core-event"` channel:

```rust
handle.emit("core-event", &event).ok();
```

The frontend listens on that channel in `main.tsx`:

```typescript
startCoreListener((event) => {
  useAppStore.getState().applyEvent(event);
});
```

### Serialisation shape

All Rust enums use `#[serde(tag = "type", content = "payload")]`
which means every event serialises as:

```json
{ "type": "PageChanged", "payload": { ...fields } }
```

Unit variants (no fields) serialise without a `payload` key:

```json
{ "type": "LibraryEmpty" }
```

The TypeScript types in `events.ts` mirror this exactly. **If you
add a new event variant in Rust, you must add a matching type in
`events.ts` and handle it in `appStore.ts`.**

### Sort order serialisation

`SortOrder` uses `#[serde(tag = "type")]` (without `content`),
so the frontend must send it wrapped:

```typescript
// Correct — matches serde(tag = "type") on SortOrder enum
invoke('set_sort_order', { order: { type: 'NameAsc' } });

// Wrong — Rust will reject this
invoke('set_sort_order', { order: 'NameAsc' });
```

---

## 5. Data flow walkthrough

Here is the complete journey of a user pressing the **Next** button:

```
1. User clicks "Next →" button in NavBar.tsx
   → onClick calls: await invoke('next_page')

2. Tauri routes 'next_page' to commands::next_page()
   → Locks TauriAppState Mutex
   → Calls AppState::next()

3. AppState::next() in core/src/app_state.rs
   → Calls NavigationEngine::next(total)
   → If navigation moved: builds PageState via build_page_state()
   → Returns Vec<AppEvent> containing PageChanged(page_state)
   → If already on last page: returns NavigationBoundary { kind: LastPage }

4. commands::next_page() receives Vec<AppEvent>
   → Calls emit_all(&handle, events)
   → Each event is serialised and sent to WebView on "core-event"

5. main.tsx event listener receives the event
   → Calls useAppStore.getState().applyEvent(event)

6. appStore.ts applyEvent() matches on event.type
   → case 'PageChanged': set({ page: event.payload })

7. React re-renders NavBar and ImageGrid
   → NavBar shows new page number
   → ImageGrid renders new image tiles
```

Every user action follows this same loop. There are no shortcuts.

---

## 6. Development setup

### Prerequisites

- Rust stable (`rustup install stable`)
- Node.js 18+ (`node --version`)
- Tauri CLI v2 (`cargo install tauri-cli --version "^2.0" --locked`)

**Linux only:**
```bash
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libappindicator3-dev \
  librsvg2-dev \
  patchelf
```

### First run

```bash
git clone https://github.com/yourname/photo-selector.git
cd photo-selector
npm install
cargo tauri dev
```

The app opens in a native window with hot reload. Rust changes
trigger a recompile; React changes update instantly via Vite HMR.

### Useful commands

```bash
# Run all Rust tests
cargo test --workspace

# Run only core tests
cargo test -p photo-selector-core

# Run a specific test by name
cargo test page_state_images_have_file_size

# Check Rust compiles without building
cargo check --workspace

# TypeScript type check
npx tsc --noEmit

# Build release bundle
cargo tauri build
```

---

## 7. Running tests

All business logic is tested in the core crate. Tests use
`tempfile` to create real temporary directories and write actual
files — there is no mocking.

```bash
cargo test -p photo-selector-core
```

Expected output:
```
running 40 tests
test image_cache::tests::file_size_populated_at_insert ... ok
test image_cache::tests::mark_failed_updates_load_state ... ok
test image_index::tests::default_sort_is_name_asc ... ok
...
test result: ok. 40 passed; 0 failed
```

**There are no frontend tests yet.** If you add them, put them
in `src/__tests__/` using Vitest.

---

## 8. How to add a new feature

Follow this checklist in order. Skipping steps causes the layers
to get out of sync.

### Step 1 — Define the new event(s) in core

Open `core/src/events.rs` and add a new variant to `AppEvent`:

```rust
/// Example: user requested a rating change
RatingChanged {
    path: PathBuf,
    rating: u8,   // 1–5 stars
},
```

### Step 2 — Implement the logic in core

Add a method to `AppState` in `core/src/app_state.rs`:

```rust
pub fn set_rating(&mut self, path: PathBuf, rating: u8)
    -> Vec<AppEvent>
{
    // ... business logic ...
    vec![AppEvent::RatingChanged { path, rating }]
}
```

**Write a test immediately** before moving to the next step:

```rust
#[test]
fn set_rating_emits_rating_changed() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.jpg"), "").unwrap();

    let mut app = AppState::new(1);
    app.load_dir(dir.path());

    let events = app.set_rating(
        dir.path().join("a.jpg"), 5
    );

    assert!(events.iter().any(|e| matches!(
        e, AppEvent::RatingChanged { rating: 5, .. }
    )));
}
```

Run `cargo test -p photo-selector-core` — it must pass before
you touch any other layer.

### Step 3 — Add the Tauri command

Add to `src-tauri/src/commands.rs`:

```rust
#[tauri::command]
pub fn set_rating(
    path: String,
    rating: u8,
    state: State<'_, TauriAppState>,
    handle: AppHandle,
) -> Result<(), String> {
    let events = state.0.lock()
        .map_err(|e| e.to_string())?
        .set_rating(PathBuf::from(path), rating);
    emit_all(&handle, events);
    Ok(())
}
```

Register it in `src-tauri/src/lib.rs`:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    commands::set_rating,   // ← add
])
```

### Step 4 — Add the TypeScript event type

Open `src/store/events.ts` and add the new variant:

```typescript
export type AppEvent =
  // ... existing variants ...
  | { type: 'RatingChanged'; payload: { path: string; rating: number } }
```

### Step 5 — Handle the event in the store

Open `src/store/appStore.ts` and add a case in `applyEvent`:

```typescript
case 'RatingChanged': {
  const { path, rating } = event.payload;
  // update whatever store state you added
  break;
}
```

### Step 6 — Add any new store state

If the new feature needs UI state, add it to the `AppStore`
interface and initialise it in `create()`:

```typescript
interface AppStore {
  // ... existing fields ...
  ratings: Record<string, number>;  // path → star rating
}

// in create():
ratings: {},
```

### Step 7 — Build the UI

Add the frontend component or update existing ones. Call the
new command using `invoke`:

```typescript
await invoke('set_rating', { path: image.path, rating: 5 });
```

### Step 8 — Update `README.md`

Add the new feature to the features list and keyboard shortcuts
table if applicable.

---

## 9. How to fix a bug

### Identify which layer the bug is in

**If the bug is in business logic** (wrong sort order, undo
not working, wrong file count) — the bug is in `core/`. Write
a failing test that reproduces it, then fix the code until the
test passes.

**If the bug is data not reaching the frontend** (event fires
in Rust but nothing happens in UI) — check the serialisation.
Add a `console.log` at the top of `applyEvent` in `appStore.ts`:

```typescript
applyEvent: (event: AppEvent) => {
  console.log('[event]', event.type, event);
  // ... rest unchanged
```

Open DevTools, reproduce the bug, and check what arrives.
Common causes:
- Event payload shape mismatch (Rust sends `payload.foo`,
  TypeScript reads `event.foo`)
- Missing `case` in `applyEvent` switch
- New event variant not added to `events.ts`

**If the bug is visual only** (layout broken, button not
responding, wrong colour) — the bug is in a React component.
Use React DevTools to inspect component state.

### Writing a regression test

For every bug fixed in core, add a test that would have caught
it. Name it descriptively:

```rust
#[test]
fn undo_after_last_image_does_not_panic() { ... }

#[test]
fn sort_order_preserved_after_undo() { ... }
```

---

## 10. Code conventions

### Rust

- All `AppState` methods return `Vec<AppEvent>` or
  `Result<Vec<AppEvent>, String>`. Never `void`.
- Business logic belongs in `core/`. Tauri commands are bridges
  only — no `if`, no `match` on values, no logic.
- Every new public function in core gets at least one test.
- Use `unwrap_or`, `unwrap_or_else`, `ok()` to handle errors
  gracefully rather than panicking in user-facing paths.
- New `AppEvent` variants must have `#[cfg_attr(feature = "tauri",
  derive(serde::Serialize, serde::Deserialize))]` on any
  new types they reference.

### TypeScript

- One `useAppStore(s => s.field)` selector per value — never
  return an object literal from a selector (causes infinite
  re-render loop).
- All store mutations happen inside `applyEvent`. Components
  never call `set()` directly.
- Keep components focused. If a component is over ~150 lines,
  split it.
- Use `async/await` for all `invoke()` calls.

### CSS

- All colours and spacing use CSS variables defined in
  `index.css`. Never hardcode hex values in component styles.
- The design uses `var(--font-mono)` for all data/metadata
  display and `var(--font-ui)` for controls and labels.

### Git

- Branch from `main` for all changes.
- Branch naming: `feat/description`, `fix/description`,
  `chore/description`.
- Commit messages: imperative mood, present tense.
  `Add rating feature` not `Added rating feature`.
- One logical change per commit.
- All tests must pass before opening a pull request.

---

## 11. What not to do

**Don't add business logic to Tauri commands.**
If you need to compute something, do it in core and return it
via an event. Commands are bridges.

**Don't call `invoke()` from inside the Zustand store.**
The store is updated by events only. If an event needs to
trigger a follow-up action, that follow-up belongs in the
component that dispatched the original command.

**Don't add background threads or workers without a clear
performance need.** The app had CPU issues from premature
optimisation. For JPEG/PNG, the browser renders images natively
and efficiently. Only add threading if you have measured a
real bottleneck.

**Don't return data from commands as the return value.**
All results come back as events on `"core-event"`. The only
exception is `get_metadata` which is a pure read with no side
effects.

**Don't use object literal selectors in Zustand.**

```typescript
// Wrong — creates new object every render → infinite loop
const { page, stats } = useAppStore(s => ({ page: s.page, stats: s.stats }));

// Correct — one selector per value
const page  = useAppStore(s => s.page);
const stats = useAppStore(s => s.stats);
```

**Don't mix `border` shorthand and `borderColor` in React
inline styles.** React's style diffing breaks when both are
present. Always use the full `border` property:

```typescript
// Wrong
border: '1px solid var(--border)',
borderColor: hovered ? 'var(--accent)' : undefined,

// Correct
border: hovered ? '1px solid var(--accent)' : '1px solid var(--border)',
```

---

## 12. Glossary

| Term | Meaning |
|------|---------|
| `AppState` | The main Rust struct that owns all session state. The only struct Tauri commands call directly. |
| `AppEvent` | A Rust enum variant describing one state change. All changes are expressed as events. |
| `PageState` | A snapshot of the current visible page — images, totals, page number. Sent to the frontend on every navigation. |
| `ImageEntry` | An entry in `ImageIndex` — path, filename, file size, date modified. Populated at scan time. |
| `Image` | An entry in `ImageCache` — extends `ImageEntry` with load state and optional metadata. Sent to frontend inside `PageState`. |
| `ImageLoadState` | Whether an image is `Pending`, `Ready`, or `Failed`. Always `Pending` in the current implementation (thumbnails not implemented). |
| `NavigationEngine` | Owns `current_index` and `view_count`. Computes page ranges. |
| `UndoStack` | Bounded LIFO stack (capacity 50) of `UndoEntry` records. Cleared on `load_dir`. |
| `TauriAppState` | Wraps `AppState` in a `Mutex<>` for safe sharing across Tauri command threads. |
| `applyEvent` | The Zustand store method that receives every `AppEvent` and updates UI state. The only place store mutations happen. |
| `emit_all` | Helper in `commands.rs` that iterates a `Vec<AppEvent>` and calls `handle.emit()` for each one. |
| `convertFileSrc` | Tauri API that converts a local filesystem path to a `tauri://` URL the WebView can load securely. |
| `invoke` | Tauri API for calling a Rust command from the frontend. |
| culling | The photography term for sorting through photos to keep the best ones. |
