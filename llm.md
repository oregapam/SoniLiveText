# Project Context: SoniLiveText

This document is designed to help an AI agent (LLM) understand the **SoniLiveText** codebase. It outlines the project's purpose, architecture, and where to find key functionalities. Use this map to navigate the code efficiently.

---

## 🚀 Project Overview

**SoniLiveText** (renamed from `soniox_windows`) is a Rust-based Windows application that provides real-time subtitles and translation for system audio (or microphone).
- **Core Stack:** Rust, `eframe` (egui) for UI, `wasapi` for audio capture, `tungstenite` for WebSockets (Soniox API).
- **Key Behavior:** It runs as a transparent, click-through overlay window that displays text received from the Soniox speech-to-text API.

---

## 📂 Codebase Navigation Strategy

If you are asked to modify specific features, look in these locations:

### 1. Configuration & Startup
*   **Where to look:** `src/main.rs`, `src/types/settings.rs`
*   **What you'll find:**
    - Application entry point (`main`, `run`).
    - Loading and validating `config.toml`.
    - `SettingsApp` struct definition.
    - **Prompt:** "Check how the app validates the API key or adds a new config parameter."

### 2. Audio Capture (The "Ear")
*   **Where to look:** `src/windows/audio.rs`
*   **What you'll find:**
    - Lower-level Windows API interaction (WASAPI).
    - Logic for capturing loopback (system sound) or microphone input.
    - **Prompt:** "How does the app capture audio bytes? Where is the loopback initialization?"

### 3. AI & Network Communication (The "Brain")
*   **Where to look:** `src/soniox/` directory (`request.rs`, `modes.rs`, `transcribe_mode.rs`, `translate_mode.rs`, `stream.rs`)
*   **What you'll find:**
    - Construction of JSON requests for the Soniox API.
    - `SonioxMode` trait and its implementations (`TranscribeMode`, `TranslateMode`) handling the specific logic for each mode.
    - WebSocket connection management (`start_soniox_stream`).
    - Handling API responses (transcription, translation, endpoint detection).
    - **Prompt:** "Find where the WebSocket message is sent or where the transcription response is parsed."

### 4. User Interface & Rendering (The "Face")
*   **Where to look:** `src/gui/` directory (likely `app.rs`, `utils.rs`)
*   **What you'll find:**
    - The `eframe` / `egui` update loop.
    - Rendering text on the screen.
    - **Endpoint Detection:** Immediate flushing of the event queue when "final" tokens are detected to reduce latency.
    - **Smart Freeze:** Stability logic (`update_animation`) splits text at the last whitespace to prevent word merging when using low/zero timeouts.
    - Handling window transparency and "click-through" behavior.
    - **Prompt:** "Where is the subtitle text actually drawn on the screen? How are fonts loaded?"

### 5. Data Structures
*   **Where to look:** `src/types/`
*   **What you'll find:**
    - Shared structs for Audio messages, API responses, and Settings.
    - **Prompt:** "Where is the struct that defines the Soniox API response format?"

### 6. App Initialization & Wiring
*   **Where to look:** `src/lib.rs`
*   **What you'll find:**
    - The `initialize_app` function.
    - Spawning of async tasks (audio capture thread, network thread).
    - Channel creation for communication between threads.

---

## 🛠️ Common Tasks & Workflows

*   **Adding a new Setting:**
    1.  Add field to `SettingsApp` in `src/types/settings.rs`.
    2.  Update `validate()` method in the same file. **IMPORTANT:** All configuration parameters (variables) MUST be mandatory in `config.toml`. The `validate()` method must return an error if any field is missing or invalid. Use defaults only where absolutely necessary, but prefer explicit user configuration.
    3.  Update usages in `main.rs` or wherever the setting is consumed.
    4.  Update `config.toml.example`.

*   **Changing Window Behavior:**
    - Code controlling window attributes (always-on-top, transparency) is primarily in `src/main.rs` (initial setup) and potentially `src/gui/` if updated at runtime.

*   **modifying API Request:**
    - Look at `src/soniox/transcribe_mode.rs` or `translate_mode.rs` to change what parameters (like `enable_endpoint_detection`) are sent to the server.

*   **Documentation Updates:**
    - **CRITICAL:** If you implement changes or new features that are relevant to the end-user (basic usage), you **MUST** update `README.md` to reflect these changes immediately.

*   **Post-Update / Release Workflow:**
    - After completing code changes/updates, ALWAYS offer the user to run `cargo build --release`.
    - **Config Handling:** If the user agrees to the build:
        1. Check if `target/release/config.toml` exists.
        2. **ONLY** copy `config.toml` (from root) to `target/release/config.toml` if it DOES NOT exist in the destination. Never overwrite the release config automatically.
    - **Execution:** Do NOT offer to run the application via `cargo run` (debug) or by launching the executable yourself. Instead, instruct the user to navigate to the `target/release` folder and execute the `.exe` manually.

---

## 📝 Important Notes for the AI

*   **Crate Name:** The project package name in `Cargo.toml` is `sonilivetext`. In code, imports refer to `sonilivetext::...`.
*   **Error Handling:** The project uses specific error types defined in `src/errors.rs`.
*   **Async:** The project relies heavily on `tokio` for async runtime.
*   **Configuration Rules:** All configuration parameters (current and future) are **MANDATORY**. Implementing `validate()` to enforce this is required for every new setting.

When starting a task, verify the file content in these locations to get the latest context.
