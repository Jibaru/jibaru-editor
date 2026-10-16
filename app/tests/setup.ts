// Generates the small media files the UI tests import (ffmpeg on PATH).

import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import { FIXTURES, fixture } from './paths'

export default function setup() {
  fs.mkdirSync(FIXTURES, { recursive: true })
  fs.mkdirSync(fixture('folder'), { recursive: true })
  const ff = (out: string, args: string[]) => {
    if (fs.existsSync(out)) return
    execFileSync('ffmpeg', ['-v', 'error', '-y', ...args, out])
  }
  ff(fixture('clip.mp4'), ['-f', 'lavfi', '-i', 'testsrc2=size=640x360:rate=30:duration=4', '-f', 'lavfi', '-i', 'sine=frequency=440:duration=4', '-c:v', 'mpeg4', '-q:v', '5', '-c:a', 'aac', '-shortest'])
  ff(fixture('green.mp4'), ['-f', 'lavfi', '-i', 'color=c=0x00ff00:size=640x360:rate=30:duration=3', '-c:v', 'mpeg4', '-q:v', '5'])
  ff(fixture('tone.wav'), ['-f', 'lavfi', '-i', 'sine=frequency=220:duration=3'])
  ff(fixture('still.png'), ['-f', 'lavfi', '-i', 'testsrc=size=320x180', '-frames:v', '1'])
  ff(fixture('folder/a.mp4'), ['-f', 'lavfi', '-i', 'testsrc=size=320x180:rate=30:duration=1', '-c:v', 'mpeg4'])
  ff(fixture('folder/b.wav'), ['-f', 'lavfi', '-i', 'sine=frequency=330:duration=1'])
  // A 2x2x2 warm LUT for the Color > LUT test.
  const lut = ['TITLE "warm"', 'LUT_3D_SIZE 2']
  for (let b = 0; b < 2; b++) for (let g = 0; g < 2; g++) for (let r = 0; r < 2; r++) lut.push(`${Math.min(1, r + 0.1)} ${g} ${b * 0.9}`)
  fs.writeFileSync(fixture('look.cube'), lut.join('\n') + '\n')
}
