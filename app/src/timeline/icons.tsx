// Timeline toolbar icons (Jibaru's own, same 24x24 / 1.5px grid as ui/icons).
import type { ReactNode, SVGProps } from 'react'

type P = SVGProps<SVGSVGElement> & { size?: number }

function make(paths: ReactNode) {
  return function Icon({ size = 20, ...rest }: P) {
    return (
      <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" strokeLinejoin="round" aria-hidden {...rest}>
        {paths}
      </svg>
    )
  }
}

const Ai = ({ x = 15, y = 19.5 }: { x?: number; y?: number }) => (
  <text x={x} y={y} fontSize="7.5" fontWeight="700" fill="currentColor" stroke="none" fontFamily="-apple-system, sans-serif">AI</text>
)

export const IcAiExtend = make(<><path d="M4 5.5h6M4 18.5h6M7 5.5v13" /><path d="M11 12h6M14.5 9.5 17 12l-2.5 2.5" /><path d="M19 3.5v3M17.5 5h3" stroke="#55e5c5" /></>)
export const IcAudioStretch = make(<><path d="M9 17V7l7-2v10" /><circle cx="7" cy="17" r="2" /><circle cx="14" cy="15" r="2" /><path d="M2.5 12h1.5M20 12h1.5" /></>)
export const IcSmartBgm = make(<><circle cx="9" cy="13" r="5.5" /><circle cx="9" cy="13" r="1.6" /><path d="M14 4.5v6" /><Ai x={14.2} y={21} /></>)
export const IcObjectRemover = make(<><rect x="3.5" y="4.5" width="13" height="13" rx="3" /><path d="M8.5 8.5v5l4-2.5z" /><path d="m15 15.5 3-3 3 3-3 3z" /></>)
export const IcSpeechToText = make(<><rect x="7" y="7.5" width="13" height="12.5" rx="3" /><path d="M11 11.5h5M13.5 11.5V17" /><path d="M3.5 6a3 3 0 0 1 0 4M5.5 4a6 6 0 0 1 0 8" /></>)
export const IcTranslate = make(<><path d="M3.5 6h9M8 4v2M5.5 6c.6 3 2.6 5.3 5.5 6.5M10.5 6c-.6 3-2.6 5.3-6 6.5" /><path d="m12.5 20 3.5-8.5 3.5 8.5M13.8 17h4.4" /><path d="M19 3v3M17.5 4.5h3" /></>)
export const IcColorPalette = make(<><path d="M12 3.5a8.5 8.5 0 1 0 0 17c1.2 0 1.8-.9 1.4-2-.5-1.3.4-2.5 1.8-2.5h1.8c2 0 3.5-1.6 3.5-3.6A8.7 8.7 0 0 0 12 3.5z" /><circle cx="8" cy="10" r="1" /><circle cx="12" cy="7.5" r="1" /><Ai x={13.5} y={20.5} /></>)
export const IcAiMate = make(<><circle cx="12" cy="12" r="8.5" fill="#55e5c5" stroke="#55e5c5" /><rect x="6.5" y="9" width="11" height="6.5" rx="3.2" fill="#14181b" stroke="none" /><circle cx="10" cy="12.2" r="1" fill="#55e5c5" stroke="none" /><circle cx="14" cy="12.2" r="1" fill="#55e5c5" stroke="none" /></>)
export const IcSnapTool = make(<><rect x="4" y="9" width="16" height="6" rx="2" /><path d="M12 3.5v17" /><path d="M7 4.5 8 6M17 4.5 16 6M7 19.5 8 18M17 19.5 16 18" /></>)
export const IcTextToVideo = make(<><rect x="3.5" y="5.5" width="12" height="13" rx="3" /><path d="M7 9.5h5M9.5 9.5V15" /><path d="M15.5 10 20.5 7v10l-5-3" /></>)
export const IcImageToVideo = make(<><rect x="3.5" y="4.5" width="14" height="13" rx="3" /><circle cx="8" cy="9" r="1.4" /><path d="m3.5 15 4-3.5 4 3 2-1.5 4 3.5" /><path d="m16 15 4.5 2.5L16 20z" /></>)
export const IcRenderPreview = make(<><path d="M12 3.5a8.5 8.5 0 1 1-6.5 3" /><path d="M5.5 3.5v3h3" /><path d="m10 9 5 3-5 3z" /></>)
export const IcQuickPreview = make(<><rect x="3.5" y="4.5" width="17" height="12" rx="3" /><path d="M8 20h8" /><circle cx="15.5" cy="15.5" r="3.5" fill="var(--panel)" /><path d="M15.5 13.8v1.7l1.2.8" /></>)
export const IcSlideEdit = make(<><rect x="7.5" y="5" width="9" height="14" rx="2.5" /><path d="M4.5 9.5 2.5 12l2 2.5M19.5 9.5l2 2.5-2 2.5" /></>)
export const IcKeyframing = make(<><path d="m10 6.5 5.5 5.5-5.5 5.5L4.5 12z" /><path d="m15 6.5 5.5 5.5-5.5 5.5" /><path d="M5.5 2.5v4M3.5 4.5h4" /></>)
export const IcAiMatting = make(<><rect x="3.5" y="3.5" width="17" height="17" rx="3" strokeDasharray="3 2.5" /><circle cx="12" cy="10" r="2.5" /><path d="M7.5 17a4.5 4.5 0 0 1 9 0" /></>)
export const IcAiTracking = make(<><path d="M4 8V5.5A1.5 1.5 0 0 1 5.5 4H8M16 4h2.5A1.5 1.5 0 0 1 20 5.5V8M20 16v2.5a1.5 1.5 0 0 1-1.5 1.5H16M8 20H5.5A1.5 1.5 0 0 1 4 18.5V16" /><path d="M12 9v6M9 12h6" /></>)
export const IcQuickText = make(<><path d="M5 6V4.5h14V6M12 4.5v15M9.5 19.5h5" /></>)
export const Ic3dText = make(<><path d="M5 6.5V4.5h12v2M11 4.5v11" /><path d="M8.5 15.5h5" /><path d="M17 4.5 19.5 7v2M11 15.5l2.5 2.5h0M13.5 18h-3" /></>)
export const IcShapeRect = make(<rect x="4.5" y="4.5" width="15" height="15" rx="2.5" />)
export const IcShapeArrow = make(<path d="M5 19 19 5M10 5h9v9" />)
export const IcShapeEllipse = make(<ellipse cx="12" cy="12" rx="8.5" ry="6" />)
export const IcShapeTriangle = make(<path d="M12 4.5 20 19H4z" />)
export const IcShapeLine = make(<path d="M5 19 19 5" />)
export const IcShapePolygon = make(<path d="m12 3.5 8 5.8-3 9.2H7l-3-9.2z" />)
export const IcShapeStar = make(<path d="m12 3.5 2.6 5.3 5.9.9-4.3 4.1 1 5.8L12 16.9l-5.2 2.7 1-5.8-4.3-4.1 5.9-.9z" />)
export const IcQuickSplit = make(<><path d="m5 17 10.5-10.5 2.5 2.5L7.5 19.5H5z" /><path d="M14 4.5h6v6" /></>)
export const IcSelectForward = make(<><path d="M7 4.5 15 9l-3.6 1.1L10 14z" /><path d="M14 15h6M17 12.5l3 2.5-3 2.5M4 19.5h16" /></>)
export const IcSelectBackward = make(<><path d="M7 4.5 15 9l-3.6 1.1L10 14z" /><path d="M20 15h-6M17 12.5 14 15l3 2.5M4 19.5h16" /></>)
export const IcZoomTool = make(<><circle cx="11" cy="11" r="6.5" /><path d="m16 16 4.5 4.5M11 8v6M8 11h6" /></>)
