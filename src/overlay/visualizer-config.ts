export type OverlaySize = 'small' | 'medium' | 'large';

// The canvas is taller than the resting capsule (25% of its height each side
// of centre) so drops have room to flow out above and below it.
export const VISUALIZER_CONFIG: Record<
    OverlaySize,
    { bars: number; pixelWidth: number; pixelHeight: number; blobs: number; className: string }
> = {
    small: { bars: 14, pixelWidth: 2, pixelHeight: 2, blobs: 4, className: 'w-[104px] h-[56px]' },
    medium: { bars: 16, pixelWidth: 2, pixelHeight: 2, blobs: 5, className: 'w-[144px] h-[64px]' },
    large: { bars: 24, pixelWidth: 3, pixelHeight: 3, blobs: 7, className: 'w-[224px] h-[72px]' },
};
