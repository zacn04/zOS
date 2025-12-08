# **zOS — not an operating system**

![Rust](https://img.shields.io/badge/Rust-stable-orange)
![SvelteKit](https://img.shields.io/badge/SvelteKit-frontend-red)
![Tauri](https://img.shields.io/badge/Tauri-desktop-lightgrey)
![Ollama](https://img.shields.io/badge/Ollama-local%20LLM-blue)
![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)
![Status: Experimental](https://img.shields.io/badge/status-experimental-yellow)

**zOS** (“Zac’s Operating Space”) is a **local-first agentic reasoning runtime** built with **Rust + Tauri** and powered entirely by **local Ollama LLMs**.
It provides structured pipelines for **proof analysis, problem solving, algorithmic reasoning**, and other technical workflows—without cloud APIs.

zOS manages **multi-model routing**, **fault-tolerant structured extraction**, **repair loops**, and **task-aware fallback logic**, wrapped in a SvelteKit desktop interface designed for interactive practice and analysis.

---

# **Why zOS Exists**

Modern LLMs are powerful but unstable in structured workflows: they truncate, hallucinate formats, and produce incomplete JSON.
zOS explores how far a **local, self-contained agent runtime** can stabilize these behaviors using:

* validation & sanitization layers
* multi-step repair attempts
* fallback regeneration chains
* circuit breaking & exponential backoff
* automatic model selection for different task types

**The goal of zOS is to provide a practical, local-first environment for technical
reasoning tasks. The fault-tolerance mechanisms were built out of necessity:
local LLMs frequently produce truncated or invalid structured output, so zOS
implements enough routing and repair logic to keep the workflows usable.**


---

# **Core Capabilities**

### **Agentic Proof Pipeline**

Two-pass evaluation using local models:

1. **Analysis pass:** detect structure, steps, logical gaps
2. **Evaluation pass:** follow-up questions + final assessment

The pipeline includes validation hooks and recovery logic for malformed output.

---

### **Fault-Tolerant Structured Extraction**

zOS attempts to extract stable structured data from unstable LLM output using:

* truncation detection
* strict + lenient JSON parsing heuristics
* sanitization and partial-repair rules
* DeepSeek → Qwen regeneration chains
* fallback routing for repeated failures

These components form the backbone of the zOS reasoning runtime.

---

### **Model Routing Engine**

A small routing layer dispatches tasks to the appropriate local model:

| Task Type          | Default Model         |
| ------------------ | --------------------- |
| Proof Analysis     | `deepseek-r1:7b`      |
| Problem Gen        | `qwen2.5:7b-math`     |
| General / Fallback | `qwen2.5:7b-instruct` |

The router includes:

* circuit breaker
* exponential backoff
* automatic fallback selection

---

### **Skill & Session Engine**

In addition to the agent runtime, zOS tracks:

* a 10-dimensional skill vector (`[0.0, 1.0]` per domain)
* session history + deltas
* per-domain trends
* simple daily plan heuristics
* static + auto-generated problem pools

These systems exist to support long-term reasoning practice.

---

# **Feature Overview**

### **Solve Mode**

* Suggests problems based on weakest skills
* Accepts free-form solutions/proofs
* Runs the analysis → evaluation pipeline
* Adjusts skills based on detected issues
* Uses LLM-generated or static problems

### **Learn Mode**

* Browse problems by domain (ML Theory, RL, Algorithms, Debugging, Proof Strategy, etc.)
* Filters static problems from `problems/`

### **Improve Mode**

* Current skill levels
* Daily plan (weakest skills + negative trends)
* Ability to reset all user data

### **History & Analytics**

* Per-session logs
* Skill trajectories
* Session counts & difficulty trends
* Static SVG analysis charts

---

# **Technical Architecture**

## **Backend (Rust, Tauri)**

* Task router w/ fallback, backoff, and circuit breaking
* Proof pipeline (analysis + evaluation passes)
* Structured output extraction & repair logic
* Problem generation with JSON validation
* Session + skill store (persisted in app data)
* Daily plan heuristic
* Central `AppState` for caches and paths

## **Frontend (SvelteKit)**

* Tauri-routed SvelteKit SPA
* Dark-mode responsive UI
* History, analytics, and session views
* SVG-based charts

## **Local LLMs (Ollama)**

Default configuration:

* `deepseek-r1:7b` — proof/solution analysis and evaluation
* `qwen2-math:7b` — math/technical problem generation (experimental)
* `qwen2.5:7b-instruct` — general or fallback tasks

Models are configurable via `models.toml` in the app data directory.

---

# **Skill Domains**

The 10-dimensional skill vector tracks:

* RL Theory
* ML Theory
* AI Research
* Coding Debugging
* Algorithms
* Production Engineering
* Analysis & Math
* Putnam/Competition
* Proof Strategy
* Logical Reasoning

---

# **Installation**

### Requirements

* Rust (stable)
* Node.js 18+
* pnpm
* Ollama with required models pulled and available

Suggested setup on macOS:

* Install Ollama: `brew install ollama`
* Pull models:

  * `ollama pull deepseek-r1:7b`
  * `ollama pull qwen2-math:7b`
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
* `problem_model = "qwen2-math:7b"`
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
* Linux: `~/.local/share/com.zacnwo.zos/`
* Windows: `%APPDATA%/com.zacnwo.zos/`

Example:

```toml
proof_model   = "deepseek-r1:7b"
problem_model = "qwen2.5:7b-math"
general_model = "qwen2.5:7b-instruct"
```

### Environment

`ZOS_USE_STATIC_EXAMPLES=true` — bypass LLM problem generation
Ollama endpoint: `http://localhost:11434`

---

# **Data Storage**

Stored under the platform’s app data directory:

* `skills.json` — skill vector
* `data/sessions/*.json` — session logs
* `data/daily_plan.json`
* `problems_cache.json`
* `models.toml`
* `problems/autogen/*.json`

---

# **How the System Works**

### **Problem Flow**

1. Try cache
2. Try daily plan
3. Try LLM generation (default)
4. Fallback to static problems if enabled

### **Proof Pipeline**

1. User submits solution
2. Analysis pass (structure, gaps)
3. Evaluation pass (follow-ups + verdict)
4. Skill updates applied

### **Daily Plan**

* Generated automatically every 24 hours
* Targets weakest or regressing skills

### **Analytics**

* Skill trajectories
* Session trends
* Difficulty over time

---

# **Known Limitations**

* LLM problem generation frequently fails due to JSON instability
* Static fallback recommended for reliable use
* Reasoning quality varies with local models and hardware
* No background prefetch worker yet
* Skill model is simplistic

---

# **Usage**

1. Start Ollama
2. Launch zOS
3. Select *Solve*, *Learn*, *Improve*, or *History*
4. Submit proofs/solutions and iterate through feedback loops
5. Review analytics over time

---

# **License**

MIT

# **Acknowledgments**

Tauri, SvelteKit, Ollama, DeepSeek-R1, Qwen Math, Qwen Instruct

