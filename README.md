# zOS – Local Learning Assistant

![Rust](https://img.shields.io/badge/Rust-stable-orange)
![SvelteKit](https://img.shields.io/badge/SvelteKit-frontend-red)
![Tauri](https://img.shields.io/badge/Tauri-desktop-lightgrey)
![Ollama](https://img.shields.io/badge/Ollama-local%20LLM-blue)
![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)
![Status: Experimental](https://img.shields.io/badge/status-experimental-yellow)

A Tauri-based desktop application for practicing technical problem-solving with LLM-powered feedback.
Built with Rust and SvelteKit, running entirely on local Ollama models.

**Status:** Experimental personal project. Proof feedback and LLM problem generation are unstable and may fail or behave inconsistently.

---

## What is zOS?

zOS (“Zac’s Operating Space”) is a local-first learning environment for math, ML, algorithms, proofs, and reasoning.
It is not an operating system; the name refers to a personal workspace for technical practice.

zOS integrates a Rust backend (via Tauri), a SvelteKit frontend, and local LLMs through Ollama to provide:

* Structured problem-solving workflows
* A simple two-step proof/solution feedback loop
* Lightweight skill tracking across 10 domains
* Session history and basic analytics
* Optional LLM-based problem generation

Everything runs locally; no cloud APIs are used.

---

## Overview

zOS provides:

* **Proof Feedback (experimental):** Two-step LLM analysis and evaluation of submitted proofs/solutions
* **Skill Tracking:** A 10-dimensional skill vector with basic incremental updates
* **Problem Management:** Static JSON problems plus optional LLM-based generation
* **Session History:** Logs and displays past problem-solving sessions
* **Daily Planning:** A simple heuristic daily plan based on weakest skills and trends

This is an exploratory tool, not a polished tutoring product.

---

## Features

### Solve Mode

* Recommends problems based on weakest skills
* Accepts free-form text solutions/proofs
* Runs a two-step proof pipeline:

  * Analysis: identifies structure, steps, and issues
  * Evaluation: follow-up questions and final assessment
* Updates skills based on detected issues and answer quality
* Uses static problems only if the `ZOS_USE_STATIC_EXAMPLES=true` environment variable is set

### Learn Mode

* Browses problems by topic (RL Theory, ML Theory, Coding, Algorithms, etc.)
* Lists all static problems in the `problems/` directory
* Filters problems by skill domain

### Improve Mode

* Displays current skill levels across 10 domains with progress bars
* Shows the current daily plan (when generated)
* Recommends work on weaker areas
* Can reset all stored skill and history data

### History and Analytics

* Lists all past problem-solving sessions
* Computes basic analytics:

  * Skill progression over time
  * Session counts
  * Average difficulty
  * Weekly trend indicators
* Renders charts as static SVGs (non-interactive)

---

## Technical Architecture

### Backend (Rust)

* Tauri-based desktop backend
* Model router: routes proof, problem, and general tasks to configured Ollama models with basic fallback behavior
* Proof pipeline:

  * Step 1: solution/proof analysis
  * Step 2: evaluation with follow-up questions
* Problem generator: experimental LLM-based JSON generation, fragile due to parsing/formatting issues
* Skill model: per-domain scalar in `[0.0, 1.0]` with additive increments/decrements
* Session logging: saves each attempt with metadata and skill deltas
* Daily plan generator: simple heuristic focused on weakest skills and negative trends
* Central application state via `AppState` for in-memory caches and filesystem paths

### Frontend (SvelteKit)

* SvelteKit SPA routed via Tauri
* TypeScript for type safety
* Responsive UI with dark theme support
* History/Improve views rendering SVG analytics charts

### LLM Models (via Ollama)

Defaults:

* `deepseek-r1:7b` — proof/solution analysis and evaluation
* `qwen2.5:7b-math` — math/technical problem generation (experimental)
* `qwen2.5:7b-instruct` — general or fallback tasks

Models are configurable via `models.toml` in the app data directory.

### Skill Domains

The skill vector tracks 10 areas:

* RL Theory
* ML Theory
* AI Research
* Coding Debugging
* Algorithms
* Production Engineering
* Analysis and Math
* Putnam/Competition
* Proof Strategy
* Logical Reasoning

---

## Prerequisites

* Rust (latest stable)
* Node.js (v18 or later)
* pnpm
* Ollama with required models pulled and available

Suggested setup on macOS:

* Install Ollama: `brew install ollama`
* Pull models:

  * `ollama pull deepseek-r1:7b`
  * `ollama pull qwen2.5:7b-math`
  * `ollama pull qwen2.5:7b-instruct`
* Start Ollama: `ollama serve`

---

## Installation

1. Clone the repository: `git clone <repository-url>`
2. Enter the project directory: `cd personal-os`
3. Install dependencies: `pnpm install`
4. Start Ollama if needed: `ollama serve`

---

## Development

Run in development mode:

* `pnpm tauri dev`

This starts the Svelte dev server, builds and runs the Tauri app, and enables hot reload for frontend and backend.

Build a production bundle:

* `pnpm tauri build`

The built application will be under `src-tauri/target/release/bundle/`.

Run tests:

* Rust tests: `cd src-tauri && cargo test`
* TypeScript/Svelte checks: `pnpm check`

---

## Configuration

### Model Configuration

Models are configured via `models.toml` in the platform-specific app data directory:

* macOS: `~/Library/Application Support/com.zacnwo.zos/models.toml`
* Windows: `%APPDATA%/com.zacnwo.zos/models.toml`
* Linux: `~/.local/share/com.zacnwo.zos/models.toml`

Default values:

* `proof_model = "deepseek-r1:7b"`
* `problem_model = "qwen2.5:7b-math"`
* `general_model = "qwen2.5:7b-instruct"`

If the file does not exist, defaults are used.

### Environment Variables

* `ZOS_USE_STATIC_EXAMPLES=true`

  * Enables static problems in Solve mode
  * Without this, Solve mode prefers LLM-based generation, which is experimental and may fail
  * Learn mode always shows static problems regardless

### Ollama Endpoint

The app expects Ollama at `http://localhost:11434` with the configured models available.

---

## Project Structure

`personal-os/`

* `src/` Svelte frontend

  * `routes/`

    * `+page.svelte` Home page
    * `solve/` Problem solving UI
    * `learn/` Topic browsing
    * `improve/` Skills and daily plan
    * `history/` Sessions and analytics
* `src-tauri/` Rust backend

  * `problems/` problem loading, selection, generation, cache
  * `skills/` skill vector and persistent store
  * `sessions/` session records and history
  * `brain/` daily plan logic and persistence
  * `analytics/` metrics computation
  * `pipelines/` router, proof pipeline, Ollama client, JSON extraction
  * `models/` model wrappers and registry
  * `state/` app-level state and caches
  * `config/` model config loader
* `problems/` static JSON problem files
* `package.json`

---

## Data Storage

User data is stored in platform-specific application data directories:

* macOS: `~/Library/Application Support/com.zacnwo.zos/`
* Windows: `%APPDATA%/com.zacnwo.zos/`
* Linux: `~/.local/share/com.zacnwo.zos/`

Files include:

* `skills.json` — current skill levels (0.0–1.0 per domain)
* `data/sessions/*.json` — individual session records
* `data/daily_plan.json` — daily plan (24-hour validity)
* `data/problems_cache.json` — prefetched problems (if used)
* `problems/autogen/*.json` — auto-generated problems (if generation succeeds)
* `models.toml` — optional model override

---

## How It Works

### Problem Flow

When a problem is requested in Solve mode:

1. Check the problem cache (if populated)
2. Check daily plan tasks
3. Fall back to LLM-based generation for the weakest skill
4. Use static problems only if `ZOS_USE_STATIC_EXAMPLES=true` is set

### Solving a Problem

* User submits a written solution/proof
* Proof pipeline runs:

  * Analysis pass: attempts to identify structure and issues
  * Evaluation pass: asks questions and provides a final judgment
* Skill updates are applied:

  * Decreases on detected issues (roughly −0.02 to −0.03 per issue type)
  * Small increases for correct or mostly correct answers
  * Values clamped to `[0.0, 1.0]`

### Daily Plan

* Generated on startup if missing or expired
* Focuses on two weakest skills plus skills with negative trends
* Passive plan; user still manually requests problems
* Expires after 24 hours

### Analytics

* Derived from session history
* Computes skill trajectories, session counts, average difficulty, and simple trends
* Visualized as static SVG charts

---

## Known Limitations

1. LLM-based problem generation is fragile and frequently fails due to JSON/formatting errors.
2. Solve mode uses static problems only when `ZOS_USE_STATIC_EXAMPLES=true` is set.
3. Runtime behavior depends on Ollama stability and available system RAM.
4. Problem cache prefetching is implemented but not wired into a background worker.
5. The skill update rule is simplistic and hand-tuned, not statistically grounded.
6. Analytics are basic and non-interactive.

---

## Usage

Basic workflow:

1. Start Ollama, then launch zOS.
2. Use Solve mode to request a recommended problem.
3. Submit your solution, read feedback, and answer follow-up questions if prompted.
4. Use Improve mode to review skill levels and the daily plan.
5. Use History mode to review sessions and trends.

If problem generation fails repeatedly, set `ZOS_USE_STATIC_EXAMPLES=true` and rely on static problems.

---

## Troubleshooting

“No problems available”

* Set `ZOS_USE_STATIC_EXAMPLES=true`.
* Ensure Ollama is running with `ollama serve`.
* Verify models with `ollama list`.
* Confirm JSON problem files exist in `problems/`.

“Model error” when analyzing proofs

* Ensure Ollama is reachable at `http://localhost:11434`.
* Confirm required models are pulled.
* Check Tauri/Rust logs for raw LLM output.

Problems not generating

* Expected sometimes; generator is experimental.
* Prefer static problems via `ZOS_USE_STATIC_EXAMPLES=true`.
* Check Ollama logs for out-of-memory or model failures.

JSON extraction errors

* Inspect raw responses in logs.
* Try switching models via `models.toml`.
* Failures are normal at this stage.

---

## Contributing

This is a personal project, but suggestions, issues, and small contributions are welcome.

---

## License

MIT

---

## Acknowledgments

* Tauri
* SvelteKit
* Ollama
* DeepSeek-R1, Qwen Math, Qwen Instruct
