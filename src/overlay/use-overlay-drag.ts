import { invoke } from '@tauri-apps/api/core';
import type { MouseEvent } from 'react';

const PRIMARY_BUTTON = 0;

export const useOverlayDrag = () => {
    const onMouseDown = (event: MouseEvent) => {
        if (event.button !== PRIMARY_BUTTON) return;
        invoke('start_overlay_drag').catch(() => {});
    };

    // Double-click snaps the overlay back to its top/bottom placement.
    const onDoubleClick = () => {
        invoke('reset_overlay_custom_position').catch(() => {});
    };

    return { onMouseDown, onDoubleClick };
};
