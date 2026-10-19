import React from "react";
import { AbsoluteFill, Html5Audio, Sequence, Series, staticFile } from "remotion";
import { C } from "./theme";
import { CutFlash, Punch } from "./components/common";
import { Hook } from "./scenes/Hook";
import { Reel, REEL, REEL_BEAT } from "./scenes/Reel";
import { Count } from "./scenes/Count";
import { EditorShowcase } from "./scenes/EditorShowcase";
import { FeatureExport, FeatureGpu, FeaturePlatforms, FeatureWfp } from "./scenes/Features";
import { Agent } from "./scenes/Agent";
import { Outro, RealApp } from "./scenes/Outro";

/** Storyboard: [component, frames]. Order and timing of the whole promo. */
export const BEATS: [string, React.FC, number][] = [
  ["Hook", Hook, 54],
  ["Reel", Reel, REEL.length * REEL_BEAT],
  ["+600", Count, 60],
  ["Editor", EditorShowcase, 150],
  ["RealApp", RealApp, 45],
  ["Wfp", FeatureWfp, 54],
  ["GPU", FeatureGpu, 54],
  ["Export", FeatureExport, 60],
  ["Platforms", FeaturePlatforms, 48],
  ["Agent", Agent, 105],
  ["Outro", Outro, 90],
];

export const PROMO_FRAMES = BEATS.reduce((a, [, , d]) => a + d, 0);

const starts = BEATS.reduce<number[]>((acc, [, , d], i) => [...acc, (acc[i] ?? 0) + d], [0]);

/** Synthesized sfx: whoosh at each beat, tick at each reel cut, hit on the logo. */
const Sfx: React.FC = () => (
  <>
    {starts.slice(1, -1).map((f, i) => (
      <Sequence key={`w${i}`} from={Math.max(0, f - 6)} durationInFrames={20}>
        <Html5Audio src={staticFile("sfx/whoosh.wav")} volume={0.35} />
      </Sequence>
    ))}
    {REEL.slice(1).map((_, i) => (
      <Sequence key={`t${i}`} from={starts[1] + (i + 1) * REEL_BEAT - 4} durationInFrames={14}>
        <Html5Audio src={staticFile("sfx/whoosh.wav")} volume={0.18} />
      </Sequence>
    ))}
    <Sequence from={0} durationInFrames={12}>
      <Html5Audio src={staticFile("sfx/hit.wav")} volume={0.6} />
    </Sequence>
    <Sequence from={starts[BEATS.length - 1] + 2} durationInFrames={12}>
      <Html5Audio src={staticFile("sfx/hit.wav")} volume={0.6} />
    </Sequence>
  </>
);

export const Promo: React.FC<{ sfx?: boolean }> = ({ sfx = true }) => (
  <AbsoluteFill style={{ backgroundColor: C.bg }}>
    <Series>
      {BEATS.map(([name, Scene, d], i) => (
        <Series.Sequence key={name} durationInFrames={d} name={name}>
          {i === 0 || name === "Reel" ? (
            <Scene />
          ) : (
            <>
              <Punch>
                <Scene />
              </Punch>
              <CutFlash frames={5} max={0.3} />
            </>
          )}
        </Series.Sequence>
      ))}
    </Series>
    {sfx && <Sfx />}
  </AbsoluteFill>
);
