import { type Page } from '@playwright/test'
import { expect, test, boot, menuItem, project, seek, state } from './helpers'
import { fixture } from './paths'

const tab = (page: Page, name: string) => page.locator('.lib-tabs button', { hasText: new RegExp(`^${name}$`) }).click()
const side = (page: Page, name: string) => page.locator('.lib-side button', { hasText: new RegExp(`^${name}$`) }).click()

test('import menu: files and a folder', async ({ page }) => {
  await boot(page, { clips: false })
  await page.evaluate(() => (window as any).__jbMock.setProject({ ...(window as any).__jbMock.project(), media: [] }))
  await page.evaluate(p => (window as any).__jbMock.queuePick([p]), fixture('tone.wav'))
  await page.locator('.lib-import').click()
  await menuItem(page, 'Import Media Files...')
  await expect(page.locator('.media-card')).toHaveCount(1)
  await page.evaluate(p => (window as any).__jbMock.queuePick([p]), fixture('folder'))
  await page.locator('.lib-import').click()
  await menuItem(page, 'Import Media Folder...')
  await expect(page.locator('.media-card')).toHaveCount(3)
  expect((await project(page)).media.map((m: any) => m.name).sort()).toEqual(['a.mp4', 'b.wav', 'tone.wav'])
  await expect(page.locator('.lib-record button').first()).toBeDisabled()
})

test('media sidebar filters by kind; cloud sections show an empty state', async ({ page }) => {
  await boot(page)
  await side(page, 'Audio')
  await expect(page.locator('.media-card')).toHaveCount(1)
  await side(page, 'Images')
  await expect(page.locator('.media-card .name')).toHaveText(['still.png'])
  await side(page, 'Cloud Media')
  await expect(page.locator('.lib-empty')).toContainText('not available')
  await side(page, 'Project Media')
  await expect(page.locator('.media-card')).toHaveCount(4)
})

test('search, sort, unused filter, more menu and thumbnail size', async ({ page }) => {
  await boot(page)
  await page.locator('.lib-search input').fill('green')
  await expect(page.locator('.media-card .name')).toHaveText(['green.mp4'])
  await page.locator('.lib-search input').fill('')
  await page.locator('.lib-toolbar button[title="Sort and filter"]').click()
  await menuItem(page, 'Sort by Name')
  await expect(page.locator('.media-card .name')).toHaveText(['clip.mp4', 'green.mp4', 'still.png', 'tone.wav'])
  await page.locator('.lib-toolbar button[title="Sort and filter"]').click()
  await menuItem(page, 'Show Unused Only')
  await expect(page.locator('.media-card .name')).toHaveText(['still.png'])
  await page.locator('.lib-toolbar button[title="More"]').click()
  await menuItem(page, /Delete Unused Media/)
  expect((await project(page)).media).toHaveLength(3)
  await page.locator('.lib-toolbar button[title="Sort and filter"]').click()
  await menuItem(page, 'Show Unused Only')
  await page.locator('.lib-toolbar button[title="More"]').click()
  await menuItem(page, 'Select All')
  expect((await state(page)).selectedMedia).toHaveLength(3)
  const slider = page.locator('.lib-size')
  const box = (await slider.boundingBox())!
  await page.mouse.click(box.x + box.width - 1, box.y + box.height / 2)
  const w = await page.locator('.library').evaluate(el => getComputedStyle(el).getPropertyValue('--card-w'))
  expect(parseInt(w)).toBeGreaterThan(180)
})

test('media card: rename, add to timeline, source monitor, delete', async ({ page }) => {
  const b = await boot(page)
  const card = page.locator(`.media-card[data-media="${b.media.still}"]`)
  await card.click({ button: 'right' })
  await menuItem(page, 'Rename')
  await card.locator('input.rename').fill('poster.png')
  await card.locator('input.rename').press('Enter')
  expect((await project(page)).media.find((m: any) => m.id === b.media.still).name).toBe('poster.png')
  await card.locator('button.add').click()
  const clips = (await project(page)).sequences[0].tracks[0].clips
  expect(clips[clips.length - 1].source.media_id).toBe(b.media.still)
  await card.click({ button: 'right' })
  await menuItem(page, 'Open in Source Monitor')
  expect((await state(page)).playerMode).toBe('source')
})

test('drag media from the library to the timeline', async ({ page }) => {
  const b = await boot(page)
  const card = page.locator(`.media-card[data-media="${b.media.still}"]`)
  const from = (await card.boundingBox())!
  const tl = (await page.locator('.timeline').boundingBox())!
  await page.mouse.move(from.x + 20, from.y + 20)
  await page.mouse.down()
  await page.mouse.move(from.x + 60, from.y + 60, { steps: 4 })
  await page.mouse.move(tl.x + tl.width * 0.8, tl.y + tl.height * 0.45, { steps: 8 })
  await page.mouse.up()
  const used = (await project(page)).sequences[0].tracks.some((t: any) => t.clips.some((c: any) => c.source.media_id === b.media.still))
  expect(used).toBe(true)
})

test('titles, effects, filters and transitions: + and double-click apply, favorites', async ({ page }) => {
  const b = await boot(page)
  await tab(page, 'Titles')
  await seek(page, 3.5)
  await page.locator('.catalog-card[data-item="lower_third"]').dblclick()
  let titles = (await project(page)).sequences[0].tracks.flatMap((t: any) => t.clips).filter((c: any) => c.source.type === 'title')
  expect(titles.map((c: any) => c.name)).toContain('Lower Third')
  await tab(page, 'Filters')
  await page.evaluate(id => (window as any).__jb.select([id]), b.clips.video)
  await page.locator('.catalog-card').first().hover()
  await page.locator('.catalog-card').first().locator('button.add').click()
  let v = (await project(page)).sequences[0].tracks[0].clips[0]
  expect(v.effects?.length).toBe(1)
  await tab(page, 'Transitions')
  await seek(page, 3.9)
  await page.locator('.catalog-card[data-item="jb.dissolve"]').dblclick()
  v = (await project(page)).sequences[0].tracks[0].clips[0]
  expect(v.transition_out?.kind).toBe('jb.dissolve')
  await tab(page, 'Effects')
  const card = page.locator('.catalog-card[data-item="jb.vhs"]')
  await card.hover()
  await card.locator('button.fav').click()
  await side(page, 'Favorites')
  await expect(page.locator('.catalog-card')).toHaveCount(1)
  await card.click({ button: 'right' })
  await menuItem(page, 'Remove from Favorites')
  await expect(page.locator('.catalog-card')).toHaveCount(0)
  titles = []
})

test('stock media, audio libraries, stickers and templates show their empty states', async ({ page }) => {
  await boot(page)
  for (const t of ['Stock Media', 'Stickers', 'Templates']) {
    await tab(page, t)
    await expect(page.locator('.lib-empty')).toContainText('not available')
  }
  await tab(page, 'Audio')
  await expect(page.locator('.media-card')).toHaveCount(1)
  await side(page, 'Music')
  await expect(page.locator('.lib-empty')).toContainText('not available')
})
