import { useTranslation } from '@/i18n';
import { Button } from '@/components/button';
import { Typography } from '@/components/typography';
import { motion } from 'framer-motion';
import { Qwen } from '@lobehub/icons';
import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Page } from '@/components/page';
import { ModelCard, RecommendedModel } from '@/components/model-card';
import { AlertCircle, RefreshCw } from 'lucide-react';
import { getPresetLabel, getPromptByPreset } from '../../llm-connect.helpers';
import { ModelCombobox } from '../../model-combobox/model-combobox';

import { OllamaModel, LLMConnectSettings } from '../../hooks/use-llm-connect';

interface StepModelProps {
    onNext: () => void;
    pullModel: (model: string) => Promise<void>;
    updateSettings: (updates: Partial<LLMConnectSettings>) => Promise<void>;
    models: OllamaModel[];
    fetchModels: () => Promise<OllamaModel[]>;
    isInstallOnly?: boolean;
    isRemote?: boolean;
    remoteModels?: OllamaModel[];
    fetchRemoteModels?: () => Promise<OllamaModel[]>;
}

interface OllamaPullProgressPayload {
    status: string;
    digest?: string;
    total?: number;
    completed?: number;
}

export const StepModel = ({
    onNext,
    pullModel,
    updateSettings,
    models,
    fetchModels,
    isInstallOnly = false,
    isRemote = false,
    remoteModels = [],
    fetchRemoteModels,
}: StepModelProps) => {
    const { t, i18n } = useTranslation();
    const [selectedModel, setSelectedModel] = useState<string | null>(null);
    const [downloadingModel, setDownloadingModel] = useState<string | null>(null);
    const [progress, setProgress] = useState<number>(0);
    const [downloadedModels, setDownloadedModels] = useState<Set<string>>(new Set());
    const [error, setError] = useState<string | null>(null);
    const [isRefreshing, setIsRefreshing] = useState(false);

    const recommendedModels: RecommendedModel[] = [
        {
            id: 'qwen3.5:4b',
            name: 'Qwen 3.5 (4B)',
            description: t('Built into Murmure, runs fully offline'),
            bullets: [t('Cleans up and formats your dictation'), t('Uses your graphics card, or the CPU')],
            size: t('~ 3 GB on disk'),
            vram: t('4 GB VRAM recommended'),
            icon: Qwen.Color,
            tags: [t('Built-in'), t('Private')],
            recommended: true,
        },
    ];

    const applyModelToFirstMode = async (modelName: string) => {
        await updateSettings({
            model: modelName,
            modes: [
                {
                    name: t(getPresetLabel('clean')),
                    prompt: getPromptByPreset('clean', i18n.language),
                    model: modelName,
                    provider: isRemote ? 'remote' : 'local',
                    wake_word: '',
                },
            ],
            active_mode_index: 0,
            command: { provider: isRemote ? 'remote' : 'local', model: modelName, enabled: false },
        });
    };

    useEffect(() => {
        if (isRemote) return;

        const unlisten = listen<OllamaPullProgressPayload>('llm-pull-progress', (event) => {
            const { total, completed, status } = event.payload;
            if (status === 'success') {
                setProgress(100);
            } else if (total && completed) {
                setProgress(Math.round((completed / total) * 100));
            }
        });

        void fetchModels();

        return () => {
            unlisten.then((fn) => fn());
        };
    }, []);

    const isModelDownloaded = (modelId: string) => {
        return downloadedModels.has(modelId) || models.some((m) => m.name === modelId);
    };

    const handleDownload = async (modelId: string) => {
        if (isModelDownloaded(modelId)) {
            if (!isInstallOnly) {
                await applyModelToFirstMode(modelId);
            }
            setSelectedModel(modelId);
            return;
        }

        setDownloadingModel(modelId);
        setProgress(0);
        setError(null);
        try {
            await pullModel(modelId);
            setDownloadedModels((prev) => new Set(prev).add(modelId));

            if (!isInstallOnly) {
                await applyModelToFirstMode(modelId);
            }
            setSelectedModel(modelId);
        } catch (error: unknown) {
            console.error('Failed to download model', error);
            const errorMessage = error instanceof Error ? error.message : String(error);
            setError(errorMessage || t('Failed to download model. Please check your connection and try again.'));
        } finally {
            setDownloadingModel(null);
            setProgress(0);
        }
    };

    const handleRemoteSelect = async (modelName: string) => {
        setSelectedModel(modelName);
        if (!isInstallOnly) {
            await applyModelToFirstMode(modelName);
        }
    };

    const handleRefreshRemoteModels = async () => {
        if (fetchRemoteModels) {
            setIsRefreshing(true);
            try {
                await fetchRemoteModels();
            } catch {
                // Error handled in hook
            } finally {
                setIsRefreshing(false);
            }
        }
    };

    const title = isInstallOnly ? t('Install the Model') : t('Set up the AI model');
    const getSubtitle = () => {
        if (isInstallOnly) return t('Download the built-in model to use with your prompts.');
        if (isRemote) return t('Choose a model available on your server.');
        return t('Murmure runs this model on your computer. If it was not installed with Murmure, it downloads once (about 3 GB).');
    };
    const subtitle = getSubtitle();
    const finishButtonText = isInstallOnly ? t('Done') : t('Finish Setup');

    if (isRemote) {
        return (
            <motion.div
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -20 }}
                className="flex flex-col items-center max-w-4xl mx-auto space-y-3 py-4 h-fit"
            >
                <div className="text-center space-y-2">
                    <Typography.MainTitle>{title}</Typography.MainTitle>
                    <Typography.Paragraph className="text-muted-foreground max-w-lg mx-auto">
                        {subtitle}
                    </Typography.Paragraph>
                </div>

                <div className="w-full bg-surface border border-surface-border rounded-xl p-4 flex justify-center">
                    <ModelCombobox
                        models={remoteModels}
                        value={selectedModel ?? ''}
                        onValueChange={handleRemoteSelect}
                        placeholder={t('Select or type a model')}
                    />
                </div>

                <div className="flex flex-col items-center gap-1">
                    <Button
                        onClick={handleRefreshRemoteModels}
                        variant="subtle"
                        className="text-muted-foreground hover:text-foreground"
                        disabled={isRefreshing}
                    >
                        <RefreshCw className={`w-4 h-4 mr-2 ${isRefreshing ? 'animate-spin' : ''}`} />
                        {t('Refresh list')}
                    </Button>
                    <p className="text-xs text-muted-foreground">
                        {t('With OpenAI-compatible, you cannot install new models from Murmure.')}
                    </p>
                </div>

                <div className="flex justify-between w-full">
                    <div />
                    <div>
                        <Page.PrimaryButton
                            onClick={onNext}
                            disabled={!selectedModel}
                            size="lg"
                            className="px-8"
                            data-testid="llm-connect-next-button"
                        >
                            {finishButtonText}
                        </Page.PrimaryButton>
                    </div>
                </div>
            </motion.div>
        );
    }

    return (
        <motion.div
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -20 }}
            className="flex flex-col items-center max-w-4xl mx-auto space-y-8 py-8 h-fit"
        >
            <div className="text-center space-y-4">
                <Typography.MainTitle>{title}</Typography.MainTitle>
                <Typography.Paragraph className="text-muted-foreground max-w-lg mx-auto">
                    {subtitle}
                </Typography.Paragraph>
            </div>

            <div className="grid grid-cols-1 gap-4 w-full max-w-sm">
                {recommendedModels.map((model) => (
                    <ModelCard
                        key={model.id}
                        model={model}
                        isSelected={selectedModel === model.id}
                        isDownloaded={isModelDownloaded(model.id)}
                        isDownloading={downloadingModel === model.id}
                        progress={progress}
                        onSelect={handleDownload}
                    />
                ))}
            </div>

            <div className="flex flex-col items-center gap-4 w-full">
                {error && (
                    <div className="flex items-center gap-2 text-red-400 bg-red-400/10 px-4 py-2 rounded-lg text-sm animate-in fade-in slide-in-from-bottom-2">
                        <AlertCircle className="w-4 h-4" />
                        {error}
                    </div>
                )}
            </div>

            <div className="flex justify-between w-full">
                <div />
                <div>
                    <Page.PrimaryButton
                        onClick={onNext}
                        disabled={!isInstallOnly && !selectedModel}
                        size="lg"
                        className="px-8"
                        data-testid="llm-connect-next-button"
                    >
                        {finishButtonText}
                    </Page.PrimaryButton>
                </div>
            </div>
        </motion.div>
    );
};
