import { Outlet } from '@tanstack/react-router';
import { SidebarProvider, SidebarInset } from '../../components/sidebar';
import { AppSidebar } from './app-sidebar/app-sidebar';
import clsx from 'clsx';
import { Bounce, ToastContainer } from 'react-toastify';
import { AccessibilityListener } from './listeners/accessibility-listener';
import { RecordingErrorListener } from './listeners/recording-error-listener';
import { LlmErrorListener } from './listeners/llm-error-listener';
import { LlmModeNotConfiguredListener } from './listeners/llm-mode-not-configured-listener';
import { ToastEventListener } from './listeners/toast-event-listener';
import { ConfigImportedListener } from './listeners/config-imported-listener';
import { WaylandListener } from './listeners/wayland-listener';
import { TrayMenuI18nListener } from './listeners/tray-menu-i18n-listener';
import { AmbientBackdrop } from './ambient-backdrop/ambient-backdrop';
import { useGlassStyle } from '@/components/hooks/use-glass-style';

export const Layout = () => {
    useGlassStyle();
    return (
        <SidebarProvider open={true} onOpenChange={() => {}} className="bg-transparent dark">
            <AmbientBackdrop />
            <AccessibilityListener />
            <RecordingErrorListener />
            <LlmErrorListener />
            <LlmModeNotConfiguredListener />
            <ToastEventListener />
            <ConfigImportedListener />
            <WaylandListener />
            <TrayMenuI18nListener />
            <AppSidebar />
            <SidebarInset
                className={clsx('bg-transparent', 'text-white', 'pr-8', 'pt-8', 'flex', 'items-center', 'pl-[17.2rem]')}
            >
                <div className="relative max-w-[800px] w-full pb-12" data-testid="murmure-content">
                    <Outlet />
                </div>
            </SidebarInset>
            <ToastContainer
                position="bottom-right"
                autoClose={3000}
                hideProgressBar={false}
                newestOnTop={false}
                closeOnClick={false}
                rtl={false}
                pauseOnFocusLoss
                draggable
                pauseOnHover
                theme="dark"
                transition={Bounce}
            />
        </SidebarProvider>
    );
};
