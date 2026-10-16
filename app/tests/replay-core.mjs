// Replays the commands recorded by the UI tests (JB_CORE_JOURNAL) on the real
// jb-core through the `jb` CLI, so every command the panels send is checked
// against the Rust command bus, not only against the browser mock.
//
//   JB_CORE_JOURNAL=/tmp/jb-journal pnpm test:ui
//   node tests/replay-core.mjs /tmp/jb-journal [path/to/jb]

import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

const dir = process.argv[2]
const jb = process.argv[3] ?? path.resolve(import.meta.dirname, '../../target/debug/jb')
if (!dir) {
  console.error('usage: node tests/replay-core.mjs <journal dir> [jb binary]')
  process.exit(2)
}
const work = fs.mkdtempSync(path.join(os.tmpdir(), 'jb-replay-'))
let failed = 0
let commands = 0
for (const f of fs.readdirSync(dir).filter(f => f.endsWith('.json')).sort()) {
  const { initial, commands: cmds } = JSON.parse(fs.readFileSync(path.join(dir, f), 'utf8'))
  const proj = path.join(work, 'p.jbp')
  const list = path.join(work, 'cmds.jsonl')
  fs.writeFileSync(proj, JSON.stringify({ format: 'jibaru-project', version: 1, project: initial }))
  fs.writeFileSync(list, cmds.map(c => JSON.stringify(c)).join('\n') + '\n')
  commands += cmds.length
  try {
    execFileSync(jb, ['--json', 'exec', proj, '--file', list, '--output', path.join(work, 'out.jbp')], { stdio: 'pipe' })
    console.log(`ok   ${f} (${cmds.length} commands)`)
  } catch (e) {
    failed++
    const out = String(e.stdout || e.stderr || e.message)
    console.log(`FAIL ${f}\n     ${out.trim().slice(0, 600)}`)
  }
}
console.log(`\n${commands} commands replayed, ${failed} journal(s) rejected by jb-core`)
process.exit(failed ? 1 : 0)
