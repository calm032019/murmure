import { createRouter, createRoute, createRootRoute, Navigate } from '@tanstack/react-router';
import { Home } from './features/home/home';
import { Layout } from './features/layout/layout';
import { Shortcuts } from './features/settings/shortcuts/shortcuts';
import { CustomDictionary } from './features/personalize/custom-dictionary/custom-dictionary';
import { FormattingRules } from './features/personalize/formatting-rules/formatting-rules';
import { System } from './features/settings/system/system';
import { LLMConnect } from './features/extensions/llm-connect/llm-connect';
import { ImportExport } from './features/settings/import-export/import-export';
import { LocalApi } from './features/extensions/local-api/local-api';

const rootRoute = createRootRoute({
    component: () => <Layout />,
});

const indexRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/',
    component: Home,
});

const settingsShortcutsRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/settings/shortcuts',
    component: Shortcuts,
});

const personalizeCustomDictionaryRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/personalize/custom-dictionary',
    component: CustomDictionary,
});

const personalizeFormattingRulesRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/personalize/formatting-rules',
    component: FormattingRules,
});

const extensionsLLMConnectRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/extensions/llm-connect',
    component: LLMConnect,
});

const personalizeLLMConnectRedirectRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/personalize/llm-connect',
    component: () => <Navigate to="/extensions/llm-connect" />,
});

const settingsSystemRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/settings/system',
    component: System,
});

const settingsIndexRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/settings',
    component: () => <Navigate to="/settings/shortcuts" />,
});

const personalizeIndexRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/personalize',
    component: () => <Navigate to="/personalize/custom-dictionary" />,
});

const extensionsLocalApiRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/extensions/local-api',
    component: LocalApi,
});

const extensionsIndexRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/extensions',
    component: () => <Navigate to="/extensions/llm-connect" />,
});

const settingsImportExportRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/settings/import-export',
    component: ImportExport,
});

const routeTree = rootRoute.addChildren([
    indexRoute,
    settingsIndexRoute,
    settingsShortcutsRoute,
    settingsSystemRoute,
    settingsImportExportRoute,
    personalizeIndexRoute,
    personalizeCustomDictionaryRoute,
    personalizeFormattingRulesRoute,
    personalizeLLMConnectRedirectRoute,
    extensionsIndexRoute,
    extensionsLLMConnectRoute,
    extensionsLocalApiRoute,
]);

export const router = createRouter({ routeTree });
