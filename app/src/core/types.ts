// TypeScript mirror of the serde JSON produced by `jb-core`
// (crates/jb-core/src/{model,command,param,time}.rs). Keep in sync.
// All times are ticks of 100 ns (TICKS_PER_SECOND = 10_000_000).

export type Id = string
export type Ticks = number
export type Extra = unknown

export const TICKS_PER_SECOND = 10_000_000

export interface Rational {
  num: number
  den: number
}

// ---------- param.rs ----------

export type Value =
  | { type: 'float'; value: number }
  | { type: 'vec2'; value: [number, number] }
  | { type: 'color'; value: [number, number, number, number] }
  | { type: 'bool'; value: boolean }
  | { type: 'text'; value: string }

export type Interpolation = 'linear' | 'hold' | 'ease_in' | 'ease_out' | 'ease_in_out'

export interface Keyframe {
  time: Ticks
  value: Value
  interpolation?: Interpolation
}

export interface Param {
  value: Value
  keyframes?: Keyframe[]
}

// ---------- model.rs ----------

export interface ProjectSettings {
  width: number
  height: number
  frame_rate: Rational
  sample_rate: number
  background?: [number, number, number, number]
}

export type MediaKind = 'video' | 'audio' | 'image'

export interface MediaAsset {
  id: Id
  name: string
  path: string
  kind: MediaKind
  duration: Ticks
  width?: number
  height?: number
  frame_rate?: Rational
  has_audio?: boolean
  extra?: Extra
}

export type TrackKind = 'video' | 'audio'

export interface Track {
  id: Id
  kind: TrackKind
  name: string
  locked?: boolean
  muted?: boolean
  hidden?: boolean
  clips: Clip[]
  extra?: Extra
}

export interface Sequence {
  id: Id
  name: string
  tracks: Track[]
  extra?: Extra
}

export interface Stroke {
  color: [number, number, number, number]
  width: number
}

export interface Shadow {
  color: [number, number, number, number]
  offset: [number, number]
  blur: number
}

export interface TitleData {
  text: string
  font_family: string
  font_size: number
  color: [number, number, number, number]
  position: [number, number]
  bold?: boolean
  italic?: boolean
  stroke?: Stroke
  shadow?: Shadow
  background?: [number, number, number, number]
}

export type ClipSource =
  | { type: 'media'; media_id: Id }
  | { type: 'sequence'; sequence_id: Id }
  | ({ type: 'title' } & TitleData)
  | { type: 'color'; color: [number, number, number, number] }
  | { type: 'adjustment' }

export interface Speed {
  rate: number
  reverse?: boolean
  ramp?: Param
  maintain_pitch?: boolean
}

export type FitMode = 'fit' | 'fill' | 'stretch' | 'none'

export interface Transform {
  position_x: Param
  position_y: Param
  scale_x: Param
  scale_y: Param
  rotation: Param
  flip_h?: boolean
  flip_v?: boolean
  crop?: [number, number, number, number]
  fit?: FitMode
}

export type BlendMode =
  | 'normal' | 'multiply' | 'screen' | 'overlay' | 'darken' | 'lighten' | 'color_dodge'
  | 'color_burn' | 'hard_light' | 'soft_light' | 'difference' | 'exclusion' | 'add'

export interface AudioSettings {
  gain_db: Param
  pan: Param
  fade_in: Ticks
  fade_out: Ticks
  pitch?: number
  muted?: boolean
}

export interface EffectInstance {
  id: Id
  kind: string
  enabled?: boolean
  params?: Record<string, Param>
  range?: [Ticks, Ticks]
  extra?: Extra
}

export type TransitionAlignment = 'center' | 'end_at_cut' | 'start_at_cut'

export interface Transition {
  id: Id
  kind: string
  duration: Ticks
  alignment?: TransitionAlignment
  params?: Record<string, Param>
  extra?: Extra
}

export interface Clip {
  id: Id
  name: string
  source: ClipSource
  start: Ticks
  end: Ticks
  source_in?: Ticks
  speed?: Speed
  transform?: Transform
  opacity?: Param
  blend_mode?: BlendMode
  audio?: AudioSettings
  effects?: EffectInstance[]
  animations?: EffectInstance[]
  transition_out?: Transition
  link_group?: Id
  disabled?: boolean
  extra?: Extra
}

export interface Project {
  id: Id
  name: string
  settings: ProjectSettings
  media: MediaAsset[]
  sequences: Sequence[]
  main_sequence: Id
  extra?: Extra
}

// ---------- command.rs ----------

export type Command =
  | { op: 'add_media'; asset: MediaAsset }
  | { op: 'remove_media'; media_id: Id }
  | { op: 'add_track'; sequence_id: Id | null; kind: TrackKind; index: number | null }
  | { op: 'remove_track'; track_id: Id }
  | { op: 'set_track_flags'; track_id: Id; locked: boolean | null; muted: boolean | null; hidden: boolean | null }
  | { op: 'add_clip'; track_id: Id; clip: Clip }
  | { op: 'remove_clip'; clip_id: Id; ripple: boolean }
  | { op: 'move_clip'; clip_id: Id; start: Ticks; track_id: Id | null }
  | { op: 'trim_clip'; clip_id: Id; start: Ticks | null; end: Ticks | null }
  | { op: 'split_clip'; clip_id: Id; at: Ticks }
  | { op: 'set_speed'; clip_id: Id; rate: number; reverse: boolean | null }
  | { op: 'set_transform'; clip_id: Id; transform: Transform }
  | { op: 'set_opacity'; clip_id: Id; opacity: Param }
  | { op: 'set_audio'; clip_id: Id; audio: AudioSettings }
  | { op: 'add_effect'; clip_id: Id; effect: EffectInstance }
  | { op: 'remove_effect'; clip_id: Id; effect_id: Id }
  | { op: 'set_effect_params'; clip_id: Id; effect_id: Id; params: Record<string, Param> }
  | { op: 'set_transition'; clip_id: Id; transition: Transition | null }
  | { op: 'set_title'; clip_id: Id; title: TitleData }
  | { op: 'rename_project'; name: string }
  | { op: 'batch'; commands: Command[] }

/** What the backend returns after every state change. */
export interface Snapshot {
  project: Project
  /** File the project was opened from / saved to, if any. */
  path: string | null
  can_undo: boolean
  can_redo: boolean
  /** True when there are changes since the last save. */
  dirty: boolean
  /** Monotonic counter, bumped on every change. */
  revision: number
  /** True when the project came from a Filmora `.wfp` and can be saved back to one. */
  wfp?: boolean
  /** Warnings from the last save (edits the target format cannot express). */
  save_warnings?: string[]
}
