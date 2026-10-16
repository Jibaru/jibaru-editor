// Jibaru Editor's own line icons: 24x24 grid, 1.5px rounded strokes, drawn to
// sit at the same size and weight as the rest of the UI.
import type { ReactNode, SVGProps } from 'react'

type P = SVGProps<SVGSVGElement> & { size?: number }

function make(paths: ReactNode, fill = false) {
  return function Icon({ size = 20, ...rest }: P) {
    return (
      <svg
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill={fill ? 'currentColor' : 'none'}
        stroke={fill ? 'none' : 'currentColor'}
        strokeWidth={1.5}
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden
        {...rest}
      >
        {paths}
      </svg>
    )
  }
}

// ---------- Library tabs ----------
export const IcMedia = make(<><rect x="3.5" y="4.5" width="17" height="15" rx="3" /><path d="M3.5 9h17" /><path d="M10.5 12v4.5l4-2.25z" /></>)
export const IcStock = make(<><path d="M5 10.5V18a1.5 1.5 0 0 0 1.5 1.5h11A1.5 1.5 0 0 0 19 18v-7.5" /><path d="M4 7.5 5.5 4.5h13L20 7.5a2.6 2.6 0 0 1-5.3.5 2.6 2.6 0 0 1-5.4 0 2.6 2.6 0 0 1-5.3-.5z" /><path d="M10 19.5v-4h4v4" /></>)
export const IcAudio = make(<><path d="M9.5 17.5V6.5l10-2.5v11" /><circle cx="7" cy="17.5" r="2.5" /><circle cx="17" cy="15" r="2.5" /><path d="M9.5 9.5l10-2.5" /></>)
export const IcTitles = make(<><path d="M5 6V4.5h14V6" /><path d="M12 4.5v15" /><path d="M9.5 19.5h5" /></>)
export const IcTransitions = make(<><rect x="3.5" y="5" width="17" height="14" rx="3" /><path d="M8 9.5v5l3-2.5zM16 9.5v5l-3-2.5z" /></>)
export const IcEffects = make(<><path d="m11 3.5 2.2 4.4 4.8.7-3.5 3.4.8 4.8-4.3-2.3-4.3 2.3.8-4.8L4 8.6l4.8-.7z" /><path d="m15.5 16.5 3 3.5" /></>)
export const IcFilters = make(<><circle cx="12" cy="8.5" r="4.5" /><circle cx="8.5" cy="14.5" r="4.5" /><circle cx="15.5" cy="14.5" r="4.5" /></>)
export const IcStickers = make(<><path d="M19.5 12.5V7a2.5 2.5 0 0 0-2.5-2.5H7A2.5 2.5 0 0 0 4.5 7v10A2.5 2.5 0 0 0 7 19.5h5.5a7 7 0 0 0 7-7z" /><path d="M12.5 19.5v-3a4 4 0 0 1 4-4h3" /><path d="M9 10h.01M14 10h.01" strokeWidth={2.2} /><path d="M9 13.5c.8.8 1.8 1.2 3 1" /></>)
export const IcTemplates = make(<><rect x="3.5" y="4.5" width="17" height="15" rx="3" /><path d="M7 15.5h10" /><rect x="13" y="8" width="4" height="4" rx="1" /></>)

// ---------- Generic ----------
export const IcPlus = make(<path d="M12 5v14M5 12h14" />)
export const IcMinus = make(<path d="M5 12h14" />)
export const IcCheck = make(<path d="m6.5 12.5 3.5 3.5 7.5-8" strokeWidth={2.2} />)
export const IcChevronDown = make(<path d="m7 10 5 5 5-5" />)
export const IcChevronRight = make(<path d="m10 7 5 5-5 5" />)
export const IcChevronLeft = make(<path d="m14 7-5 5 5 5" />)
export const IcAlignLeft = make(<path d="M4.5 6.5h15M4.5 10.5h9M4.5 14.5h15M4.5 18.5h9" />)
export const IcAlignCenter = make(<path d="M4.5 6.5h15M7.5 10.5h9M4.5 14.5h15M7.5 18.5h9" />)
export const IcAlignRight = make(<path d="M4.5 6.5h15M10.5 10.5h9M4.5 14.5h15M10.5 18.5h9" />)
export const IcHeartFill = make(<path d="M12 19.5s-7.5-4.4-7.5-9.7A4.2 4.2 0 0 1 12 7.4a4.2 4.2 0 0 1 7.5 2.4c0 5.3-7.5 9.7-7.5 9.7z" />, true)
export const IcCaretDown = make(<path d="M7 10h10l-5 5.5z" />, true)
export const IcCaretUp = make(<path d="M7 14h10l-5-5.5z" />, true)
export const IcCaretRight = make(<path d="M10 7v10l5.5-5z" />, true)
export const IcDoubleRight = make(<path d="m7 7 5 5-5 5M12 7l5 5-5 5" />)
export const IcSearch = make(<><circle cx="11" cy="11" r="6" /><path d="m15.5 15.5 4 4" /></>)
export const IcFilter = make(<><path d="M4 5h16l-6 7.5V19l-4-1.5v-5z" /><path d="M16 15.5h4M16 18.5h4" /></>)
export const IcMore = make(<path d="M5.5 12h.01M12 12h.01M18.5 12h.01" strokeWidth={2.6} />)
export const IcClose = make(<path d="M6 6l12 12M18 6 6 18" />)
export const IcHelp = make(<><circle cx="12" cy="12" r="8" /><path d="M9.8 9.8a2.3 2.3 0 1 1 3.3 2c-.7.4-1.1.9-1.1 1.7M12 16.5h.01" /></>)
export const IcDiamond = make(<path d="M12 4.5 19.5 12 12 19.5 4.5 12z" />)
export const IcReset = make(<><path d="M5.5 12a6.5 6.5 0 1 0 2-4.7" /><path d="M5 4.5v3.5h3.5" /></>)
export const IcLock = make(<><rect x="5.5" y="10.5" width="13" height="9" rx="2" /><path d="M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5" /><path d="M12 14v2" /></>)
export const IcUnlock = make(<><rect x="5.5" y="10.5" width="13" height="9" rx="2" /><path d="M8.5 10.5V8a3.5 3.5 0 0 1 6.6-1.6" /><path d="M12 14v2" /></>)
export const IcSpeaker = make(<><path d="M4.5 9.5h3l4.5-4v13l-4.5-4h-3z" /><path d="M15.5 9a4 4 0 0 1 0 6M17.8 6.8a7 7 0 0 1 0 10.4" /></>)
export const IcSpeakerOff = make(<><path d="M4.5 9.5h3l4.5-4v13l-4.5-4h-3z" /><path d="m15.5 9.5 5 5M20.5 9.5l-5 5" /></>)
export const IcEye = make(<><path d="M2.5 12S6 6 12 6s9.5 6 9.5 6-3.5 6-9.5 6-9.5-6-9.5-6z" /><circle cx="12" cy="12" r="2.6" /></>)
export const IcEyeOff = make(<><path d="M2.5 12S6 6 12 6s9.5 6 9.5 6-3.5 6-9.5 6-9.5-6-9.5-6z" /><path d="M4 4l16 16" /></>)
export const IcWand = make(<><path d="m10 10 9.5 9.5" /><path d="m8.5 13.5 3-3 2 2-3 3z" /><path d="M5 5l1 1M9 3v1.5M3 9h1.5M5 13l1-1M13 5l-1 1" /></>)
export const IcGear = make(<><circle cx="12" cy="12" r="2.8" /><path d="M12 3.5l1.6 2.1 2.6-.6.6 2.6 2.1 1.6-1.2 2.4 1.2 2.4-2.1 1.6-.6 2.6-2.6-.6L12 20.5l-1.6-2.1-2.6.6-.6-2.6-2.1-1.6L6.3 12 5.1 9.6l2.1-1.6.6-2.6 2.6.6z" /></>)
export const IcLinkOff = make(<><path d="M9.5 14.5 7 17a2.8 2.8 0 0 1-4-4l2.5-2.5M14.5 9.5 17 7a2.8 2.8 0 0 1 4 4l-2.5 2.5" /><path d="M5 5l14 14" /></>)
export const IcLink = make(<><rect x="3" y="8" width="9" height="8" rx="3" /><rect x="12" y="8" width="9" height="8" rx="3" /><path d="M9 12h6" /></>)
export const IcTrackHeight = make(<><rect x="4" y="5" width="9" height="5" rx="1.5" /><rect x="8" y="14" width="12" height="5" rx="1.5" /><path d="M8 10v4" /></>)

// ---------- Timeline toolbar ----------
export const IcLayout = make(<><rect x="4" y="4" width="6.5" height="6.5" rx="2" /><rect x="13.5" y="13.5" width="6.5" height="6.5" rx="2" /><rect x="4" y="13.5" width="6.5" height="6.5" rx="2" /><path d="m16.75 3.8 3.45 3.45-3.45 3.45-3.45-3.45z" /></>)
export const IcPointer = make(<path d="M6 4.5 18 10l-5.3 1.7L10.5 17z" />)
export const IcUndo = make(<><path d="M9 6 5 10l4 4" /><path d="M5 10h9.5a4.5 4.5 0 0 1 0 9H11" /></>)
export const IcRedo = make(<><path d="m15 6 4 4-4 4" /><path d="M19 10H9.5a4.5 4.5 0 0 0 0 9H13" /></>)
export const IcTrash = make(<><path d="M4.5 6.5h15M9.5 6.5V4.5h5v2M6.5 6.5l1 13h9l1-13" /><path d="M10.5 10v6M13.5 10v6" /></>)
export const IcScissors = make(<><circle cx="7" cy="17" r="2.7" /><circle cx="17" cy="17" r="2.7" /><path d="M8.8 15 16 4.5M15.2 15 8 4.5" /></>)
export const IcCrop = make(<><path d="M7 3.5V17h13.5" /><path d="M3.5 7H17v13.5" /><path d="M3.5 20.5 20.5 3.5" strokeDasharray="2 2.5" /></>)
export const IcShapes = make(<><rect x="4" y="9" width="10" height="10" rx="2.5" /><circle cx="15" cy="9" r="5" /></>)
export const IcPen = make(<><path d="M5 5.5 13.5 4l6.5 6.5-1.5 8.5z" /><circle cx="12" cy="12" r="1.5" /><path d="M5 5.5l5.9 5.4M15 19.5l4.5-4.5" /></>)
export const IcSpeed = make(<><path d="M19.5 12a7.5 7.5 0 1 1-4-6.6" /><path d="m12 12 3-4" /><circle cx="12" cy="12" r="1" /></>)
export const IcDuration = make(<><circle cx="12" cy="13" r="7" /><path d="M9.5 3.5h5M12 13V9.5" /></>)
export const IcKeyframeAdd = make(<><path d="m13 7 5 5-5 5-5-5z" /><path d="M5 3.5v5M2.5 6h5" /></>)
export const IcFreeze = make(<><rect x="7" y="5" width="10" height="14" rx="2" /><path d="M7 9.5 3.5 12 7 14.5M17 9.5l3.5 2.5-3.5 2.5" /></>)
export const IcAutoEdit = make(<><path d="M4 5h8M4 12h8M4 19h8M12 5v14" /><path d="M15.5 12H20" /><path d="m18.5 4 .6 1.4 1.4.6-1.4.6-.6 1.4-.6-1.4-1.4-.6 1.4-.6z" fill="currentColor" stroke="none" /></>)
export const IcAudioBeat = make(<><path d="M10 17V7l7-2v10" /><circle cx="8" cy="17" r="2" /><circle cx="15" cy="15" r="2" /><path d="M4 10l-1.5 2L4 14M20 10l1.5 2-1.5 2" /></>)
export const IcMarker = make(<path d="M7 4.5h10v15l-5-4-5 4z" />)
export const IcMic = make(<><rect x="9" y="3.5" width="6" height="11" rx="3" /><path d="M6 11.5a6 6 0 0 0 12 0M12 17.5V20M9 20.5h6" /></>)
export const IcMixer = make(<><path d="M6 4v16M12 4v16M18 4v16" /><circle cx="6" cy="14" r="1.8" fill="var(--panel)" /><circle cx="12" cy="9" r="1.8" fill="var(--panel)" /><circle cx="18" cy="15" r="1.8" fill="var(--panel)" /></>)
export const IcPip = make(<><rect x="3.5" y="5" width="13" height="13" rx="2.5" /><path d="M10 9v5l3-2.5z" /><circle cx="17.5" cy="16.5" r="3" /></>)
export const IcFit = make(<><rect x="3.5" y="5" width="17" height="14" rx="3" /><path d="M7 12h10M9 10l-2 2 2 2M15 10l2 2-2 2" /></>)
export const IcZoomOut = make(<><circle cx="12" cy="12" r="8" /><path d="M8.5 12h7" /></>)
export const IcZoomIn = make(<><circle cx="12" cy="12" r="8" /><path d="M8.5 12h7M12 8.5v7" /></>)
export const IcGrid = make(<><path d="M5 5h5M14 5h5M5 9.5h5M14 9.5h5M5 14h5M14 14h5M5 18.5h5M14 18.5h5" strokeWidth={2.4} /></>)
export const IcSnap = make(<><path d="M6 4v7a6 6 0 0 0 12 0V4" /><path d="M6 8h3.5M14.5 8H18" /></>)

// ---------- Player ----------
export const IcPrevFrame = make(<><path d="M15 5.5v13L5.5 12z" /><path d="M19 5.5v13" /></>)
export const IcNextFrame = make(<><path d="M9 5.5v13l9.5-6.5z" /><path d="M5 5.5v13" /></>)
export const IcPlay = make(<path d="M7 4.8v14.4L19 12z" />)
export const IcPause = make(<path d="M8 5v14M16 5v14" strokeWidth={2.2} />)
export const IcStop = make(<rect x="5.5" y="5.5" width="13" height="13" rx="1" />)
export const IcMarkIn = make(<path d="M14.5 4.5c-2 0-3 1-3 3v2.5c0 1.2-.7 2-2 2 1.3 0 2 .8 2 2v2.5c0 2 1 3 3 3" />)
export const IcMarkOut = make(<path d="M9.5 4.5c2 0 3 1 3 3v2.5c0 1.2.7 2 2 2-1.3 0-2 .8-2 2v2.5c0 2-1 3-3 3" />)
export const IcSnapshot = make(<><path d="M4 8.5A1.5 1.5 0 0 1 5.5 7h2.3l1.4-2h5.6l1.4 2h2.3A1.5 1.5 0 0 1 20 8.5v9a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 4 17.5z" /><circle cx="12" cy="12.5" r="3.2" /></>)
export const IcFullscreen = make(<path d="M5 5l5 5M5 5v4M5 5h4M19 19l-5-5M19 19v-4M19 19h-4" />)
export const IcSplitView = make(<><rect x="3.5" y="5" width="7" height="6" rx="1.2" /><rect x="3.5" y="13" width="7" height="6" rx="1.2" /><rect x="12.5" y="5" width="8" height="14" rx="1.5" /></>)
export const IcScope = make(<><rect x="3.5" y="5" width="17" height="14" rx="2" /><path d="m6 14 3-4 2.5 3.5 2.5-5 2 4 2-1.5" /></>)

// ---------- Title bar ----------
export const IcGift = make(<><rect x="4" y="9" width="16" height="11" rx="1.5" /><path d="M3.5 9h17M12 9v11M12 9c-1.5-3-5.5-4-5.5-1.5S12 9 12 9zm0 0c1.5-3 5.5-4 5.5-1.5S12 9 12 9z" /></>)
export const IcSend = make(<path d="m4 11.5 16-7-5.5 15.5-3-6.5zM11.5 13.5 20 4.5" />)
export const IcFeedback = make(<><path d="M12 4a8 8 0 0 0-6.9 12L4 20l4-1.1A8 8 0 1 0 12 4z" /><path d="M9 11h6M9 14h4" /></>)
export const IcMonitor = make(<><rect x="3.5" y="5" width="17" height="12" rx="2" /><path d="M3.5 13.5h17M9 20h6" /></>)
export const IcSave = make(<><path d="M5 4.5h11l3 3V19a.5.5 0 0 1-.5.5h-13A.5.5 0 0 1 5 19z" /><path d="M8 4.5V9h7V4.5M8 19.5V14h8v5.5" /></>)
export const IcCloud = make(<><path d="M7 18.5a4.5 4.5 0 0 1-.6-9A6 6 0 0 1 18 10a4.3 4.3 0 0 1-.5 8.5z" /><path d="M12 16v-5M9.8 13.2 12 11l2.2 2.2" /></>)
export const IcHeadset = make(<><path d="M5 14v-2a7 7 0 0 1 14 0v2" /><rect x="4" y="13.5" width="4" height="5.5" rx="1.5" /><rect x="16" y="13.5" width="4" height="5.5" rx="1.5" /><path d="M18 19c0 1-1 1.5-3 1.5h-2" /></>)
export const IcApps = make(<><rect x="4.5" y="4.5" width="5.5" height="5.5" rx="1" /><rect x="14" y="4.5" width="5.5" height="5.5" rx="1" /><rect x="4.5" y="14" width="5.5" height="5.5" rx="1" /><rect x="14" y="14" width="5.5" height="5.5" rx="1" /></>)
export const IcExport = make(<><path d="M12 15V4.5M8 8.5l4-4 4 4" /><path d="M5 13v5.5A1.5 1.5 0 0 0 6.5 20h11a1.5 1.5 0 0 0 1.5-1.5V13" /></>)
export const IcSparkle = make(<path d="M12 3.5c.6 3.9 2.6 5.9 6.5 6.5-3.9.6-5.9 2.6-6.5 6.5-.6-3.9-2.6-5.9-6.5-6.5 3.9-.6 5.9-2.6 6.5-6.5z" />, true)

// ---------- Library toolbar ----------
export const IcRecord = make(<><circle cx="12" cy="12" r="7.5" /><circle cx="12" cy="12" r="3" fill="currentColor" /></>)
export const IcWebcam = make(<><circle cx="12" cy="10" r="6" /><circle cx="12" cy="10" r="2.2" /><path d="M8 19.5h8M12 16v3.5" /></>)
export const IcFilm = make(<><rect x="4" y="4.5" width="16" height="15" rx="2.5" /><path d="M4 8.5h16M8 4.5v4M12 4.5v4M16 4.5v4" /><path d="M10.5 11.5v5l4-2.5z" /></>)
export const IcImage = make(<><rect x="3.5" y="5" width="17" height="14" rx="2.5" /><circle cx="9" cy="10" r="1.6" /><path d="m4 17.5 5-5 4 4 2.5-2.5 4.5 4" /></>)
export const IcVideoBadge = make(<><rect x="3" y="4" width="18" height="16" rx="3.5" fill="currentColor" stroke="none" /><path d="M9.5 8.5v7l6-3.5z" fill="#1d5e50" stroke="none" /></>)
export const IcNote = make(<><path d="M9 17V6.5L18 4.5v10" /><circle cx="6.8" cy="17" r="2.2" /><circle cx="15.8" cy="14.5" r="2.2" /></>)
export const IcFolder = make(<path d="M4 7.5A1.5 1.5 0 0 1 5.5 6h4l2 2h7A1.5 1.5 0 0 1 20 9.5v8a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 4 17.5z" />)

// ---------- Properties ----------
export const IcFlipH = make(<><path d="M12 4v16" strokeDasharray="2 2" /><path d="M9.5 7 4 17h5.5zM14.5 7 20 17h-5.5z" /></>)
export const IcFlipV = make(<><path d="M4 12h16" strokeDasharray="2 2" /><path d="M7 9.5 17 4v5.5zM7 14.5 17 20v-5.5z" /></>)
export const IcRotateCw = make(<><rect x="4" y="9" width="11" height="10" rx="2" /><path d="M13 4.5h2.5A3.5 3.5 0 0 1 19 8v2.5M17 8.5l2 2 2-2" /></>)
export const IcRotateCcw = make(<><rect x="9" y="9" width="11" height="10" rx="2" /><path d="M11 4.5H8.5A3.5 3.5 0 0 0 5 8v2.5M3 8.5l2 2 2-2" /></>)
export const IcCorner = make(<path d="M4.5 9V7a2.5 2.5 0 0 1 2.5-2.5h2M15 4.5h2A2.5 2.5 0 0 1 19.5 7v2M19.5 15v2a2.5 2.5 0 0 1-2.5 2.5h-2M9 19.5H7A2.5 2.5 0 0 1 4.5 17v-2" />)
export const IcEyedropper = make(<><path d="m14 6 4 4M5 19l1-4 8.5-8.5 3 3L9 18z" /><path d="m15.5 4.5 1.5-1.5a2.1 2.1 0 0 1 3 3l-1.5 1.5" /></>)
export const IcNone = make(<><circle cx="12" cy="12" r="8" /><path d="m6.5 6.5 11 11" /></>)
export const IcDownload = make(<><path d="M12 5v10M8 11l4 4 4-4M6 19h12" /></>)
export const IcCompare = make(<><rect x="3.5" y="5" width="17" height="14" rx="2" /><path d="M12 5v14" /><path d="M12 5h6.5A1.5 1.5 0 0 1 20 6.5v11a1.5 1.5 0 0 1-1.5 1.5H12z" fill="currentColor" /></>)
export const IcMaskPen = IcPen
export const IcRect = make(<rect x="5" y="5" width="14" height="14" rx="0.5" />)
export const IcCircle = make(<circle cx="12" cy="12" r="7.5" />)
export const IcLines = make(<path d="M8 4v16M16 4v16" />)
export const IcLine = make(<path d="M12 4v16" />)
export const IcHeart = make(<path d="M12 19.5s-7.5-4.5-7.5-10A4 4 0 0 1 12 7.2a4 4 0 0 1 7.5 2.3c0 5.5-7.5 10-7.5 10z" />)
export const IcStar = make(<path d="m12 4 2.4 5 5.4.6-4 3.7 1.1 5.4L12 16l-4.9 2.7 1.1-5.4-4-3.7 5.4-.6z" />)
export const IcBrush = make(<><path d="m19.5 4.5-8 8M9 12.5l2.5 2.5-2 2c-1.5 1.5-4 1-4.5 1 .5-.5 0-3 1.5-4.5z" /><path d="M4 21c4-2 8 2 12-1" /></>)
