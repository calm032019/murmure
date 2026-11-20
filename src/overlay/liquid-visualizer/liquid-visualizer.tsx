import { useEffect, useRef } from 'react';
import { cn } from '@/components/lib/utils';
import { useLevelState } from '@/features/home/audio-visualizer/hooks/use-level-state';
import type { ColorScheme } from '@/components/lib/color-scheme';
import {
    advanceLiquid,
    createLiquidState,
    createLiquidSurface,
    drawLiquid,
    type GlassStyle,
    type LiquidState,
    type LiquidSurface,
} from './liquid-visualizer.helpers';

interface LiquidVisualizerProps {
    blobs: number;
    colorScheme?: ColorScheme;
    glassStyle?: GlassStyle;
    isProcessing?: boolean;
    level?: number;
    // Fraction of device resolution to shade at. The glass is computed per
    // pixel, so large canvases render smaller and let CSS scale them up.
    resolution?: number;
    thickness?: number;
    className?: string;
}

export const LiquidVisualizer = ({
    blobs,
    colorScheme = 'standard',
    glassStyle = 'clear',
    isProcessing = false,
    level: levelProp,
    resolution = 1,
    thickness = 1,
    className,
}: LiquidVisualizerProps) => {
    const { level: micLevel } = useLevelState();
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const surfaceRef = useRef<LiquidSurface | null>(null);
    const stateRef = useRef<LiquidState | null>(null);
    if (stateRef.current == null || stateRef.current.blobs.length !== blobs) {
        stateRef.current = createLiquidState(blobs);
    }

    useEffect(() => {
        const canvas = canvasRef.current;
        const ctx = canvas?.getContext('2d');
        if (canvas == null || ctx == null) return;

        const observer = new ResizeObserver(() => {
            const pixelRatio = window.devicePixelRatio * resolution;
            const width = Math.round(canvas.clientWidth * pixelRatio);
            const height = Math.round(canvas.clientHeight * pixelRatio);
            if (width === 0 || height === 0) return;
            canvas.width = width;
            canvas.height = height;
            surfaceRef.current = createLiquidSurface(ctx, width, height, colorScheme, glassStyle, thickness);
        });
        observer.observe(canvas);

        // The liquid never fully rests (it breathes when silent), so it runs
        // for as long as the overlay is on screen.
        const frame = (now: number) => {
            const state = stateRef.current;
            if (state != null) {
                advanceLiquid(state, now);
                drawLiquid(surfaceRef.current, state);
                state.rafId = requestAnimationFrame(frame);
            }
        };
        const start = () => {
            const state = stateRef.current;
            if (state == null || state.rafId !== null) return;
            state.lastFrame = null;
            state.rafId = requestAnimationFrame(frame);
        };
        const stop = () => {
            const state = stateRef.current;
            if (state?.rafId != null) cancelAnimationFrame(state.rafId);
            if (state != null) state.rafId = null;
        };
        const onVisibility = () => (document.hidden ? stop() : start());

        start();
        document.addEventListener('visibilitychange', onVisibility);
        return () => {
            document.removeEventListener('visibilitychange', onVisibility);
            observer.disconnect();
            stop();
            surfaceRef.current = null;
        };
    }, [blobs, colorScheme, glassStyle, resolution, thickness]);

    useEffect(() => {
        const state = stateRef.current;
        if (state == null) return;
        state.level = levelProp ?? micLevel;
        state.isProcessing = isProcessing;
    }, [levelProp, micLevel, isProcessing]);

    // The glass is mostly transparent, so a soft shadow is what lifts it off
    // whatever is behind the overlay.
    return (
        <canvas
            ref={canvasRef}
            className={cn('block h-full w-full', '[filter:drop-shadow(0_3px_6px_rgb(0_0_0/0.35))]', className)}
        />
    );
};
