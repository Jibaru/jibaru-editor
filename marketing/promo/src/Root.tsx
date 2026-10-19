import React from "react";
import { AbsoluteFill, Composition, Sequence, Still } from "remotion";
import { FPS } from "./theme";
import { Promo, PROMO_FRAMES } from "./Promo";
import { Outro } from "./scenes/Outro";
import { Hook } from "./scenes/Hook";
import { Reel } from "./scenes/Reel";

/** 3 s square teaser for the GIF: the hook straight into two engine cuts. */
const Teaser: React.FC = () => (
  <AbsoluteFill>
    <Sequence durationInFrames={45}>
      <Hook />
    </Sequence>
    <Sequence from={45} durationInFrames={45}>
      <Reel />
    </Sequence>
  </AbsoluteFill>
);

export const RemotionRoot: React.FC = () => (
  <>
    <Composition id="PromoVertical" component={Promo} durationInFrames={PROMO_FRAMES} fps={FPS} width={1080} height={1920} />
    <Composition id="PromoLandscape" component={Promo} durationInFrames={PROMO_FRAMES} fps={FPS} width={1920} height={1080} />
    {/* Cover: the outro end frame, square. */}
    <Still id="Cover" component={() => <Sequence from={-70}><Outro /></Sequence>} width={1080} height={1080} />
    <Composition id="Teaser" component={Teaser} durationInFrames={90} fps={FPS} width={540} height={540} />
  </>
);
