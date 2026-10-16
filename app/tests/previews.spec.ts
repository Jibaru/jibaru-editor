// Library previews (Filmora): card stills, the hover animation, playing an
// item in the player and going back to the timeline.
import { type Page } from '@playwright/test'
import { expect, test, boot, state } from './helpers'

const tab = (page: Page, name: string) => page.locator('.lib-tabs button', { hasText: new RegExp(`^${name}$`) }).click()

test('cards show a still and animate on hover', async ({ page }) => {
  await boot(page)
  await tab(page, 'Transitions')
  const card = page.locator('.catalog-card[data-item="jb.dissolve"]')
  await expect(card.locator('img.fx-still')).toHaveAttribute('src', /^data:image\/jpeg/)
  await expect(card.locator('.fx-sprite')).toHaveCount(0)
  await card.hover()
  const sprite = card.locator('.fx-sprite.ready')
  await expect(sprite).toHaveCount(1)
  // The sprite steps through its frames (15 fps).
  await expect.poll(async () => Number(await sprite.getAttribute('data-frame')), { timeout: 3000 }).toBeGreaterThan(3)
  for (const b of ['preview', 'more', 'fav', 'add']) await expect(card.locator(`button.ov.${b}`)).toBeVisible()
  await page.mouse.move(5, 5)
  await expect(card.locator('.fx-sprite')).toHaveCount(0)
})

test('stills load lazily as cards scroll into view', async ({ page }) => {
  await boot(page)
  await tab(page, 'Effects')
  const cards = page.locator('.catalog-card')
  expect(await cards.count()).toBeGreaterThan(40)
  const last = cards.last()
  await expect(last.locator('img.fx-still')).toHaveCount(0)
  await last.scrollIntoViewIfNeeded()
  await expect(last.locator('img.fx-still')).toHaveCount(1)
})

test('clicking a card plays it in the player; Timeline goes back', async ({ page }) => {
  await boot(page)
  await tab(page, 'Transitions')
  await page.locator('.catalog-card[data-item="jb.dissolve"]').click()
  await expect(page.locator('.catalog-card[data-item="jb.dissolve"]')).toHaveClass(/\bon\b/)
  await expect(page.locator('.screen.item-preview')).toHaveAttribute('data-preview', 'transition:jb.dissolve')
  await expect(page.locator('.player-head button', { hasText: 'Source' })).toHaveClass(/\bon\b/)
  await expect(page.locator('.player .tc-total')).toHaveText('00:00:02:00')
  // Plays once, then rests on its first frame (Filmora).
  await expect(page.locator('.player button[title^="Pause"]')).toBeVisible()
  await expect(page.locator('.player button[title^="Play"]')).toBeVisible({ timeout: 4000 })
  await expect(page.locator('.player .tc-box')).toHaveText('00:00:00:00')
  // Space plays the preview, not the timeline.
  await page.keyboard.press('Space')
  await expect(page.locator('.player button[title^="Pause"]')).toBeVisible()
  expect((await state(page)).playing).toBe(false)
  await page.locator('.player-head button', { hasText: 'Timeline' }).click()
  await expect(page.locator('.screen.item-preview')).toHaveCount(0)
  expect((await state(page)).playerMode).toBe('timeline')
  // The card stays selected.
  await expect(page.locator('.catalog-card[data-item="jb.dissolve"]')).toHaveClass(/\bon\b/)
})

test('titles and effects preview too; seeking the timeline ends the preview', async ({ page }) => {
  await boot(page)
  await tab(page, 'Titles')
  await page.locator('.catalog-card[data-item="lower_third"]').click()
  await expect(page.locator('.screen.item-preview')).toHaveAttribute('data-preview', 'title:lower_third')
  await tab(page, 'Effects')
  await page.locator('.catalog-card[data-item="jb.vhs"]').click()
  await expect(page.locator('.screen.item-preview')).toHaveAttribute('data-preview', 'effect:jb.vhs')
  await page.evaluate(() => (window as any).__jb.seek(10_000_000))
  await expect(page.locator('.screen.item-preview')).toHaveCount(0)
})
