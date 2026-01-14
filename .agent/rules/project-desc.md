---
trigger: always_on
---

# AI Agent Rules: SoniLiveText

## 🔴 CRITICAL: INITIALIZATION

- **First Step:** Before performing any tasks or modifications, you **MUST** read the `llm.md` file in the project root to obtain the necessary architectural context and project information.

## 🛠 Configuration & Settings

- **Mandatory Fields:** All configuration parameters in `config.toml` must be mandatory.
- **Validation Logic:** When adding new settings to `src/types/settings.rs`, you must update the `validate()` method.
- **Validation Strictness:** The `validate()` method must return an error if any field is missing or invalid. Use defaults only where absolutely necessary; prefer explicit user configuration.
- **Example Updates:** Always update `config.toml.example` when the settings structure changes.

## 📝 Documentation

- **README Updates:** If you implement changes or new features relevant to end-user usage, you **MUST** update `README.md` immediately.

## 🚀 Build & Post-Update Workflow

- **Post-Update Offer:** After completing code changes, always offer the user to run `cargo build --release`.
- **Config Deployment:** If the user agrees to a release build:
  1. Check if `target/release/config.toml` exists.
  2. **Only** copy `config.toml` from the root to `target/release/` if it **does not** already exist there.
  3. **Never** automatically overwrite an existing release config.
- **Execution Constraints:** - Do **not** offer to run the application via `cargo run` (debug).
  - Do **not** launch the executable yourself.
  - **Instruct** the user to navigate to `target/release` and execute the `.exe` manually.

## 💻 Development Standards

- **Imports:** The crate name is `sonilivetext`. Internal imports must refer to `sonilivetext::...`.
- **Error Handling:** Use the specific error types defined in `src/errors.rs`.
- **Async Runtime:** The project uses `tokio`; ensure async tasks are compatible.
- **UI Framework:** UI changes must be compatible with `eframe`/`egui` and maintain the transparent, click-through overlay behavior.