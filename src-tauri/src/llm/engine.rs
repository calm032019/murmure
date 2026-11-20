//! Built-in LLM engine: a bundled llama.cpp `llama-server` running Qwen3.5-4B.
//!
//! Murmure starts the server lazily as a hidden child process bound to
//! 127.0.0.1 on a free port, protected by a random per-launch API key, and
//! stops it after a period of inactivity or when the app exits. Requests go
//! through the OpenAI-compatible chat endpoint with thinking disabled: text
//! cleanup is rewriting, and Qwen3.5's default reasoning only adds latency
//! (or returns empty content once the token budget is spent).

use crate::llm::types::{
    ChatTemplateKwargs, OllamaPullResponse, OpenAIChatMessage, OpenAIChatRequest,
    OpenAIChatResponse,
};
use log::{info, warn};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

/// Model identifier shown in the UI and stored in mode settings.
pub const MODEL_ID: &str = "qwen3.5:4b";
pub const MODEL_FILE: &str = "Qwen_Qwen3.5-4B-Q4_K_M.gguf";
const MODEL_URL: &str =
    "https://huggingface.co/bartowski/Qwen_Qwen3.5-4B-GGUF/resolve/main/Qwen_Qwen3.5-4B-Q4_K_M.gguf";
const MODEL_SHA256: &str = "13c16f426047e2de38cd075bdade4a7bcbc8c774384876f677740cda65f8a983";
const MODEL_SIZE: u64 = 3_013_027_808;

const ENGINE_DIR: &str = "llm-engine";
#[cfg(target_os = "windows")]
const SERVER_BIN: &str = "llama-server.exe";
#[cfg(not(target_os = "windows"))]
const SERVER_BIN: &str = "llama-server";

const CONTEXT_SIZE: &str = "4096";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(90);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const IDLE_SHUTDOWN: Duration = Duration::from_secs(10 * 60);

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);
static ENGINE: LazyLock<Mutex<Option<RunningEngine>>> = LazyLock::new(|| Mutex::new(None));

struct RunningEngine {
    child: Child,
    port: u16,
    api_key: String,
    last_used: Instant,
}

impl RunningEngine {
    fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

// ---------------------------------------------------------------- paths

fn engine_binary(app: &AppHandle) -> Option<PathBuf> {
    crate::utils::resources::resolve_resource_path(app, &format!("{}/{}", ENGINE_DIR, SERVER_BIN))
        .or_else(|| {
            let path = app
                .path()
                .resolve(
                    format!("resources/{}/{}", ENGINE_DIR, SERVER_BIN),
                    tauri::path::BaseDirectory::Resource,
                )
                .ok()?;
            path.exists().then_some(path)
        })
}

/// Where an in-app download is stored.
fn downloaded_model_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("models");
    Ok(dir.join(MODEL_FILE))
}

/// Model copied next to the app by the installer (setup.exe + model side by side).
fn installed_model_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .resolve(
            format!("models/{}", MODEL_FILE),
            tauri::path::BaseDirectory::Resource,
        )
        .ok()
}

fn is_complete_model(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.len() == MODEL_SIZE)
        .unwrap_or(false)
}

pub fn model_path(app: &AppHandle) -> Option<PathBuf> {
    let candidates = [installed_model_path(app), downloaded_model_path(app).ok()];
    candidates
        .into_iter()
        .flatten()
        .find(|path| is_complete_model(path))
}

pub fn is_engine_available(app: &AppHandle) -> bool {
    engine_binary(app).is_some()
}

pub fn is_model_available(app: &AppHandle) -> bool {
    model_path(app).is_some()
}

// ---------------------------------------------------------------- process

fn hidden_command(program: &Path) -> Command {
    let mut command = Command::new(program);
    if let Some(dir) = program.parent() {
        command.current_dir(dir);
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[derive(Debug, PartialEq)]
struct GpuDevice {
    id: String,
    name: String,
}

/// Parses `llama-server --list-devices` lines such as
/// `  Vulkan0: NVIDIA RTX 3500 Ada Generation Laptop GPU (12015 MiB, 11247 MiB free)`.
fn parse_devices(output: &str) -> Vec<GpuDevice> {
    output
        .lines()
        .filter_map(|line| {
            let (id, rest) = line.trim().split_once(": ")?;
            if id.is_empty() || id.contains(' ') {
                return None;
            }
            let name = rest.rsplit_once(" (").map_or(rest, |(name, _)| name);
            Some(GpuDevice {
                id: id.to_string(),
                name: name.trim().to_string(),
            })
        })
        .collect()
}

fn is_integrated_gpu(name: &str) -> bool {
    let lower = name.to_lowercase();
    match () {
        _ if lower.contains("intel") => !lower.contains(" arc"),
        _ if lower.ends_with("radeon(tm) graphics") || lower.ends_with("radeon graphics") => true,
        _ => lower.contains("microsoft basic render") || lower.contains("llvmpipe"),
    }
}

/// llama.cpp splits a model across every GPU it sees, which on laptops pairs
/// the dedicated GPU with the slow integrated one (40 s instead of 0.5 s).
/// Pin to one device: the first dedicated GPU, else the first device.
fn pick_device(devices: &[GpuDevice]) -> Option<&GpuDevice> {
    devices
        .iter()
        .find(|device| !is_integrated_gpu(&device.name))
        .or_else(|| devices.first())
}

fn detect_device(binary: &Path) -> Option<String> {
    let output = hidden_command(binary)
        .arg("--list-devices")
        .stdin(Stdio::null())
        .output()
        .ok()?;
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let devices = parse_devices(&text);
    let chosen = pick_device(&devices).map(|device| device.id.clone());
    info!("LLM engine devices: {:?}, using {:?}", devices, chosen);
    chosen
}

fn free_port() -> Result<u16, String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    listener
        .local_addr()
        .map(|addr| addr.port())
        .map_err(|e| e.to_string())
}

fn spawn_server(app: &AppHandle) -> Result<RunningEngine, String> {
    let binary =
        engine_binary(app).ok_or("The built-in AI engine is missing from this install.")?;
    let model = model_path(app).ok_or("The AI model is not installed yet.")?;
    let port = free_port()?;
    let api_key = uuid::Uuid::new_v4().simple().to_string();

    let mut command = hidden_command(&binary);
    command
        .arg("-m")
        .arg(&model)
        .args(["--host", "127.0.0.1", "--port", &port.to_string()])
        .args(["--api-key", &api_key])
        .args(["-c", CONTEXT_SIZE, "-np", "1", "-fa", "off", "--no-webui"]);
    match detect_device(&binary) {
        Some(device) => {
            command.args(["--device", &device, "-ngl", "99"]);
        }
        None => {
            command.args(["-ngl", "0"]);
        }
    }

    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start the AI engine: {}", e))?;
    kill_with_app::attach(&child);
    info!("LLM engine started on port {}", port);

    Ok(RunningEngine {
        child,
        port,
        api_key,
        last_used: Instant::now(),
    })
}

async fn wait_until_ready(base_url: &str, api_key: &str) -> Result<(), String> {
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    while Instant::now() < deadline {
        let ready = HTTP_CLIENT
            .get(format!("{}/health", base_url))
            .bearer_auth(api_key)
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .is_ok_and(|response| response.status().is_success());
        if ready {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    Err("The AI engine did not start in time.".to_string())
}

/// Returns (base_url, api_key) of a running, healthy engine, starting it if needed.
async fn ensure_running(app: &AppHandle) -> Result<(String, String), String> {
    let mut guard = ENGINE.lock().await;
    if let Some(engine) = guard.as_mut() {
        match engine.child.try_wait() {
            Ok(None) => {
                engine.last_used = Instant::now();
                return Ok((engine.base_url(), engine.api_key.clone()));
            }
            _ => {
                warn!("LLM engine exited unexpectedly, restarting");
                *guard = None;
            }
        }
    }

    let mut engine = spawn_server(app)?;
    let base_url = engine.base_url();
    if let Err(e) = wait_until_ready(&base_url, &engine.api_key).await {
        let _ = engine.child.kill();
        return Err(e);
    }
    let api_key = engine.api_key.clone();
    *guard = Some(engine);
    drop(guard);
    spawn_idle_watchdog();
    Ok((base_url, api_key))
}

fn spawn_idle_watchdog() {
    tauri::async_runtime::spawn(async {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let mut guard = ENGINE.lock().await;
            let Some(engine) = guard.as_mut() else {
                return;
            };
            if engine.last_used.elapsed() >= IDLE_SHUTDOWN {
                info!("LLM engine idle, stopping it to free memory");
                let _ = engine.child.kill();
                let _ = engine.child.wait();
                *guard = None;
                return;
            }
        }
    });
}

pub fn shutdown() {
    if let Ok(mut guard) = ENGINE.try_lock() {
        if let Some(mut engine) = guard.take() {
            let _ = engine.child.kill();
            let _ = engine.child.wait();
        }
    }
}

// ---------------------------------------------------------------- inference

/// Removes any `<think>…</think>` block a model may emit despite thinking being off.
fn strip_thinking(text: &str) -> String {
    let without = match (text.find("<think>"), text.find("</think>")) {
        (Some(start), Some(end)) if end > start => {
            format!("{}{}", &text[..start], &text[end + "</think>".len()..])
        }
        (None, Some(end)) => text[end + "</think>".len()..].to_string(),
        _ => text.to_string(),
    };
    without.trim().to_string()
}

pub async fn generate(
    app: &AppHandle,
    system_prompt: Option<&str>,
    user_prompt: &str,
) -> Result<String, String> {
    let (base_url, api_key) = ensure_running(app).await?;

    let mut messages = Vec::new();
    if let Some(system) = system_prompt {
        messages.push(OpenAIChatMessage {
            role: "system".to_string(),
            content: system.to_string(),
        });
    }
    messages.push(OpenAIChatMessage {
        role: "user".to_string(),
        content: user_prompt.to_string(),
    });

    let request_body = OpenAIChatRequest {
        model: MODEL_ID.to_string(),
        messages,
        stream: false,
        think: None,
        temperature: Some(0.0),
        chat_template_kwargs: Some(ChatTemplateKwargs {
            enable_thinking: false,
        }),
    };

    let response = HTTP_CLIENT
        .post(format!("{}/v1/chat/completions", base_url))
        .bearer_auth(&api_key)
        .json(&request_body)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("The AI engine did not respond: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "The AI engine returned an error: {}",
            response.status()
        ));
    }

    let chat: OpenAIChatResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to read the AI engine response: {}", e))?;

    if let Some(engine) = ENGINE.lock().await.as_mut() {
        engine.last_used = Instant::now();
    }

    chat.choices
        .first()
        .map(|choice| strip_thinking(&choice.message.content))
        .ok_or_else(|| "The AI engine returned an empty response".to_string())
}

/// Loads the model ahead of time so the first cleanup is fast.
pub async fn warmup(app: &AppHandle) -> Result<(), String> {
    if !is_model_available(app) {
        return Ok(());
    }
    ensure_running(app).await.map(|_| ())
}

// ---------------------------------------------------------------- download

fn emit_progress(app: &AppHandle, status: &str, completed: Option<u64>) {
    let _ = app.emit(
        "llm-pull-progress",
        OllamaPullResponse {
            status: status.to_string(),
            digest: None,
            total: Some(MODEL_SIZE),
            completed,
        },
    );
}

/// Downloads the model once, verifies its SHA-256 and moves it into place.
/// Progress uses the same `llm-pull-progress` event the onboarding UI listens to.
pub async fn download_model(app: AppHandle) -> Result<(), String> {
    if is_model_available(&app) {
        emit_progress(&app, "success", Some(MODEL_SIZE));
        return Ok(());
    }

    let final_path = downloaded_model_path(&app)?;
    let dir = final_path.parent().ok_or("Invalid model directory")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let partial_path = final_path.with_extension("gguf.part");

    let mut response = HTTP_CLIENT
        .get(MODEL_URL)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("Download failed: {}", response.status()));
    }

    let mut file = std::fs::File::create(&partial_path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut completed: u64 = 0;
    let mut last_emit = Instant::now();
    emit_progress(&app, "downloading", Some(0));

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("Download interrupted: {}", e))?
    {
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        hasher.update(&chunk);
        completed += chunk.len() as u64;
        if last_emit.elapsed() >= Duration::from_millis(200) {
            emit_progress(&app, "downloading", Some(completed));
            last_emit = Instant::now();
        }
    }
    file.flush().map_err(|e| e.to_string())?;
    drop(file);

    emit_progress(&app, "verifying", Some(completed));
    let digest = format!("{:x}", hasher.finalize());
    if completed != MODEL_SIZE || digest != MODEL_SHA256 {
        let _ = std::fs::remove_file(&partial_path);
        return Err("The downloaded model failed its integrity check. Please try again.".into());
    }

    std::fs::rename(&partial_path, &final_path).map_err(|e| e.to_string())?;
    emit_progress(&app, "success", Some(MODEL_SIZE));
    info!("LLM model downloaded to {}", final_path.display());
    Ok(())
}

// ---------------------------------------------------------------- orphan guard

/// Ties the engine to Murmure's lifetime: on Windows the child joins a job
/// object that the OS kills when Murmure exits, even after a crash, so the
/// engine never lingers holding gigabytes of VRAM.
#[cfg(target_os = "windows")]
mod kill_with_app {
    use std::os::windows::io::AsRawHandle;
    use std::process::Child;
    use std::sync::OnceLock;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };

    static JOB: OnceLock<Option<usize>> = OnceLock::new();

    fn job() -> Option<usize> {
        *JOB.get_or_init(|| unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return None;
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            // The handle is intentionally never closed: the OS closes it
            // when Murmure exits, which kills every process in the job.
            (ok != 0).then_some(handle as usize)
        })
    }

    pub fn attach(child: &Child) {
        if let Some(job) = job() {
            unsafe {
                AssignProcessToJobObject(job as _, child.as_raw_handle() as _);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod kill_with_app {
    pub fn attach(_child: &std::process::Child) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAPTOP: &str = "Available devices:\n  Vulkan0: NVIDIA RTX 3500 Ada Generation Laptop GPU (12015 MiB, 11247 MiB free)\n  Vulkan1: Intel(R) RaptorLake-S Mobile Graphics Controller (16220 MiB, 15452 MiB free)\n";

    #[test]
    fn should_parse_vulkan_devices() {
        let devices = parse_devices(LAPTOP);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].id, "Vulkan0");
        assert_eq!(
            devices[1].name,
            "Intel(R) RaptorLake-S Mobile Graphics Controller"
        );
    }

    #[test]
    fn should_prefer_the_dedicated_gpu_even_when_listed_second() {
        let devices = vec![
            GpuDevice {
                id: "Vulkan0".into(),
                name: "Intel(R) UHD Graphics 770".into(),
            },
            GpuDevice {
                id: "Vulkan1".into(),
                name: "AMD Radeon RX 7800 XT".into(),
            },
        ];
        assert_eq!(pick_device(&devices).unwrap().id, "Vulkan1");
    }

    #[test]
    fn should_fall_back_to_the_integrated_gpu_when_it_is_the_only_one() {
        let devices = vec![GpuDevice {
            id: "Vulkan0".into(),
            name: "AMD Radeon(TM) Graphics".into(),
        }];
        assert_eq!(pick_device(&devices).unwrap().id, "Vulkan0");
        assert!(pick_device(&[]).is_none());
    }

    #[test]
    fn should_treat_intel_arc_as_dedicated() {
        assert!(!is_integrated_gpu("Intel(R) Arc(TM) A770 Graphics"));
        assert!(is_integrated_gpu("Intel(R) Iris(R) Xe Graphics"));
    }

    #[test]
    fn should_strip_thinking_blocks() {
        assert_eq!(strip_thinking("<think>\nhmm\n</think>\n\nHello."), "Hello.");
        assert_eq!(strip_thinking("reasoning</think>Hello."), "Hello.");
        assert_eq!(strip_thinking("  Hello.  "), "Hello.");
    }
}
