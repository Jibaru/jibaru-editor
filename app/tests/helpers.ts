// Shared setup: open the app on the mock backend with a small project.

import { expect, test as base, type Page } from '@playwright/test'
import fs from 'node:fs'
import path from 'node:path'
import { fixture } from './paths'

// With JB_CORE_JOURNAL=<dir>, every test leaves the project it started from
// and the commands the UI sent; tests/replay-core.mjs replays them on the real
// jb-core (the `jb` CLI) to check the mock and the core agree on every command.
const journalDir = process.env.JB_CORE_JOURNAL

/** Playwright's `test` with the journal fixture; specs import it from here. */
export const test = base.extend<{ journal: void }>({
  journal: [
    async ({ page }, use, info) => {
      await use()
      if (!journalDir) return
      const j = await page.evaluate(() => (window as any).__jbMock?.journal()).catch(() => null)
      if (!j?.commands?.length) return
      fs.mkdirSync(journalDir, { recursive: true })
      const name = `${path.basename(info.file, '.spec.ts')}--${info.title}`.replace(/[^\w.-]+/g, '_').slice(0, 120)
      fs.writeFileSync(path.join(journalDir, `${name}.json`), JSON.stringify(j))
    },
    { auto: true },
  ],
})
export { expect }

export interface Booted {
  /** Clip ids by name: `video` (clip.mp4 on V1 at 0..4 s), `title`, `audio` (tone.wav on A1). */
  clips: Record<string, string>
  media: Record<string, string>
  errors: string[]
}

/** Waits until no backend call is running (edits are async). */
export async function idle(page: Page) {
  await page.waitForFunction(() => (window as any).__jb.inflight() === 0)
  await page.evaluate(() => new Promise(r => requestAnimationFrame(() => r(null))))
}

/** The project as the mock backend holds it (what the core would have). */
export const project = async (page: Page) => {
  await idle(page)
  return page.evaluate(() => (window as any).__jbMock.project())
}

export const clipOf = async (page: Page, id: string) => {
  const p = await project(page)
  for (const s of p.sequences) for (const t of s.tracks) for (const c of t.clips) if (c.id === id) return c
  return null
}

export async function boot(page: Page, opts: { clips?: boolean } = {}): Promise<Booted> {
  const errors: string[] = []
  page.on('pageerror', e => errors.push(String(e)))
  page.on('console', m => m.type() === 'error' && !/Failed to load resource/.test(m.text()) && errors.push(m.text()))
  await page.goto('/')
  await page.waitForSelector('.app:not(.loading)')
  const paths = [fixture('clip.mp4'), fixture('tone.wav'), fixture('still.png'), fixture('green.mp4')]
  const ids = await page.evaluate(
    async ({ paths, withClips }) => {
      const w = window as any
      const assets = await w.__jbMock.probe(paths)
      const id = () => crypto.randomUUID()
      const f = (v: number) => ({ value: { type: 'float', value: v } })
      const tr = { position_x: f(0), position_y: f(0), scale_x: f(1), scale_y: f(1), rotation: f(0), flip_h: false, flip_v: false, crop: [0, 0, 0, 0], fit: 'fit' }
      const audio = { gain_db: f(0), pan: f(0), fade_in: 0, fade_out: 0, pitch: 0, muted: false }
      const clip = (name: string, source: unknown, start: number, end: number) => ({
        id: id(),
        name,
        source,
        start,
        end,
        source_in: 0,
        speed: { rate: 1, reverse: false, maintain_pitch: true },
        transform: structuredClone(tr),
        opacity: f(1),
        blend_mode: 'normal',
        audio: structuredClone(audio),
      })
      const S = 10_000_000
      const video = clip('clip.mp4', { type: 'media', media_id: assets[0].id }, 0, 4 * S)
      const after = clip('green.mp4', { type: 'media', media_id: assets[3].id }, 4 * S, 7 * S)
      const title = clip('Title', { type: 'title', text: 'Hello', font_family: 'Arial', font_size: 0.08, color: [1, 1, 1, 1], position: [0.5, 0.5], bold: false, italic: false }, 0, 3 * S)
      const tone = clip('tone.wav', { type: 'media', media_id: assets[1].id }, 0, 3 * S)
      const v1 = { id: id(), kind: 'video', name: 'Video 1', locked: false, muted: false, hidden: false, clips: withClips ? [video, after] : [] }
      const v2 = { id: id(), kind: 'video', name: 'Video 2', locked: false, muted: false, hidden: false, clips: withClips ? [title] : [] }
      const a1 = { id: id(), kind: 'audio', name: 'Audio 1', locked: false, muted: false, hidden: false, clips: withClips ? [tone] : [] }
      const seq = { id: id(), name: 'Main', tracks: [v1, v2, a1] }
      w.__jbMock.setProject({
        id: id(),
        name: 'UI Test',
        settings: { width: 1920, height: 1080, frame_rate: { num: 30, den: 1 }, sample_rate: 48000, background: [0, 0, 0, 1] },
        media: assets,
        sequences: [seq],
        main_sequence: seq.id,
      })
      return {
        clips: { video: video.id, after: after.id, title: title.id, audio: tone.id },
        media: { clip: assets[0].id, tone: assets[1].id, still: assets[2].id, green: assets[3].id },
      }
    },
    { paths, withClips: opts.clips ?? true },
  )
  return { ...ids, errors }
}

/** Selects a clip (like clicking it on the timeline) and opens a properties tab. */
export async function selectClip(page: Page, id: string, tab?: string) {
  await page.evaluate(id => (window as any).__jb.select([id]), id)
  if (tab) await page.locator('.prop-tabs button', { hasText: new RegExp(`^${tab}$`) }).click()
}

export const seek = (page: Page, seconds: number) => page.evaluate(t => (window as any).__jb.seek(t), Math.round(seconds * 10_000_000))

export const module = (page: Page, title: string) => page.locator(`section.module[data-module="${title}"]`)

/** Turns a module's switch on or off. */
export async function setModule(page: Page, title: string, on: boolean) {
  const sw = module(page, title).locator('.module-head .switch').first()
  if ((await sw.getAttribute('aria-checked')) !== String(on)) await sw.click()
  await expect(sw).toHaveAttribute('aria-checked', String(on))
}

/** Opens a module body when it is collapsed. */
export async function openModule(page: Page, title: string) {
  const m = module(page, title)
  if (!(await m.locator('.module-body').count())) await m.locator('.module-title').click()
}

/** Types a value into a slider row's number field. */
export async function setRow(page: Page, scope: ReturnType<Page['locator']>, label: string, value: number) {
  const row = scope.locator('.prow', { has: page.locator('.plabel', { hasText: new RegExp(`^${label}`) }) }).first()
  const input = row.locator('.numfield input').first()
  await input.click()
  await input.fill(String(value))
  await input.press('Enter')
}

/** Picks an option of a dropdown (ui/controls Select) inside `scope`. */
export async function choose(page: Page, scope: ReturnType<Page['locator']>, option: string | RegExp) {
  await scope.locator('.select-btn').first().click()
  await page.locator('.select-list button', { hasText: option }).first().click()
}

/** A property row by its label. */
export const row = (page: Page, scope: ReturnType<Page['locator']>, label: string) =>
  scope.locator('.prow', { has: page.locator('.plabel', { hasText: new RegExp(`^${label}`) }) }).first()

/** Clicks a context/dropdown menu item (ui/ContextMenu). */
export const menuItem = (page: Page, label: string | RegExp) => page.locator('.ctx-menu button', { hasText: label }).first().click()

export const state = async (page: Page) => (await idle(page), page.evaluate(() => {
  const s = (window as any).__jb.getState()
  return { playhead: s.playhead, playing: s.playing, markIn: s.markIn, markOut: s.markOut, volume: s.volume, previewQuality: s.previewQuality, showStats: s.showStats, playerMode: s.playerMode, selection: s.selection, selectedMedia: s.selectedMedia, playerFullscreen: s.playerFullscreen, sourceRanges: s.sourceRanges, snapping: s.snapping, layout: s.layout, exportOpen: s.exportOpen, libraryTab: s.libraryTab, zoom: s.zoom }
}))

export const effect = (clip: any, kind: string) => (clip.effects ?? []).find((e: any) => e.kind === kind)
export const pval = (e: any, name: string) => e?.params?.[name]?.value?.value

export async function undo(page: Page) {
  await page.evaluate(() => (window as any).__jb.undo())
}
