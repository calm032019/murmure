import { LiquidVisualizer } from '@/overlay/liquid-visualizer/liquid-visualizer';
import { useGlassStyle } from '@/components/hooks/use-glass-style';
import { useLLMState } from './audio-visualizer/hooks/use-llm-state';
import { History } from './history/history';
import { Page } from '@/components/page';
import { Statistics } from './statistics/statistics';
import { WaylandCliOnboardingCard } from '../onboarding/wayland-cli-onboarding-card/wayland-cli-onboarding-card';
import { WaylandClipboardFallbackCard } from '../onboarding/wayland-clipboard-fallback-card/wayland-clipboard-fallback-card';
import { useIsWayland } from '@/components/hooks/use-linux-session-type';
import { RecordLabel } from '@/components/record-label';
import { MicLabel } from './mic-label/mic-label';
import { MicDisconnectedBanner } from './mic-disconnected-banner/mic-disconnected-banner';

export const Home = () => {
    const isWayland = useIsWayland();
    const glassStyle = useGlassStyle();
    const { isProcessing } = useLLMState();

    return (
        <main className="space-y-4 relative">
            <Page.Header className="space-y-4">
                <Statistics />
            </Page.Header>
            {isWayland && <WaylandCliOnboardingCard />}
            <MicDisconnectedBanner />

            <div className="space-y-4">
                <div className="space-y-2 flex flex-col items-center">
                    <div className="w-full rounded-2xl border border-surface-border bg-surface p-2 space-y-4 relative">
                        <LiquidVisualizer
                            className="h-44"
                            blobs={9}
                            resolution={0.5}
                            thickness={0.5}
                            glassStyle={glassStyle}
                            isProcessing={isProcessing}
                        />
                        <MicLabel />
                        <RecordLabel />
                    </div>
                </div>

                {isWayland && <WaylandClipboardFallbackCard />}

                <div className="flex justify-center pt-4">
                    <History />
                </div>
            </div>
        </main>
    );
};
