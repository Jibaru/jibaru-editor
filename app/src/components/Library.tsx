import { useMemo, useRef, useState, type ReactNode } from 'react'
import { backend } from '../backend'
import { mediaInUse, seconds, shortDuration } from '../core/model'
import type { MediaAsset, MediaKind } from '../core/types'
import { CatalogBody } from './CatalogLibrary'
import { appendMedia, deleteSelection, exec, importMedia, openMenu, setState, toast, useEditor, type LibraryTab } from '../store'
import { sep, type MenuItem } from '../ui/ContextMenu'
import { startLibraryDrag } from '../drag'
import {
  IcAudio,
  IcCheck,
  IcChevronDown,
  IcEffects,
  IcFilm,
  IcFilter,
  IcFilters,
  IcImage,
  IcMedia,
  IcMic,
  IcMore,
  IcNote,
  IcPlus,
  IcRecord,
  IcSearch,
  IcStickers,
  IcStock,
  IcTemplates,
  IcTitles,
  IcTransitions,
  IcWebcam,
  IcCaretDown,
  IcCaretRight,
} from '../ui/icons'

const TABS: { id: LibraryTab; label: string; icon: (p: { size?: number; strokeWidth?: number }) => ReactNode }[] = [
  { id: 'media', label: 'Media', icon: IcMedia },
  { id: 'stock', label: 'Stock Media', icon: IcStock },
  { id: 'audio', label: 'Audio', icon: IcAudio },
  { id: 'titles', label: 'Titles', icon: IcTitles },
  { id: 'transitions', label: 'Transitions', icon: IcTransitions },
  { id: 'effects', label: 'Effects', icon: IcEffects },
  { id: 'filters', label: 'Filters', icon: IcFilters },
  { id: 'stickers', label: 'Stickers', icon: IcStickers },
  { id: 'templates', label: 'Templates', icon: IcTemplates },
]

/**
 * A sidebar entry: what it shows (`view`), or an empty-state message for
 * Wondershare cloud content that Jibaru does not have.
 */
type SideItem = { label: string; caret?: 'open' | 'closed'; child?: boolean; view?: string; empty?: string }

const CLOUD = 'is Wondershare cloud content and is not available in Jibaru Editor.'

const SIDEBARS: Partial<Record<LibraryTab, SideItem[]>> = {
  media: [
    { label: 'Project Media', caret: 'open', view: 'all' },
    { label: 'Videos', child: true, view: 'video' },
    { label: 'Audio', child: true, view: 'audio' },
    { label: 'Images', child: true, view: 'image' },
    { label: 'Global Media', caret: 'closed', empty: 'Global media (shared between projects) is not available yet.' },
    { label: 'Cloud Media', empty: `Cloud media ${CLOUD}` },
    { label: 'Editing Presets', caret: 'closed', empty: 'Saved editing presets are not available yet. Color presets saved with "Save as Custom" are in Color > Preset > Custom.' },
    { label: 'Photos Library', empty: 'The Photos library is not available yet. Use Import to add photos.' },
  ],
  audio: [
    { label: 'Project Audio', caret: 'open', view: 'all' },
    { label: 'Music', empty: `The music library ${CLOUD} Import your own music to use it.` },
    { label: 'Sound Effects', empty: `The sound effects library ${CLOUD} Import your own sounds to use them.` },
  ],
  stock: [
    { label: 'Videos', caret: 'open', empty: `Stock video ${CLOUD}` },
    { label: 'Photos', empty: `Stock photos ${CLOUD}` },
  ],
  stickers: [{ label: 'Default', caret: 'open', empty: `The sticker library ${CLOUD}` }],
  templates: [{ label: 'Default', caret: 'open', empty: `The template library ${CLOUD}` }],
}

const SIZE_KEY = 'jb.library.size'
const storedSize = () => {
  try {
    return Number(localStorage.getItem(SIZE_KEY)) || 116
  } catch {
    return 116
  }
}

export function Library() {
  const tab = useEditor(s => s.libraryTab)
  const [sides, setSides] = useState<Partial<Record<LibraryTab, number>>>({})
  const [size, setSizeState] = useState(storedSize)
  const setSize = (v: number) => {
    setSizeState(v)
    try {
      localStorage.setItem(SIZE_KEY, String(Math.round(v)))
    } catch {
      /* storage unavailable */
    }
  }
  const side = sides[tab] ?? 0
  const setSide = (i: number) => setSides(s => ({ ...s, [tab]: i }))
  const catalog = tab === 'titles' || tab === 'transitions' || tab === 'effects' || tab === 'filters'
  const sidebar: SideItem[] = SIDEBARS[tab] ?? []
  const entry = sidebar[side] ?? sidebar[0]
  return (
    <section className="panel library" style={{ ['--card-w' as string]: `${size}px` }}>
      <div className="panel-head lib-tabs">
        {TABS.map(t => (
          <button key={t.id} className={tab === t.id ? 'on' : ''} onClick={() => setState({ libraryTab: t.id })}>
            <t.icon size={25} strokeWidth={1.7} />
            <span>{t.label}</span>
          </button>
        ))}
      </div>
      <div className={`lib-body ${catalog ? 'catalog' : ''}`}>
        {catalog ? (
          <CatalogBody tab={tab} />
        ) : (
          <>
            <aside className="lib-side">
              {sidebar.map((s, i) => (
                <button key={s.label} className={`${i === side ? 'on' : ''} ${s.child ? 'child' : ''} ${s.caret || !s.child ? 'boxed' : ''}`} onClick={() => setSide(i)}>
                  <span className="caret">{s.caret === 'open' ? <IcCaretDown size={10} /> : s.caret === 'closed' ? <IcCaretRight size={10} /> : null}</span>
                  {s.label}
                </button>
              ))}
            </aside>
            <div className="lib-main">
              {entry?.empty ? (
                <>
                  <Toolbar size={size} setSize={setSize} importButton={tab === 'media' || tab === 'audio'} />
                  <div className="lib-empty">{entry.empty}</div>
                </>
              ) : tab === 'media' ? (
                <MediaGrid size={size} setSize={setSize} kind={entry?.view && entry.view !== 'all' ? (entry.view as MediaKind) : null} />
              ) : (
                <MediaGrid size={size} setSize={setSize} kind="audio" />
              )}
            </div>
          </>
        )}
      </div>
    </section>
  )
}

type SortKey = 'added' | 'name' | 'duration' | 'type'

function Toolbar({
  importButton = true,
  size,
  setSize,
  filterMenu,
  moreMenu,
}: {
  importButton?: boolean
  size: number
  setSize: (v: number) => void
  filterMenu?: () => MenuItem[]
  moreMenu?: () => MenuItem[]
}) {
  const at = (e: React.MouseEvent, items: MenuItem[]) => {
    const r = e.currentTarget.getBoundingClientRect()
    openMenu(r.left, r.bottom + 2, items)
  }
  return (
    <div className="lib-toolbar">
      {importButton && (
        <button
          className="lib-import"
          onClick={e =>
            at(e, [
              { label: 'Import Media Files...', action: () => importMedia() },
              {
                label: 'Import Media Folder...',
                action: async () => {
                  const dir = await backend.pickFiles({ title: 'Import Media Folder', directory: true, multiple: false })
                  if (dir?.[0]) await importMedia(dir)
                },
              },
              sep,
              { label: 'Import from a Camera or Phone', disabled: true },
            ])
          }
        >
          Import <IcChevronDown size={14} />
        </button>
      )}
      {importButton && (
        <div className="lib-record">
          <button title="Screen recording is not available yet" disabled>
            <IcRecord size={16} />
          </button>
          <i />
          <button title="Voiceover recording is not available yet" disabled>
            <IcMic size={16} />
          </button>
          <i />
          <button title="Webcam recording is not available yet" disabled>
            <IcWebcam size={16} />
          </button>
        </div>
      )}
      <span className="spacer" />
      <SizeSlider value={size} onChange={setSize} />
      <button className="ticon" title={filterMenu ? 'Sort and filter' : 'Nothing to filter here'} disabled={!filterMenu} onClick={e => filterMenu && at(e, filterMenu())}>
        <IcFilter size={18} />
      </button>
      <button className="ticon" title="More" disabled={!moreMenu} onClick={e => moreMenu && at(e, moreMenu())}>
        <IcMore size={18} />
      </button>
    </div>
  )
}

/** Thumbnail size (Filmora's slider next to the filter button). */
function SizeSlider({ value, onChange }: { value: number; onChange: (v: number) => void }) {
  const ref = useRef<HTMLDivElement>(null)
  const MIN = 80
  const MAX = 200
  const k = (value - MIN) / (MAX - MIN)
  const at = (x: number) => {
    const r = ref.current!.getBoundingClientRect()
    return Math.round(MIN + Math.max(0, Math.min(1, (x - r.left) / r.width)) * (MAX - MIN))
  }
  return (
    <div
      className="lib-size"
      ref={ref}
      title="Thumbnail size"
      role="slider"
      aria-valuemin={MIN}
      aria-valuemax={MAX}
      aria-valuenow={value}
      onPointerDown={e => {
        onChange(at(e.clientX))
        const mv = (ev: PointerEvent) => onChange(at(ev.clientX))
        const up = () => {
          window.removeEventListener('pointermove', mv)
          window.removeEventListener('pointerup', up)
        }
        window.addEventListener('pointermove', mv)
        window.addEventListener('pointerup', up)
      }}
    >
      <div className="lib-size-track" />
      <div className="lib-size-knob" style={{ left: `calc(${k * 100}% - ${k * 14}px)` }} />
    </div>
  )
}

function Search({ value, onChange, placeholder }: { value: string; onChange: (v: string) => void; placeholder: string }) {
  return (
    <label className="lib-search">
      <IcSearch size={15} />
      <input value={value} placeholder={placeholder} onChange={e => onChange(e.target.value)} onKeyDown={e => e.stopPropagation()} />
    </label>
  )
}

function MediaGrid({ kind, size, setSize }: { kind: MediaKind | null; size: number; setSize: (v: number) => void }) {
  const project = useEditor(s => s.snap?.project)
  const selected = useEditor(s => s.selectedMedia)
  const [query, setQuery] = useState('')
  const [sort, setSort] = useState<SortKey>('added')
  const [unusedOnly, setUnusedOnly] = useState(false)
  const media = useMemo(() => {
    const list = (project?.media ?? []).filter(m => (!kind || m.kind === kind) && m.name.toLowerCase().includes(query.toLowerCase()) && (!unusedOnly || !mediaInUse(project!, m.id)))
    if (sort === 'name') list.sort((a, b) => a.name.localeCompare(b.name))
    if (sort === 'duration') list.sort((a, b) => b.duration - a.duration)
    if (sort === 'type') list.sort((a, b) => a.kind.localeCompare(b.kind))
    return list
  }, [project, query, kind, sort, unusedOnly])
  if (!project) return null
  const unused = project.media.filter(m => !mediaInUse(project, m.id))
  return (
    <>
      <Toolbar
        size={size}
        setSize={setSize}
        filterMenu={() => [
          { label: 'Sort by Date Added', checked: sort === 'added', action: () => setSort('added') },
          { label: 'Sort by Name', checked: sort === 'name', action: () => setSort('name') },
          { label: 'Sort by Duration', checked: sort === 'duration', action: () => setSort('duration') },
          { label: 'Sort by Type', checked: sort === 'type', action: () => setSort('type') },
          sep,
          { label: 'Show Unused Only', checked: unusedOnly, action: () => setUnusedOnly(!unusedOnly) },
        ]}
        moreMenu={() => [
          { label: 'Select All', disabled: !media.length, action: () => setState({ selectedMedia: media.map(m => m.id), selection: [] }) },
          {
            label: `Delete Unused Media (${unused.length})`,
            disabled: !unused.length,
            action: () => exec({ op: 'batch', commands: unused.map(m => ({ op: 'remove_media', media_id: m.id })) }),
          },
        ]}
      />
      <Search value={query} onChange={setQuery} placeholder={kind === 'audio' ? 'Search audio' : 'Search media'} />
      <div
        className="lib-grid"
        onMouseDown={e => {
          if (e.target === e.currentTarget) setState({ selectedMedia: [] })
        }}
      >
        <div className="card import-card" onClick={() => importMedia()}>
          <div className="thumb">
            <IcPlus size={30} strokeWidth={1.6} />
          </div>
          <div className="name">Import Media</div>
        </div>
        {media.map(m => (
          <MediaCard key={m.id} media={m} used={mediaInUse(project, m.id)} selected={selected.includes(m.id)} />
        ))}
      </div>
    </>
  )
}

function MediaCard({ media, used, selected }: { media: MediaAsset; used: boolean; selected: boolean }) {
  const [broken, setBroken] = useState(false)
  const [renaming, setRenaming] = useState<string | null>(null)
  const proxy = useEditor(s => s.proxies[media.id])
  const range = useEditor(s => s.sourceRanges[media.id])
  const at = media.kind === 'video' ? Math.min(seconds(media.duration) * 0.1, 3) : 0
  const openSource = () => setState({ playerMode: 'source', sourceMedia: media.id, sourceTime: 0, playing: false })
  const rename = (name: string) => {
    setRenaming(null)
    if (name.trim() && name.trim() !== media.name) exec({ op: 'rename_media', media_id: media.id, name: name.trim() })
  }
  return (
    <div
      className={`card media-card ${selected ? 'on' : ''}`}
      data-media={media.id}
      onPointerDown={e => {
        if (e.button !== 0 || renaming != null) return
        setState(s => ({
          selectedMedia: e.metaKey || e.ctrlKey || e.shiftKey ? [...new Set([...s.selectedMedia, media.id])] : [media.id],
          selection: [],
        }))
        startLibraryDrag(e, { type: 'media', id: media.id, label: media.name })
      }}
      onDoubleClick={openSource}
      onContextMenu={e => {
        e.preventDefault()
        setState({ selectedMedia: [media.id], selection: [] })
        openMenu(e.clientX, e.clientY, [
          { label: 'Add to Timeline', action: () => appendMedia(media) },
          { label: 'Open in Source Monitor', action: openSource },
          sep,
          { label: 'Rename', action: () => setRenaming(media.name) },
          { label: backend.kind === 'tauri' && navigator.userAgent.includes('Windows') ? 'Show in Explorer' : 'Reveal in Finder', disabled: !backend.engine, action: () => backend.engine?.reveal(media.path) },
          { label: 'Copy File Path', action: () => navigator.clipboard?.writeText(media.path).then(() => toast('Path copied'), () => toast(media.path)) },
          sep,
          { label: used ? 'Delete (in use on the timeline)' : 'Delete', shortcut: '⌫', disabled: used, action: () => deleteSelection() },
        ])
      }}
    >
      <div className="thumb">
        {media.kind === 'audio' ? (
          <div className="audio-thumb">
            <IcNote size={30} />
          </div>
        ) : broken ? (
          <div className="placeholder">
            <IcImage size={34} />
          </div>
        ) : (
          <img src={backend.media.thumb(media.path, at, 136)} alt="" draggable={false} onError={() => setBroken(true)} loading="lazy" />
        )}
        {media.kind !== 'image' && <span className="dur">{shortDuration(range ? (range[1] - range[0]) * 10_000_000 : media.duration)}</span>}
        {proxy && proxy.state !== 'failed' && (
          <span className={`proxy-badge ${proxy.state === 'ready' ? '' : 'busy'}`} title={proxy.state === 'ready' ? `Preview proxy: ${proxy.source === 'camera' ? 'camera LRV' : 'generated'}` : 'Generating preview proxy'}>
            {proxy.state === 'ready' ? 'PROXY' : proxy.state === 'generating' ? `PROXY ${Math.round(proxy.progress * 100)}%` : 'PROXY …'}
          </span>
        )}
        <span className="kind">{media.kind === 'audio' ? <IcNote size={15} /> : media.kind === 'image' ? <IcImage size={15} /> : <IcFilm size={15} />}</span>
        {used ? (
          <span className="used" title="Used on the timeline">
            <IcCheck size={11} />
          </span>
        ) : (
          <button
            className="add"
            title="Add to timeline"
            onPointerDown={e => e.stopPropagation()}
            onClick={e => {
              e.stopPropagation()
              appendMedia(media)
            }}
          >
            <IcPlus size={13} strokeWidth={2} />
          </button>
        )}
      </div>
      {renaming != null ? (
        <input
          className="name rename"
          autoFocus
          value={renaming}
          onChange={e => setRenaming(e.target.value)}
          onBlur={() => rename(renaming)}
          onPointerDown={e => e.stopPropagation()}
          onKeyDown={e => {
            e.stopPropagation()
            if (e.key === 'Enter') rename(renaming)
            if (e.key === 'Escape') setRenaming(null)
          }}
        />
      ) : (
        <div className="name" title={media.name}>
          {media.name}
        </div>
      )}
    </div>
  )
}

