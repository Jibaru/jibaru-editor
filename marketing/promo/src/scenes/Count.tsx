import React from "react";
import { AbsoluteFill, Easing, Img, interpolate, spring, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { C, fonts } from "../theme";
import { useLayout } from "../layout";
import { Backdrop, Words, clamp } from "../components/common";

export const CARD_IDS = [
  "candy_swirl", "cube_rotate", "glitch_digital", "film_burn", "light_leak", "fireball_swirl",
  "comic_page", "flip_pages_right", "carousel_spin_left", "cine_film_burn", "cross_zoom", "spin_zoom",
  "dry_brush_sweeps", "glitch_bands", "fast_light_leaks", "vhs", "glitch", "mosaic",
  "chromatic_aberration", "camera_shake", "gaussian_blur", "dissolve", "blur_dissolve", "fast_zoom",
];

/** "+600 transiciones y efectos": a tilted wall of real library cards and a counter. */
export const Count: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const { W, H, vertical, u } = useLayout();
  const cols = vertical ? 4 : 8;
  const rows = vertical ? 9 : 5;
  const cw = vertical ? 330 : 300 * u;
  const ch = (cw * 9) / 16;
  const gap = 22 * u;
  const n = interpolate(frame, [4, 34], [0, 600], { ...clamp, easing: Easing.out(Easing.cubic) });
  const pop = spring({ frame: frame - 34, fps, config: { damping: 9, stiffness: 300 } });
  const scroll = frame * 4 * u;
  return (
    <AbsoluteFill>
      <Backdrop />
      <AbsoluteFill style={{ perspective: 1800 * u, overflow: "hidden" }}>
        <div
          style={{
            position: "absolute",
            left: W / 2,
            top: H / 2,
            transform: `translate(-50%, -50%) rotateX(28deg) rotateZ(-14deg) translateY(${-scroll}px)`,
            display: "grid",
            gridTemplateColumns: `repeat(${cols + 2}, ${cw}px)`,
            gap,
          }}
        >
          {Array.from({ length: (cols + 2) * (rows + 3) }).map((_, i) => {
            const p = spring({ frame: frame - (i % 11), fps, config: { damping: 16, stiffness: 200 } });
            return (
              <div
                key={i}
                style={{
                  width: cw,
                  height: ch,
                  borderRadius: 12 * u,
                  overflow: "hidden",
                  border: `${2 * u}px solid ${C.line}`,
                  opacity: p * 0.75,
                  transform: `translateZ(${(1 - p) * 400 * u}px)`,
                }}
              >
                <Img src={staticFile(`cards/${CARD_IDS[(i * 7) % CARD_IDS.length]}.jpg`)} style={{ width: "100%", height: "100%", objectFit: "cover" }} />
              </div>
            );
          })}
        </div>
      </AbsoluteFill>
      <AbsoluteFill style={{ background: `radial-gradient(ellipse at center, ${C.bg}f0 0%, ${C.bg}c0 35%, ${C.bg}20 75%)` }} />
      <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", flexDirection: "column" }}>
        <div
          style={{
            fontFamily: fonts.display,
            fontWeight: 700,
            fontSize: (vertical ? 300 : 260) * u,
            lineHeight: 0.9,
            letterSpacing: "-0.06em",
            color: C.accent,
            textShadow: `0 0 ${60 * u}px ${C.accent}66`,
            transform: `scale(${1 + pop * 0.08 - (frame > 34 ? 0.08 : 0)})`,
            fontVariantNumeric: "tabular-nums",
          }}
        >
          +{Math.round(n)}
        </div>
        <Words text={vertical ? "transiciones\ny efectos" : "transiciones y efectos"} size={(vertical ? 96 : 84) * u} delay={8} stagger={3} />
        <div style={{ marginTop: 26 * u, fontFamily: fonts.ui, fontSize: (vertical ? 32 : 28) * u, color: C.dim, opacity: interpolate(frame, [22, 32], [0, 1], clamp) }}>
          Filtros, títulos y animaciones en GPU
        </div>
      </AbsoluteFill>
    </AbsoluteFill>
  );
};
