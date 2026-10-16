import { type Page } from '@playwright/test'
import { expect, test, boot, choose, clipOf, effect, module, openModule, pval, row, seek, selectClip, setModule, setRow, undo } from './helpers'

const S = 10_000_000
const fv = (p: any) => p?.value?.value

async function setup(page: Page) {
  const b = await boot(page)
  await selectClip(page, b.clips.video, 'Video')
  return b
}

test('transform: scale (linked and unlinked), position, rotate, flip', async ({ page }) => {
  const b = await setup(page)
  const t = module(page, 'Transform')
  await setRow(page, t, 'Width', 150)
  let c = await clipOf(page, b.clips.video)
  expect([fv(c.transform.scale_x), fv(c.transform.scale_y)]).toEqual([1.5, 1.5])
  await t.locator('.scale-lock').click()
  await setRow(page, t, 'Height', 80)
  c = await clipOf(page, b.clips.video)
  expect([fv(c.transform.scale_x), fv(c.transform.scale_y)]).toEqual([1.5, 0.8])
  await t.locator('.xy-row').first().locator('input').first().fill('192')
  await t.locator('.xy-row').first().locator('input').first().press('Enter')
  c = await clipOf(page, b.clips.video)
  expect(fv(c.transform.position_x)).toBeCloseTo(0.1)
  await row(page, t, 'Rotate').locator('input').click()
  await row(page, t, 'Rotate').locator('input').fill('30')
  await row(page, t, 'Rotate').locator('input').press('Enter')
  await expect.poll(async () => fv((await clipOf(page, b.clips.video)).transform.rotation)).toBe(30)
  await t.locator('button[title="Rotate 90° clockwise"]').click()
  await t.locator('button[title="Flip horizontal"]').click()
  c = await clipOf(page, b.clips.video)
  expect(fv(c.transform.rotation)).toBe(120)
  expect(c.transform.flip_h).toBe(true)
  await undo(page)
  expect((await clipOf(page, b.clips.video)).transform.flip_h).toBe(false)
})

test('keyframes: diamond adds/removes, edits at the playhead write keys, arrows jump', async ({ page }) => {
  const b = await setup(page)
  const t = module(page, 'Transform')
  const posRow = t.locator('.xy-row').first()
  await seek(page, 1)
  await posRow.locator('.kf').click()
  let c = await clipOf(page, b.clips.video)
  expect(c.transform.position_x.keyframes.map((k: any) => k.time)).toEqual([S])
  expect(c.transform.position_y.keyframes.map((k: any) => k.time)).toEqual([S])
  await seek(page, 3)
  await posRow.locator('input').first().fill('384')
  await posRow.locator('input').first().press('Enter')
  c = await clipOf(page, b.clips.video)
  expect(c.transform.position_x.keyframes.map((k: any) => [k.time, k.value.value])).toEqual([
    [S, 0],
    [3 * S, 0.2],
  ])
  await posRow.locator('.kf-arrow[title="Previous keyframe"]').click()
  await expect.poll(async () => (await page.evaluate(() => (window as any).__jb.getState().playhead))).toBe(S)
  await expect(posRow.locator('.kf.on')).toHaveCount(1)
  await posRow.locator('.kf').click()
  c = await clipOf(page, b.clips.video)
  expect(c.transform.position_x.keyframes.map((k: any) => k.time)).toEqual([3 * S])
  // Module diamond keys every transform param at once.
  await seek(page, 2)
  await t.locator('.module-head .kf').click()
  c = await clipOf(page, b.clips.video)
  for (const k of ['position_x', 'position_y', 'scale_x', 'scale_y', 'rotation']) expect(c.transform[k].keyframes.some((x: any) => x.time === 2 * S)).toBe(true)
})

test('transform switch resets and restores; corner radius', async ({ page }) => {
  const b = await setup(page)
  const t = module(page, 'Transform')
  await setRow(page, t, 'Width', 120)
  await setModule(page, 'Transform', false)
  expect(fv((await clipOf(page, b.clips.video)).transform.scale_x)).toBe(1)
  await setModule(page, 'Transform', true)
  expect(fv((await clipOf(page, b.clips.video)).transform.scale_x)).toBe(1.2)
  await row(page, t, 'Corner Radius').locator('input').fill('25')
  await row(page, t, 'Corner Radius').locator('input').press('Enter')
  expect(pval(effect(await clipOf(page, b.clips.video), 'jb.corner_radius'), 'radius')).toBe(25)
})

test('compositing: blend mode and opacity', async ({ page }) => {
  const b = await setup(page)
  await openModule(page, 'Compositing')
  const m = module(page, 'Compositing')
  await choose(page, row(page, m, 'Blend Mode'), /^Multiply$/)
  await setRow(page, m, 'Opacity', 40)
  let c = await clipOf(page, b.clips.video)
  expect(c.blend_mode).toBe('multiply')
  expect(fv(c.opacity)).toBeCloseTo(0.4)
  await setModule(page, 'Compositing', false)
  c = await clipOf(page, b.clips.video)
  expect([c.blend_mode, fv(c.opacity)]).toEqual(['normal', 1])
  await setModule(page, 'Compositing', true)
  c = await clipOf(page, b.clips.video)
  expect(c.blend_mode).toBe('multiply')
})

for (const [title, kind, label, value, param] of [
  ['Background', 'jb.background', 'Blur Level', 70, 'blur'],
  ['Shadow', 'jb.shadow', 'Distance', 30, 'distance'],
  ['Auto Enhance', 'jb.auto_enhance', 'Amount', 80, 'amount'],
  ['Motion Blur', 'jb.motion_blur', 'Intensity', 60, 'intensity'],
  ['Video Denoise', 'jb.denoise', 'Strength', 70, 'strength'],
  ['Lens Correction', 'jb.lens_correction', 'Adjust Level', 40, 'level'],
] as const) {
  test(`${title}: switch adds/disables ${kind}, slider sets ${param}`, async ({ page }) => {
    const b = await setup(page)
    await setModule(page, title, true)
    await openModule(page, title)
    await setRow(page, module(page, title), label, value)
    let e = effect(await clipOf(page, b.clips.video), kind)
    expect(e.enabled).toBe(true)
    expect(pval(e, param)).toBe(value)
    await setModule(page, title, false)
    e = effect(await clipOf(page, b.clips.video), kind)
    expect(e.enabled).toBe(false)
  })
}

test('background color mode and shadow color', async ({ page }) => {
  const b = await setup(page)
  await setModule(page, 'Background', true)
  await openModule(page, 'Background')
  await choose(page, row(page, module(page, 'Background'), 'Type'), 'Color')
  await module(page, 'Background').locator('input[type=color]').fill('#ff0000')
  const e = effect(await clipOf(page, b.clips.video), 'jb.background')
  expect(pval(e, 'mode')).toBe(1)
  expect(e.params.color.value.value[0]).toBeCloseTo(1)
})

test('stabilization runs the motion analysis', async ({ page }) => {
  const b = await setup(page)
  await setModule(page, 'Stabilization', true)
  await expect.poll(async () => pval(effect(await clipOf(page, b.clips.video), 'jb.stabilize'), 'data')).toContain('|')
  await openModule(page, 'Stabilization')
  await expect(module(page, 'Stabilization')).toContainText('Motion analyzed')
})

test('cloud AI modules are cleanly unavailable', async ({ page }) => {
  await setup(page)
  await expect(module(page, 'AI Video Enhancer').locator('.switch')).toBeDisabled()
})

test('library effects get a module: params, disable, delete', async ({ page }) => {
  const b = await setup(page)
  await page.locator('.lib-tabs button', { hasText: /^Effects$/ }).click()
  await page.locator('.lib-side button', { hasText: /^Basic$/ }).click()
  await page.locator('.catalog-card[data-item="jb.vineta"]').dblclick()
  const m = module(page, 'Vignette')
  await expect(m).toHaveCount(1)
  await setRow(page, m, 'Intensity', 80)
  expect(pval(effect(await clipOf(page, b.clips.video), 'jb.vineta'), 'intensity')).toBe(80)
  await m.locator('.module-head .switch').click()
  expect(effect(await clipOf(page, b.clips.video), 'jb.vineta').enabled).toBe(false)
  await m.locator('button[title="Delete effect"]').click()
  expect(effect(await clipOf(page, b.clips.video), 'jb.vineta')).toBeUndefined()
})

test('masks: shapes, parameters, invert, remove; unsupported shapes disabled', async ({ page }) => {
  const b = await setup(page)
  await page.locator('.sub-tabs button', { hasText: 'Mask' }).click()
  await expect(page.locator('.mask-btn[data-mask="pen"]')).toBeDisabled()
  await page.locator('.mask-btn[data-mask="ellipse"]').click()
  expect(effect(await clipOf(page, b.clips.video), 'jb.mask.ellipse')).toBeTruthy()
  await page.locator('.mask-btn[data-mask="rectangle"]').click()
  let c = await clipOf(page, b.clips.video)
  expect(effect(c, 'jb.mask.ellipse')).toBeUndefined()
  expect(effect(c, 'jb.mask.rectangle')).toBeTruthy()
  const m = page.locator('.mask-pane')
  await setRow(page, m, 'Blur Strength', 30)
  await setRow(page, m, 'Roundness', 50)
  await row(page, m, 'Invert Mask').locator('.switch').click()
  c = await clipOf(page, b.clips.video)
  const e = effect(c, 'jb.mask.rectangle')
  expect([pval(e, 'feather'), pval(e, 'roundness'), pval(e, 'invert')]).toEqual([30, 50, true])
  await m.locator('button[title="Remove mask"]').click()
  expect(effect(await clipOf(page, b.clips.video), 'jb.mask.rectangle')).toBeUndefined()
})

test('chroma key: switch, color, tolerance, edges, alpha channel', async ({ page }) => {
  const b = await setup(page)
  await page.locator('.sub-tabs button', { hasText: 'Matting' }).click()
  await setModule(page, 'Chroma Key', true)
  const m = module(page, 'Chroma Key')
  await m.locator('button[title="Blue screen"]').click()
  await setRow(page, m, 'Tolerance', 35)
  await setRow(page, m, 'Edge Feather', 2)
  await row(page, m, 'Alpha Channel').locator('.switch').click()
  const e = effect(await clipOf(page, b.clips.video), 'jb.chroma_key')
  expect(e.params.color.value.value).toEqual([0, 0, 1, 1])
  expect([pval(e, 'tolerance'), pval(e, 'edge_feather'), pval(e, 'alpha_channel')]).toEqual([35, 2, true])
  await expect(module(page, 'AI Portrait').locator('.switch')).toBeDisabled()
})

test('reset restores the video tab defaults', async ({ page }) => {
  const b = await setup(page)
  await setRow(page, module(page, 'Transform'), 'Width', 150)
  await setModule(page, 'Shadow', true)
  await page.locator('.prop-footer button', { hasText: 'Reset' }).click()
  const c = await clipOf(page, b.clips.video)
  expect(fv(c.transform.scale_x)).toBe(1)
  expect(effect(c, 'jb.shadow')).toBeUndefined()
})
