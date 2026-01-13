# SoniLiveText

**Advanced Real-time AI Subtitles & Translation for Windows**

SoniLiveText is a powerful, lightweight Windows application that generates real-time subtitles for any audio playing on your system. Powered by the Soniox AI engine, it provides high-accuracy speech-to-text and instant translation, displayed in a non-intrusive, customizable overlay window.

Unlike standard windows, SoniLiveText renders text directly onto the screen background, allowing you to click through it and work uninterrupted while keeping track of spoken content.

---

## 🚀 Key Features

*   **System-Wide Audio Capture**: Uses Windows WASAPI Loopback to capture sound directly from your system with zero latency. No virtual cables or third-party drivers needed.
*   **"Ghost" Overlay**: Subtitles are drawn on a click-through, transparent layer. It floats above other windows (or at the bottom) without stealing focus or blocking mouse interactions.
*   **High-Accuracy AI Transcription**: Leverages the Soniox API for state-of-the-art speech recognition.
*   **Live Translation**: Instantly translate spoken text into your target language.
*   **Stable Typewriter Animation**: Advanced stabilization logic ensures text flows smoothly, sentence by sentence, without jumping or flickering. "Wait-and-stream" technology prevents eye strain.
*   **Speaker Identification**: Distinguishes between different speakers in a conversation.
*   **Resilient Connectivity**: Automatically reconnects if the server connection drops.
*   **Highly Configurable**: Customize window position, size, colors, fonts, and behavior via a simple configuration file.
*   **Microphone Support**: Can optionally switch to microphone input for dictation or meeting transcription.

## 🛠️ Installation & Build

Currently, SoniLiveText is distributed as source code. You will need to build it yourself using the Rust toolchain.

### Prerequisites
*   **Rust Compiler**: Install the latest stable version from [rust-lang.org](https://www.rust-lang.org/tools/install).

### Build Instructions

1.  **Clone the repository:**
    ```powershell
    git clone https://github.com/oregapam/SoniLiveText.git SoniLiveText
    cd SoniLiveText
    ```

2.  **Build the release binary:**
    ```powershell
    cargo build --release
    ```
    The executable will be located at `target/release/sonilivetext.exe`.

3.  **Setup Configuration:**
    *   Copy the `config.toml.example` file to the same directory as your executable.
    *   Rename it to `config.toml`.
    *   Edit the file with your API key and preferences (see below).

## ⚙️ Configuration (`config.toml`)

The application is entirely controlled via the `config.toml` file.

**IMPORTANT: All parameters listed below are MANDATORY.** The application will verify their presence at startup and exit with an error message if any field is missing.

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `api_key` | String | Your Soniox API key. |
| `model` | String | AI Model version: `"stt-rt-v3"` (stable) or `"stt-rt-v3-preview"` (latest). |
| `language_hints` | Array | List of expected source languages (e.g., `["en", "ru", "hu"]`). |
| `target_language` | String | Language code to translate into (e.g., `"hu"`). |
| `enable_translate` | Boolean | Set to `true` to enable live translation. |
| `context` | String | Context hint for the AI to improve accuracy (e.g., specific terminology). |
| `level` | String | Logging level (e.g., `"debug"`, `"info"`). |
| `enable_high_priority`| Boolean | If `true`, the window tries to stay on top of other applications. |
| `show_window_border` | Boolean | If `true`, draws a border (useful for positioning). |
| `enable_speakers` | Boolean | If `true`, attempts to identify and label different speakers. |
| `text_color` | Array | RGB text color, e.g., `[255, 255, 0]` for yellow. |
| `window_anchor` | String | Positioning anchor: `bottom_center`, `top_left`, `center`, etc. |
| `window_offset` | Array | `[x, y]` offset from the anchor point. |
| `window_width` | Float | Width of the subtitle area in pixels. |
| `window_height` | Float | Height of the subtitle area in pixels. |
| `audio_input` | String | Source: `"loopback"` (system audio) or `"microphone"`. |
| `font_size` | Float | Font size for the text (e.g. `24.0`). |
| `show_interim` | Boolean | If `true`, shows unstable interim text (grayed out) before finalizing. |
| `stability_timeout_ms` | Integer | Latency buffer in ms. Higher values = more stability, lower = faster display. Default ~2000ms. |

## ❓ Troubleshooting

### `link.exe` not found on Windows

On Windows, this project may fail to build with errors similar to:

```text
error: linker `link.exe` not found
  = note: program not found

note: the msvc targets depend on the msvc linker but `link.exe` was not found
note: please ensure that Visual Studio 2017 or later, or Build Tools for Visual Studio were installed with the Visual C++ option.
note: VS Code is a different product, and is not sufficient.

error: could not compile `proc-macro2` (build script) due to 1 previous error
...
```

This means Rust is using an MSVC target on Windows, but the Microsoft C++ toolchain (and its linker `link.exe`) is not installed or not visible in your environment.

#### Solution: Install the MSVC build tools

Download and install the **Build Tools for Visual Studio** from the official Microsoft downloads page.

During installation, make sure to select the **“Desktop development with C++”** workload, which includes:
*   MSVC C++ toolset
*   Windows 10/11 SDK

After installation, open a new terminal (Command Prompt or PowerShell) so the environment variables are refreshed.

Run your build again, for example:

```bash
cargo build --release
```

If `link.exe` is correctly installed, the error should disappear.

##  CREDITS & ACKNOWLEDGEMENTS

This project is a fork and advanced evolution of **[soniox_windows](https://github.com/eoftgge/soniox_windows)**.

I deeply appreciate the work of the original author, **[eoftgge](https://github.com/eoftgge)**, who created the core architecture for capturing Windows audio and interfacing with the Soniox API. This project wouldn't exist without their initial contribution.

**SoniLiveText** aims to extend this foundation with enhanced UI features, better customizability, and broader language support.

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.
