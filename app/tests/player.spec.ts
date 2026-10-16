import { expect, test, boot, choose, menuItem, project, seek, state, undo } from './helpers'

const S = 10_000_000
const ctl = (page: import('@playwright/test').Page, title: string) => page.locator(`.player .controls-row button[title^="${title}"]`)

test('aspect ratio menu changes the project resolution (undoable)', async ({ page }) => {
  await boot(page)
  await choose(page, page.locator('.player .ratio'), /^9:16$/)
  expect((await project(page)).settings).toMatchObject({ width: 1080, height: 1920 })
  await expect(page.locator('.player .ratio .select-btn')).toHaveText('9:16')
  await choose(page, page.locator('.player .ratio'), /^1:1$/)
  expect((await project(page)).settings).toMatchObject({ width: 1080, height: 1080 })
  await undo(page)
  await undo(page)
  expect((await project(page)).settings).toMatchObject({ width: 1920, height: 1080 })
})

test('quality menu sets the preview size and is remembered', async ({ page }) => {
  await boot(page)
  await choose(page, page.locator('.player .quality'), 'Full Quality')
  expect((await state(page)).previewQuality).toBe('full')
  expect(await page.evaluate(() => localStorage.getItem('jb.previewQuality'))).toBe('full')
})

test('transport: frame steps, play/pause and stop', async ({ page }) => {
  await boot(page)
  await ctl(page, 'Next frame').click()
  await ctl(page, 'Next frame').click()
  expect((await state(page)).playhead).toBe(Math.ceil((2 * S) / 30))
  await ctl(page, 'Previous frame').click()
  expect((await state(page)).playhead).toBe(Math.ceil(S / 30))
  await ctl(page, 'Play').click()
  expect((await state(page)).playing).toBe(true)
  await page.waitForTimeout(300)
  await ctl(page, 'Pause').click()
  const st = await state(page)
  expect(st.playing).toBe(false)
  expect(st.playhead).toBeGreaterThan(S / 30)
  await ctl(page, 'Stop').click()
  expect((await state(page)).playhead).toBe(0)
})

test('mark in/out, settings menu and markers', async ({ page }) => {
  await boot(page)
  await seek(page, 1)
  await ctl(page, 'Mark in').click()
  await seek(page, 3)
  await ctl(page, 'Mark out').click()
  expect(await state(page)).toMatchObject({ markIn: S, markOut: 3 * S })
  await page.locator('.player .controls-row button[title="Settings"]').click()
  await menuItem(page, 'Go to Mark In')
  expect((await state(page)).playhead).toBe(S)
  await page.locator('.player .controls-row button[title="Settings"]').click()
  await menuItem(page, 'Show Playback Statistics')
  expect((await state(page)).showStats).toBe(true)
  await page.locator('.player .controls-row button[title="Settings"]').click()
  await menuItem(page, 'Clear In/Out Marks')
  expect(await state(page)).toMatchObject({ markIn: null, markOut: null })
  await seek(page, 2)
  await ctl(page, 'Add marker').click()
  const markers = (await project(page)).sequences[0].markers
  expect(markers).toHaveLength(1)
  expect(markers[0]).toMatchObject({ time: 2 * S, name: 'Marker 1' })
  await page.keyboard.press('Escape')
  await seek(page, 3)
  await page.keyboard.press('m')
  expect((await project(page)).sequences[0].markers).toHaveLength(2)
})

test('snapshot saves the current frame', async ({ page }) => {
  await boot(page)
  await seek(page, 1)
  const download = page.waitForEvent('download')
  await page.locator('.player button[title="Snapshot"]').click()
  expect((await download).suggestedFilename()).toBe('UI Test 00-00-01-00.png')
})

test('volume popover sets the preview volume and mutes', async ({ page }) => {
  await boot(page)
  await page.locator('.player button[title="Volume"]').click()
  await page.locator('.vol-pop input[type=range]').fill('0.4')
  expect((await state(page)).volume).toBeCloseTo(0.4)
  await page.locator('.vol-pop button[title="Mute"]').click()
  expect((await state(page)).volume).toBe(0)
  await page.locator('.vol-pop button[title="Unmute"]').click()
  expect((await state(page)).volume).toBeCloseTo(0.4)
})

test('full screen preview fills the window and Esc leaves it', async ({ page }) => {
  await boot(page)
  await page.locator('.player button[title="Full screen"]').click()
  await expect(page.locator('.app.player-fs')).toHaveCount(1)
  await page.keyboard.press('Escape')
  await expect(page.locator('.app.player-fs')).toHaveCount(0)
})

test('video scopes draw over the player', async ({ page }) => {
  await boot(page)
  await page.locator('.player button[title="Video scopes"]').click()
  await expect(page.locator('.scopes canvas')).toHaveCount(2)
  await page.locator('.player button[title="Video scopes"]').click()
  await expect(page.locator('.scopes')).toHaveCount(0)
})

test('source monitor: open, mark a range, add it to the timeline', async ({ page }) => {
  const b = await boot(page)
  await expect(page.locator('.player-head button', { hasText: 'Source' })).toBeDisabled()
  await page.locator(`.media-card[data-media="${b.media.clip}"]`).dblclick()
  expect((await state(page)).playerMode).toBe('source')
  const video = page.locator('.screen.source video')
  await video.evaluate((v: HTMLVideoElement) => new Promise(r => (v.readyState >= 1 ? r(null) : v.addEventListener('loadedmetadata', () => r(null)))))
  await video.evaluate((v: HTMLVideoElement) => (v.currentTime = 1))
  await page.waitForTimeout(200)
  await page.keyboard.press('i')
  await video.evaluate((v: HTMLVideoElement) => (v.currentTime = 2.5))
  await page.waitForTimeout(200)
  await page.locator('.player button[title="Mark out (O)"]').click()
  const r = (await state(page)).sourceRanges[b.media.clip]
  expect(r[0]).toBeCloseTo(1, 1)
  expect(r[1]).toBeCloseTo(2.5, 1)
  await page.locator('.player button[title^="Add to timeline"]').click()
  const v1 = (await project(page)).sequences[0].tracks[0]
  const added = v1.clips[v1.clips.length - 1]
  expect(added.source_in).toBeCloseTo(S, -6)
  expect((added.end - added.start) / S).toBeCloseTo(1.5, 1)
  await page.locator('.player-head button', { hasText: 'Timeline' }).click()
  expect((await state(page)).playerMode).toBe('timeline')
})
