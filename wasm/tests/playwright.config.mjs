import { defineConfig } from '@playwright/test';

export default defineConfig({
    testMatch: '**/*.spec.mjs',
    use: { baseURL: 'http://127.0.0.1:4173' },
    webServer: {
        command: 'python3 -m http.server 4173 --bind 127.0.0.1 --directory ../../docs',
        url: 'http://127.0.0.1:4173/',
    },
});
