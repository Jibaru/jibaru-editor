import React from "react";
import { AbsoluteFill, interpolate, Sequence, useCurrentFrame } from "remotion";
import { C, fonts } from "../theme";
import { useLayout } from "../layout";
import { Clip, CutFlash, clamp, useSpring } from "../components/common";

/** Transitions rendered by our engine (registry id -> on-screen name). */
/** `framed`: in vertical, show the whole 16:9 frame over a blurred fill (3D moves read better uncropped). */
export const REEL: { clip: string; name: string; id: string; trim: number; framed?: boolean }[] = [
  { clip: "cube_rotate", name: "Cubo 3D", id: "jb.cube_rotate", trim: 10, framed: true },
  { clip: "glitch_digital", name: "Glitch", id: "jb.glitch_digital", trim: 14 },
  { clip: "cine_film_burn", name: "Film burn", id: "jb.cine_film_burn", trim: 18 },
  { clip: "light_leak", name: "Light leak", id: "jb.light_leak", trim: 14 },
  { clip: "fireball_swirl", name: "Espiral de fuego", id: "jb.fireball_swirl", trim: 16 },
  { clip: "comic_page", name: "Cómic", id: "jb.comic_page", trim: 15, framed: true },
];
export const REEL_BEAT = 30;

const Label: React.FC<{ name: string; id: string; index: number }> = ({ name, id, index }) => {
  const frame = useCurrentFrame();
  const { vertical, u, safeBottom } = useLayout();
  const p = useSpring(1, { damping: 14, stiffness: 260 });
  const out = interpolate(frame, [REEL_BEAT - 5, REEL_BEAT], [0, 1], clamp);
  const size = vertical ? 96 : 88 * u;
  return (
    <div
      style={{
        position: "absolute",
        left: vertical ? 70 : 110 * u,
        bottom: vertical ? safeBottom + 30 : 100 * u,
        transform: `translateX(${(1 - p) * -120 * u + out * 60 * u}px) skewX(-8deg)`,
        opacity: interpolate(p, [0, 0.3], [0, 1], clamp) * (1 - out),
      }}
    >
      <div style={{ fontFamily: fonts.mono, fontSize: 24 * u, color: C.accent, marginBottom: 6 * u, letterSpacing: "0.02em" }}>
        {String(index + 1).padStart(2, "0")} / {id}
      </div>
      <div
        style={{
          fontFamily: fonts.display,
          fontWeight: 700,
          fontSize: size,
          lineHeight: 1,
          color: C.white,
          letterSpacing: "-0.04em",
          textShadow: "0 6px 40px #000c",
          background: `linear-gradient(transparent 62%, ${C.accent}cc 62%, ${C.accent}cc 90%, transparent 90%)`,
          backgroundSize: `${p * 100}% 100%`,
          backgroundRepeat: "no-repeat",
          padding: `0 ${8 * u}px`,
        }}
      >
        {name}
      </div>
    </div>
  );
};

/** 6 engine transitions, one per second, full screen. */
export const Reel: React.FC = () => {
  const frame = useCurrentFrame();
  const { vertical, u, safeTop } = useLayout();
  const tag = useSpring(0);
  return (
    <AbsoluteFill style={{ backgroundColor: "#000" }}>
      {REEL.map((r, i) => (
        <Sequence key={r.clip} from={i * REEL_BEAT} durationInFrames={REEL_BEAT}>
          {vertical && r.framed ? (
            <>
              <Clip name={r.clip} trimBefore={r.trim} zoom={[1.1, 1.2]} durationInFrames={REEL_BEAT} style={{ filter: "blur(40px) brightness(0.6)" }} />
              <div style={{ position: "absolute", left: 0, top: (1920 - 1080 * 9 / 16) / 2 - 60, width: 1080, height: (1080 * 9) / 16, boxShadow: "0 30px 90px #000c" }}>
                <Clip name={r.clip} trimBefore={r.trim} zoom={[1, 1.04]} durationInFrames={REEL_BEAT} />
              </div>
            </>
          ) : (
            <Clip name={r.clip} trimBefore={r.trim} zoom={i % 2 ? [1.14, 1.02] : [1.02, 1.14]} durationInFrames={REEL_BEAT} />
          )}
          <AbsoluteFill style={{ background: "linear-gradient(180deg, #0008 0%, transparent 28%, transparent 60%, #000b 100%)" }} />
          <Label name={r.name} id={r.id} index={i} />
          <CutFlash frames={4} max={0.35} />
        </Sequence>
      ))}
      {/* persistent tag */}
      <div
        style={{
          position: "absolute",
          top: vertical ? safeTop + 10 : 60 * u,
          left: 0,
          right: 0,
          display: "flex",
          justifyContent: "center",
          opacity: interpolate(tag, [0, 1], [0, 1]),
          transform: `translateY(${(1 - tag) * -30 * u}px)`,
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 14 * u,
            padding: `${12 * u}px ${26 * u}px`,
            borderRadius: 50 * u,
            background: "#0b0d0fcc",
            border: `${2 * u}px solid ${C.accent}66`,
            fontFamily: fonts.ui,
            fontWeight: 600,
            fontSize: (vertical ? 34 : 28) * u,
            color: C.text,
          }}
        >
          <span style={{ width: 14 * u, height: 14 * u, borderRadius: "50%", background: C.playhead, opacity: Math.floor(frame / 8) % 2 ? 1 : 0.35 }} />
          Render real de nuestro motor
        </div>
      </div>
    </AbsoluteFill>
  );
};
