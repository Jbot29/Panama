# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

Panama is a desktop spaced repetition learning app built in Rust using egui/eframe. The core loop: SRS flashcard review to retain knowledge, AI tutors to learn new things via Socratic dialogue, and quizzes to find gaps — each feeding the others. AI tutors are powered by the Anthropic API (claude-haiku-4-5-20251001 via `ureq`).

## Commands

```bash
cargo run -r          # build and run (release mode)
cargo build -r        # build only
cargo check           # fast type-check without building
```

No test suite exists. `cargo check` is the fastest way to validate changes.

## Architecture

### Views

The `View` enum in `app_structs.rs` drives which panel renders in the central area. The left sidebar is always visible.

| View | File | Purpose |
|---|---|---|
| `Base` | — | Splash screen |
| `Cards` | `ui_cards.rs` | Hub: due count, Review / New Card buttons |
| `Review` | `ui_cards.rs` | SRS flashcard review |
| `NewCard` | `ui_cards.rs` | Create card with drawing canvas + German helper chat |
| `EditCard` | `ui_cards.rs` | Edit existing card; auto-saves image on Save |
| `Tutors` | `ui_tutors.rs` | List of available tutors |
| `CreateTutor` | `ui_create_tutor.rs` | AI-generate a new tutor config |
| `TutorDetail` | `ui_tutor_detail.rs` | Node list, prereqs, quiz generation per node |
| `TutorSession` | `ui_tutor_session.rs` | Active Socratic dialogue with Claude |
| `Quiz` | `ui_quiz.rs` | Multiple-choice quiz driven by node quiz files |

### State

All runtime state lives on `MyApp` in `app.rs`. The struct owns the SQLite connection, current card, textures, audio player, drawing strokes, and all tutor/quiz/chat state. There is no sub-state — everything is flat fields on `MyApp`.

### Flashcard Loop

`DBCard::random_next` selects a random due card (due_date ≤ now, hidden = 0). The user rates 1–4; `MyApp::on_card_response` runs FSRS to compute the next review date and writes it back via `DBCardUpdateSRS`. Setting `refresh_card = true` triggers `load_next_card` on the next frame.

The FSRS `Card` struct is serialized as JSON into the `scores` TEXT column. `due_date` is a Unix timestamp (i64).

### Drawing / Annotation

Cards have a 500×320 drawing canvas. `DrawStroke` holds points, width, and a base `Color32` plus a `value` (0–1 brightness). The tinted color is computed at render time via `ui::value_tinted(color, value)` — strokes store the base color, never the pre-tinted version.

`export_front_image(id)` composites strokes onto the existing PNG (or blank canvas) using `imageproc`. It is called automatically when saving a card — there is no separate "Save Image" button. Images live at `media/images/front_{id}.png`; a JSON sidecar at `media/images/front_{id}.json` holds the raw strokes for re-editing.

### AI Tutor System

All tutor logic lives in `tutor.rs`. Tutors are fully data-driven — each is a directory under `tutors/` with a `config.toml`. Current tutors: `deutsch-c1`, `math-probability`, `kreativitaet`.

**Config structure (`tutors/<slug>/config.toml`):**
- `friendly_name` — display name
- `system_prompt` — sent to Claude; supports `{node_name}` and `{node_description}` placeholders
- `[[nodes]]` — seed topics, each with `name`, `description`, and optional `quiz_file`

**Adding a new tutor:** create `tutors/<slug>/config.toml` and it appears in the Tutors list automatically.

**Node/edge tables** (`nodes`, `edges` in `nodes.rs`) — unified across all tutors via `tutor_slug`. Each node has `mastery_score` (0.0–1.0), `times_reviewed`, `last_reviewed`, and `quiz_file`. Edges with `relationship = 'prerequisite'` are active: if a node has an unmastered prereq (score < 0.6), the prereq is studied first.

**`select_weakest_node`** — picks the row with lowest `mastery_score`, breaking ties by `last_reviewed ASC NULLS FIRST`, then checks prereqs.

**`ask_claude` / `ask_claude_raw`** — spawns a thread, POSTs to `https://api.anthropic.com/v1/messages`, sends result over `mpsc::channel`. `ask_claude_raw` takes an explicit system prompt and messages vec; `ask_claude` builds them from a `TutorNode` + system prompt template.

**Async polling** — each async operation has a dedicated `poll_*` method called every frame in `update()` via `try_recv()`. Calls `ctx.request_repaint_after(100ms)` while loading. Active pollers: `poll_tutor`, `poll_diagram`, `poll_create_tutor`, `poll_card_chat`, `poll_detail_quiz`.

**Mastery updates** — "Got it ✓" sends +0.15 delta, "Still struggling ↓" sends -0.1. After rating, immediately picks the new weakest node and fires a fresh kickoff message.

When building `api_messages`, `"system"` role messages are filtered out — they exist only for local display (amber color, `→` prefix).

### Create Tutor Flow

`ui_create_tutor.rs` — Subject + Context inputs → fires `ask_claude_raw` with a curriculum-designer system prompt → parses JSON response into `GeneratedTutor` (friendly_name, slug, system_prompt, nodes) → previews the result → on Save writes `tutors/<slug>/config.toml` and reloads the tutor list.

`slugify` converts the friendly name to a kebab-case directory name.

### Tutor Detail View

`ui_tutor_detail.rs` — Two-column layout:
- **Left**: node list sorted weakest-first, colored mastery dot + `[id] name` + score%
- **Right**: selected node detail — mastery bar, prereq list with remove (×), add prereq by name or numeric ID, Study This Now (pins node for next session), Mark Struggling (−0.1 delta), and quiz generation

**Pinning a node**: `tutor_pinned_node_id` is checked in `init_tutor_session` before falling back to `select_weakest_node`.

### Per-Node Quiz Generation

From Tutor Detail, clicking "Generate Quiz" on a node fires `ask_claude_raw` with a prompt asking for 6 multiple-choice questions as JSON. `poll_detail_quiz` receives the response, calls `quiz::parse_quiz_response`, writes the TOML to `tutors/{slug}/quizzes/{node_id}.toml`, and sets `quiz_file = "{node_id}"` on the node in the DB. "Regenerate Quiz" overwrites the existing file.

Quiz TOML format:
```toml
topic = "Node Name"

[[questions]]
question = "Question text?"
choices = ["A", "B", "C", "D"]
correct = 0  # 0-based index
```

### Quiz View

`ui_quiz.rs` — Tutor-aware. Selecting a tutor calls `select_weakest_quiz_node` (picks the node whose most recent quiz session has the lowest score, NULLS FIRST). After completing a quiz, `save_quiz_session` records the result and `update_node_mastery` adjusts the node's score by `(score - 0.5) * 0.3` — so 100% adds +0.15, 0% subtracts −0.15.

Quiz files are loaded from `tutors/{slug}/quizzes/{quiz_file}.toml`.

### Chat Commands

If the user's input starts with `/`, `process_chat_command(input, tutor_slug)` in `app.rs` handles it. The result is displayed as a `"system"` role message.

| Command | Description |
|---|---|
| `/new-card "front" "back"` | Insert a flashcard into the SRS deck |
| `/flag "topic" ["description"]` | Add a node at mastery 0.3, surfaces it quickly |
| `/nodes [search]` | List nodes for the current tutor; filter by name |
| `/prereq "child" "parent"` | Mark parent as prerequisite of child (names or bare integer IDs) |
| `/diagram` | Generate an SVG diagram for the current node via Claude |
| `/summary` | Summarize the session and suggest flashcards |
| `/help` | List commands |

`parse_quoted_args` extracts `"..."` substrings. `/prereq` also accepts bare integers (node IDs from `/nodes`).

### Markdown Rendering

Assistant messages render through `egui_commonmark::CommonMarkViewer`. A single `CommonMarkCache` lives on `MyApp`. System prompts instruct Claude to use Markdown and Unicode math (never LaTeX `$$...$$`).

### New Card Helper Chat

The New Card view has a split layout: left = German cloze card assistant chat (backed by `ask_claude_raw`, polled by `poll_card_chat`), right = card form + drawing canvas. The chat is throwaway — cleared when leaving the view.

### Database

Single SQLite file (`srs.db`). Path comes from `config::db_path()`. Opened once in `MyApp::new_with_db`; the `Connection` lives on `MyApp` for the lifetime of the app.

All table creation is idempotent (`CREATE TABLE IF NOT EXISTS`). Tables: `cards`, `nodes`, `edges`, `quiz_sessions`.

### Paths (portable, derived from a base dir)

All data paths derive from a single base directory resolved once at startup in `src/config.rs`. `base_dir()` walks up from the executable until it finds the folder containing `tutors/` — so a distributed build finds data sitting next to the binary, and a `cargo run` dev build walks up from `target/release/` to the repo root. Falls back to the current dir. No machine-specific absolute paths; to relocate data later (env var, OS data dir) only `resolve_base_dir()` changes.

Helpers, all relative to `base_dir()`:
- `config::db_path()` → `srs.db`
- `config::media_images()` → `media/images/front_{id}.png` (+ `.json` stroke sidecar), `back_{id}.png`
- `config::media_audio()` → `media/audio/back_{id}.mp3`
- `config::tutors_dir()` → `tutors/`

No machine-specific absolute paths remain in `src/`. (`View::Base` is a plain heading; the old splash-image feature has been removed.)

### Keyboard Shortcuts (Review view only)

`1–4` = Again/Hard/Good/Easy, `SPACE` = flip, `H` = hide, `E` = edit, `P` = play audio
