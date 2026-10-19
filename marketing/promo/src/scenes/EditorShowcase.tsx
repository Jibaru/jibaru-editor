import React from "react";
import { AbsoluteFill, Easing, Freeze, Img, interpolate, OffthreadVideo, spring, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { C, fonts } from "../theme";
import { useLayout } from "../layout";
import { Backdrop, HeadlineArea, Words, clamp } from "../components/common";
import { LibraryGrid, PlayerChrome, Timeline, TopTabs, type TLClip } from "../components/Editor";

const CARDS = [
  { id: "dissolve", label: "Dissolve" },
  { id: "cross_zoom", label: "Cross Zoom" },
  { id: "cube_rotate", label: "Cube Rotate" },
  { id: "glitch_digital", label: "Digital Glitch" },
  { id: "film_burn", label: "Film Burn" },
  { id: "light_leak", label: "Light Leak" },
  { id: "candy_swirl", label: "Candy Swirl" },
  { id: "flip_pages_right", label: "Flip Pages" },
];
const PICK = 1; // Cross Zoom
const TL_SECONDS = 8;
const CUT = 4;
const DRAG = [40, 66] as const;
const SWEEP = [76, 136] as const;

const tc = (s: number) => {
  const sec = Math.floor(s);
  const ff = Math.floor((s - sec) * 30);
  return `00:00:${String(sec).padStart(2, "0")}:${String(ff).padStart(2, "0")}`;
};

/** Recreated editor: clips slide in, a transition drops on the cut, playhead plays it. */
export const EditorShowcase: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const { vertical, u, safeTop } = useLayout();
  const sp = (d: number, cfg = {}) => spring({ frame: frame - d, fps, config: { damping: 16, stiffness: 190, mass: 0.7, ...cfg } });

  // ---- geometry
  const g = vertical
    ? {
        player: { x: 60, y: safeTop + 200, w: 960 },
        lib: { x: 60, y: safeTop + 860, w: 960, h: 200, cols: 4, cardW: 210 },
        tl: { x: 60, y: safeTop + 1080, w: 960, h: 230 },
        tracks: ["V1", "A1"],
        v1: 0,
      }
    : {
        player: { x: 1110 * u, y: 170 * u, w: 750 * u },
        lib: { x: 60 * u, y: 170 * u, w: 1030 * u, h: 520 * u, cols: 4, cardW: 220 * u },
        tl: { x: 60 * u, y: 712 * u, w: 1800 * u, h: 320 * u },
        tracks: ["V2", "V1", "A1"],
        v1: 1,
      };

  // ---- timeline content
  const clipIn = (i: number) => sp(10 + i * 6);
  const clips: TLClip[] = [
    { track: g.v1, start: 0, len: CUT, kind: "video", thumb: "frames/scene-a.jpg", label: "lago.mp4", p: clipIn(0) },
    { track: g.v1, start: CUT, len: TL_SECONDS - CUT, kind: "video", thumb: "frames/scene-b.jpg", label: "atardecer.mp4", p: clipIn(1) },
    { track: g.v1 + 1, start: 0, len: TL_SECONDS, kind: "audio", label: "musica.wav", p: clipIn(2) },
  ];
  if (!vertical) clips.push({ track: 0, start: 1, len: 2.4, kind: "title", label: "T  Título", p: clipIn(3) });

  const drop = sp(DRAG[1], { damping: 9, stiffness: 260 });
  const dropP = frame >= DRAG[1] ? drop : 0;
  const playT = interpolate(frame, SWEEP as unknown as number[], [0, TL_SECONDS], clamp);

  // ---- drag ghost: from the picked card to the cut
  const cardH = (g.lib.cardW * 9) / 16;
  const libGridX = g.lib.x + 22 * u;
  const libGridY = g.lib.y + (vertical ? 62 : 150) * u;
  const from = { x: libGridX + (PICK % g.lib.cols) * (g.lib.cardW + 20 * u), y: libGridY + Math.floor(PICK / g.lib.cols) * (cardH + 46 * u) };
  const head = 78 * u;
  const ruler = 34 * u;
  const trackH = (g.tl.h - ruler - 16 * u) / g.tracks.length;
  const to = {
    x: g.tl.x + head + (CUT / TL_SECONDS) * (g.tl.w - head) - g.lib.cardW * 0.25,
    y: g.tl.y + ruler + 8 * u + g.v1 * trackH + trackH / 2 - cardH * 0.25,
  };
  const dp = interpolate(frame, DRAG as unknown as number[], [0, 1], { ...clamp, easing: Easing.inOut(Easing.cubic) });
  const dragging = frame >= DRAG[0] - 6 && frame < DRAG[1] + 2;
  const lift = interpolate(frame, [DRAG[0] - 6, DRAG[0]], [0, 1], clamp);
  const gx = interpolate(dp, [0, 1], [from.x, to.x]);
  const gy = interpolate(dp, [0, 1], [from.y, to.y]) - Math.sin(dp * Math.PI) * 120 * u;
  const gs = interpolate(dp, [0, 1], [1, 0.5]) * (1 + lift * 0.06);

  const enter = (d: number, dx: number, dy: number) => {
    const p = sp(d);
    return { opacity: interpolate(p, [0, 0.4], [0, 1], clamp), transform: `translate(${(1 - p) * dx}px, ${(1 - p) * dy}px)` };
  };

  return (
    <AbsoluteFill>
      <Backdrop />
      <HeadlineArea top={vertical ? safeTop + 30 : 46 * u}>
        <Words text="Arrastra. Suelta. Listo." size={(vertical ? 92 : 78) * u} stagger={4} accent={["Listo"]} />
      </HeadlineArea>

      {/* library */}
      <div style={{ position: "absolute", left: g.lib.x, top: g.lib.y, width: g.lib.w, height: g.lib.h, background: C.panel, borderRadius: 14 * u, overflow: "hidden", border: `1px solid ${C.line}`, ...enter(0, -200 * u, 0) }}>
        {!vertical && <TopTabs active="transitions" u={u} />}
        <div style={{ padding: `${(vertical ? 14 : 16) * u}px ${22 * u}px`, fontFamily: fonts.ui, fontWeight: 700, fontSize: 18 * u, color: C.text, letterSpacing: "0.04em", display: "flex", justifyContent: "space-between" }}>
          <span>{vertical ? "TRANSITIONS · TRENDING" : "TRENDING"}</span>
          <span style={{ color: C.dim, fontWeight: 500, letterSpacing: 0 }}>Search all transitions</span>
        </div>
        <div style={{ padding: `0 ${22 * u}px` }}>
          <LibraryGrid
            cards={vertical ? CARDS.slice(0, 4) : CARDS}
            cols={g.lib.cols}
            cardW={g.lib.cardW}
            u={u}
            appear={(i) => sp(4 + i * 2)}
            highlight={frame >= DRAG[0] - 14 ? PICK : -1}
            hidden={dragging ? PICK : -1}
          />
        </div>
      </div>

      {/* player */}
      <div style={{ position: "absolute", left: g.player.x, top: g.player.y, ...enter(3, 0, -120 * u) }}>
        <PlayerChrome w={g.player.w} u={u} progress={playT / TL_SECONDS} timecode={tc(playT)} label={dropP > 0 ? "Cross Zoom" : "lago.mp4"}>
          <Freeze frame={Math.round(interpolate(frame, SWEEP as unknown as number[], [0, 59], clamp))}>
            <OffthreadVideo src={staticFile("clips/cross_zoom.mp4")} muted style={{ width: "100%", height: "100%", objectFit: "cover" }} />
          </Freeze>
        </PlayerChrome>
      </div>

      {/* timeline */}
      <div style={{ position: "absolute", left: g.tl.x, top: g.tl.y, ...enter(6, 0, 200 * u) }}>
        <Timeline
          w={g.tl.w}
          h={g.tl.h}
          u={u}
          duration={TL_SECONDS}
          clips={clips}
          playhead={playT}
          tracks={g.tracks}
          transitions={dropP > 0 ? [{ at: CUT, track: g.v1, p: dropP }] : []}
        />
      </div>

      {/* drag ghost + pointer */}
      {dragging && (
        <div style={{ position: "absolute", left: gx, top: gy, transform: `scale(${gs})`, transformOrigin: "top left", opacity: interpolate(frame, [DRAG[1] - 2, DRAG[1] + 2], [1, 0], clamp) }}>
          <div style={{ width: g.lib.cardW, height: cardH, borderRadius: 10 * u, overflow: "hidden", border: `${3 * u}px solid ${C.accent}`, boxShadow: `0 ${20 * u}px ${50 * u}px #000c, 0 0 ${30 * u}px ${C.accent}88`, transform: `rotate(${Math.sin(dp * Math.PI) * -6}deg)` }}>
            <Img src={staticFile(`cards/${CARDS[PICK].id}.jpg`)} style={{ width: "100%", height: "100%", objectFit: "cover" }} />
          </div>
          <svg width={54 * u} height={54 * u} viewBox="0 0 24 24" style={{ position: "absolute", left: g.lib.cardW * 0.55, top: cardH * 0.5 }}>
            <path d="M4 2 L4 19 L8.5 14.8 L11.5 21.5 L14.5 20.2 L11.6 13.6 L17.5 13.4 Z" fill="#fff" stroke="#000" strokeWidth={1.2} strokeLinejoin="round" />
          </svg>
        </div>
      )}

      {/* landing ripple */}
      {frame >= DRAG[1] && frame < DRAG[1] + 16 && (
        <div
          style={{
            position: "absolute",
            left: g.tl.x + head + (CUT / TL_SECONDS) * (g.tl.w - head),
            top: to.y + cardH * 0.25,
            width: 0,
            height: 0,
          }}
        >
          <div
            style={{
              position: "absolute",
              left: -interpolate(frame - DRAG[1], [0, 16], [20, 140]) * u,
              top: -interpolate(frame - DRAG[1], [0, 16], [20, 140]) * u,
              width: interpolate(frame - DRAG[1], [0, 16], [40, 280]) * u,
              height: interpolate(frame - DRAG[1], [0, 16], [40, 280]) * u,
              borderRadius: "50%",
              border: `${4 * u}px solid ${C.accent}`,
              opacity: interpolate(frame - DRAG[1], [0, 16], [1, 0]),
            }}
          />
        </div>
      )}
    </AbsoluteFill>
  );
};
