// Visual check helper (not part of the suite): JB_SHOTS=<dir> pnpm test:ui zz-shots
import { test, boot, selectClip } from './helpers'

const dir = process.env.JB_SHOTS
test.skip(!dir, 'set JB_SHOTS to a folder to take screenshots')

test('screenshots', async ({ page }) => {
  const b = await boot(page)
  await page.waitForTimeout(500)
  await page.screenshot({ path: `${dir}/00-app.png` })
  const shots: [string, string, string?][] = [
    ['video', 'Video'],
    ['video', 'Audio'],
    ['video', 'Speed'],
    ['video', 'Animation'],
    ['video', 'Color'],
    ['title', 'Titles'],
  ]
  for (const [clip, tab, sub] of shots) {
    await selectClip(page, b.clips[clip], tab)
    if (sub) await page.locator('.sub-tabs > button', { hasText: sub }).click()
    await page.waitForTimeout(200)
    await page.locator('.properties').screenshot({ path: `${dir}/${clip}-${tab}${sub ? '-' + sub : ''}.png` })
  }
  for (const sub of ['Mask', 'Matting']) {
    await selectClip(page, b.clips.video, 'Video')
    await page.locator('.sub-tabs > button', { hasText: sub }).click()
    await page.locator('.properties').screenshot({ path: `${dir}/video-Video-${sub}.png` })
  }
  for (const sub of ['HSL', 'Curves', 'Color Wheels']) {
    await selectClip(page, b.clips.video, 'Color')
    await page.locator('.sub-tabs > button', { hasText: sub }).click()
    await page.locator('.properties').screenshot({ path: `${dir}/video-Color-${sub}.png` })
  }
  await selectClip(page, b.clips.video, 'Speed')
  await page.locator('.sub-tabs > button', { hasText: 'Speed Ramping' }).click()
  await page.locator('[data-ramp="montage"]').click()
  await page.locator('[data-ramp="custom"]').click()
  await page.locator('.properties').screenshot({ path: `${dir}/video-Speed-ramp.png` })
  await page.locator('.lib-tabs button', { hasText: /^Effects$/ }).click()
  await page.locator('.library').screenshot({ path: `${dir}/lib-effects.png` })
  await page.locator('.player').screenshot({ path: `${dir}/player.png` })
})
