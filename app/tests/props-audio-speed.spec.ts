import { expect, test, boot, choose, clipOf, module, openModule, row, seek, selectClip, setModule, setRow } from './helpers'

const S = 10_000_000
const fv = (p: any) => p?.value?.value

test('audio basic: volume (keyframes), balance, fades, pitch, channels, mute', async ({ page }) => {
  const b = await boot(page)
  await selectClip(page, b.clips.audio, 'Audio')
  const m = module(page, 'Adjustment')
  await setRow(page, m, 'Volume', -6)
  await row(page, m, 'Sound Balance').locator('input').click()
  await row(page, m, 'Sound Balance').locator('input').fill('-50')
  await row(page, m, 'Sound Balance').locator('input').press('Enter')
  await setRow(page, m, 'Fade In', 0.5)
  await setRow(page, m, 'Fade Out', 1)
  await setRow(page, m, 'Pitch', 2)
  await choose(page, row(page, m, 'Audio Channels'), 'Left to Stereo')
  await row(page, m, 'Mute').locator('.switch').click()
  let a = (await clipOf(page, b.clips.audio)).audio
  expect(fv(a.gain_db)).toBe(-6)
  expect(fv(a.pan)).toBe(-0.5)
  expect([a.fade_in, a.fade_out, a.pitch, a.channels, a.muted]).toEqual([0.5 * S, S, 2, 'left', true])
  // Volume keyframes through the diamond.
  await seek(page, 1)
  await row(page, m, 'Volume').locator('.kf').click()
  await seek(page, 2)
  await setRow(page, m, 'Volume', -20)
  a = (await clipOf(page, b.clips.audio)).audio
  expect(a.gain_db.keyframes.map((k: any) => [k.time, k.value.value])).toEqual([
    [S, -6],
    [2 * S, -20],
  ])
})

test('equalizer: presets and custom bands', async ({ page }) => {
  const b = await boot(page)
  await selectClip(page, b.clips.audio, 'Audio')
  const m = module(page, 'Adjustment')
  await choose(page, row(page, m, 'Equalizer'), 'Low Bass')
  let eq = (await clipOf(page, b.clips.audio)).audio.equalizer
  expect(eq.preset).toBe('low_bass')
  await row(page, m, 'Equalizer').locator('button', { hasText: 'Setting' }).click()
  const band = page.locator('.eq-band input[aria-label="1K Hz"]')
  await band.fill('6')
  await band.dispatchEvent('pointerup')
  eq = (await clipOf(page, b.clips.audio)).audio.equalizer
  expect(eq.bands[5]).toBe(6)
  await expect(row(page, m, 'Equalizer').locator('.select-btn')).toHaveText('Custom')
  await page.locator('.eq-bands button', { hasText: 'Reset' }).click()
  expect((await clipOf(page, b.clips.audio)).audio.equalizer).toBeUndefined()
})

test('auto normalization measures and sets a target', async ({ page }) => {
  const b = await boot(page)
  await selectClip(page, b.clips.audio, 'Audio')
  await setModule(page, 'Auto Normalization', true)
  await openModule(page, 'Auto Normalization')
  let n = (await clipOf(page, b.clips.audio)).audio.normalize
  expect(n).toMatchObject({ target_lufs: -23 })
  await choose(page, row(page, module(page, 'Auto Normalization'), 'Target'), 'Streaming')
  n = (await clipOf(page, b.clips.audio)).audio.normalize
  expect(n.target_lufs).toBe(-14)
  await expect(module(page, 'Auto Normalization')).toContainText('Measured')
  await setModule(page, 'Auto Normalization', false)
  expect((await clipOf(page, b.clips.audio)).audio.normalize).toBeUndefined()
})

test('denoise sets the noise reduction; AI modules are unavailable', async ({ page }) => {
  const b = await boot(page)
  await selectClip(page, b.clips.audio, 'Audio')
  await page.locator('.sub-tabs button', { hasText: 'Denoise' }).click()
  await setModule(page, 'Normal Denoise', true)
  expect((await clipOf(page, b.clips.audio)).audio.denoise_db).toBe(20)
  await setRow(page, module(page, 'Normal Denoise'), 'Strength', 75)
  expect((await clipOf(page, b.clips.audio)).audio.denoise_db).toBe(30)
  await expect(module(page, 'AI Speech Enhancement').locator('.switch')).toBeDisabled()
  await setModule(page, 'Normal Denoise', false)
  expect((await clipOf(page, b.clips.audio)).audio.denoise_db ?? null).toBeNull()
})

test('uniform speed: rate with ripple, duration, reverse, maintain pitch', async ({ page }) => {
  const b = await boot(page)
  await selectClip(page, b.clips.video, 'Speed')
  const pane = page.locator('.properties .plain')
  await row(page, pane, 'Speed').locator('input').click()
  await row(page, pane, 'Speed').locator('input').fill('2')
  await row(page, pane, 'Speed').locator('input').press('Enter')
  let c = await clipOf(page, b.clips.video)
  expect(c.speed.rate).toBe(2)
  expect(c.end).toBe(2 * S)
  // Ripple Edit pulled the next clip back to the new end.
  expect((await clipOf(page, b.clips.after)).start).toBe(2 * S)
  const dur = row(page, pane, 'Duration').locator('input')
  await expect(dur).toHaveValue('00:00:02:00')
  await dur.fill('00:00:01:00')
  await dur.press('Enter')
  c = await clipOf(page, b.clips.video)
  expect(c.speed.rate).toBe(4)
  await row(page, pane, 'Reverse Speed').locator('.switch').click()
  await row(page, pane, 'Maintain Pitch').locator('.switch').click()
  c = await clipOf(page, b.clips.video)
  expect([c.speed.reverse, c.speed.maintain_pitch]).toEqual([true, false])
  await row(page, pane, 'Ripple Edit').locator('.switch').click()
  expect(await page.evaluate(() => localStorage.getItem('jb.speed.ripple'))).toBe('0')
  await expect(page.locator('.select-list button:disabled')).toHaveCount(0)
  await choose(page, row(page, pane, 'Frame Interpolation'), /Frame Sampling/)
})

test('speed ramping: presets, custom curve editing, none', async ({ page }) => {
  const b = await boot(page)
  await selectClip(page, b.clips.video, 'Speed')
  await page.locator('.sub-tabs button', { hasText: 'Speed Ramping' }).click()
  await page.locator('[data-ramp="flash_in"]').click()
  let c = await clipOf(page, b.clips.video)
  expect(c.speed.curve.preset).toBe('flash_in')
  await expect(page.locator('[data-ramp="flash_in"]')).toHaveClass(/on/)
  await page.locator('[data-ramp="custom"]').click()
  const pt = page.locator('.ramp-point[data-point="2"]')
  const box = (await pt.boundingBox())!
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
  await page.mouse.down()
  await page.mouse.move(box.x + box.width / 2, box.y - 30, { steps: 4 })
  await page.mouse.up()
  c = await clipOf(page, b.clips.video)
  expect(c.speed.curve.points[2].speed).toBeGreaterThan(1.5)
  await page.locator('[data-ramp="none"]').click()
  c = await clipOf(page, b.clips.video)
  expect(c.speed.curve).toBeUndefined()
  expect(Math.abs(c.end - 4 * S)).toBeLessThan(100)
})
