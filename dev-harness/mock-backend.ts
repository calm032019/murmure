// Dev-only: lets the UI run in a plain browser so the theme can be
// iterated on without a Rust build. Not part of the Tauri build inputs.
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';

const settings: Record<string, unknown> = {
    record_shortcut: 'ctrl+space',
    last_transcript_shortcut: 'ctrl+shift+space',
    command_shortcut: 'ctrl+shift+x',
    llm_mode_1_shortcut: 'ctrl+shift+1',
    llm_mode_2_shortcut: 'ctrl+shift+2',
    llm_mode_3_shortcut: 'ctrl+shift+3',
    llm_mode_4_shortcut: 'ctrl+shift+4',
    cancel_shortcut: 'ctrl+backspace',
    record_mode: 'toggle_to_talk',
    overlay_mode: 'recording',
    overlay_position: 'bottom',
    overlay_size: 'small',
    api_enabled: false,
    api_port: 4800,
    copy_to_clipboard: false,
    paste_method: 'ctrl_v',
    auto_insert: true,
    persist_history: true,
    language: 'en',
    sound_enabled: true,
    sound_volume: 50,
    lower_output_while_recording: true,
    output_volume_while_recording: 50,
    log_level: 'info',
    streaming_preview: true,
    streaming_text_width: 450,
    streaming_font_size: 11,
    streaming_max_lines: 5,
    result_panel_mode: 'command',
    result_panel_duration_secs: 5,
    show_in_dock: true,
    silence_timeout_ms: 1500,
    onboarding: {
        used_home_shortcut: true,
        transcribed_outside_app: true,
        added_dictionary_word: false,
        congrats_dismissed: false,
    },
};

const transcriptions = [
    { id: 1, text: 'Can you send the revised quote over to the site team before lunch?', timestamp: Date.now() / 1000 - 120 },
    { id: 2, text: 'Remind me to check the delivery window for Thursday.', timestamp: Date.now() / 1000 - 3600 },
    { id: 3, text: 'The shelter frame arrives on the second truck, not the first.', timestamp: Date.now() / 1000 - 7200 },
];

const responses: Record<string, unknown> = {
    get_all_settings: settings,
    get_current_language: 'en',
    get_recording_mode: 'standard',
    get_onboarding_state: settings.onboarding,
    get_recent_transcriptions: transcriptions,
    get_usage_stats: { words_this_month: 12480, total_words: 48211, wpm: 148, time_saved_minutes: 312 },
    get_mic_list: [
        { id: 'default', label: 'Default microphone' },
        { id: 'usb', label: 'Microphone (Realtek USB Audio)' },
    ],
    get_current_mic_id: 'usb',
    get_current_mic_label: 'Microphone (Realtek USB Audio)',
    get_dictionary: ['Zappshelter', 'Murmure', 'Parakeet'],
    get_formatting_settings: { rules: [], built_in: {} },
    get_llm_connect_settings: { modes: [], active_mode_index: 0, url: '', model: '', prompts: [] },
    get_linux_session_type: 'x11',
    get_layout_fallback_state: null,
    is_transform_processing: false,
    get_wake_word_enabled: false,
    get_api_enabled: false,
    get_api_port: 4800,
    get_silence_timeout_ms: 1500,
    get_paired_devices: [],
    get_smartmic_enabled: false,
    get_http_api_status: false,
    'plugin:app|version': '2.0.3',
    'plugin:app|name': 'Murmure',
    'plugin:autostart|is_enabled': false,
    'plugin:updater|check': null,
};

export const installMockBackend = (label: string, overrides: Record<string, unknown> = {}) => {
    mockWindows(label);
    Object.assign(settings, overrides);
    mockIPC(
        (cmd) => {
            if (cmd in responses) return responses[cmd];
            if (cmd.startsWith('get_')) {
                console.warn('[mock] unmocked getter', cmd);
            }
            return null;
        },
        { shouldMockEvents: true }
    );
};
