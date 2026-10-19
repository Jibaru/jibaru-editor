import React from "react";
import { Img, interpolate, staticFile } from "remotion";
import { C, fonts } from "../theme";
import { clamp } from "./common";

/* ------------------------------------------------------------------ icons */

const Icon: React.FC<{ name: string; size: number; color: string }> = ({ name, size, color }) => {
  const s = { fill: "none", stroke: color, strokeWidth: 2, strokeLinecap: "round", strokeLinejoin: "round" } as const;
  const shapes: Record<string, React.ReactNode> = {
    media: (
      <>
        <rect x="3" y="4" width="18" height="16" rx="3" {...s} />
        <path d="M10 9 L15 12 L10 15 Z" {...s} />
      </>
    ),
    audio: (
      <>
        <path d="M9 18 V6 L19 4 V16" {...s} />
        <circle cx="7" cy="18" r="2.5" {...s} />
        <circle cx="17" cy="16" r="2.5" {...s} />
      </>
    ),
    titles: <path d="M5 5 H19 M12 5 V20" {...s} />,
    transitions: (
      <>
        <rect x="3" y="4" width="18" height="16" rx="3" {...s} />
        <path d="M7 8 L11 12 L7 16 Z M17 8 L13 12 L17 16 Z" {...s} />
      </>
    ),
    effects: <path d="M12 3 L14.6 9 L21 9.6 L16 13.8 L17.6 20 L12 16.6 L6.4 20 L8 13.8 L3 9.6 L9.4 9 Z" {...s} />,
    filters: (
      <>
        <circle cx="12" cy="8.5" r="5" {...s} />
        <circle cx="8.5" cy="14.5" r="5" {...s} />
        <circle cx="15.5" cy="14.5" r="5" {...s} />
      </>
    ),
  };
  return (
    <svg width={size} height={size} viewBox="0 0 24 24">
      {shapes[name]}
    </svg>
  );
};

/* ------------------------------------------------------------- top tabs */

const TABS = [
  ["media", "Media"],
  ["audio", "Audio"],
  ["titles", "Titles"],
  ["transitions", "Transitions"],
  ["effects", "Effects"],
  ["filters", "Filters"],
] as const;

export const TopTabs: React.FC<{ active: string; u: number; compact?: boolean }> = ({ active, u, compact }) => (
  <div style={{ display: "flex", justifyContent: "space-around", padding: `${14 * u}px ${10 * u}px`, background: C.gutter, borderBottom: `1px solid ${C.line}` }}>
    {TABS.map(([id, label]) => {
      const on = id === active;
      const col = on ? C.accent : C.text;
      return (
        <div key={id} style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 6 * u, color: col, fontFamily: fonts.ui, fontWeight: 500, fontSize: (compact ? 17 : 20) * u }}>
          <Icon name={id} size={(compact ? 30 : 34) * u} color={col} />
          {label}
        </div>
      );
    })}
  </div>
);

/* --------------------------------------------------------- library grid */

export type Card = { id: string; label: string };

export const LibraryGrid: React.FC<{
  cards: Card[];
  cols: number;
  cardW: number;
  u: number;
  /** 0..1 appearance progress per card index. */
  appear?: (i: number) => number;
  highlight?: number;
  hidden?: number;
}> = ({ cards, cols, cardW, u, appear = () => 1, highlight = -1, hidden = -1 }) => (
  <div style={{ display: "grid", gridTemplateColumns: `repeat(${cols}, ${cardW}px)`, gap: `${18 * u}px ${20 * u}px` }}>
    {cards.map((c, i) => {
      const p = appear(i);
      const on = i === highlight;
      return (
        <div
          key={c.id + i}
          style={{
            opacity: interpolate(p, [0, 0.5], [0, 1], clamp) * (i === hidden ? 0.35 : 1),
            transform: `translateY(${(1 - p) * 30 * u}px) scale(${0.9 + 0.1 * p})`,
          }}
        >
          <div
            style={{
              width: cardW,
              height: (cardW * 9) / 16,
              borderRadius: 8 * u,
              overflow: "hidden",
              border: `${on ? 3 * u : 1.5 * u}px solid ${on ? C.accent : C.line}`,
              boxShadow: on ? `0 0 ${24 * u}px ${C.accent}88` : "none",
              boxSizing: "border-box",
            }}
          >
            <Img src={staticFile(`cards/${c.id}.jpg`)} style={{ width: "100%", height: "100%", objectFit: "cover", display: "block" }} />
          </div>
          <div style={{ marginTop: 8 * u, fontFamily: fonts.ui, fontSize: 17 * u, color: on ? C.text : C.dim, fontWeight: on ? 600 : 400, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis", width: cardW }}>
            {c.label}
          </div>
        </div>
      );
    })}
  </div>
);

/* --------------------------------------------------------------- player */

export const PlayerChrome: React.FC<{
  w: number;
  u: number;
  progress: number;
  timecode: string;
  children: React.ReactNode;
  label?: string;
}> = ({ w, u, progress, timecode, children, label = "Cross Zoom" }) => {
  const vh = (w * 9) / 16;
  return (
    <div style={{ width: w, background: C.panel, borderRadius: 14 * u, overflow: "hidden", border: `1px solid ${C.line}` }}>
      <div style={{ display: "flex", gap: 22 * u, padding: `${12 * u}px ${18 * u}px`, fontFamily: fonts.ui, fontWeight: 600, fontSize: 19 * u, background: C.gutter }}>
        <span style={{ color: C.text }}>Timeline</span>
        <span style={{ color: C.dim }}>Source</span>
        <span style={{ marginLeft: "auto", color: C.dim, fontWeight: 500, fontSize: 16 * u, padding: `${3 * u}px ${12 * u}px`, borderRadius: 6 * u, background: C.panel2 }}>Full Quality</span>
      </div>
      <div style={{ position: "relative", width: w, height: vh, background: "#000", overflow: "hidden" }}>{children}</div>
      <div style={{ display: "flex", alignItems: "center", gap: 16 * u, padding: `${12 * u}px ${18 * u}px` }}>
        <div style={{ flex: 1, height: 6 * u, borderRadius: 3 * u, background: C.panel2, position: "relative" }}>
          <div style={{ width: `${progress * 100}%`, height: "100%", borderRadius: 3 * u, background: C.accent }} />
          <div style={{ position: "absolute", left: `calc(${progress * 100}% - ${9 * u}px)`, top: -6 * u, width: 18 * u, height: 18 * u, borderRadius: "50%", background: "#cfd8db" }} />
        </div>
        <span style={{ fontFamily: fonts.mono, fontSize: 17 * u, color: C.text, background: C.gutter, padding: `${4 * u}px ${10 * u}px`, borderRadius: 6 * u }}>{timecode}</span>
        <span style={{ fontFamily: fonts.ui, fontSize: 16 * u, color: C.dim }}>{label}</span>
      </div>
    </div>
  );
};

/* ------------------------------------------------------------- timeline */

export type TLClip = {
  track: number; // 0 = top video track
  start: number; // seconds
  len: number;
  kind: "video" | "audio" | "title";
  thumb?: string; // public path
  label?: string;
  /** Entrance progress 0..1. */
  p?: number;
  /** Optional silence regions [start, len] (seconds, relative to clip) for audio. */
  flat?: [number, number][];
};

const wave = (i: number) => {
  const v = Math.abs(Math.sin(i * 1.7) * 0.6 + Math.sin(i * 0.37) * 0.3 + Math.sin(i * 5.1) * 0.25);
  return 0.18 + Math.min(1, v) * 0.82;
};

export const Waveform: React.FC<{ w: number; h: number; seed: number; flat?: [number, number][]; len: number; color?: string }> = ({
  w,
  h,
  seed,
  flat = [],
  len,
  color = "#58c7aa",
}) => {
  const bars = Math.max(4, Math.floor(w / 5));
  return (
    <svg width={w} height={h} style={{ position: "absolute", left: 0, top: 0 }}>
      {Array.from({ length: bars }).map((_, i) => {
        const t = (i / bars) * len;
        const silent = flat.some(([s, l]) => t >= s && t <= s + l);
        const a = silent ? 0.04 : wave(i + seed * 13);
        return <rect key={i} x={i * 5} y={h / 2 - (a * h * 0.42)} width={3} height={Math.max(1, a * h * 0.84)} fill={color} opacity={0.85} rx={1} />;
      })}
    </svg>
  );
};

export const Timeline: React.FC<{
  w: number;
  h: number;
  u: number;
  duration: number; // seconds visible
  clips: TLClip[];
  playhead: number; // seconds
  tracks: string[]; // labels, top to bottom
  /** Transition boxes: at seconds, track index, progress 0..1 (drop). */
  transitions?: { at: number; track: number; p: number }[];
  /** Red highlight regions: [start, len, opacity]. */
  marks?: [number, number, number][];
  children?: React.ReactNode;
}> = ({ w, h, u, duration, clips, playhead, tracks, transitions = [], marks = [], children }) => {
  const head = 78 * u;
  const ruler = 34 * u;
  const trackH = (h - ruler - 16 * u) / tracks.length;
  const areaW = w - head;
  const x = (s: number) => (s / duration) * areaW;
  return (
    <div style={{ position: "relative", width: w, height: h, background: C.panel, borderRadius: 14 * u, overflow: "hidden", border: `1px solid ${C.line}` }}>
      {/* ruler */}
      <div style={{ position: "absolute", left: head, top: 0, width: areaW, height: ruler, background: C.gutter, borderBottom: `1px solid ${C.line}` }}>
        {Array.from({ length: Math.ceil(duration) + 1 }).map((_, s) => (
          <div key={s} style={{ position: "absolute", left: x(s), top: 0, height: ruler, borderLeft: `1px solid ${C.line}` }}>
            <span style={{ position: "absolute", left: 5 * u, top: 6 * u, fontFamily: fonts.mono, fontSize: 13 * u, color: C.dim, whiteSpace: "nowrap" }}>
              00:{String(s).padStart(2, "0")}
            </span>
          </div>
        ))}
      </div>
      {/* marks */}
      {marks.map(([s, l, o], i) => (
        <div
          key={i}
          style={{
            position: "absolute",
            left: head + x(s),
            top: ruler,
            width: Math.max(0, x(l)),
            height: h - ruler,
            background: `repeating-linear-gradient(45deg, ${C.playhead}55 0 ${8 * u}px, ${C.playhead}22 ${8 * u}px ${16 * u}px)`,
            borderLeft: `${2 * u}px solid ${C.playhead}`,
            borderRight: `${2 * u}px solid ${C.playhead}`,
            boxSizing: "border-box",
            opacity: o,
          }}
        />
      ))}
      {/* clips */}
      {clips.map((c, i) => {
        const p = c.p ?? 1;
        const top = ruler + 8 * u + c.track * trackH;
        const cw = x(c.len) - 4 * u;
        const ch = trackH - 6 * u;
        const isAudio = c.kind === "audio";
        const bg = isAudio ? C.audio : c.kind === "title" ? "#5a3f8c" : C.clip;
        const hdr = isAudio ? "#2a6a5c" : c.kind === "title" ? "#7656b0" : C.clipHeader;
        return (
          <div
            key={i}
            style={{
              position: "absolute",
              left: head + x(c.start) + 2 * u + (1 - p) * areaW * 0.7,
              top,
              width: cw,
              height: ch,
              borderRadius: 7 * u,
              overflow: "hidden",
              background: bg,
              border: `${1.5 * u}px solid ${hdr}`,
              boxSizing: "border-box",
              opacity: interpolate(p, [0, 0.3], [0, 1], clamp),
            }}
          >
            <div style={{ height: 22 * u, background: hdr, fontFamily: fonts.ui, fontSize: 13 * u, fontWeight: 600, color: "#e9fffa", padding: `${3 * u}px ${8 * u}px`, boxSizing: "border-box", whiteSpace: "nowrap", overflow: "hidden" }}>
              {c.label}
            </div>
            {c.thumb && !isAudio && (
              <div
                style={{
                  position: "absolute",
                  top: 22 * u,
                  left: 0,
                  right: 0,
                  bottom: 0,
                  backgroundImage: `url(${staticFile(c.thumb)})`,
                  backgroundSize: "auto 100%",
                  backgroundRepeat: "repeat-x",
                  opacity: 0.9,
                }}
              />
            )}
            {isAudio && (
              <div style={{ position: "absolute", top: 22 * u, left: 0, right: 0, bottom: 0 }}>
                <Waveform w={cw} h={ch - 22 * u} seed={i} len={c.len} flat={c.flat} />
              </div>
            )}
          </div>
        );
      })}
      {/* track headers */}
      {tracks.map((t, i) => (
        <div
          key={t}
          style={{
            position: "absolute",
            left: 0,
            top: ruler + 8 * u + i * trackH,
            width: head,
            height: trackH - 6 * u,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontFamily: fonts.ui,
            fontWeight: 700,
            fontSize: 17 * u,
            color: C.dim,
            background: C.gutter,
            borderRight: `1px solid ${C.line}`,
          }}
        >
          {t}
        </div>
      ))}
      {/* transitions */}
      {transitions.map((t, i) => {
        const bw = 56 * u;
        const bh = 40 * u;
        const top = ruler + 8 * u + t.track * trackH + 22 * u + (trackH - 28 * u - bh) / 2;
        const drop = interpolate(t.p, [0, 1], [-160 * u, 0]);
        return (
          <div
            key={i}
            style={{
              position: "absolute",
              left: head + x(t.at) - bw / 2,
              top: top + drop,
              width: bw,
              height: bh,
              borderRadius: 7 * u,
              background: `linear-gradient(135deg, ${C.accent}, #1aa889)`,
              boxShadow: `0 ${6 * u}px ${18 * u}px #0009, 0 0 ${18 * u * t.p}px ${C.accent}`,
              opacity: interpolate(t.p, [0, 0.2], [0, 1], clamp),
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            <svg width={30 * u} height={22 * u} viewBox="0 0 30 22">
              <path d="M3 3 L13 11 L3 19 Z M27 3 L17 11 L27 19 Z" fill={C.bg} />
            </svg>
          </div>
        );
      })}
      {children}
      {/* playhead */}
      <div style={{ position: "absolute", left: head + x(playhead) - 1.5 * u, top: 0, width: 3 * u, height: h, background: C.playhead, boxShadow: `0 0 ${10 * u}px ${C.playhead}` }}>
        <div style={{ position: "absolute", top: 0, left: -9 * u, width: 21 * u, height: 22 * u, background: C.playhead, clipPath: "polygon(0 0, 100% 0, 100% 60%, 50% 100%, 0 60%)" }} />
      </div>
    </div>
  );
};
