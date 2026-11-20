use crate::audio::types::RecordingMode;
use crate::dictionary;
use crate::llm::helpers::{
    load_llm_connect_settings, load_remote_api_key, resolve_api_prompt, validate_remote_request,
};
use crate::llm::types::SecretString;
use crate::llm::types::{
    ApiLlmRequest, LLMConnectSettings, LLMProvider, OllamaModel, OpenAIChatMessage,
    OpenAIChatRequest, OpenAIChatResponse, OpenAIModelsResponse,
};
use log::warn;
use std::sync::LazyLock;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Shared client — rebuilding per request churns TLS, DNS, and pool allocations.
static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

fn normalize_url(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

fn with_bearer_auth(
    request: reqwest::RequestBuilder,
    api_key: Option<&str>,
) -> reqwest::RequestBuilder {
    match api_key {
        Some(key) if !key.is_empty() => request.header("Authorization", format!("Bearer {}", key)),
        _ => request,
    }
}

fn map_remote_http_error(status: reqwest::StatusCode) -> String {
    match status.as_u16() {
        401 | 403 => "Authentication failed. Check your API key.".to_string(),
        _ => format!("Server returned error: {}", status),
    }
}

/// Extracts the `<role>...</role>` block from the prompt as a system prompt.
/// Returns (system_prompt, user_prompt). If no `<role>` tag is found,
/// system_prompt is None and the full prompt is returned as user_prompt.
fn extract_system_prompt(prompt: &str) -> (Option<String>, String) {
    let Some(start) = prompt.find("<role>") else {
        return (None, prompt.to_string());
    };
    let Some(end) = prompt.find("</role>") else {
        return (None, prompt.to_string());
    };

    let system = prompt[start + "<role>".len()..end].trim().to_string();
    let user = format!(
        "{}{}",
        prompt[..start].trim(),
        prompt[end + "</role>".len()..].trim()
    )
    .trim()
    .to_string();

    if system.is_empty() {
        (None, user)
    } else {
        (Some(system), user)
    }
}

async fn generate_remote(
    remote_url: &str,
    api_key: Option<&SecretString>,
    model: &str,
    system_prompt: Option<&str>,
    user_prompt: &str,
) -> Result<String, String> {
    let key_str = api_key.map(|k| k.expose());
    validate_remote_request(remote_url, key_str)?;

    let url = format!("{}/chat/completions", normalize_url(remote_url));

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
        model: model.to_string(),
        messages,
        stream: false,
        think: None,
        temperature: None,
        chat_template_kwargs: None,
    };

    let request = with_bearer_auth(
        HTTP_CLIENT
            .post(&url)
            .json(&request_body)
            .timeout(Duration::from_secs(60)),
        key_str,
    );

    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to connect to remote server: {}", e))?;

    if !response.status().is_success() {
        return Err(map_remote_http_error(response.status()));
    }

    let chat_response: OpenAIChatResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse remote response: {}", e))?;

    chat_response
        .choices
        .first()
        .map(|c| c.message.content.trim().to_string())
        .ok_or_else(|| "Remote server returned empty response".to_string())
}

async fn dispatch_to_llm(
    app: &AppHandle,
    settings: &LLMConnectSettings,
    provider: &LLMProvider,
    model: &str,
    system_prompt: Option<&str>,
    user_prompt: &str,
    emit_ui: bool,
) -> Result<String, String> {
    if model.is_empty() {
        return Err("No model selected".to_string());
    }

    if emit_ui {
        let _ = app.emit("llm-processing-start", ());
    }

    let result = match provider {
        LLMProvider::Local => crate::llm::engine::generate(app, system_prompt, user_prompt).await,
        LLMProvider::Remote => {
            let api_key = load_remote_api_key();
            generate_remote(
                &settings.remote_url,
                api_key.as_ref(),
                model,
                system_prompt,
                user_prompt,
            )
            .await
        }
    };

    if emit_ui {
        let _ = app.emit("llm-processing-end", ());
    }
    result
}

fn active_mode_target(settings: &LLMConnectSettings) -> Result<(LLMProvider, String), String> {
    if !settings.is_enabled() {
        return Err("Prompt Mode is disabled".to_string());
    }
    settings
        .modes
        .get(settings.active_mode_index)
        .map(|mode| (mode.provider.clone(), mode.model.clone()))
        .ok_or_else(|| "No active mode selected".to_string())
}

pub async fn post_process_with_llm(
    app: &AppHandle,
    transcription: String,
    force_bypass: bool,
) -> Result<String, String> {
    if force_bypass {
        return Ok(transcription);
    }

    let settings = load_llm_connect_settings(app);
    post_process_with_mode(
        app,
        &settings,
        transcription,
        settings.active_mode_index,
        true,
    )
    .await
}

pub async fn post_process_with_mode(
    app: &AppHandle,
    settings: &LLMConnectSettings,
    transcription: String,
    index: usize,
    emit_ui: bool,
) -> Result<String, String> {
    let mode = settings.modes.get(index).ok_or("No active mode selected")?;

    let dictionary_words = dictionary::load(app)
        .unwrap_or_default()
        .into_iter()
        .collect::<Vec<String>>()
        .join(", ");

    let prompt = mode
        .prompt
        .replace("{{TRANSCRIPT}}", &transcription)
        .replace("{transcript}", &transcription)
        .replace("{{DICTIONARY}}", &dictionary_words)
        .replace("{dictionary}", &dictionary_words);

    let (system_prompt, user_prompt) = extract_system_prompt(&prompt);
    if !settings.is_enabled() {
        return Err("Prompt Mode is disabled".to_string());
    }
    dispatch_to_llm(
        app,
        settings,
        &mode.provider,
        &mode.model,
        system_prompt.as_deref(),
        &user_prompt,
        emit_ui,
    )
    .await
}

pub async fn process_api_custom_prompt(
    app: &AppHandle,
    settings: &LLMConnectSettings,
    provider: &LLMProvider,
    model: &str,
    instruction: &str,
    transcription: &str,
) -> Result<String, String> {
    dispatch_to_llm(
        app,
        settings,
        provider,
        model,
        Some(instruction),
        transcription,
        false,
    )
    .await
}

pub async fn process_api_request(
    app: &AppHandle,
    request: &ApiLlmRequest,
    transcription: String,
) -> Result<String, String> {
    let settings = load_llm_connect_settings(app);
    match request {
        ApiLlmRequest::Custom {
            provider,
            model,
            instruction,
        } => {
            process_api_custom_prompt(app, &settings, provider, model, instruction, &transcription)
                .await
        }
        ApiLlmRequest::SavedPrompt { name } => {
            let index = resolve_api_prompt(&settings, name).map_err(|e| e.to_string())?;
            post_process_with_mode(app, &settings, transcription, index, false).await
        }
    }
}

/// The built-in engine ships with Murmure, so "connecting" means its files are present.
pub async fn test_local_engine(app: &AppHandle) -> Result<bool, String> {
    if crate::llm::engine::is_engine_available(app) {
        Ok(true)
    } else {
        Err("The built-in AI engine is missing from this install.".to_string())
    }
}

/// Lists the built-in model once it is installed.
pub async fn fetch_local_models(app: &AppHandle) -> Result<Vec<OllamaModel>, String> {
    Ok(match crate::llm::engine::is_model_available(app) {
        true => vec![OllamaModel {
            name: crate::llm::engine::MODEL_ID.to_string(),
        }],
        false => Vec::new(),
    })
}

async fn fetch_openai_models_raw(
    url: &str,
    api_key: Option<&SecretString>,
) -> Result<OpenAIModelsResponse, String> {
    let key_str = api_key.map(|k| k.expose());
    validate_remote_request(url, key_str)?;

    let models_url = format!("{}/models", normalize_url(url));

    let request = with_bearer_auth(
        HTTP_CLIENT
            .get(&models_url)
            .timeout(Duration::from_secs(10)),
        key_str,
    );

    let response = request
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    if !response.status().is_success() {
        return Err(map_remote_http_error(response.status()));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

pub async fn test_remote_connection(
    url: String,
    api_key: Option<SecretString>,
) -> Result<usize, String> {
    let response = fetch_openai_models_raw(&url, api_key.as_ref()).await?;
    Ok(response.data.len())
}

pub async fn fetch_remote_models(
    url: String,
    api_key: Option<SecretString>,
) -> Result<Vec<OllamaModel>, String> {
    let response = fetch_openai_models_raw(&url, api_key.as_ref()).await?;
    Ok(response
        .data
        .into_iter()
        .map(|m| OllamaModel { name: m.id })
        .collect())
}

pub async fn warmup_ollama_model(app: &AppHandle, mode: RecordingMode) -> Result<(), String> {
    let settings = load_llm_connect_settings(app);
    let provider = match mode {
        RecordingMode::Standard | RecordingMode::Llm => match active_mode_target(&settings) {
            Ok((provider, _)) => provider,
            Err(_) => return Ok(()),
        },
    };
    match provider {
        LLMProvider::Local => crate::llm::engine::warmup(app).await,
        LLMProvider::Remote => Ok(()),
    }
}

pub fn warmup_ollama_model_background(app: &AppHandle, mode: RecordingMode) {
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = warmup_ollama_model(&app_handle, mode).await {
            warn!("LLM warmup failed: {}", e);
        }
    });
}

/// Same as `switch_active_mode` but skips the overlay flash, for the
/// wake-word LLM path: a back-to-back recording overlay would race the
/// flash and leave the final window invisible.
pub fn switch_active_mode_silent(app: &AppHandle, index: usize) {
    switch_active_mode_internal(app, index, false);
}

fn switch_active_mode_internal(app: &AppHandle, index: usize, flash: bool) {
    let mut settings = load_llm_connect_settings(app);
    if !settings.is_enabled() {
        warn!(
            "LLM Connect disabled: switch_active_mode({}) ignored",
            index
        );
        return;
    }
    let Some(mode) = settings.modes.get(index) else {
        return;
    };
    let mode_name = mode.name.clone();

    // Always flash, even when re-pressing the shortcut for the already-active
    // mode: the user expects visual feedback for every press.
    if settings.active_mode_index != index {
        settings.active_mode_index = index;
        if crate::llm::helpers::save_llm_connect_settings(app, &settings).is_ok() {
            let _ = app.emit("llm-settings-updated", &settings);
        }
    }

    if flash {
        crate::overlay::overlay::flash_text_in_overlay_internal(app, mode_name);
    }
}
