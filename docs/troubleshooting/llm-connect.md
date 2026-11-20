# Prompt Mode Issues

## "The AI model is not installed yet"

The built-in model file was not installed with Murmure and has not been downloaded yet.

**Fix**: open **Extensions** > **Prompt Mode** > **Advanced configuration** and click **Download model** (about 3 GB, once). You can also place `Qwen_Qwen3.5-4B-Q4_K_M.gguf` next to the Murmure setup file and run the setup again.

## "The downloaded model failed its integrity check"

The download was corrupted or cut short. Murmure deletes the bad file. Click **Download model** again.

## "The AI engine did not start in time"

The engine has 90 seconds to load the model. This can fail on a very slow disk or when the computer is short of memory.

**Fix**: close other heavy applications and try again. The engine needs about 3 GB of free graphics memory (or RAM without a graphics card).

## "The built-in AI engine is missing from this install"

The engine files were removed, often by antivirus software. Reinstall Murmure.

## LLM Output Contains Quotes or Tags

Some models wrap their output in quotes (`"..."`) or include thinking tags (`<think>...</think>`).

**Fix**:

1. Use recommended models: Qwen 3.5, Ministral
2. Add to your system prompt: "Output only the result. No quotes, no thinking, no explanation."

## LLM Response is Very Slow

- **No GPU**: without a graphics card, the built-in AI runs on the CPU and is slower.
- **First request**: the first request starts the engine and loads the model (a few seconds). The next ones take about a second. After 10 minutes without use the engine stops, so the next request is a first request again.

## Server Connection Issues

For OpenAI-compatible servers running on another machine:

1. Verify the server is reachable: `curl http://<server>:<port>/v1/models`
2. Check firewall settings on both machines
3. Ensure the URL in Murmure includes the protocol (`http://` or `https://`)
4. For Ollama on another machine, make sure `OLLAMA_HOST=0.0.0.0` is set on the server and use `http://<server>:11434/v1`

!!! note "Server without a /models endpoint"
    Some OpenAI-compatible servers, such as the Claude API, do not expose `/models`. Test Connection then fails on the model list, but the server can still be used: type the exact model name in the model field and the OpenAI-compatible provider stays available. A failed model list never locks the provider.

!!! note "Proxy support"
    HTTP proxy for Prompt Mode is not yet supported. If you need proxy support in an enterprise environment, please comment on [#286](https://github.com/Kieirra/murmure/issues/286).
