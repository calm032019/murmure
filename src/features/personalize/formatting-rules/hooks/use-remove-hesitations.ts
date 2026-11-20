import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { AppSettings } from '@/features/settings/settings.types';

type DictationToggle = 'remove_hesitations' | 'voice_commands' | 'self_corrections';

const useDictationToggle = (key: DictationToggle, command: string) => {
    const [enabled, setEnabled] = useState<boolean>(true);
    const { t } = useTranslation();

    useEffect(() => {
        invoke<AppSettings>('get_all_settings').then((settings) => {
            setEnabled(settings[key]);
        });
    }, [key]);

    const handleSetEnabled = async (value: boolean) => {
        try {
            setEnabled(value);
            await invoke(command, { enabled: value });
        } catch {
            toast.error(t('Failed to save setting'));
            setEnabled(!value);
        }
    };

    return [enabled, handleSetEnabled] as const;
};

export const useRemoveHesitations = () => {
    const [removeHesitations, setRemoveHesitations] = useDictationToggle(
        'remove_hesitations',
        'set_remove_hesitations'
    );
    return { removeHesitations, setRemoveHesitations };
};

export const useVoiceCommands = () => {
    const [voiceCommands, setVoiceCommands] = useDictationToggle('voice_commands', 'set_voice_commands');
    return { voiceCommands, setVoiceCommands };
};

export const useSelfCorrections = () => {
    const [selfCorrections, setSelfCorrections] = useDictationToggle('self_corrections', 'set_self_corrections');
    return { selfCorrections, setSelfCorrections };
};
