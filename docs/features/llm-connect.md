# Prompt Mode

![Prompt Mode](../assets/llm-connect.png)

Prompt Mode (previously called LLM Connect) lets you post-process your transcription with a Large Language Model, running locally or on a server before it's inserted. This is useful for translation, grammar correction, medical formatting, code generation, and more.

## Requirements

You need one of:

- **Built-in AI** - Murmure runs the Qwen 3.5 (4B) model on your computer. Nothing else to install.
- **OpenAI-compatible** - LM Studio, Magnitude, llama.cpp, vLLM, Ollama, or a cloud provider.

!!! note "Platform"
    Built-in AI currently ships with the Windows build. On macOS and Linux, use the OpenAI-compatible option.

## Setup with Built-in AI

Murmure includes its own AI engine (llama.cpp) and uses the Qwen 3.5 (4B) model. It runs on your graphics card when there is one, and on the CPU otherwise. Nothing leaves your computer.

### The model file

The model is about 3 GB, too large to fit inside the installer. It is delivered in one of two ways:

- **Installed with Murmure**: if the setup file comes with `Qwen_Qwen3.5-4B-Q4_K_M.gguf` in the same folder, the installer copies the model automatically.
- **Downloaded once**: otherwise, Murmure downloads it the first time you set up Prompt Mode, and shows a progress bar. The file is checked against its known checksum before it is used.

### Open the Prompt Mode onboarding in Murmure

1. Open Murmure > **Extensions** > **Prompt Mode** (or Settings > Prompt Mode) and click **Enable Prompt Mode**
2. Click **Use built-in AI**
3. Click the **Qwen 3.5 (4B)** card. If the model is not on your computer yet, Murmure downloads it
4. Click **Finish Setup** to complete the setup

The engine starts the first time a prompt is used (a few seconds), then answers in about a second. It stops on its own after 10 minutes without use, and whenever Murmure closes.

!!! note "Thinking is off"
    Qwen 3.5 can "think" before answering. Murmure always turns this off: cleanup and formatting are rewriting tasks, and thinking only makes them slower.

!!! warning "No GPU = Slow"
    Without a graphics card, the model runs on the CPU and each prompt takes noticeably longer.

## Setup with OpenAI-compatible

Murmure supports any OpenAI-compatible API: LM Studio, Magnitude, llama.cpp, vLLM, or a cloud provider. The server can run on your computer, on your network, or online.

1. Open Murmure > **Extensions** > **Prompt Mode** and click **Enable Prompt Mode**
2. Follow the 3-step onboarding wizard. First click **Choose OpenAI-compatible**
3. Enter the server URL. See the examples below
4. Choose the model: pick it from the list (Murmure fetches the available models from the server), or type the exact model name in the field if your server does not provide a model list, for example `claude-haiku-4-5`
5. Click **Finish Setup** to complete the setup

**URL examples:**

- LM Studio: `http://localhost:1234/v1`
- Magnitude: `http://127.0.0.1:10100/inference/v1`
- llama.cpp (`llama-server`): `http://localhost:<port>/v1`. Use the port displayed by `llama-server` when it starts, as the default port depends on the version

!!! note "Privacy"
    Murmure shows a privacy warning for this option only when the URL is not local or private (anything other than `localhost`, `127.0.0.1`, `10.x`, `192.168.x` or `172.16` to `172.31.x`). A local OpenAI-compatible server stays private.

!!! note "Ollama"
    Ollama is no longer needed. If you already use it, you can still connect it as an OpenAI-compatible server with the URL `http://localhost:11434/v1`.

You can mix providers across your prompts - for example, Prompt 1 using Built-in AI and Prompt 2 using OpenAI-compatible.

![Prompt Mode advanced configuration](../assets/llm-connect-advanced.png)

## Prompt Templates

Prompt Mode supports multiple saved prompts, up to 4. Each prompt can have its own:

- Provider (Built-in AI or OpenAI-compatible)
- Model
- System prompt
- User prompt (with `{{text}}` placeholder for the transcription)

### Built-in Presets

- **Translation** - Translate transcription to another language
- **Medical** - Format for medical dictation (INN terminology)
- **Development** - Format for code-related dictation
- **Voice Dictation** - Clean up spoken text for written form

### Custom Prompts

Write your own system prompt to customize behavior. The `{{text}}` placeholder in the user prompt is replaced with your transcription.

**Example - Fix grammar and punctuation:**

System prompt:

```
You are a French text editor. Fix grammar, spelling, and punctuation.
Output only the corrected text, nothing else.
```

User prompt:

```
{{text}}
```

## Two Ways to Use a Prompt

Each prompt's tab shows a small bar above the prompt editor, with one entry per gesture and a help icon that explains its steps.

| Gesture | Input | Instruction |
| --- | --- | --- |
| **Dictate** | your voice | the prompt's saved text |
| **Transform** | selected text | the prompt's saved text, applied instantly |

### Dictate

Each of the 4 prompts has its own keyboard shortcut for Dictate (`Ctrl+Shift+1` through `Ctrl+Shift+4` by default). Pressing one starts recording immediately, and the prompt is applied to your speech in a single action.

### Transform

Each prompt also has its own, independent shortcut for Transform (`Ctrl+Alt+Shift+1` through `Ctrl+Alt+Shift+4` by default). Select text in any application, press the shortcut, and the saved prompt is applied directly to your selection, no dictation needed. A sound and a wave animation play while the model processes your selection.

If nothing is selected, Murmure shows a toast asking you to select text first and makes no LLM call. If the LLM call fails, your selection is left untouched.

If a prompt is empty, Dictate and Transform show a toast: "Prompt N is not configured. Open Prompt Mode to set it up."

## Shortcuts

Dictate and Transform shortcuts are independent and configurable per prompt in **Settings > Shortcuts**, listed as **Dictate with {prompt name}** and **Transform with {prompt name}**. Each can be rebound to any key combination, including a mouse button or an `F13`-`F20` key.

On Linux Wayland, where the compositor owns keyboard shortcuts, use the CLI instead: `murmure --llm-mode <N>` for Dictate and `murmure --llm-transform <N>` for Transform. See [CLI](cli.md).

## Known Issues

- Some models wrap output in quotes or add `<think>` tags. The most effective fix is to create a custom [Formatting Rule](formatting-rules.md) with regex to strip them automatically (e.g., `<think>[\s\S]*?</think>` replaced by nothing). You can also try adding "Output only the result, no quotes, no thinking" to your prompt, or switch to recommended models (Qwen, Ministral).
- **macOS**: The default Dictate shortcuts (`Ctrl+Shift+1..4`) may leak characters on macOS. If this occurs, rebind them to modifier-only combos in Settings > Shortcuts.

See [Prompt Mode Troubleshooting](../troubleshooting/llm-connect.md) for more help.
