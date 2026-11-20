import { useLLMState } from '@/features/home/audio-visualizer/hooks/use-llm-state';
import { useStreamingState } from './streaming-text/use-streaming-state';
import { StreamingText } from './streaming-text/streaming-text';
import clsx from 'clsx';
import { VISUALIZER_CONFIG } from './visualizer-config';
import { useOverlayConfig } from './use-overlay-config';
import { useRecordingMode } from './use-recording-mode';
import { useOverlayError } from './use-overlay-error';
import { useModeFlash } from './mode-flash/hooks/use-mode-flash';
import { useLlmPromptFlash } from './mode-flash/hooks/use-llm-prompt-flash';
import { ModeFlash } from './mode-flash/mode-flash';
import { CancelButton } from './cancel-button';
import { useOverlayInputRegion } from './use-overlay-input-region';
import { useOverlayDrag } from './use-overlay-drag';
import { useTransformProcessing } from './use-transform-processing';
import { ResultPanel } from './result-panel/result-panel';
import { LiquidVisualizer } from './liquid-visualizer/liquid-visualizer';
import { useGlassStyle } from '@/components/hooks/use-glass-style';
import { useResultPanel } from './result-panel/hooks/use-result-panel';

export const Overlay = () => {
    const { overlaySize, overlayPosition, streamingTextSettings, resultPanelDurationSecs } = useOverlayConfig();
    const glassStyle = useGlassStyle();
    const resultPanel = useResultPanel(resultPanelDurationSecs, overlayPosition !== undefined);
    const recordingMode = useRecordingMode();
    const error = useOverlayError();
    const { frozenSegments, provisional, hasStreamingText } = useStreamingState();
    const { text: flashText, isFadingOut } = useModeFlash();
    const { promptName } = useLlmPromptFlash();
    const { isProcessing } = useLLMState();
    const { isTransformProcessing } = useTransformProcessing();
    const setRoot = useOverlayInputRegion();
    const dragHandlers = useOverlayDrag();
    const showPromptName = promptName != null && !hasStreamingText;

    const renderContent = () => {
        if (error != null) {
            return (
                <span
                    data-interactive
                    className={clsx(
                        'text-[8px]',
                        'font-medium',
                        'truncate',
                        'flex',
                        'items-center',
                        'justify-center',
                        'h-9',
                        'px-3',
                        'rounded-full',
                        'glass-pill',
                        'animate-in',
                        'fade-in',
                        'zoom-in',
                        'duration-200',
                        'text-red-500'
                    )}
                >
                    {error}
                </span>
            );
        }
        if (flashText != null && !hasStreamingText) {
            return <ModeFlash text={flashText} isFadingOut={isFadingOut} />;
        }
        if (resultPanel.result != null) {
            return (
                <ResultPanel
                    key={resultPanel.showId}
                    text={resultPanel.result.text}
                    promptName={resultPanel.result.promptName}
                    mode={resultPanel.result.mode}
                    textWidth={streamingTextSettings.textWidth}
                    fontSize={streamingTextSettings.fontSize}
                    maxLines={streamingTextSettings.maxLines}
                    durationSecs={resultPanel.shownDurationSecs}
                    isPaused={resultPanel.isPaused}
                    isCopied={resultPanel.isCopied}
                    onCopy={resultPanel.copy}
                    onPause={resultPanel.pause}
                    onResume={resultPanel.resume}
                    onClose={resultPanel.close}
                />
            );
        }

        const visualizer = (
            <div className="relative w-fit">
                <div
                    data-interactive
                    {...dragHandlers}
                    className={clsx('cursor-grab', VISUALIZER_CONFIG[overlaySize].className)}
                >
                    <LiquidVisualizer
                        blobs={VISUALIZER_CONFIG[overlaySize].blobs}
                        glassStyle={glassStyle}
                        colorScheme={isTransformProcessing ? 'llm' : recordingMode}
                        isProcessing={isProcessing || isTransformProcessing}
                    />
                </div>
                {/* Sized to the resting capsule so the button sits on its corner. */}
                <div className="pointer-events-none absolute inset-x-0 top-1/4 bottom-1/4">
                    <CancelButton size={overlaySize} />
                </div>
            </div>
        );

        const textBlock = (() => {
            if (showPromptName) {
                return (
                    <div
                        data-interactive
                        className={clsx(
                            'max-w-[160px]',
                            'truncate',
                            'text-center',
                            'rounded-full',
                            'glass-pill',
                            'px-2',
                            'py-1',
                            'text-[10px]',
                            'font-normal',
                            'text-white',
                            'transition-opacity',
                            'duration-200'
                        )}
                    >
                        {promptName}
                    </div>
                );
            }
            if (hasStreamingText) {
                return (
                    <div data-interactive className={clsx('w-fit', 'rounded-2xl', 'glass-pill')}>
                        <StreamingText
                            frozenSegments={frozenSegments}
                            provisional={provisional}
                            textWidth={streamingTextSettings.textWidth}
                            fontSize={streamingTextSettings.fontSize}
                            maxLines={streamingTextSettings.maxLines}
                        />
                    </div>
                );
            }
            return null;
        })();

        const topBlock = textBlock != null && (
            <div className={clsx(overlayPosition === 'bottom' ? 'mb-0.5' : 'mt-0.5')}>{textBlock}</div>
        );

        return (
            <div className="flex flex-col items-center w-full">
                {overlayPosition === 'bottom' ? (
                    <>
                        {topBlock}
                        {visualizer}
                    </>
                ) : (
                    <>
                        {visualizer}
                        {topBlock}
                    </>
                )}
            </div>
        );
    };

    if (overlayPosition === undefined) return null;

    return (
        <div
            ref={setRoot}
            className={clsx(
                'w-full',
                'h-screen',
                'min-h-[36px]',
                'relative',
                'select-none',
                'flex',
                'flex-col',
                'items-center',
                overlayPosition === 'bottom' ? 'justify-end pb-3' : 'justify-start pt-3'
            )}
        >
            {renderContent()}
        </div>
    );
};
