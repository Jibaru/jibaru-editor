// Dev-only stand-in for the app's `jb://` media protocol, so the UI can run in
// a plain browser (with the mock backend) and still show real thumbnails,
// waveforms and video. It shells out to ffmpeg/ffprobe like the Tauri side.
// Endpoints mirror src-tauri/src/media.rs:
//   /__jb/thumb?path=&t=&h=   JPEG frame
//   /__jb/wave?path=&pps=     JSON { peaks: number[] } (peaks per second)
//   /__jb/file?path=          raw file with Range support
//   /__jb/probe?path=         JSON MediaAsset

import { spawn } from 'node:child_process'
import { createHash } from 'node:crypto'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import type { IncomingMessage, ServerResponse } from 'node:http'
import type { Plugin } from 'vite'

const cache = path.join(os.tmpdir(), 'jb-dev-media-cache')
fs.mkdirSync(cache, { recursive: true })

function run(cmd: string, args: string[]): Promise<Buffer> {
  return new Promise((resolve, reject) => {
    const p = spawn(cmd, args, { stdio: ['ignore', 'pipe', 'ignore'] })
    const chunks: Buffer[] = []
    p.stdout.on('data', c => chunks.push(c))
    p.on('error', reject)
    p.on('close', code => (code === 0 ? resolve(Buffer.concat(chunks)) : reject(new Error(`${cmd} exited ${code}`))))
  })
}

const key = (...parts: unknown[]) => createHash('sha1').update(parts.join('|')).digest('hex')
const pending = new Map<string, Promise<Buffer>>()

function cached(name: string, make: () => Promise<Buffer>): Promise<Buffer> {
  const file = path.join(cache, name)
  if (fs.existsSync(file)) return Promise.resolve(fs.readFileSync(file))
  let p = pending.get(name)
  if (!p) {
    p = make().then(buf => {
      fs.writeFileSync(file, buf)
      pending.delete(name)
      return buf
    })
    p.catch(() => pending.delete(name))
    pending.set(name, p)
  }
  return p
}

// Limit concurrent ffmpeg processes; thumbnail strips request many at once.
let active = 0
const queue: (() => void)[] = []
async function limited<T>(fn: () => Promise<T>): Promise<T> {
  if (active >= 4) await new Promise<void>(r => queue.push(r))
  active++
  try {
    return await fn()
  } finally {
    active--
    queue.shift()?.()
  }
}

const isImage = (p: string) => /\.(jpe?g|png|heic|webp|bmp|gif|tiff?)$/i.test(p)

async function thumb(file: string, t: number, h: number): Promise<Buffer> {
  const stat = fs.statSync(file)
  const tt = isImage(file) ? 0 : Math.max(0, t)
  return cached(key('thumb', file, stat.mtimeMs, tt.toFixed(2), h) + '.jpg', () =>
    limited(() =>
      run('ffmpeg', [
        '-v', 'error', ...(tt > 0 ? ['-ss', tt.toFixed(3)] : []), '-i', file,
        '-frames:v', '1', '-vf', `scale=-2:${h}`, '-q:v', '5', '-f', 'image2', '-c:v', 'mjpeg', 'pipe:1',
      ]),
    ),
  )
}

async function wave(file: string, pps: number): Promise<Buffer> {
  const stat = fs.statSync(file)
  return cached(key('wave', file, stat.mtimeMs, pps) + '.json', async () => {
    const rate = 8000
    const pcm = await limited(() => run('ffmpeg', ['-v', 'error', '-i', file, '-vn', '-ac', '1', '-ar', String(rate), '-f', 's16le', 'pipe:1']))
    const samples = new Int16Array(pcm.buffer, pcm.byteOffset, Math.floor(pcm.byteLength / 2))
    const per = Math.max(1, Math.round(rate / pps))
    const peaks: number[] = []
    for (let i = 0; i < samples.length; i += per) {
      let m = 0
      for (let j = i; j < Math.min(i + per, samples.length); j++) m = Math.max(m, Math.abs(samples[j]))
      peaks.push(Math.round((m / 32768) * 1000) / 1000)
    }
    return Buffer.from(JSON.stringify({ pps, peaks }))
  })
}

async function probe(file: string) {
  const out = await run('ffprobe', ['-v', 'error', '-print_format', 'json', '-show_format', '-show_streams', file])
  const info = JSON.parse(out.toString())
  const streams: { codec_type: string; width?: number; height?: number; avg_frame_rate?: string; disposition?: { attached_pic?: number } }[] =
    info.streams ?? []
  const video = streams.find(s => s.codec_type === 'video' && !s.disposition?.attached_pic)
  const audio = streams.find(s => s.codec_type === 'audio')
  const image = isImage(file)
  const seconds = Number(info.format?.duration ?? 0)
  let frame_rate: { num: number; den: number } | undefined
  if (video?.avg_frame_rate && video.avg_frame_rate !== '0/0') {
    const [num, den] = video.avg_frame_rate.split('/').map(Number)
    if (num && den) frame_rate = { num, den }
  }
  return {
    id: key('media', file).slice(0, 16),
    name: path.basename(file),
    path: file,
    kind: image ? 'image' : video ? 'video' : 'audio',
    duration: image ? 5 * 10_000_000 : Math.round(seconds * 10_000_000),
    width: video?.width ?? 0,
    height: video?.height ?? 0,
    ...(frame_rate && !image ? { frame_rate } : {}),
    has_audio: !!audio,
  }
}

const MIME: Record<string, string> = {
  '.mp4': 'video/mp4', '.mov': 'video/quicktime', '.m4v': 'video/mp4', '.webm': 'video/webm', '.mkv': 'video/x-matroska',
  '.mp3': 'audio/mpeg', '.m4a': 'audio/mp4', '.wav': 'audio/wav', '.aac': 'audio/aac', '.ogg': 'audio/ogg',
  '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.png': 'image/png', '.webp': 'image/webp', '.gif': 'image/gif',
}

function serveFile(req: IncomingMessage, res: ServerResponse, file: string) {
  const size = fs.statSync(file).size
  const type = MIME[path.extname(file).toLowerCase()] ?? 'application/octet-stream'
  const range = /bytes=(\d*)-(\d*)/.exec(req.headers.range ?? '')
  if (range) {
    const start = range[1] ? Number(range[1]) : 0
    const end = range[2] ? Math.min(Number(range[2]), size - 1) : size - 1
    res.writeHead(206, { 'Content-Type': type, 'Content-Range': `bytes ${start}-${end}/${size}`, 'Accept-Ranges': 'bytes', 'Content-Length': end - start + 1 })
    fs.createReadStream(file, { start, end }).pipe(res)
  } else {
    res.writeHead(200, { 'Content-Type': type, 'Content-Length': size, 'Accept-Ranges': 'bytes' })
    fs.createReadStream(file).pipe(res)
  }
}

const MEDIA_EXT = /\.(mp4|mov|m4v|mkv|webm|avi|mts|mp3|wav|m4a|aac|flac|ogg|jpe?g|png|heic|webp|gif)$/i

function listMedia(dir: string): string[] {
  if (!fs.statSync(dir).isDirectory()) return [dir]
  const out: string[] = []
  for (const name of fs.readdirSync(dir).sort()) {
    if (name.startsWith('.')) continue
    const p = path.join(dir, name)
    if (fs.statSync(p).isDirectory()) out.push(...listMedia(p))
    else if (MEDIA_EXT.test(name)) out.push(p)
  }
  return out
}

export function devMediaServer(): Plugin {
  return {
    name: 'jb-dev-media',
    apply: 'serve',
    configureServer(server) {
      server.middlewares.use('/__jb', (req, res) => {
        const url = new URL(req.url ?? '/', 'http://x')
        const file = url.searchParams.get('path') ?? ''
        const fail = (code: number, msg: string) => {
          res.statusCode = code
          res.end(msg)
        }
        if (!file || !fs.existsSync(file)) return fail(404, 'not found')
        const route = url.pathname.replace(/^\//, '')
        const send = (type: string) => (buf: Buffer) => {
          res.writeHead(200, { 'Content-Type': type, 'Cache-Control': 'max-age=31536000' })
          res.end(buf)
        }
        const onErr = (e: Error) => fail(500, e.message)
        if (route === 'thumb') {
          thumb(file, Number(url.searchParams.get('t') ?? 0), Number(url.searchParams.get('h') ?? 90)).then(send('image/jpeg'), onErr)
        } else if (route === 'wave') {
          wave(file, Number(url.searchParams.get('pps') ?? 20)).then(send('application/json'), onErr)
        } else if (route === 'probe') {
          probe(file).then(m => send('application/json')(Buffer.from(JSON.stringify(m))), onErr)
        } else if (route === 'file') {
          serveFile(req, res, file)
        } else if (route === 'list') {
          // Media files under a folder (Import Media Folder), like the app's import_media.
          send('application/json')(Buffer.from(JSON.stringify(listMedia(file))))
        } else fail(404, 'unknown route')
      })
    },
  }
}
