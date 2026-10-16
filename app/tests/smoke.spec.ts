import { expect, test, boot, selectClip } from './helpers'

test('every properties tab renders without errors', async ({ page }) => {
  const b = await boot(page)
  for (const [clip, tabs] of [
    ['video', ['Video', 'Audio', 'Speed', 'Animation', 'Color']],
    ['title', ['Titles', 'Animation']],
    ['audio', ['Audio', 'Speed']],
  ] as const) {
    await selectClip(page, b.clips[clip])
    for (const t of tabs) {
      await page.locator('.prop-tabs button', { hasText: new RegExp(`^${t}$`) }).click()
      await expect(page.locator('.properties .prop-tabs button.on')).toHaveText(t)
      for (const sub of await page.locator('.sub-tabs > button:not(.sub-more):not(.sub-compare)').allTextContents()) {
        await page.locator('.sub-tabs > button', { hasText: sub }).first().click()
      }
    }
  }
  for (const tab of ['Media', 'Stock Media', 'Audio', 'Titles', 'Transitions', 'Effects', 'Filters', 'Stickers', 'Templates']) {
    await page.locator('.lib-tabs button', { hasText: new RegExp(`^${tab}$`) }).click()
    for (const side of await page.locator('.lib-side button').all()) await side.click()
  }
  expect(b.errors).toEqual([])
})
