import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import type { AppSettings } from '@/features/settings/settings.types';

export type GlassStyle = 'clear' | 'frosted';

const isGlassStyle = (value: unknown): value is GlassStyle => value === 'clear' || value === 'frosted';

export const saveGlassStyle = (style: string) => {
    if (isGlassStyle(style)) invoke('set_glass_style', { style }).catch(() => {});
};

// Shared by the main window and the overlay; the backend broadcasts
// `glass-style-changed` to both when the setting changes.
export const useGlassStyle = () => {
    const [glassStyle, setGlassStyle] = useState<GlassStyle>('frosted');

    useEffect(() => {
        invoke<AppSettings>('get_all_settings')
            .then((settings) => {
                if (isGlassStyle(settings.glass_style)) setGlassStyle(settings.glass_style);
            })
            .catch(() => {});
        const unlisten = listen<string>('glass-style-changed', (event) => {
            if (isGlassStyle(event.payload)) setGlassStyle(event.payload);
        });
        return () => {
            unlisten.then((u) => u());
        };
    }, []);

    // The CSS theme keys off this attribute (see tailwind.css).
    useEffect(() => {
        document.documentElement.dataset.glass = glassStyle;
    }, [glassStyle]);

    return glassStyle;
};
