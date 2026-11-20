import { useTranslation } from '@/i18n';
import { Typography } from '@/components/typography';
import { SettingsUI } from '@/components/settings-ui';
import { Page } from '@/components/page';
import { Server } from 'lucide-react';
import { ConnectionStatus } from '../../hooks/use-llm-connect';
import { ConnectionButton } from '../connection-button/connection-button';

interface LocalServerSectionProps {
    localUrl: string;
    onLocalUrlChange: (value: string) => void;
    onLocalUrlBlur: () => void;
    isTestingLocal: boolean;
    localConnectionStatus: ConnectionStatus;
    onTestLocal: () => void;
    showInstallModel: boolean;
    onInstallModel: () => void;
}

export const LocalServerSection = ({
    isTestingLocal,
    localConnectionStatus,
    onTestLocal,
    showInstallModel,
    onInstallModel,
}: LocalServerSectionProps) => {
    const { t } = useTranslation();

    return (
        <SettingsUI.Section title={t('Built-in AI')} icon={Server}>
            <SettingsUI.Item>
                <SettingsUI.Description>
                    <Typography.Title>{t('Engine')}</Typography.Title>
                    <Typography.Paragraph>{t('Qwen 3.5 (4B), running on this computer')}</Typography.Paragraph>
                </SettingsUI.Description>
                <div className="flex items-center gap-3">
                    <ConnectionButton isTesting={isTestingLocal} status={localConnectionStatus} onClick={onTestLocal} />
                </div>
            </SettingsUI.Item>
            {showInstallModel && (
                <>
                    <SettingsUI.Separator />
                    <SettingsUI.Item>
                        <SettingsUI.Description>
                            <Typography.Title>{t('Models')}</Typography.Title>
                        </SettingsUI.Description>
                        <Page.SecondaryButton onClick={onInstallModel} size="sm">
                            {t('Download model')}
                        </Page.SecondaryButton>
                    </SettingsUI.Item>
                </>
            )}
        </SettingsUI.Section>
    );
};
