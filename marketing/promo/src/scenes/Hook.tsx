import React from "react";
import { AbsoluteFill, interpolate, useCurrentFrame } from "remotion";
import { C, fonts } from "../theme";
import { useLayout } from "../layout";
import { Clip, Pill, Words, clamp } from "../components/common";
import { LogoMark } from "../components/Logo";

/** 0-1.8 s: the hook. Engine footage full screen, headline lands on frame 0. */
export const Hook: React.FC = () => {
  const frame = useCurrentFrame();
  const { vertical, square, u, safeTop } = useLayout();
  const size = vertical ? 150 : 132 * u;
  const shake = frame > 26 && frame < 32 ? Math.sin(frame * 9) * 6 * u : 0;
  return (
    <AbsoluteFill>
      <Clip name="candy_swirl" trimBefore={16} playbackRate={0.8} zoom={[1.3, 1.05]} durationInFrames={54} />
      <AbsoluteFill style={{ background: "radial-gradient(ellipse at center, #0009 0%, #0005 45%, #000b 100%)" }} />
      <AbsoluteFill
        style={{
          justifyContent: "center",
          alignItems: "center",
          flexDirection: "column",
          gap: 40 * u,
          paddingTop: vertical ? safeTop * 0.4 : 0,
          transform: `translateX(${shake}px)`,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 16 * u, opacity: interpolate(frame, [0, 6], [0, 1], clamp) }}>
          <LogoMark size={(vertical ? 64 : 54) * u} p={interpolate(frame, [0, 16], [0.2, 1], clamp)} />
          <span style={{ fontFamily: fonts.display, fontWeight: 700, fontSize: (vertical ? 44 : 36) * u, color: C.text, letterSpacing: "-0.02em" }}>Jibaru Editor</span>
        </div>
        <Words text={vertical || square ? "Edita video\ncomo un pro." : "Edita video como un pro."} size={size} delay={-2} stagger={3} accent={["pro"]} style={{ textShadow: "0 6px 40px #000, 0 2px 8px #000a" }} />
        <div style={{ display: "flex", gap: 24 * u, marginTop: 10 * u, flexWrap: "wrap", justifyContent: "center" }}>
          <Pill size={(vertical ? 52 : 44) * u} delay={20} solid>
            Gratis
          </Pill>
          <Pill size={(vertical ? 52 : 44) * u} delay={25}>
            Open source
          </Pill>
        </div>
      </AbsoluteFill>
    </AbsoluteFill>
  );
};
