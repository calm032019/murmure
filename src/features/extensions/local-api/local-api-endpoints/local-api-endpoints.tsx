import { CodeXml } from 'lucide-react';
import { SettingsUI } from '@/components/settings-ui';
import { useLlmExtensionsEnabled } from '@/features/extensions/llm-connect/hooks/use-llm-extensions-enabled';
import { useTranslation } from '@/i18n';
import { EndpointDoc } from './endpoint-doc/endpoint-doc';
import { ENDPOINTS } from './local-api-endpoints.helpers';

interface LocalApiEndpointsProps {
    port: number;
    isServerRunning: boolean;
}

export const LocalApiEndpoints = ({ port, isServerRunning }: LocalApiEndpointsProps) => {
    const { t } = useTranslation();
    const availability = useLlmExtensionsEnabled();

    if (!availability.isLoaded) {
        return null;
    }

    return (
        <SettingsUI.Section title={t('Endpoints')} icon={CodeXml}>
            <div className="p-4 space-y-4">
                {ENDPOINTS.map((definition) => (
                    <EndpointDoc
                        key={definition.path}
                        definition={definition}
                        port={port}
                        isServerRunning={isServerRunning}
                        availability={availability}
                    />
                ))}
            </div>
        </SettingsUI.Section>
    );
};
