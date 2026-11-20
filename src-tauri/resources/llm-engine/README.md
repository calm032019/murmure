# Built-in LLM engine

llama.cpp `llama-server`, Windows Vulkan build **b12540**:
https://github.com/ggml-org/llama.cpp/releases/download/b12540/llama-b12540-bin-win-vulkan-x64.zip

Copy into this folder: `llama-server.exe`, `llama-server-impl.dll`, `llama.dll`, `llama-common.dll`,
`ggml.dll`, `ggml-base.dll`, `ggml-vulkan.dll`, `ggml-cpu-*.dll`, `mtmd.dll`, `libomp.dll`,
`LICENSE-LLVM-OpenMP`, plus llama.cpp's MIT licence as `LICENSE-llama.cpp`.
The binaries are git-ignored and bundled as resources by `tauri.conf.json`.
