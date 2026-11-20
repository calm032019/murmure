import {
    Home,
    Settings,
    ChevronRight,
    Keyboard,
    BookText,
    Power,
    Sparkles,
    Wrench,
    AlignLeft,
    ArrowDownUp,
    Puzzle,
    Plug,
} from 'lucide-react';
import { Link, useLocation } from '@tanstack/react-router';
import { useEffect, useState } from 'react';
import {
    Sidebar,
    SidebarHeader,
    SidebarContent,
    SidebarFooter,
    SidebarGroup,
    SidebarMenu,
    SidebarMenuItem,
    SidebarMenuButton,
    SidebarMenuSub,
    SidebarMenuSubItem,
    SidebarMenuSubButton,
} from '../../../components/sidebar';
import { useGetVersion } from '../hooks/use-get-version';
import { useTranslation } from '@/i18n';
import { useDictionaryRevampSeen } from '@/features/personalize/custom-dictionary/hooks/use-dictionary-revamp-seen';
import appIcon from '@/assets/app-icon.png';
import { isPathInItems } from './app-sidebar.helpers';
import { Onboarding } from '@/features/onboarding/onboarding';
import { useIsWayland } from '@/components/hooks/use-linux-session-type';
import { useShortcut, SHORTCUT_CONFIGS } from '@/features/settings/shortcuts/hooks/use-shortcut';

const getPersonalizeSubItems = (t: (key: string) => string) => [
    {
        name: t('Dictionary'),
        url: '/personalize/custom-dictionary',
        icon: BookText,
        dataTestId: 'dictionary-tab',
    },
    {
        name: t('Formatting Rules'),
        url: '/personalize/formatting-rules',
        icon: AlignLeft,
        dataTestId: 'formatting-rules-tab',
    },
];

const getExtensionsSubItems = (t: (key: string) => string) => [
    {
        name: t('Prompt Mode'),
        url: '/extensions/llm-connect',
        icon: Sparkles,
        dataTestId: 'llm-connect-tab',
    },
    {
        name: t('Local API'),
        url: '/extensions/local-api',
        icon: Plug,
        dataTestId: 'local-api-tab',
    },
];

const getSettingsSubItems = (t: (key: string) => string) => [
    {
        name: t('Shortcuts'),
        url: '/settings/shortcuts',
        icon: Keyboard,
        dataTestId: 'shortcuts-tab',
    },
    {
        name: t('System'),
        url: '/settings/system',
        icon: Power,
        dataTestId: 'system-tab',
    },
    {
        name: t('Import / Export'),
        url: '/settings/import-export',
        icon: ArrowDownUp,
        dataTestId: 'import-export-tab',
    },
];

export const AppSidebar = () => {
    const { pathname } = useLocation();
    const [personalizeOpen, setPersonalizeOpen] = useState(true);
    const [extensionsOpen, setExtensionsOpen] = useState(pathname.startsWith('/extensions'));
    const [settingsOpen, setSettingsOpen] = useState(pathname.startsWith('/settings'));

    useEffect(() => {
        if (pathname.startsWith('/extensions')) {
            setExtensionsOpen(true);
        }
    }, [pathname]);
    const version = useGetVersion();
    const { t } = useTranslation();
    const { seen: revampSeen } = useDictionaryRevampSeen();
    const personalizeSubItems = getPersonalizeSubItems(t);
    const extensionsSubItems = getExtensionsSubItems(t);
    const settingsSubItems = getSettingsSubItems(t);
    const showRevampDot = !revampSeen;
    const isPersonalizeActive = isPathInItems(pathname, personalizeSubItems);
    const isSettingsActive = isPathInItems(pathname, settingsSubItems);
    const isExtensionsActive = isPathInItems(pathname, extensionsSubItems);
    const { shortcut: recordShortcut } = useShortcut(SHORTCUT_CONFIGS.record);
    const isWayland = useIsWayland();

    return (
        <Sidebar className="bg-transparent border-sidebar-border border-r overflow-hidden w-[15.5rem]">
            <SidebarHeader className="flex items-center justify-center bg-transparent border-b border-sidebar-border">
                <img src={appIcon} alt="logo" className="w-16 h-16" />
            </SidebarHeader>
            <SidebarContent className="bg-transparent">
                <SidebarGroup>
                    <SidebarMenu>
                        <SidebarMenuItem>
                            <SidebarMenuButton asChild isActive={pathname === '/'} data-testid="home-tab">
                                <Link to="/">
                                    <Home />
                                    <span>{t('Home')}</span>
                                </Link>
                            </SidebarMenuButton>
                        </SidebarMenuItem>

                        <SidebarMenuItem>
                            <SidebarMenuButton
                                onClick={() => setPersonalizeOpen(!personalizeOpen)}
                                data-testid="personalize-tab"
                            >
                                <Wrench className={isPersonalizeActive ? 'text-sky-400' : undefined} />
                                <span>{t('Personalize')}</span>
                                <ChevronRight
                                    className={`ml-auto transition-transform ${personalizeOpen ? 'rotate-90' : ''}`}
                                />
                            </SidebarMenuButton>
                            {personalizeOpen && (
                                <SidebarMenuSub>
                                    {personalizeSubItems.map((item) => (
                                        <SidebarMenuSubItem key={item.url} data-testid={item.dataTestId}>
                                            <SidebarMenuSubButton asChild isActive={pathname === item.url}>
                                                <Link to={item.url}>
                                                    <item.icon />
                                                    <span>{item.name}</span>
                                                    {item.url === '/personalize/custom-dictionary' && showRevampDot && (
                                                        <span
                                                            className="ml-auto h-2 w-2 rounded-full bg-sky-500"
                                                            title={t('Improved')}
                                                            aria-label={t('Improved')}
                                                        />
                                                    )}
                                                </Link>
                                            </SidebarMenuSubButton>
                                        </SidebarMenuSubItem>
                                    ))}
                                </SidebarMenuSub>
                            )}
                        </SidebarMenuItem>

                        <SidebarMenuItem>
                            <SidebarMenuButton
                                onClick={() => setSettingsOpen(!settingsOpen)}
                                data-testid="settings-tab"
                            >
                                <Settings className={isSettingsActive ? 'text-sky-400' : undefined} />
                                <span>{t('Settings')}</span>
                                <ChevronRight
                                    className={`ml-auto transition-transform ${settingsOpen ? 'rotate-90' : ''}`}
                                />
                            </SidebarMenuButton>
                            {settingsOpen && (
                                <SidebarMenuSub>
                                    {settingsSubItems.map((item) => (
                                        <SidebarMenuSubItem key={item.url} data-testid={item.dataTestId}>
                                            <SidebarMenuSubButton asChild isActive={pathname === item.url}>
                                                <Link to={item.url}>
                                                    <item.icon />
                                                    <span>{item.name}</span>
                                                </Link>
                                            </SidebarMenuSubButton>
                                        </SidebarMenuSubItem>
                                    ))}
                                </SidebarMenuSub>
                            )}
                        </SidebarMenuItem>

                        <SidebarMenuItem>
                            <SidebarMenuButton
                                onClick={() => setExtensionsOpen(!extensionsOpen)}
                                data-testid="extensions-tab"
                            >
                                <Puzzle className={isExtensionsActive ? 'text-sky-400' : undefined} />
                                <span>{t('Extensions')}</span>
                                <ChevronRight
                                    className={`ml-auto transition-transform ${extensionsOpen ? 'rotate-90' : ''}`}
                                />
                            </SidebarMenuButton>
                            {extensionsOpen && (
                                <SidebarMenuSub>
                                    {extensionsSubItems.map((item) => (
                                        <SidebarMenuSubItem key={item.url} data-testid={item.dataTestId}>
                                            <SidebarMenuSubButton asChild isActive={pathname === item.url}>
                                                <Link to={item.url}>
                                                    <item.icon />
                                                    <span>{item.name}</span>
                                                </Link>
                                            </SidebarMenuSubButton>
                                        </SidebarMenuSubItem>
                                    ))}
                                </SidebarMenuSub>
                            )}
                        </SidebarMenuItem>
                    </SidebarMenu>
                </SidebarGroup>
                {!isWayland && (
                    <div className="mt-auto px-2 pb-2 empty:hidden">
                        <Onboarding recordShortcut={recordShortcut} />
                    </div>
                )}
            </SidebarContent>
            <SidebarFooter className="bg-transparent">
                <div className="flex items-center gap-2 justify-center">
                    <p className="text-xs text-muted-foreground">{version}</p>
                </div>
            </SidebarFooter>
        </Sidebar>
    );
};
