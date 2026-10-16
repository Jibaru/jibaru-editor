import { useMemo, useState, type ReactNode } from 'react'
import { backend } from '../backend'
import { mediaInUse, seconds, shortDuration } from '../core/model'
import type { MediaAsset } from '../core/types'
import { tabCategories, tabItems, type LibraryItem } from '../library'
import { appendMedia, deleteSelection, importMedia, openMenu, setState, useEditor, type LibraryTab } from '../store'
import { sep } from '../ui/ContextMenu'
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

const TABS: { id: LibraryTab; label: string; icon: (p: { size?: number }) => ReactNode }[] = [
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

type SideItem = { label: string; caret?: 'open' | 'closed'; child?: boolean }

const SIDEBARS: Partial<Record<LibraryTab, SideItem[]>> = {
  media: [
    { label: 'Project Media', caret: 'open' },
    { label: 'Folder', child: true },
    { label: 'Global Media', caret: 'closed' },
    { label: 'Cloud Media' },
    { label: 'Editing Presets', caret: 'closed' },
    { label: 'Photos Library' },
  ],
  audio: [{ label: 'Project Audio', caret: 'open' }, { label: 'Music' }, { label: 'Sound Effects' }],
  stock: [{ label: 'Videos', caret: 'open' }, { label: 'Photos' }],
  stickers: [{ label: 'Default', caret: 'open' }],
  templates: [{ label: 'Default', caret: 'open' }],
}

export function Library() {
  const tab = useEditor(s => s.libraryTab)
  const fx = useEditor(s => s.fx)
  const [sides, setSides] = useState<Partial<Record<LibraryTab, number>>>({})
  const side = sides[tab] ?? 0
  const setSide = (i: number) => setSides(s => ({ ...s, [tab]: i }))
  const categories = tab === 'titles' || tab === 'transitions' || tab === 'effects' || tab === 'filters' ? tabCategories(tab, fx) : null
  const sidebar: SideItem[] = categories ? [{ label: 'All', caret: 'open' }, ...categories.map(c => ({ label: c, child: true }))] : (SIDEBARS[tab] ?? [])
  return (
    <section className="panel library">
      <div className="panel-head lib-tabs">
        {TABS.map(t => (
          <button key={t.id} className={tab === t.id ? 'on' : ''} onClick={() => setState({ libraryTab: t.id })}>
            <t.icon size={22} />
            <span>{t.label}</span>
          </button>
        ))}
      </div>
      <div className="lib-body">
        <aside className="lib-side">
          {sidebar.map((s, i) => (
            <button key={s.label} className={`${i === side ? 'on' : ''} ${s.child ? 'child' : ''} ${s.caret || !s.child ? 'boxed' : ''}`} onClick={() => setSide(i)}>
              <span className="caret">{s.caret === 'open' ? <IcCaretDown size={10} /> : s.caret === 'closed' ? <IcCaretRight size={10} /> : null}</span>
              {s.label}
            </button>
          ))}
        </aside>
        <div className="lib-main">{tab === 'media' ? <MediaGrid /> : <CatalogGrid tab={tab} category={categories && side > 0 ? categories[side - 1] : null} />}</div>
      </div>
    </section>
  )
}

function Toolbar({ importButton = true }: { importButton?: boolean }) {
  return (
    <div className="lib-toolbar">
      {importButton && (
        <button className="lib-import" onClick={() => importMedia()}>
          Import <IcChevronDown size={14} />
        </button>
      )}
      {importButton && (
        <div className="lib-record">
          <button title="Record screen"><IcRecord size={16} /></button>
          <i />
          <button title="Record voiceover"><IcMic size={16} /></button>
          <i />
          <button title="Record webcam"><IcWebcam size={16} /></button>
        </div>
      )}
      <span className="spacer" />
      <div className="lib-size" title="Thumbnail size">
        <div className="lib-size-track" />
        <div className="lib-size-knob" />
      </div>
      <button className="ticon" title="Filter"><IcFilter size={18} /></button>
      <button className="ticon" title="More"><IcMore size={18} /></button>
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

function MediaGrid() {
  const project = useEditor(s => s.snap?.project)
  const selected = useEditor(s => s.selectedMedia)
  const [query, setQuery] = useState('')
  const media = useMemo(() => (project?.media ?? []).filter(m => m.name.toLowerCase().includes(query.toLowerCase())), [project, query])
  if (!project) return null
  return (
    <>
      <Toolbar />
      <Search value={query} onChange={setQuery} placeholder="Search media" />
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
  const at = media.kind === 'video' ? Math.min(seconds(media.duration) * 0.1, 3) : 0
  return (
    <div
      className={`card media-card ${selected ? 'on' : ''}`}
      data-media={media.id}
      onPointerDown={e => {
        if (e.button !== 0) return
        setState(s => ({
          selectedMedia: e.metaKey || e.shiftKey ? [...new Set([...s.selectedMedia, media.id])] : [media.id],
          selection: [],
        }))
        startLibraryDrag(e, { type: 'media', id: media.id, label: media.name })
      }}
      onDoubleClick={() => setState({ playerMode: 'source', sourceMedia: media.id, sourceTime: 0, playing: false })}
      onContextMenu={e => {
        e.preventDefault()
        setState({ selectedMedia: [media.id], selection: [] })
        openMenu(e.clientX, e.clientY, [
          { label: 'Add to Timeline', action: () => appendMedia(media) },
          { label: 'Open in Source Monitor', action: () => setState({ playerMode: 'source', sourceMedia: media.id, sourceTime: 0, playing: false }) },
          sep,
          { label: 'Rename', disabled: true },
          { label: 'Reveal in Finder', disabled: true },
          sep,
          { label: 'Delete', shortcut: '⌫', disabled: used, action: () => deleteSelection() },
        ])
      }}
    >
      <div className="thumb">
        {media.kind === 'audio' ? (
          <div className="audio-thumb"><IcNote size={30} /></div>
        ) : broken ? (
          <div className="placeholder"><IcImage size={34} /></div>
        ) : (
          <img src={backend.media.thumb(media.path, at, 136)} alt="" draggable={false} onError={() => setBroken(true)} loading="lazy" />
        )}
        {media.kind !== 'image' && <span className="dur">{shortDuration(media.duration)}</span>}
        <span className="kind">{media.kind === 'audio' ? <IcNote size={15} /> : media.kind === 'image' ? <IcImage size={15} /> : <IcFilm size={15} />}</span>
        {used ? (
          <span className="used"><IcCheck size={11} /></span>
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
      <div className="name" title={media.name}>{media.name}</div>
    </div>
  )
}

function CatalogGrid({ tab, category }: { tab: LibraryTab; category: string | null }) {
  const project = useEditor(s => s.snap?.project)
  const fx = useEditor(s => s.fx)
  const [query, setQuery] = useState('')
  const items: LibraryItem[] = useMemo(() => {
    if (tab === 'audio')
      return (project?.media ?? [])
        .filter(m => m.kind === 'audio')
        .map(m => ({ type: 'media', id: m.id, label: m.name, sample: 'audio' }))
    return tabItems(tab, fx).filter(i => !category || i.category === category)
  }, [tab, project, fx, category])
  const shown = items.filter(i => i.label.toLowerCase().includes(query.toLowerCase()))
  return (
    <>
      <Toolbar importButton={tab === 'audio'} />
      <Search value={query} onChange={setQuery} placeholder={`Search ${TABS.find(t => t.id === tab)?.label.toLowerCase()}`} />
      {shown.length === 0 ? (
        <div className="lib-empty">{tab === 'audio' ? 'Import music or sound effects to see them here.' : 'Nothing here yet.'}</div>
      ) : (
        <div className="lib-grid">
          {shown.map(item => (
            <div key={item.id} className="card catalog-card" onPointerDown={e => e.button === 0 && startLibraryDrag(e, item)}>
              <div className={`thumb sample-${item.sample ?? tab}`}>
                {item.sample === 'audio' ? <div className="audio-thumb"><IcNote size={30} /></div> : <CatalogSample item={item} />}
              </div>
              <div className="name">{item.label}</div>
            </div>
          ))}
        </div>
      )}
    </>
  )
}

function CatalogSample({ item }: { item: LibraryItem }) {
  if (item.type === 'title') return <span className="title-sample">{item.label === 'Default Title' ? 'Title' : item.label}</span>
  if (item.type === 'transition') return <span className={`transition-sample tr-${item.id}`}><i className="a" /><i className="b" /></span>
  return <span className={`fx-sample fx-${item.id.replace(/\./g, '-')}`} />
}
