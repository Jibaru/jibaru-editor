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
  height?: number
  extra?: Extra
}

export interface Marker {
  id: Id
  time: Ticks
  duration?: Ticks
  name?: string
  comment?: string
  color?: [number, number, number, number]
  extra?: Extra
}

export interface Sequence {
  id: Id
  name: string
  tracks: Track[]
  markers?: Marker[]
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
  alignment?: TextAlignment
  letter_spacing?: number
  line_spacing?: number
}

export type TextAlignment = 'left' | 'center' | 'right'

export type ClipSource =
  | { type: 'media'; media_id: Id }
  | { type: 'sequence'; sequence_id: Id }
  | ({ type: 'title' } & TitleData)
  | { type: 'color'; color: [number, number, number, number] }
  | { type: 'adjustment' }

export interface SpeedPoint {
  /** 0..1 position in the clip's source range. */
  position: number
  speed: number
}

export interface Speed {
  rate: number
  reverse?: boolean
  ramp?: Param
  curve?: { preset: string; points: SpeedPoint[] }
  freeze?: boolean
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
  | 'linear_burn' | 'darker_color' | 'lighter_color' | 'vivid_light' | 'linear_light'
  | 'pin_light' | 'hard_mix' | 'subtract' | 'divide'

export interface AudioSettings {
  gain_db: Param
  pan: Param
  fade_in: Ticks
  fade_out: Ticks
  pitch?: number
  muted?: boolean
  normalize?: { target_lufs: number; measured_lufs?: number }
  equalizer?: { preset: string; bands: number[] }
  /** Noise reduction strength (maximum attenuation in dB). */
  denoise_db?: number
  /** Channel routing (Filmora's Audio Channels). */
  channels?: AudioChannels
  /** Voice filter preset (`telephone`, `echo`, `big_room`, ...). */
  voice_filter?: string
  /** Mains hum removal at 50 or 60 Hz. */
  hum_removal?: number
  /** Hiss removal strength 0..100. */
  hiss_removal?: number
}

export type AudioChannels = 'stereo' | 'left' | 'right' | 'mono' | 'swap'

export interface EffectInstance {
  id: Id
  kind: string
  enabled?: boolean
  params?: Record<string, Param>
  range?: [Ticks, Ticks]
  placement?: AnimationPlacement
  extra?: Extra
}

export type AnimationPlacement = 'in' | 'out' | 'loop' | 'combo'

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
  group?: Id
  disabled?: boolean
  markers?: Marker[]
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

export type ClipAttribute = 'transform' | 'crop' | 'opacity' | 'blend_mode' | 'effects' | 'animations' | 'audio' | 'speed'

export interface TitleStylePatch {
  text?: string
  font_family?: string
  font_size?: number
  color?: [number, number, number, number]
  bold?: boolean
  italic?: boolean
  alignment?: TextAlignment
  position?: [number, number]
  stroke?: Stroke | null
  shadow?: Shadow | null
  background?: [number, number, number, number] | null
  letter_spacing?: number
  line_spacing?: number
  animation?: { kind: string; placement?: AnimationPlacement; duration?: Ticks }
}

export type Command =
  | { op: 'add_media'; asset: MediaAsset }
  | { op: 'remove_media'; media_id: Id }
  | { op: 'rename_media'; media_id: Id; name: string }
  | { op: 'add_track'; sequence_id: Id | null; kind: TrackKind; index: number | null }
  | { op: 'remove_track'; track_id: Id }
  | { op: 'set_track_flags'; track_id: Id; locked: boolean | null; muted: boolean | null; hidden: boolean | null }
  | { op: 'move_track'; track_id: Id; index: number }
  | { op: 'rename_track'; track_id: Id; name: string }
  | { op: 'set_track_height'; track_id: Id; height: number | null }
  | { op: 'add_clip'; track_id: Id; clip: Clip }
  | { op: 'insert_clip'; track_id: Id; clip: Clip }
  | { op: 'overwrite_clip'; track_id: Id; clip: Clip }
  | { op: 'remove_clip'; clip_id: Id; ripple: boolean }
  | { op: 'move_clip'; clip_id: Id; start: Ticks; track_id: Id | null }
  | { op: 'trim_clip'; clip_id: Id; start: Ticks | null; end: Ticks | null }
  | { op: 'ripple_trim'; clip_id: Id; edge: 'start' | 'end'; to: Ticks }
  | { op: 'roll_edit'; clip_id: Id; at: Ticks }
  | { op: 'slip_clip'; clip_id: Id; delta: Ticks }
  | { op: 'split_clip'; clip_id: Id; at: Ticks }
  | { op: 'duplicate_clip'; clip_id: Id; start: Ticks | null; track_id: Id | null }
  | { op: 'replace_media'; clip_id: Id; media_id: Id }
  | { op: 'set_clip_enabled'; clip_id: Id; enabled: boolean }
  | { op: 'freeze_frame'; clip_id: Id; at: Ticks; duration: Ticks }
  | { op: 'detach_audio'; clip_id: Id; track_id: Id | null }
  | { op: 'link_clips'; clip_ids: Id[] }
  | { op: 'unlink_clips'; clip_ids: Id[] }
  | { op: 'group_clips'; clip_ids: Id[] }
  | { op: 'ungroup_clips'; clip_ids: Id[] }
  | { op: 'create_compound_clip'; clip_ids: Id[]; name: string | null }
  | { op: 'paste_attributes'; from_clip_id: Id; to_clip_ids: Id[]; attributes: ClipAttribute[] }
  | { op: 'set_speed'; clip_id: Id; rate: number; reverse: boolean | null }
  | { op: 'set_speed_ramp'; clip_id: Id; preset: string; points?: SpeedPoint[] | null }
  | { op: 'set_reverse'; clip_id: Id; reverse: boolean }
  | { op: 'set_transform'; clip_id: Id; transform: Transform }
  | { op: 'set_crop'; clip_id: Id; crop: [number, number, number, number] }
  | { op: 'set_flip'; clip_id: Id; horizontal: boolean | null; vertical: boolean | null }
  | { op: 'set_rotation'; clip_id: Id; degrees: number }
  | { op: 'set_fit_mode'; clip_id: Id; fit: FitMode }
  | { op: 'set_blend_mode'; clip_id: Id; blend_mode: BlendMode }
  | { op: 'set_opacity'; clip_id: Id; opacity: Param }
  | { op: 'set_param'; clip_id: Id; param: string; value: Value }
  | { op: 'add_keyframe'; clip_id: Id; param: string; time: Ticks; value: Value; interpolation?: Interpolation | null }
  | { op: 'remove_keyframe'; clip_id: Id; param: string; time: Ticks }
  | { op: 'move_keyframe'; clip_id: Id; param: string; from: Ticks; to: Ticks }
  | { op: 'clear_keyframes'; clip_id: Id; param: string }
  | { op: 'set_audio'; clip_id: Id; audio: AudioSettings }
  | { op: 'set_volume'; clip_id: Id; gain_db: number }
  | { op: 'set_pan'; clip_id: Id; pan: number }
  | { op: 'set_pitch'; clip_id: Id; semitones: number }
  | { op: 'set_mute'; clip_id: Id; muted: boolean }
  | { op: 'set_audio_channels'; clip_id: Id; channels: AudioChannels }
  | { op: 'set_voice_filter'; clip_id: Id; filter: string | null }
  | { op: 'set_audio_cleanup'; clip_id: Id; hum_removal?: number | null; hiss_removal?: number | null }
  | { op: 'set_maintain_pitch'; clip_id: Id; maintain_pitch: boolean }
  | { op: 'set_audio_fades'; clip_id: Id; fade_in: Ticks | null; fade_out: Ticks | null }
  | { op: 'set_normalization'; clip_id: Id; target_lufs: number | null; measured_lufs?: number | null }
  | { op: 'set_equalizer'; clip_id: Id; preset?: string | null; bands?: number[] | null }
  | { op: 'add_effect'; clip_id: Id; effect: EffectInstance }
  | { op: 'remove_effect'; clip_id: Id; effect_id: Id }
  | { op: 'set_effect_params'; clip_id: Id; effect_id: Id; params: Record<string, Param> }
  | { op: 'set_effect_enabled'; clip_id: Id; effect_id: Id; enabled: boolean }
  | { op: 'add_animation'; clip_id: Id; kind: string; placement: AnimationPlacement; duration?: Ticks | null; params?: Record<string, Param>; id?: Id | null }
  | { op: 'remove_animation'; clip_id: Id; animation_id?: Id | null; placement?: AnimationPlacement | null }
  | { op: 'replace_animation'; clip_id: Id; animation_id: Id; kind?: string | null; duration?: Ticks | null; params?: Record<string, Param> | null }
  | { op: 'set_transition'; clip_id: Id; transition: Transition | null }
  | { op: 'set_title'; clip_id: Id; title: TitleData }
  | ({ op: 'set_title_style'; clip_id: Id } & TitleStylePatch)
  | { op: 'add_marker'; sequence_id?: Id | null; clip_id?: Id | null; marker: Marker }
  | { op: 'update_marker'; marker: Marker }
  | { op: 'remove_marker'; marker_id: Id }
  | { op: 'set_project_settings'; width?: number | null; height?: number | null; frame_rate?: Rational | null; background?: [number, number, number, number] | null }
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
  /** Edits kept only for Jibaru (in the .wfp sidecar); Filmora won't show them. */
  save_notes?: string[]
}
