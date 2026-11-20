import type { ColorScheme } from '@/components/lib/color-scheme';

export type GlassStyle = 'clear' | 'frosted';

type Rgb = [number, number, number];

// Glow colour per recording mode, left â†’ centre â†’ right, matching the pixel
// visualizer's palettes so modes stay recognisable through the glass.
const PALETTES: Record<ColorScheme, [Rgb, Rgb, Rgb]> = {
    standard: [
        [59, 130, 246],
        [34, 211, 238],
        [99, 102, 241],
    ],
    llm: [
        [171, 48, 232],
        [221, 60, 221],
        [128, 77, 230],
    ],
    command: [
        [233, 86, 12],
        [249, 200, 6],
        [249, 148, 6],
    ],
};

const FIELD_THRESHOLD = 1;
const EDGE_SOFTNESS = 0.08; // field units over which the edge anti-aliases
const LEVEL_GAIN = 1.6;
const LEVEL_ATTACK = 0.18;
const LEVEL_RELEASE = 0.06;
// Per-drop easing (per second): each drop chases the voice level on its own
// timescale, so size and reach glide rather than snap with each level update.
const DRIVE_EASE = 5;
// A drop may travel at most this far past the tube's surface, as a fraction
// of its own radius, so it always stays joined by a neck and never detaches.
const NECK_HOLD = 0.9;
const IDLE_SWELL = 0.12;
const CAPSULE_RADIUS = 0.19; // of canvas height; the resting glass pill
const LIGHT: Rgb = normalize([-0.45, -0.7, 0.55]);
const HALF_VECTOR: Rgb = normalize([LIGHT[0], LIGHT[1], LIGHT[2] + 1]);

interface Blob {
    baseX: number; // 0..1 along the capsule
    speed: number; // radians per second of the vertical flow
    phase: number;
    weight: number; // how strongly this drop reacts to the voice
    drive: number; // smoothed 0..1 swell, eased towards the voice every frame
    ease: number; // per-drop easing multiplier so neighbours lag each other
}

export interface LiquidState {
    level: number;
    displayed: number;
    isProcessing: boolean;
    time: number;
    lastFrame: number | null;
    blobs: Blob[];
    rafId: number | null;
}

export interface LiquidSurface {
    ctx: CanvasRenderingContext2D;
    width: number;
    height: number;
    image: ImageData;
    colorScheme: ColorScheme;
    glassStyle: GlassStyle;
    thickness: number; // scales capsule and drop size; travel stays height-relative
}

function normalize(v: Rgb): Rgb {
    const len = Math.hypot(v[0], v[1], v[2]);
    return [v[0] / len, v[1] / len, v[2] / len];
}

const smoothstep = (edge0: number, edge1: number, x: number) => {
    const t = Math.min(1, Math.max(0, (x - edge0) / (edge1 - edge0)));
    return t * t * (3 - 2 * t);
};

const mix = (a: Rgb, b: Rgb, t: number): Rgb => [
    a[0] + (b[0] - a[0]) * t,
    a[1] + (b[1] - a[1]) * t,
    a[2] + (b[2] - a[2]) * t,
];

const rampColor = (ramp: [Rgb, Rgb, Rgb], x: number): Rgb =>
    x < 0.5 ? mix(ramp[0], ramp[1], x * 2) : mix(ramp[1], ramp[2], (x - 0.5) * 2);

// Thin-film rainbow for the iridescent glint along the lower rim.
const iridescence = (t: number): Rgb => [
    160 + 95 * Math.cos(6.283 * (t + 0.0)),
    160 + 95 * Math.cos(6.283 * (t + 0.33)),
    160 + 95 * Math.cos(6.283 * (t + 0.67)),
];

export const createLiquidState = (blobCount: number): LiquidState => ({
    level: 0,
    displayed: 0,
    isProcessing: false,
    time: 0,
    lastFrame: null,
    rafId: null,
    blobs: Array.from({ length: blobCount }, (_, i) => {
        const centred = 1 - Math.abs((i + 0.5) / blobCount - 0.5) * 2;
        return {
            baseX: (i + 0.5) / blobCount,
            speed: 2.2 + ((i * 7) % 5) * 0.18,
            // Alternate drops start on opposite sides so the capsule bulges
            // up and down at once, like liquid sloshing.
            phase: i * 0.95 + (i % 2) * Math.PI,
            weight: 0.6 + 0.4 * centred,
            drive: IDLE_SWELL,
            ease: 0.7 + ((i * 3) % 4) * 0.15,
        };
    }),
});

export const createLiquidSurface = (
    ctx: CanvasRenderingContext2D,
    width: number,
    height: number,
    colorScheme: ColorScheme,
    glassStyle: GlassStyle,
    thickness = 1
): LiquidSurface => ({
    ctx,
    width,
    height,
    image: ctx.createImageData(width, height),
    colorScheme,
    glassStyle,
    thickness,
});

export const advanceLiquid = (state: LiquidState, now: number) => {
    const dt = state.lastFrame == null ? 1 / 60 : Math.min(0.05, (now - state.lastFrame) / 1000);
    state.lastFrame = now;
    state.time += dt;

    // The backend sends linear RMS; the square root lifts normal speech
    // into the expressive range, as the pixel visualizer does.
    const target = Math.min(1, Math.sqrt(state.level) * LEVEL_GAIN);
    const rate = target > state.displayed ? LEVEL_ATTACK : LEVEL_RELEASE;
    state.displayed += (target - state.displayed) * rate;

    const energy = state.isProcessing ? 0.4 : IDLE_SWELL + state.displayed * (1 - IDLE_SWELL);
    for (const blob of state.blobs) {
        // Frame-rate independent exponential ease.
        const k = 1 - Math.exp(-DRIVE_EASE * blob.ease * dt);
        blob.drive += (energy * blob.weight - blob.drive) * k;
    }
};

interface Geometry {
    capsuleY: number;
    capsuleX0: number;
    capsuleX1: number;
    capsuleR2: number;
    blobs: { x: number; y: number; r2: number }[];
    voice: number;
}

const placeShapes = (state: LiquidState, width: number, height: number, thickness: number): Geometry => {
    const midY = height / 2;
    const capsuleR = height * CAPSULE_RADIUS * thickness;
    const x0 = capsuleR + 2;
    const x1 = width - capsuleR - 2;
    const span = x1 - x0;
    const spacing = span / state.blobs.length;

    const blobs = state.blobs.map((blob) => {
        const drive = blob.drive;
        // Silent drops shrink inside the capsule so its outline rests clean.
        // Drops thin out less than the tube does, so a slim tube still swells.
        const dropScale = 0.5 + 0.5 * thickness;
        const radius = Math.min(spacing * 0.75, height * (0.07 + 0.24 * drive) * dropScale);
        const flow = Math.sin(state.time * blob.speed + blob.phase);
        // At rest the drops sit inside the capsule; the voice pushes them out
        // above and below it, stretching a neck but never breaking it.
        const travel = Math.max(0, midY - radius - 2);
        const neckLimit = capsuleR + radius * NECK_HOLD;
        const amplitude = Math.min(travel, neckLimit, height * (0.02 + 0.5 * drive));
        const drift = state.isProcessing ? Math.sin(state.time * 1.3 + blob.phase) * spacing * 0.6 : 0;
        return {
            x: Math.min(x1, Math.max(x0, x0 + blob.baseX * span + drift)),
            y: midY + flow * amplitude,
            r2: radius * radius,
        };
    });

    const voice = state.isProcessing ? 0.5 : state.displayed;
    return { capsuleY: midY, capsuleX0: x0, capsuleX1: x1, capsuleR2: capsuleR * capsuleR, blobs, voice };
};

const fieldAt = (g: Geometry, px: number, py: number) => {
    const cx = Math.min(g.capsuleX1, Math.max(g.capsuleX0, px));
    const cdx = px - cx;
    const cdy = py - g.capsuleY;
    let field = g.capsuleR2 / (cdx * cdx + cdy * cdy + 1);
    for (const b of g.blobs) {
        const dx = px - b.x;
        const dy = py - b.y;
        field += b.r2 / (dx * dx + dy * dy + 1);
    }
    return field;
};

// Liquid glass has a flat face and a rounded bevel only at the rim: a
// quarter-circle profile over a narrow band of the field above threshold.
// Inside that band the face is flat, so merged drops show no seams.
const BEVEL = 0.7;
const bevelHeight = (field: number) => {
    const t = Math.min(1, Math.max(0, (field - FIELD_THRESHOLD) / BEVEL));
    return Math.sqrt(1 - (1 - t) * (1 - t));
};
const heightAt = (g: Geometry, px: number, py: number) => bevelHeight(fieldAt(g, px, py));

export const drawLiquid = (surface: LiquidSurface | null, state: LiquidState) => {
    if (surface == null) return;
    const { ctx, width, height, image, colorScheme, glassStyle } = surface;
    const g = placeShapes(state, width, height, surface.thickness);
    const ramp = PALETTES[colorScheme];
    const frosted = glassStyle === 'frosted';
    const data = image.data;
    const relief = height * 0.32;

    for (let py = 0; py < height; py++) {
        for (let px = 0; px < width; px++) {
            const i = (py * width + px) * 4;
            const field = fieldAt(g, px + 0.5, py + 0.5);
            const coverage = smoothstep(FIELD_THRESHOLD - EDGE_SOFTNESS, FIELD_THRESHOLD + EDGE_SOFTNESS, field);
            if (coverage <= 0) {
                data[i + 3] = 0;
                continue;
            }

            const h = bevelHeight(field);
            let nx = -(heightAt(g, px + 1.5, py + 0.5) - h) * relief;
            let ny = -(heightAt(g, px + 0.5, py + 1.5) - h) * relief;
            let nz = 1;
            const len = Math.hypot(nx, ny, nz);
            nx /= len;
            ny /= len;
            nz /= len;

            // How close to the silhouette we are: 0 on the flat face, 1 at the rim.
            const rim = Math.pow(1 - nz, 1.4);
            const facing = nx * LIGHT[0] + ny * LIGHT[1];
            const specular = Math.pow(Math.max(0, nx * HALF_VECTOR[0] + ny * HALF_VECTOR[1] + nz * HALF_VECTOR[2]), 24);

            // Composite layers front to back onto transparent, "over" style.
            let r = 0;
            let gr = 0;
            let b = 0;
            let a = 0;
            const over = (cr: number, cg: number, cb: number, ca: number) => {
                const na = ca + a * (1 - ca);
                if (na <= 0) return;
                r = (cr * ca + r * a * (1 - ca)) / na;
                gr = (cg * ca + gr * a * (1 - ca)) / na;
                b = (cb * ca + b * a * (1 - ca)) / na;
                a = na;
            };

            // Body: clear glass barely tints; frosted glass is milky and
            // brighter towards the top, where it catches more light.
            if (frosted) {
                const milk = 0.42 + 0.16 * (1 - py / height);
                over(236, 241, 250, milk);
            } else {
                over(255, 255, 255, 0.07);
            }

            // Voice glow: the mode colour pools in the thick middle of the glass.
            const glowColor = rampColor(ramp, px / width);
            const glow = (frosted ? 0.08 : 0.02) + 0.38 * g.voice;
            const depth = smoothstep(FIELD_THRESHOLD, FIELD_THRESHOLD * 3, field);
            over(glowColor[0], glowColor[1], glowColor[2], glow * depth * (frosted ? 0.7 : 1));

            // Refraction band: just inside the rim the glass bends the
            // background away and reads as a darker line.
            const band = smoothstep(0.08, 0.3, rim) * (1 - smoothstep(0.35, 0.7, rim));
            over(20, 24, 34, band * (frosted ? 0.14 : 0.3));

            // Bright rim, strongest where it faces the light and the bounce opposite it.
            const rimLight = smoothstep(0.3, 0.85, rim) * (0.5 + 0.5 * Math.abs(facing));
            over(255, 255, 255, Math.min(1, rimLight * 0.95));

            // Iridescent glint along the lower edge.
            const lower = smoothstep(0.15, 0.85, ny) * smoothstep(0.5, 0.95, rim);
            if (lower > 0) {
                const film = iridescence(px / width + ny * 0.3);
                over(film[0], film[1], film[2], lower * 0.45);
            }

            over(255, 255, 255, Math.min(1, specular * smoothstep(0.2, 0.7, rim)));

            data[i] = r;
            data[i + 1] = gr;
            data[i + 2] = b;
            data[i + 3] = a * coverage * 255;
        }
    }

    ctx.putImageData(image, 0, 0);
};
