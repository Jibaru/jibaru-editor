// Timeline transitions, markers and the timeline toolbar (mock backend).

import { boot, clipOf, expect, idle, menuItem, project, seek, selectClip, state, test, undo } from './helpers'
import type { Page } from '@playwright/test'

const S = 10_000_000

const tb = (page: Page, id: string) => page.locator(`.tl-toolbar [data-tb="${id}"]`)

async function zoom(page: Page, pps: number) {
  await page.evaluate(z => (window as any).__jb.setState({ zoom: z }), pps)
  await page.evaluate(() => ((document.querySelector('.tl-scroll') as HTMLElement).scrollLeft = 0))
}

/** Adds the default transition on the cut at 4 s with Apply Quick Transition (⇧T). */
async function quickTransition(page: Page) {
  await seek(page, 4)
  await page.keyboard.press('Shift+T')
  await idle(page)
}

test('a transition is drawn on the cut, selected, edited in its panel and deleted', async ({ page }) => {
  const b = await boot(page)
  await zoom(page, 60)
  await quickTransition(page)
  let v = await clipOf(page, b.clips.video)
  expect(v.transition_out.kind).toBe('jb.dissolve')
  expect(v.transition_out.duration).toBe(2 * S)
  expect(v.transition_out.alignment).toBe('center')

  // Drawn centered on the cut: 1 s each side at 60 px/s.
  const box = page.locator(`.transition-box[data-transition="${b.clips.video}"]`)
  await expect(box).toBeVisible()
  const lane = await box.evaluate(el => el.parentElement!.getBoundingClientRect().left)
  const r = (await box.boundingBox())!
  expect(Math.round(r.x - lane)).toBe(180)
  expect(Math.round(r.width)).toBe(120)
  await expect(box.locator('.transition-thumb')).toBeVisible()

  // ⇧T selected it; the properties panel shows the Transitions tab.
  await expect(box).toHaveClass(/sel/)
  const panel = page.locator('.transition-props')
  await expect(panel.locator('.prop-tabs button.on')).toHaveText('Transitions')
  await expect(panel.locator('.tr-duration')).toHaveValue('00:00:02:00')

  // Hover tooltip with the name and duration.
  await page.evaluate(() => (window as any).__jb.select([]))
  await expect(box).not.toHaveClass(/sel/)
  await box.hover({ position: { x: 30, y: 10 } })
  await expect(page.locator('.tb-tip.cursor')).toContainText('Transitions-')
  await expect(page.locator('.tb-tip.cursor')).toContainText('Duration: 00:00:02:00')
  await box.click({ position: { x: 30, y: 10 } })
  await expect(box).toHaveClass(/sel/)

  // Duration field.
  await panel.locator('.tr-duration').fill('00:00:01:00')
  await panel.locator('.tr-duration').press('Enter')
  await idle(page)
  v = await clipOf(page, b.clips.video)
  expect(v.transition_out.duration).toBe(S)

  // Alignment cards.
  await panel.locator('[data-align="end_at_cut"]').click()
  await idle(page)
  expect((await clipOf(page, b.clips.video)).transition_out.alignment).toBe('end_at_cut')
  await panel.locator('[data-align="center"]').click()
  await idle(page)
  expect((await clipOf(page, b.clips.video)).transition_out.alignment).toBe('center')

  // Dragging the right edge grows a centered transition on both sides.
  const rb = (await box.boundingBox())!
  await page.mouse.move(rb.x + rb.width - 2, rb.y + rb.height / 2)
  await page.mouse.down()
  await page.mouse.move(rb.x + rb.width + 28, rb.y + rb.height / 2, { steps: 5 })
  await page.mouse.up()
  await idle(page)
  v = await clipOf(page, b.clips.video)
  expect(Math.abs(v.transition_out.duration - 2 * S)).toBeLessThan(S / 15)

  // Duration dialog from the toolbar.
  await tb(page, 'duration').click()
  await page.locator('.duration-dialog .tc-input').fill('00:00:01:15')
  await page.locator('.duration-dialog .dlg-btn.primary').click()
  await idle(page)
  expect((await clipOf(page, b.clips.video)).transition_out.duration).toBe(1.5 * S)

  // Right-click menu, then Delete.
  await box.click({ button: 'right', position: { x: 10, y: 10 } })
  for (const item of ['Cut', 'Copy', 'Delete', 'Delete All Transitions', 'Apply Quick Transition', 'Duration', 'Locate in the Resource Panel', 'Find Similar', 'Enable Timeline Snapping'])
    await expect(page.locator('.ctx-menu button', { hasText: item }).first()).toBeVisible()
  await page.locator('.ctx-menu button', { has: page.locator('.label', { hasText: /^Delete$/ }) }).click()
  await idle(page)
  expect((await clipOf(page, b.clips.video)).transition_out).toBeUndefined()
  await expect(box).toHaveCount(0)
  await undo(page)
  await idle(page)
  expect((await clipOf(page, b.clips.video)).transition_out).toBeTruthy()

  // Delete key removes the selected transition.
  await page.locator(`.transition-box[data-transition="${b.clips.video}"]`).click({ position: { x: 20, y: 10 } })
  await page.keyboard.press('Backspace')
  await idle(page)
  expect((await clipOf(page, b.clips.video)).transition_out).toBeUndefined()
  expect(b.errors).toEqual([])
})

test('Apply to all puts the transition on every cut; Set as Default is used by Quick Transition', async ({ page }) => {
  const b = await boot(page)
  await zoom(page, 60)
  // A third clip so there are two cuts on Video 1.
  await page.evaluate(async id => {
    const w = window as any
    const p = w.__jb.getState().snap.project
    const t = p.sequences[0].tracks.find((t: any) => t.clips.some((c: any) => c.id === id))
    const c = structuredClone(t.clips[1])
    c.id = crypto.randomUUID()
    c.start = 7 * 10_000_000
    c.end = 9 * 10_000_000
    await w.__jb.exec({ op: 'add_clip', track_id: t.id, clip: c })
  }, b.clips.video)
  await quickTransition(page)
  const panel = page.locator('.transition-props')
  await panel.locator('.tr-duration').fill('00:00:00:20')
  await panel.locator('.tr-duration').press('Enter')
  await idle(page)
  await panel.locator('.split-btn .btn-accent').first().click()
  await idle(page)
  const p = await project(page)
  const v1 = p.sequences[0].tracks.find((t: any) => t.clips.some((c: any) => c.id === b.clips.video))
  expect(v1.clips[0].transition_out.duration).toBe(Math.round((20 / 30) * S))
  expect(v1.clips[1].transition_out?.kind).toBe('jb.dissolve')
  await panel.locator('.btn-gray', { hasText: 'Set as Default' }).click()
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem('jb.defaultTransition') ?? 'null')?.duration)).toBe(Math.round((20 / 30) * S))
})

test('the toolbar changes with the selection like Filmora', async ({ page }) => {
  const b = await boot(page)
  const visible = async () => page.locator('.tl-toolbar [data-tb]').evaluateAll(els => els.map(e => e.getAttribute('data-tb')))
  // Nothing selected: tools, then the AI group, no More.
  expect(await visible()).toEqual(expect.arrayContaining(['custom', 'pointer', 'undo', 'redo', 'delete', 'split', 'text', 'shapes', 'pen', 'ai-extend', 'snap', 'marker', 'voiceover', 'mixer', 'quick-preview', 'fit', 'zoom-out', 'zoom-in', 'meter']))
  expect(await visible()).not.toContain('crop')
  await expect(tb(page, 'delete')).toBeDisabled()

  await selectClip(page, b.clips.video)
  expect(await visible()).toEqual(expect.arrayContaining(['crop', 'speed', 'duration', 'keyframe', 'slide', 'more']))
  await expect(tb(page, 'delete')).toBeEnabled()

  await selectClip(page, b.clips.audio)
  expect(await visible()).toEqual(expect.arrayContaining(['speed', 'duration', 'keyframe']))
  expect(await visible()).not.toContain('crop')
  await expect(tb(page, 'keyframe')).toBeDisabled()

  await selectClip(page, b.clips.title)
  expect(await visible()).toEqual(expect.arrayContaining(['duration', 'keyframe']))
  expect(await visible()).not.toContain('speed')

  // Tooltips: label and key caps.
  await tb(page, 'split').hover()
  await expect(page.locator('.tb-tip')).toContainText('Split')
  await expect(page.locator('.tb-tip kbd')).toHaveCount(2)
  expect(b.errors).toEqual([])
})

test('toolbar buttons edit through the core', async ({ page }) => {
  const b = await boot(page)
  await zoom(page, 60)
  await selectClip(page, b.clips.video)
  // Speed > Fast > 2x, Normal, Reverse.
  await tb(page, 'speed').click()
  await page.locator('.ctx-menu button', { hasText: 'Fast' }).hover()
  await menuItem(page, /^2x$/)
  await idle(page)
  expect((await clipOf(page, b.clips.video)).speed.rate).toBe(2)
  await tb(page, 'speed').click()
  await menuItem(page, /^Normal$/)
  await idle(page)
  expect((await clipOf(page, b.clips.video)).speed.rate).toBe(1)
  await tb(page, 'speed').click()
  await menuItem(page, /^Reverse$/)
  await idle(page)
  expect((await clipOf(page, b.clips.video)).speed.reverse).toBe(true)

  // Freeze frame (⌥F) at the playhead.
  await seek(page, 1)
  const len = async () => Math.max(...(await project(page)).sequences[0].tracks[0].clips.map((c: any) => c.end))
  const before = await len()
  await page.keyboard.press('Alt+KeyF')
  await idle(page)
  expect(await len()).toBe(before + 5 * S)
  await undo(page)

  // Keyframing adds transform/opacity keyframes at the playhead.
  await selectClip(page, b.clips.video)
  await seek(page, 2)
  await tb(page, 'keyframe').click()
  await idle(page)
  const kf = (await clipOf(page, b.clips.video)).transform.position_x.keyframes
  expect(kf?.[0]?.time).toBe(2 * S)

  // Duration dialog on a clip trims it.
  await tb(page, 'duration').click()
  await page.locator('.duration-dialog .tc-input').fill('00:00:02:00')
  await page.locator('.duration-dialog .dlg-btn.primary').click()
  await idle(page)
  const v = await clipOf(page, b.clips.video)
  expect(v.end - v.start).toBe(2 * S)

  // Crop dialog: 16:9 → 1:1 crop.
  await tb(page, 'crop').click()
  await page.locator('.crop-dialog .dlg-chip', { hasText: '1:1' }).click()
  await page.locator('.crop-dialog .dlg-btn.primary').click()
  await idle(page)
  const crop = (await clipOf(page, b.clips.video)).transform.crop
  expect(crop[0]).toBeGreaterThan(0.2)

  // Snapping toggle, zoom buttons.
  const snap0 = (await state(page)).snapping
  await tb(page, 'snap').click()
  expect((await state(page)).snapping).toBe(!snap0)
  const z0 = (await state(page)).zoom
  await tb(page, 'zoom-in').click()
  expect((await state(page)).zoom).toBeGreaterThan(z0)
  await tb(page, 'zoom-out').click()
  await tb(page, 'zoom-out').click()
  expect((await state(page)).zoom).toBeLessThan(z0)

  // Undo / redo buttons.
  await tb(page, 'undo').click()
  await idle(page)
  expect((await clipOf(page, b.clips.video)).transform.crop[0]).toBe(0)
  await tb(page, 'redo').click()
  await idle(page)
  expect((await clipOf(page, b.clips.video)).transform.crop[0]).toBeGreaterThan(0.2)

  // Cloud AI buttons say so instead of doing nothing.
  await tb(page, 'ai-extend').click()
  await expect(page.locator('.toast')).toContainText('not supported')
  expect(b.errors).toEqual([])
})

test('markers: add with M, edit in the Marker Editor, delete', async ({ page }) => {
  const b = await boot(page)
  await zoom(page, 60)
  await page.evaluate(() => (window as any).__jb.select([]))
  await seek(page, 1.5)
  await page.keyboard.press('m')
  await idle(page)
  let p = await project(page)
  expect(p.sequences[0].markers?.length).toBe(1)
  expect(p.sequences[0].markers[0].time).toBe(1.5 * S)
  const mk = page.locator('.tl-marker')
  await expect(mk).toHaveCount(1)
  await seek(page, 3)
  await mk.dblclick()
  await page.locator('.marker-dialog .dlg-input').first().fill('Intro')
  await page.locator('.marker-dialog .swatch').nth(4).click()
  await page.locator('.marker-dialog .dlg-btn.primary').click()
  await idle(page)
  p = await project(page)
  expect(p.sequences[0].markers[0].name).toBe('Intro')
  await mk.click({ button: 'right' })
  await menuItem(page, 'Delete Marker')
  await idle(page)
  expect((await project(page)).sequences[0].markers ?? []).toHaveLength(0)
  expect(b.errors).toEqual([])
})

test('mouse tools: quick split and select all forward', async ({ page }) => {
  const b = await boot(page)
  await zoom(page, 60)
  await tb(page, 'pointer').click()
  await menuItem(page, 'Quick Split Mode')
  const clip = page.locator(`.clip[data-clip="${b.clips.video}"]`)
  await clip.click({ position: { x: 90, y: 30 } })
  await idle(page)
  const v1 = (await project(page)).sequences[0].tracks[0]
  expect(v1.clips.length).toBe(3)
  await page.keyboard.press('a')
  await page.locator(`.clip[data-clip="${b.clips.after}"]`).click({ position: { x: 5, y: 30 } })
  const sel = (await state(page)).selection
  expect(sel).toContain(b.clips.after)
  expect(sel).not.toContain(b.clips.video)
  await page.keyboard.press('v')
  expect(await page.evaluate(() => (window as any).__jbTl.getUi().tool)).toBe('select')
})

test('audio mixer applies track gain and pan to the clips', async ({ page }) => {
  const b = await boot(page)
  await tb(page, 'mixer').click()
  const strip = page.locator('.mixer-dialog .mix-strip', { hasText: 'Audio 1' })
  await strip.locator('.mix-db input').fill('-6')
  await strip.locator('.mix-num').first().fill('0.5')
  await page.locator('.mixer-dialog .dlg-btn.primary').click()
  await idle(page)
  const a = await clipOf(page, b.clips.audio)
  expect(a.audio.gain_db.value.value).toBe(-6)
  expect(a.audio.pan.value.value).toBe(0.5)
})
