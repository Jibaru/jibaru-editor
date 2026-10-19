import React from "react";
import { AbsoluteFill, Img, interpolate, staticFile, useCurrentFrame } from "remotion";
import { C, fonts } from "../theme";
import { useLayout } from "../layout";
import { Backdrop, Clip, Words, clamp, useSpring } from "../components/common";
import { LogoMark } from "../components/Logo";

/** "Si ya editas, ya sabes usarlo": the real app (sample content only). */
export const RealApp: React.FC = () => {
  const frame = useCurrentFrame();
  const { W, vertical, u, safeTop } = useLayout();
  const p = useSpring(0, { damping: 18, stiffness: 140 });
  // Screenshot is 2292x918 (library + player).
  const iw = vertical ? 1500 : 1700 * u;
  const ih = (iw * 918) / 2292;
  const pan = vertical ? interpolate(frame, [0, 45], [0, -(iw - W + 120) ], clamp) : 0;
  return (
    <AbsoluteFill>
      <Backdrop />
      <div style={{ position: "absolute", top: vertical ? safeTop + 40 : 70 * u, left: 0, right: 0, padding: `0 ${70 * u}px` }}>
        <Words text={vertical ? "Si ya editas,\nya sabes usarlo." : "Si ya editas, ya sabes usarlo."} size={(vertical ? 92 : 80) * u} accent={["sabes", "usarlo."]} />
      </div>
      <div
        style={{
          position: "absolute",
          left: vertical ? 60 + pan : (W - iw) / 2,
          top: vertical ? safeTop + 420 : 290 * u,
          width: iw,
          height: ih,
          borderRadius: 18 * u,
          overflow: "hidden",
          border: `${2 * u}px solid ${C.line}`,
          boxShadow: `0 ${40 * u}px ${120 * u}px #000d, 0 0 ${60 * u}px ${C.accent}22`,
          transform: `perspective(${2400 * u}px) rotateX(${(1 - p) * 30}deg) rotateY(${vertical ? 0 : (1 - p) * -12}deg) scale(${0.8 + 0.2 * p})`,
          opacity: p,
        }}
      >
        <Img src={staticFile("screens/prev-click-full.jpg")} style={{ width: "100%", height: "100%" }} />
      </div>
      {vertical && (
        <div style={{ position: "absolute", left: 60, right: 60, top: safeTop + 420 + ih + 60, textAlign: "center", fontFamily: fonts.ui, fontSize: 32, color: C.dim, opacity: interpolate(frame, [12, 22], [0, 1], clamp) }}>
          Biblioteca, player y timeline de siempre.
        </div>
      )}
    </AbsoluteFill>
  );
};

export const Outro: React.FC = () => {
  const frame = useCurrentFrame();
  const { vertical, square, u } = useLayout();
  const logo = useSpring(0, { damping: 11, stiffness: 160 });
  const btn = useSpring(28, { damping: 11, stiffness: 240 });
  const pulse = 1 + Math.max(0, Math.sin((frame - 50) / 6)) * 0.03 * (frame > 50 ? 1 : 0);
  const logoSize = (vertical ? 260 : 200) * u;
  return (
    <AbsoluteFill>
      <Backdrop>
        <AbsoluteFill style={{ opacity: 0.22, filter: `blur(${18 * u}px)` }}>
          <Clip name="light_leak" trimBefore={0} zoom={[1.2, 1.3]} durationInFrames={90} />
        </AbsoluteFill>
        <AbsoluteFill style={{ background: `radial-gradient(ellipse at center, transparent 0%, ${C.bg} 75%)` }} />
      </Backdrop>
      <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", flexDirection: "column", gap: 30 * u, paddingBottom: vertical ? 120 : 0 }}>
        <div style={{ transform: `scale(${logo}) rotate(${(1 - logo) * -25}deg)`, filter: `drop-shadow(0 ${20 * u}px ${60 * u}px ${C.accent}55)` }}>
          <LogoMark size={logoSize} p={interpolate(frame, [4, 24], [0, 1], clamp)} />
        </div>
        <Words text="Jibaru Editor" size={(vertical ? 128 : 112) * u} delay={8} stagger={4} />
        <div style={{ fontFamily: fonts.ui, fontWeight: 500, fontSize: (vertical ? 40 : 34) * u, color: C.dim, textAlign: "center", opacity: interpolate(frame, [18, 28], [0, 1], clamp), transform: `translateY(${interpolate(frame, [18, 28], [20, 0], clamp) * u}px)`, padding: `0 ${60 * u}px` }}>
          {vertical || square ? (
            <>
              El editor de video open source
              <br />
              para humanos y agentes de IA.
            </>
          ) : (
            "El editor de video open source para humanos y agentes de IA."
          )}
        </div>
        <div
          style={{
            marginTop: 26 * u,
            display: "flex",
            alignItems: "center",
            gap: 18 * u,
            padding: `${26 * u}px ${56 * u}px`,
            borderRadius: 60 * u,
            background: C.accent,
            color: C.bg,
            fontFamily: fonts.display,
            fontWeight: 700,
            fontSize: (vertical ? 56 : 48) * u,
            letterSpacing: "-0.02em",
            transform: `scale(${btn * pulse})`,
            boxShadow: `0 0 ${50 * u}px ${C.accent}88`,
          }}
        >
          <svg width={48 * u} height={48 * u} viewBox="0 0 24 24">
            <path d="M12 3 V15 M6.5 10 L12 15.5 L17.5 10 M4 20 H20" fill="none" stroke={C.bg} strokeWidth="2.8" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
          Descárgalo gratis
        </div>
        <div style={{ fontFamily: fonts.mono, fontWeight: 700, fontSize: (vertical ? 38 : 34) * u, color: C.text, opacity: interpolate(frame, [36, 44], [0, 1], clamp) }}>
          github.com/Jibaru/jibaru-editor
        </div>
        <div style={{ fontFamily: fonts.ui, fontSize: (vertical ? 28 : 24) * u, color: C.dim, opacity: interpolate(frame, [42, 50], [0, 1], clamp) }}>
          macOS · Windows · Apache-2.0
        </div>
      </AbsoluteFill>
    </AbsoluteFill>
  );
};
