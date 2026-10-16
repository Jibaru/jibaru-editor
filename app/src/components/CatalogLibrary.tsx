// Titles, Transitions, Effects and Filters tabs of the library, laid out as
// in Filmora: a sidebar with "Mine" (favorites) and the tab's groups
// (Filmora's categories, see jb_render::library), and on the right a
// favorites button, "Search all ..." and a "..." menu, the group's name
// with the Commercial switch, and a two-column grid of preview cards.
// Wondershare cloud rows (AI categories, "Similar recommendations",
// premium and download badges) have no counterpart: everything is local.

import { useEffect, useMemo, useState } from 'react'
import { applyLibraryItem, favKey, favorites, groupItems, previewRef, previewTarget, tabGroups, tabItems, toggleFavorite, type LibraryItem } from '../library'
import { openMenu, previewInPlayer, useEditor } from '../store'
import { sep } from '../ui/ContextMenu'
import { Toggle } from '../ui/controls'
import { startLibraryDrag } from '../drag'
import { FxThumb } from './FxPreview'
import { IcCaretDown, IcCaretRight, IcMenuLines, IcMore, IcPlus, IcPreviewLens, IcSearch, IcStar, IcStarFill, IcStock } from '../ui/icons'

export type CatalogTab = 'titles' | 'transitions' | 'effects' | 'filters'

/** What the right side shows. */
type View = { kind: 'group'; group: string } | { kind: 'favorites' } | { kind: 'add-text' } | { kind: 'empty'; text: string }

interface Leaf {
  id: string
  label: string
  view: View
}

/** A sidebar row: a section that folds (Mine, Transitions, ...) or a single boxed entry. */
type Row = { id: string; label: string; children: Leaf[] } | { id: string; label: string; view: View }

const NOUN: Record<CatalogTab, string> = { titles: 'titles', transitions: 'transitions', effects: 'effects', filters: 'filters' }

function sidebar(tab: CatalogTab, groups: string[]): Row[] {
  const mine: Row = { id: 'mine', label: 'Mine', children: [{ id: 'favorites', label: 'Favorites', view: { kind: 'favorites' } }] }
  const leaves = groups.map(g => ({ id: `g:${g}`, label: g, view: { kind: 'group', group: g } as View }))
  switch (tab) {
    case 'titles':
      return [{ id: 'add-text', label: 'Add Text', view: { kind: 'add-text' } }, mine, { id: 'templates', label: 'Title Templates', children: leaves }]
    case 'transitions':
      return [
        mine,
        { id: 'main', label: 'Transitions', children: leaves },
        { id: 'audio', label: 'Audio Transitions', view: { kind: 'empty', text: 'Audio transitions are not available yet. Use the fade in and fade out of the Audio properties.' } },
      ]
    case 'effects':
      return [mine, { id: 'main', label: 'Video Effects', children: leaves }]
    case 'filters':
      return [mine, { id: 'main', label: 'Filters', children: leaves }]
  }
}

interface TabState {
  /** Selected sidebar entry. */
  sel: string
  open: Record<string, boolean>
}

function initial(tab: CatalogTab, groups: string[]): TabState {
  if (tab === 'titles') return { sel: 'add-text', open: {} }
  return { sel: groups[0] ? `g:${groups[0]}` : 'favorites', open: { main: true } }
}

const COMMERCIAL_KEY = 'jb.library.commercial'

function useFavorites(): Set<string> {
  const [f, setF] = useState(favorites)
  useEffect(() => {
    const on = () => setF(favorites())
    window.addEventListener('jb-favorites', on)
    return () => window.removeEventListener('jb-favorites', on)
  }, [])
  return f
}

/** Sidebar and content of a catalog tab (inside `.lib-body`). */
export function CatalogBody({ tab }: { tab: CatalogTab }) {
  const fx = useEditor(s => s.fx)
  const groups = useMemo(() => tabGroups(tab, fx), [tab, fx])
  const rows = useMemo(() => sidebar(tab, groups), [tab, groups])
  const [states, setStates] = useState<Partial<Record<CatalogTab, TabState>>>({})
  const st = states[tab] ?? initial(tab, groups)
  const set = (s: Partial<TabState>) => setStates(all => ({ ...all, [tab]: { ...st, ...s } }))
  const leaves = rows.flatMap(r => ('children' in r ? r.children : [{ id: r.id, label: r.label, view: r.view }]))
  const current = leaves.find(l => l.id === st.sel) ?? leaves[0]
  const showFavorites = () => set({ sel: 'favorites', open: { ...st.open, mine: true } })
  return (
    <>
      <aside className={`lib-side catalog-side ${tab}`}>
        {rows.map(r =>
          'children' in r ? (
            <div key={r.id} className="side-section">
              <button className="boxed" data-side={r.id} aria-expanded={!!st.open[r.id]} onClick={() => set({ open: { ...st.open, [r.id]: !st.open[r.id] } })}>
                <span className="caret">{st.open[r.id] ? <IcCaretDown size={10} /> : <IcCaretRight size={10} />}</span>
                <span className="lbl">{r.label}</span>
              </button>
              {st.open[r.id] &&
                r.children.map(c => (
                  <button key={c.id} className={`child ${c.id === current?.id ? 'on' : ''}`} data-side={c.id} title={c.label} onClick={() => set({ sel: c.id })}>
                    <span className="lbl">{c.label}</span>
                  </button>
                ))}
            </div>
          ) : (
            <button key={r.id} className={`boxed single ${r.id === current?.id ? 'on' : ''}`} data-side={r.id} title={r.label} onClick={() => set({ sel: r.id })}>
              <span className="lbl">{r.label}</span>
            </button>
          ),
        )}
      </aside>
      <div className="lib-main catalog-main">
        {current?.view.kind === 'add-text' ? (
          <AddText />
        ) : (
          <CatalogContent key={tab} tab={tab} view={current?.view ?? { kind: 'favorites' }} label={current?.label ?? ''} onFavorites={showFavorites} />
        )}
      </div>
    </>
  )
}

/** Titles > Add Text: Filmora's "ADD TEXT TO CONTINUE." with the Add Text tile. */
function AddText() {
  const item: LibraryItem = { type: 'title', id: 'default', label: 'Title' }
  return (
    <div className="add-text-view">
      <div className="lib-section-head">
        <span>Add text to continue.</span>
      </div>
      <div className="add-text-tiles">
        <button className="add-text-tile" title="Add a title at the playhead (drag it to place it)" onPointerDown={e => e.button === 0 && startLibraryDrag(e, item)} onClick={() => applyLibraryItem(item)}>
          <span>Add Text</span>
          <i>
            <IcPlus size={13} strokeWidth={2.4} />
          </i>
        </button>
      </div>
    </div>
  )
}

function CatalogContent({ tab, view, label, onFavorites }: { tab: CatalogTab; view: View; label: string; onFavorites: () => void }) {
  const fx = useEditor(s => s.fx)
  const favs = useFavorites()
  const [query, setQuery] = useState('')
  const [sort, setSort] = useState<'default' | 'name'>('default')
  const [commercial, setCommercial] = useState(() => {
    try {
      return localStorage.getItem(COMMERCIAL_KEY) === '1'
    } catch {
      return false
    }
  })
  const all = useMemo(() => tabItems(tab, fx), [tab, fx])
  const q = query.trim().toLowerCase()
  const items = useMemo(() => {
    const list = q
      ? all.filter(i => i.label.toLowerCase().includes(q))
      : view.kind === 'favorites'
        ? all.filter(i => favs.has(favKey(i)))
        : view.kind === 'group'
          ? groupItems(tab, fx, view.group)
          : []
    // Everything in Jibaru's library can be used commercially, so the switch filters nothing out.
    return sort === 'name' ? [...list].sort((a, b) => a.label.localeCompare(b.label)) : list
  }, [all, q, view, favs, tab, fx, sort])
  const verb = tab === 'titles' ? 'Add to Timeline' : tab === 'transitions' ? 'Apply to the Nearest Cut' : 'Apply to Selected Clips'
  const head = q ? 'Search results' : label
  const more = (e: React.MouseEvent) => {
    const r = e.currentTarget.getBoundingClientRect()
    openMenu(r.left, r.bottom + 2, [
      { label: 'Default Order', checked: sort === 'default', action: () => setSort('default') },
      { label: 'Sort by Name', checked: sort === 'name', action: () => setSort('name') },
      sep,
      { label: `Clear Favorites (${all.filter(i => favs.has(favKey(i))).length})`, disabled: !all.some(i => favs.has(favKey(i))), action: () => all.filter(i => favs.has(favKey(i))).forEach(toggleFavorite) },
    ])
  }
  return (
    <>
      <div className="lib-toolbar catalog-toolbar">
        <button className={`lib-store ${view.kind === 'favorites' && !q ? 'on' : ''}`} title="Favorites" onClick={onFavorites}>
          <IcStock size={17} strokeWidth={1.8} />
        </button>
        <label className="lib-search">
          <IcSearch size={15} />
          <input value={query} placeholder={`Search all ${NOUN[tab]}`} onChange={e => setQuery(e.target.value)} onKeyDown={e => e.stopPropagation()} />
        </label>
        <button className="ticon lib-more" title="More" onClick={more}>
          <IcMore size={18} />
        </button>
      </div>
      <div className="lib-section-head">
        <span>{head}</span>
        <label className="lib-commercial" title="Show only items cleared for commercial use (all of Jibaru's are)">
          <Toggle
            on={commercial}
            onChange={v => {
              setCommercial(v)
              try {
                localStorage.setItem(COMMERCIAL_KEY, v ? '1' : '0')
              } catch {
                /* storage unavailable */
              }
            }}
          />
          Commercial
        </label>
      </div>
      {view.kind === 'empty' && !q ? (
        <div className="lib-empty">{view.text}</div>
      ) : items.length === 0 ? (
        <div className="lib-empty">{view.kind === 'favorites' && !q ? 'Click the star on an item to add it to your favorites.' : 'Nothing matches your search.'}</div>
      ) : (
        <div className="lib-grid catalog-grid">
          {items.map(item => (
            <CatalogCard key={item.id} item={item} fav={favs.has(favKey(item))} verb={verb} />
          ))}
        </div>
      )}
    </>
  )
}

/**
 * A Transitions/Effects/Filters/Titles card, as in Filmora: the item's
 * still; on hover its looping preview with Preview, menu, favorite and add
 * buttons; a click selects it and plays it in the player.
 */
function CatalogCard({ item, fav, verb }: { item: LibraryItem; fav: boolean; verb: string }) {
  const [hover, setHover] = useState(false)
  const selected = useEditor(s => s.librarySel === favKey(item))
  const menu = (x: number, y: number) =>
    openMenu(x, y, [
      { label: verb, action: () => applyLibraryItem(item) },
      { label: 'Preview', action: () => previewInPlayer(previewTarget(item)) },
      { label: fav ? 'Remove from Favorites' : 'Add to Favorites', action: () => toggleFavorite(item) },
    ])
  const stop = (e: React.SyntheticEvent) => e.stopPropagation()
  return (
    <div
      className={`card catalog-card ${selected ? 'on' : ''} ${hover ? 'hover' : ''}`}
      data-item={item.id}
      title={`${item.label}: click to preview, drag to the timeline or double-click to ${verb.toLowerCase()}`}
      onPointerEnter={() => setHover(true)}
      onPointerLeave={() => setHover(false)}
      onPointerDown={e => e.button === 0 && startLibraryDrag(e, item)}
      onClick={() => previewInPlayer(previewTarget(item))}
      onDoubleClick={() => applyLibraryItem(item)}
      onContextMenu={e => {
        e.preventDefault()
        menu(e.clientX, e.clientY)
      }}
    >
      <div className="thumb">
        <FxThumb item={previewRef(item)} hover={hover} />
        <button className="ov preview" title="Preview" onPointerDown={stop} onClick={e => (stop(e), previewInPlayer(previewTarget(item)))}>
          <IcPreviewLens size={14} />
        </button>
        <button
          className="ov more"
          title="More"
          onPointerDown={stop}
          onClick={e => {
            stop(e)
            const r = e.currentTarget.getBoundingClientRect()
            menu(r.left, r.bottom + 2)
          }}
        >
          <IcMenuLines size={14} />
        </button>
        <button
          className={`ov fav ${fav ? 'on' : ''}`}
          title={fav ? 'Remove from favorites' : 'Add to favorites'}
          onPointerDown={stop}
          onClick={e => {
            stop(e)
            toggleFavorite(item)
          }}
        >
          {fav ? <IcStarFill size={14} /> : <IcStar size={14} />}
        </button>
        <button
          className="ov add"
          title={verb}
          onPointerDown={stop}
          onClick={e => {
            stop(e)
            applyLibraryItem(item)
          }}
        >
          <IcPlus size={13} strokeWidth={2} />
        </button>
      </div>
      <div className="name">{item.label}</div>
    </div>
  )
}
