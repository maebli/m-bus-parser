import { expect, test } from '@playwright/test';
import { frame, session } from './mode13-fixture.mjs';

test('Parse ignores a previous meter key when inspecting and authenticating TLS', async ({ page }) => {
    await page.goto('/?data=1040FF3F16&format=json');
    await page.locator('#aeskey').fill('invalid-stale-key');
    await page.getByLabel('M-Bus payload').fill(frame);
    await page.getByRole('button', { name: 'Inspect TLS records' }).click();
    await expect(page.locator('#output')).toContainText('session_required');

    await page.locator('#tls-import').setInputFiles({
        name: 'session.json',
        mimeType: 'application/json',
        buffer: Buffer.from(JSON.stringify(session)),
    });
    await page.getByRole('button', { name: 'Decrypt and verify' }).click();
    await expect(page.locator('#output')).toContainText('decrypted_authenticated');
    await expect(page.locator('#output')).toContainText('01132A');
});

test('Validate exports the same partial record finding in each format', async ({ page }) => {
    const frameWithTruncatedRecord = '164493157856341233038C00117A1100000001132A0413';
    await page.goto('/?data=' + frameWithTruncatedRecord + '&format=table');
    await page.getByRole('button', { name: 'Validate', exact: true }).click();
    const output = page.locator('#output');
    await expect(output.getByRole('row').filter({ hasText: 'Wireless frame length' })).toContainText('Passed');
    await expect(output.getByRole('row').filter({ hasText: 'Application decoding' })).toContainText('byte offset 3');
    for (const format of ['JSON', 'CSV', 'XML']) {
        await page.getByRole('button', { name: format, exact: true }).click();
        const link = page.locator('#download_output');
        await expect(link).toHaveAttribute('download', 'validation-report.' + format.toLowerCase());
        const text = await link.evaluate(async anchor => (await fetch(anchor.href)).text());
        if (format === 'JSON') {
            expect(JSON.parse(text).findings).toContainEqual(expect.objectContaining({
                group: 'Payload', rule: 'Application decoding', status: 'failed', offset_start: 3,
            }));
        } else if (format === 'CSV') {
            expect(text).toMatch(/"Payload","Application decoding","failed","[^"]*","3"/);
        } else {
            const row = await page.evaluate(xml => {
                const rows = new DOMParser().parseFromString(xml, 'application/xml').querySelectorAll('finding');
                const match = Array.from(rows).find(item => item.querySelector('rule').textContent === 'Application decoding');
                return ['group', 'status', 'offset_start'].map(key => match.querySelector(key).textContent);
            }, text);
            expect(row).toEqual(['Payload', 'failed', '3']);
        }
    }
});
