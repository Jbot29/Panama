# Panama

A desktop spaced-repetition learning app, built in Rust with [egui](https://github.com/emilk/egui)/eframe.

Panama combines three learning tools into one loop, each feeding the others:

- **Flashcards** — an SRS deck (powered by [FSRS](https://github.com/open-spaced-repetition)) to retain what you've learned.
- **AI Tutors** — Socratic dialogue with Claude to learn new material, organized as a graph of topic "nodes" with prerequisites and per-node mastery tracking.
- **Quizzes** — multiple-choice quizzes generated per topic to surface gaps, which then feed back into what the tutor focuses on next.

## Goal

Most study tools do one thing. Panama's idea is that retention, learning, and gap-finding are the same loop: a tutor teaches you something, a quiz reveals what didn't stick, and a flashcard locks it in — and the app routes your attention to whatever is weakest across all three.

## How It Works (high level)

- **State** — All runtime state lives on a single `MyApp` struct. Data persists to a local SQLite database (`srs.db`); there is no server or cloud component.
- **Views** — A `View` enum drives which panel renders (Cards, Review, Tutors, Tutor Detail, Tutor Session, Quiz, …) alongside an always-visible sidebar.
- **Tutors are data-driven** — Each tutor is a directory under `tutors/<slug>/` with a `config.toml` (a friendly name, a system prompt, and seed topic nodes). Drop in a new config and it appears in the app automatically. You can also generate a new tutor from a subject + context prompt.
- **Mastery & prerequisites** — Each topic node tracks a mastery score (0.0–1.0). The app picks the weakest node to study next, and won't move on to a topic until its prerequisites are mastered.
- **AI** — Tutor dialogue, quiz generation, and diagrams are powered by the [Anthropic API](https://docs.anthropic.com/) (Claude). Requests are made directly from the app over HTTPS.
- **Flashcards** — Cards support a drawing canvas and optional audio. Reviews run FSRS to schedule the next due date.

## Running

Panama is a Rust project. With a [Rust toolchain](https://rustup.rs/) installed:

```bash
cargo run -r       # build and run (release)
cargo build -r     # build only
cargo check        # fast type-check
```

### Anthropic API key

The AI features require an Anthropic API key. Set it in your environment before launching:

```bash
export ANTHROPIC_API_KEY=sk-ant-...
cargo run -r
```

Or place it in a `.env` file at the project root (this file is gitignored):

```
ANTHROPIC_API_KEY=sk-ant-...
```

Flashcards and review work without a key; only the AI tutor, quiz-generation, and diagram features need one.

### Data

All data lives next to the app: the SQLite database (`srs.db`), tutor configs (`tutors/`), and user media (`media/`). Nothing is uploaded anywhere except API requests to Anthropic.

## License

Released under the [MIT License](LICENSE).

## Disclaimer

This software is provided "as is", without warranty of any kind, express or implied. It is a personal project shared in the hope that it's useful. The author accepts no responsibility or liability for any damages, data loss, or API costs arising from its use. You are responsible for your own Anthropic API usage and any charges it incurs. Use at your own risk.
