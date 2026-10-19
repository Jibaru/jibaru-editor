# Marketing assets

`marketing/promo/` is a [Remotion](https://www.remotion.dev) project that
renders the ~30 s social media promo (on-screen copy in Spanish):

| Composition | Size | Use |
| --- | --- | --- |
| `PromoVertical` | 1080×1920, 30 fps | Reels / TikTok / Shorts (text kept out of the top 220 px and bottom 380 px) |
| `PromoLandscape` | 1920×1080, 30 fps | YouTube, X, LinkedIn |
| `Cover` | 1080×1080 still | Post cover |
| `Teaser` | 540×540, 3 s | GIF teaser |

All transition footage is rendered by our own engine on its procedural
sample scenes (`jb fx player-frames`), so the video contains no third-party
or personal footage. Generated clips are not committed:

```sh
cd marketing/promo
npm install
bash scripts/generate-clips.sh        # builds jb-cli, renders public/clips at 2160p
npm run dev                           # Remotion Studio
npm run render                        # out/*.mp4, out/jibaru-cover.png, out/jibaru-teaser.gif
node scripts/stills.mjs PromoVertical 0,60,300 0.5   # quick preview stills
```

The video has no music (add trending audio on the platform); it carries only
quiet synthesized whooshes. Filmora is mentioned only as a factual
compatibility claim (".wfp de Filmora") — never its logo or UI.
