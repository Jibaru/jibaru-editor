# Filmora catalog

Metadata-only inventory of Filmora 15.7.3 (Mac) transitions, effects, filters,
overlays, animations, titles and LUTs, built so agents can recreate each item
clean-room. The data is `docs/filmora-catalog.json` (one entry per item); check
progress with `jb fx catalog-status` (see the end of this page).

**Legal boundary.** Only names, ids/GUIDs, categories, flags, durations and
parameter schemas (name, type, default, range) were read from Filmora's
metadata. Shaders, LUT files, overlay videos, images, fonts and previews were
never copied: entries only point at the local preview files (`~/Library/...`,
outside the repository) and at the store's public preview URL, and the
measurements below are numbers derived from decoding those previews.

## Where the metadata came from

| Source | Items | What it is |
|---|---:|---|
| `store_cache` | 950 | ProgramData/resInfo.db table onlineCloudResourceInfo (store catalog rows cached while browsing) |
| `store_recent` | 34 | ProgramData/resInfo.db table cloudResourceInfo (recently used / installed store rows) |
| `store_page` | 561 | ProgramData/resInfo.db table resourcePageInfos (ordered category pages; gives page_rank) |
| `related_preset` | 4 | ProgramData/transitions_associated_resource.dat (store 'related presets') |
| `downloaded` | 36 | ProgramData/Download/Filmora/<type>/<res>/resinfo.json + Data/*.conf, filter.xml, data.json, project.json |
| `bundled` | 17 | Filmora.app/Contents/Resources/Download/Filmora (resources shipped with the app) |
| `browsed_temp` | 979 | ProgramData/Download/Temp/<type>/<res> (store items previewed but not installed) |
| `builtin` | 595 | Filmora.app/Contents/Resources/resources/wfx_effect/material/{OpenGL,OpenCL,PureC}Filter.conf, wfx_effect/Transition, resources/captions/Animation.xml, Default Effects/Preset Animation, Default Effects/LightRoom |
| `categories` |  | ProgramData/Download/Category/en/categoryTree.json (store category tree with item totals) |
| `usage` |  | docs/filmora-coverage.md (items used in the reference projects) |

The store catalog is cached by Filmora as the user browses: `resInfo.db`
holds one row per item seen (`onlineCloudResourceInfo`, with `sale_mode` = 1
for paid items) plus the ordered category pages that were opened
(`resourcePageInfos`). `Download/Temp/<type>/<item>` folders are created for
every item previewed in the store, many with their `previewVideo.mp4`.
Built-in effects come from the app bundle's GPU filter registries (GUID,
name, parameters) and the captions/animation preset XMLs.

## How much of the store is listed locally

Non-downloaded store items **are** listed, but only those Filmora cached while
the store pages were browsed. The category tree gives the full store size per
category (`store_categories` in the JSON); summed category totals overcount
because one item appears in several categories, but they show the order of
magnitude still missing from the local cache.

| Kind | Listed locally | of which store items | premium, not downloaded | Sum of store category totals |
|---|---:|---:|---:|---:|
| transition | 288 | 270 | 199 | 2941 |
| effect | 405 | 143 | 98 | 4220 |
| filter | 146 | 35 | 28 | 939 |
| overlay | 89 | 89 | 54 | 42 |
| animation | 542 | 367 | 314 | 643 |
| title | 90 | 90 | 19 | 8007 |
| lut | 19 | 0 | 0 |  |

Overlays (store type 4) are filed under the *Video Effects* categories, so
their store size is part of the effect row. Titles are rendered through the
text/title-template import path rather than the effect registry, so their
`jb` stays null here (`Basic 1`, the only title in the reference projects, is
already supported by the Filmora importer).

To list more of the store, browse more categories in Filmora (each page opened
is cached) and re-run the catalog builder.

## Counts

| Kind | Items | Premium | Free | Unknown | Downloaded / built-in | Implemented (`jb`) | Look-alike (`jb_similar`) | With preview |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| transition | 288 | 203 | 76 | 9 | 29 | 9 | 15 | 57 |
| effect | 405 | 98 | 291 | 16 | 273 | 1 | 15 | 1 |
| filter | 146 | 29 | 117 | 0 | 113 | 0 | 3 | 1 |
| overlay | 89 | 54 | 35 | 0 | 2 | 0 | 1 | 0 |
| animation | 542 | 315 | 227 | 0 | 183 | 1 | 14 | 6 |
| title | 90 | 19 | 71 | 0 | 2 | 0 | 0 | 0 |
| lut | 19 | 0 | 19 | 0 | 19 | 0 | 0 | 0 |
| **total** | **1579** | **718** | **836** | **25** | **621** | **11** | **48** | **65** |

Out of scope and only counted (store rows seen): font 575, music 50, project_template 52, sound 101, stock_color 29, stock_video 71, type_147 1, type_167 1, type_41 1, type_58 1.
No sticker rows were cached (the Stickers store was never browsed).

### By implementation group

| Group | transition | effect | filter | overlay | animation | title | lut | Total | Premium | Implemented |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| zoom_punch_shake | 56 | 27 |  |  | 54 |  |  | 137 | 96 | 2 |
| slide_push_whip | 45 | 6 |  |  | 75 |  |  | 126 | 94 | 3 |
| shape_luma_wipes | 46 | 29 |  |  | 32 |  |  | 107 | 71 | 0 |
| 3d_flip_cube | 33 | 24 |  |  | 69 |  |  | 126 | 98 | 0 |
| distortion_warp | 11 | 31 |  |  | 10 |  |  | 52 | 20 | 0 |
| glitch_digital | 15 | 28 |  | 8 | 34 |  |  | 85 | 58 | 0 |
| film_burn_leak | 9 | 9 |  | 21 | 14 |  |  | 53 | 37 | 1 |
| light_flash | 20 | 34 |  | 24 | 39 |  |  | 117 | 75 | 0 |
| overlays_particles | 9 | 14 |  | 36 | 15 |  |  | 74 | 52 | 0 |
| blur | 8 | 26 |  |  | 5 |  |  | 39 | 17 | 1 |
| fade_dissolve | 26 | 3 |  |  | 19 |  |  | 48 | 24 | 2 |
| color_looks |  |  | 146 |  |  |  | 19 | 165 | 29 | 0 |
| text_animations |  | 7 |  |  | 168 | 90 |  | 265 | 19 | 1 |
| audio_reactive |  | 44 |  |  |  |  |  | 44 | 0 | 0 |
| ai_generative | 3 | 53 |  |  |  |  |  | 56 | 17 | 0 |
| other | 7 | 70 |  |  | 8 |  |  | 85 | 11 | 1 |

### By Filmora category (store items, top 40)

| Kind | Category | Items | Premium | Implemented |
|---|---|---:|---:|---:|
| animation | Video Animation / In | 244 | 209 | 0 |
| animation | Video Animation / Out | 121 | 106 | 0 |
| transition | Transitions / Zoom | 53 | 48 | 2 |
| transition | Transitions / Fade&Dissolve | 48 | 40 | 3 |
| filter | Filters / Basic | 35 | 29 | 0 |
| transition | Transitions / Slides | 32 | 27 | 2 |
| transition | Transitions / SFX Transitions | 18 | 18 | 0 |
| transition | Transitions / 3D | 16 | 15 | 0 |
| effect | Video Effects / Shake | 16 | 14 | 0 |
| title | Title Effects / Plain Text | 13 | 6 | 0 |
| effect | Video Animation / In | 10 | 10 | 0 |
| title | New Concept | 10 | 0 | 0 |
| title | Titles | 9 | 0 | 0 |
| transition | Transitions / Speed Blur | 8 | 7 | 1 |
| title | Title Effects / 3D Titles | 8 | 0 | 0 |
| effect | Video Effects / Zoom | 7 | 7 | 0 |
| effect | Video Effects / Scan | 7 | 7 | 0 |
| transition | Transitions / Camera | 6 | 5 | 0 |
| effect | AI Portrait-2 / AI Portrait-2.0 | 6 | 4 | 0 |
| title | Title Templates / Subtitles | 6 | 0 | 0 |
| transition | Transitions / Glitch | 5 | 5 | 0 |
| transition | Old Old Archive | 5 | 0 | 0 |
| transition | Top Picks | 5 | 5 | 0 |
| effect | Video Effects / Basic | 5 | 4 | 1 |
| effect | Video Effects / Blur | 5 | 4 | 0 |
| effect | Video Effects / 3D | 5 | 5 | 0 |
| effect | Video Effects / Slideshow | 5 | 5 | 0 |
| overlay | Makeup | 5 | 0 | 0 |
| title | Basic | 5 | 0 | 0 |
| transition | Transitions / Light | 4 | 2 | 0 |
| transition | Transitions / Split | 4 | 4 | 0 |
| effect | AI Relight / Facial | 4 | 0 | 0 |
| effect | Video Effects / Flicker | 4 | 4 | 0 |
| effect | Common | 4 | 0 | 0 |
| overlay | Gaming | 4 | 3 | 0 |
| overlay | Heart | 4 | 2 | 0 |
| overlay | Love | 4 | 3 | 0 |
| overlay | Nature | 4 | 3 | 0 |
| title | Title Effects / WordArt | 4 | 1 | 0 |
| title | End Credits | 4 | 0 | 0 |

## Work queue (popularity order)

`popularity.score` ranks unimplemented items: use in the reference projects
(+1000) first, then rank in the store's *Trending* category, membership in
highlight categories (Popular, Annual Highlights, Top Picks), rank in other
cached category pages, and whether the user downloaded or previewed the item.
Filmora's cache has no download/usage counts, so this is a proxy. AI-dependent
and audio-reactive items are excluded here (see their groups).

### Transitions

| # | Name | Key | Group | Premium | Downloaded | Preview | Score | Hints |
|---:|---|---|---|---|---|---|---:|---|
| 1 | Warp Zoom 4 | `store:63902786` | zoom_punch_shake | yes | yes | local | 105.0 | browsed, downloaded, recently_used |
| 2 | Zoom | `store:44122` | zoom_punch_shake |  |  | local | 99.5 | browsed, trending |
| 3 | Lens Zoom | `store:1455666` | zoom_punch_shake |  |  | local | 98.0 | browsed, trending |
| 4 | Dissolve 02 | `store:1479502` | fade_dissolve | yes |  | url | 97.0 | browsed, trending |
| 5 | Flash | `store:39164` | light_flash |  |  | local | 96.0 | browsed, trending |
| 6 | Cinematic Transition 02 | `store:1080108` | light_flash | yes |  | local | 95 | browsed, popular_or_highlight, trending |
| 7 | Glitch Intro Transition 09 | `store:1304896` | glitch_digital | yes |  | local | 95 | browsed, popular_or_highlight, trending |
| 8 | Halo Blur | `store:1938542` | blur | yes |  | url | 95 | browsed, popular_or_highlight, trending |
| 9 | Round Zoom Out | `store:39209` | zoom_punch_shake |  |  | url | 95.0 | browsed, trending |
| 10 | Slow Zoom | `store:1650286` | zoom_punch_shake | yes |  | local | 95 | browsed, popular_or_highlight, trending |
| 11 | Evaporate 2 | `store:39153` | shape_luma_wipes |  |  | url | 89.0 | browsed, trending |
| 12 | Push Right | `store:39200` | slide_push_whip |  |  | url | 89.0 | browsed, trending |
| 13 | Push Left | `store:1650274` | slide_push_whip |  |  | local | 88.5 | browsed, trending |
| 14 | Push Up | `store:1650278` | slide_push_whip | yes |  | local | 88.0 | browsed, trending |
| 15 | Push Down | `store:1653462` | slide_push_whip | yes |  | url | 87.5 | browsed, trending |
| 16 | Blur Dissolve 2 | `store:1650250` | blur | yes |  | url | 86.0 | browsed, trending |
| 17 | Erase | `store:39150` | shape_luma_wipes |  |  | url | 86.0 | browsed, trending |
| 18 | Glitch Dissolve | `store:1668930` | glitch_digital | yes |  | url | 84.0 | browsed, trending |
| 19 | Dissolve White | `store:1668934` | fade_dissolve | yes |  | url | 83.5 | browsed, trending |
| 20 | Blur Dissolve 1 | `store:1660914` | blur |  |  | url | 70 | browsed, trending |
| 21 | Cinematic Digital Slideshow Transition 07 | `store:1375852` | glitch_digital |  |  | url | 70 | browsed, trending |
| 22 | Cinematic Transition 04 | `store:1080122` | light_flash |  |  | local | 70 | browsed, trending |
| 23 | Fast Zoom in | `store:1704930` | zoom_punch_shake | yes |  | url | 70 | browsed, trending |
| 24 | Fast Zoom out | `store:1702578` | zoom_punch_shake | yes |  | url | 70 | browsed, trending |
| 25 | Morph | `store:39181` | distortion_warp |  |  | url | 70 | browsed, trending |
| 26 | Page Curl | `store:39194` | 3d_flip_cube |  |  |  | 70 | browsed, trending |
| 27 | Shape Transition 60 | `store:358471` | shape_luma_wipes |  |  | url | 70 | browsed, trending |
| 28 | Warp Zoom 4 | `store:39257` | zoom_punch_shake |  | yes | url | 66 | browsed, downloaded |
| 29 | Flash Zoom In | `store:30779594` | zoom_punch_shake | yes |  | local | 62.0 | browsed, popular_or_highlight |
| 30 | Erase Line Left | `store:63902762` | shape_luma_wipes | yes |  | local | 39.5 | browsed |
| 31 | Clockwise Wipe 02 | `store:63912086` | shape_luma_wipes | yes |  | local | 39.0 | browsed |
| 32 | Fade Dissolve | `store:15660294` | fade_dissolve | yes |  | local | 38.5 | browsed |
| 33 | Linear Wipe Left | `store:63912098` | shape_luma_wipes | yes |  | url | 38.5 | browsed |
| 34 | Erase Slide Left | `store:63912122` | shape_luma_wipes | yes |  | url | 38.0 | browsed |
| 35 | Fade Dissolve Up | `store:15734546` | fade_dissolve | yes |  | local | 38.0 | browsed |
| 36 | Fast Wipe Up | `store:63902798` | slide_push_whip | yes |  | url | 37.5 | browsed |
| 37 | Parallax Smooth Zoom In | `store:1938550` | zoom_punch_shake |  |  | local | 37.5 | browsed |
| 38 | Push Left | `store:63912154` | slide_push_whip | yes |  | local | 37.0 | browsed |
| 39 | Blur Expose | `store:31061798` | blur | yes |  | local | 36.5 | browsed |
| 40 | Flash Zoom Out | `store:30794434` | zoom_punch_shake | yes |  | local | 36.5 | browsed |

### Effects

| # | Name | Key | Group | Premium | Downloaded | Preview | Score | Hints |
|---:|---|---|---|---|---|---|---:|---|
| 1 | Mosaic | `store:45293` | shape_luma_wipes |  | yes | local | 145 | browsed, downloaded, popular_or_highlight, trending |
| 2 | Basic Blur | `store:15637982` | blur | yes |  |  | 95 | browsed, popular_or_highlight, trending |
| 3 | Corner Pin | `store:9843262` | 3d_flip_cube | yes |  | url | 95 | browsed, popular_or_highlight, trending |
| 4 | Extreme | `store:111560` | zoom_punch_shake |  |  | url | 95 | browsed, popular_or_highlight, trending |
| 5 | Fast Zoom | `store:1856918` | zoom_punch_shake | yes |  | url | 95 | browsed, popular_or_highlight, trending |
| 6 | Glitch Effect 20 | `store:1459054` | glitch_digital | yes |  | url | 95 | browsed, popular_or_highlight, trending |
| 7 | Low FPS | `store:6547634` | other | yes |  | url | 95 | browsed, popular_or_highlight, trending |
| 8 | Motion Parallax Effect Out | `store:1875298` | slide_push_whip | yes |  | url | 95 | browsed, popular_or_highlight, trending |
| 9 | Noise Texture | `store:1938998` | glitch_digital | yes |  | url | 95 | browsed, popular_or_highlight, trending |
| 10 | Slow Zoom | `store:1854066` | zoom_punch_shake | yes |  | url | 95 | browsed, popular_or_highlight, trending |
| 11 | Smart Vignette | `store:14871162` | other | yes |  |  | 95 | browsed, popular_or_highlight, trending |
| 12 | Zoom Bounce | `store:1931358` | zoom_punch_shake | yes |  | url | 95 | browsed, popular_or_highlight, trending |
| 13 | BlendMode | `store:34544702` | fade_dissolve |  | yes |  | 60 | browsed, downloaded |
| 14 | Cool Film | `store:90420` | film_burn_leak |  | yes |  | 50 | downloaded |
| 15 | 3D Camera Movement | `store:59067738` | zoom_punch_shake | yes |  | url | 35 | browsed, popular_or_highlight |
| 16 | 3D Slideshow Style 3 | `store:59459130` | 3d_flip_cube | yes |  | url | 35 | browsed, popular_or_highlight |
| 17 | 3D Slideshow Style 4 | `store:59459138` | 3d_flip_cube | yes |  | url | 35 | browsed, popular_or_highlight |
| 18 | 3D Slideshow Style 6 | `store:59459142` | 3d_flip_cube | yes |  | url | 35 | browsed, popular_or_highlight |
| 19 | Bevel Cut | `store:1874790` | zoom_punch_shake | yes |  | url | 35 | browsed, popular_or_highlight |
| 20 | Blink Block | `store:58972770` | light_flash | yes |  | url | 35 | browsed, popular_or_highlight |
| 21 | Border Line 01 | `store:15927494` | shape_luma_wipes | yes |  |  | 35 | browsed, popular_or_highlight |
| 22 | Camera Shake | `store:34591602` | zoom_punch_shake | yes |  | url | 35 | browsed, popular_or_highlight |
| 23 | Cyber Ray Scanning 02 | `store:15710678` | glitch_digital | yes |  | url | 35 | browsed, popular_or_highlight |
| 24 | Direction Blur | `store:766760` | blur | yes |  | url | 35 | browsed, popular_or_highlight |
| 25 | Earthquake Shake | `store:14870714` | zoom_punch_shake | yes |  | url | 35 | browsed, popular_or_highlight |

### Animations

| # | Name | Key | Group | Premium | Downloaded | Preview | Score | Hints |
|---:|---|---|---|---|---|---|---:|---|
| 1 | Fade In | `store:59854706` | fade_dissolve |  | yes | local | 88.5 | browsed, downloaded |
| 2 | Fade Slide1 | `store:59859006` | fade_dissolve |  | yes | local | 88.0 | browsed, downloaded |
| 3 | Fade Out | `store:59854854` | fade_dissolve | yes | yes | local | 86.5 | browsed, downloaded |
| 4 | Zoom In | `store:59859082` | zoom_punch_shake |  | yes | local | 85.0 | browsed, downloaded |
| 5 | Left Zoom | `store:59859090` | zoom_punch_shake |  | yes | local | 81.5 | browsed, downloaded |
| 6 | Clockwise Zoom1 | `store:59858994` | zoom_punch_shake |  | yes | local | 78.5 | browsed, downloaded |
| 7 | Blue and Yellow | `slug:Dynamic_Blue_and_Yellow` | text_animations |  | yes |  | 50 | downloaded |
| 8 | None | `slug:Dynamic_None` | text_animations |  | yes |  | 50 | downloaded |
| 9 | Arcane Close | `store:59879494` | slide_push_whip | yes |  | url | 39.5 | browsed |
| 10 | Spin4 | `store:59854754` | 3d_flip_cube |  |  | url | 39.5 | browsed |
| 11 | Boom | `store:59854678` | zoom_punch_shake |  |  | url | 39.0 | browsed |
| 12 | Dissolve Left | `store:59879502` | fade_dissolve | yes |  | url | 39.0 | browsed |
| 13 | Gold Sand | `store:59879510` | film_burn_leak | yes |  | url | 38.5 | browsed |
| 14 | Old TV | `store:59871690` | glitch_digital | yes |  | url | 38.0 | browsed |
| 15 | Fade Slide3 | `store:59854830` | fade_dissolve |  |  | url | 37.5 | browsed |
| 16 | Wham | `store:59854782` | zoom_punch_shake |  |  | url | 37.5 | browsed |
| 17 | Fade ZoomOut | `store:59854714` | zoom_punch_shake |  |  | url | 37.0 | browsed |
| 18 | Fade ZoomOut | `store:59854842` | zoom_punch_shake |  |  | url | 37.0 | browsed |
| 19 | Right Zoom | `store:59854794` | zoom_punch_shake |  |  | url | 36.5 | browsed |
| 20 | Bounce ZoomOut | `store:59854686` | zoom_punch_shake |  |  | url | 36.0 | browsed |
| 21 | Grow Shrink | `store:59854858` | zoom_punch_shake |  |  | url | 36.0 | browsed |
| 22 | Fade ZoomIn | `store:59859018` | zoom_punch_shake |  |  | url | 35.5 | browsed |
| 23 | Spin2 | `store:59854866` | 3d_flip_cube | yes |  | url | 35.5 | browsed |
| 24 | No Motion | `store:59854886` | slide_push_whip |  |  |  | 35.0 | browsed |
| 25 | Pulse Close | `store:59900498` | zoom_punch_shake | yes |  | url | 34.5 | browsed |

### Filters

| # | Name | Key | Group | Premium | Downloaded | Preview | Score | Hints |
|---:|---|---|---|---|---|---|---:|---|
| 1 | AI Color Enhance | `store:34527786` | color_looks | yes | yes | local | 114.5 | browsed, downloaded, popular_or_highlight |
| 2 | Sunny Days | `store:11697292` | color_looks |  | yes |  | 88.0 | browsed, downloaded |
| 3 | AI White Balance | `store:34581342` | color_looks | yes |  | url | 64.0 | browsed, popular_or_highlight |
| 4 | B&W Style | `store:9714282` | color_looks | yes |  |  | 62.0 | browsed, popular_or_highlight |
| 5 | Aura Contrast | `store:61655942` | color_looks | yes |  | url | 60.0 | browsed, popular_or_highlight |
| 6 | Bright Contrast | `store:61661338` | color_looks | yes |  | url | 59.5 | browsed, popular_or_highlight |
| 7 | Clean Exposure | `store:61655954` | color_looks | yes |  | url | 59.0 | browsed, popular_or_highlight |
| 8 | Clear Radiance | `store:61661342` | color_looks | yes |  | url | 58.5 | browsed, popular_or_highlight |
| 9 | Defined Glow | `store:61664674` | color_looks | yes |  | url | 57.0 | browsed, popular_or_highlight |
| 10 | Natural Enhance | `store:61664690` | color_looks | yes |  | url | 56.0 | browsed, popular_or_highlight |
| 11 | Optical Boost | `store:61661350` | color_looks | yes |  | url | 55.5 | browsed, popular_or_highlight |
| 12 | Studio Clarity | `store:61664702` | color_looks | yes |  | url | 55.0 | browsed, popular_or_highlight |
| 13 | Clarity Boost | `store:11705220` | color_looks | yes |  |  | 48.5 | browsed, popular_or_highlight |
| 14 | Depth Clarity | `store:59978540` | color_looks | yes |  | url | 38.5 | browsed |
| 15 | Teal Orange | `store:11685020` | color_looks |  |  |  | 37.5 | browsed |

### Overlays

| # | Name | Key | Group | Premium | Downloaded | Preview | Score | Hints |
|---:|---|---|---|---|---|---|---:|---|
| 1 | RGB Stroke | `store:1300176` | glitch_digital |  | yes | url | 60 | browsed, downloaded |
| 2 | Strong Glitch | `store:39091` | glitch_digital |  | yes |  | 50 | downloaded |
| 3 | Basic Particle Effects Overlay 03 | `store:11459590` | overlays_particles | yes |  | url | 35 | browsed, popular_or_highlight |
| 4 | Bling Bling Overlay 04 | `store:282395` | light_flash | yes |  | url | 35 | browsed, popular_or_highlight |
| 5 | Bling Bling Pack Overlay 02 | `store:282381` | light_flash | yes |  | url | 35 | browsed, popular_or_highlight |
| 6 | Comic Speedlines Overlay 01 | `store:1597452` | overlays_particles | yes |  | url | 35 | browsed, popular_or_highlight |
| 7 | Dreamlike Mist 01 16-9 | `store:31156586` | light_flash | yes |  | url | 35 | browsed, popular_or_highlight |
| 8 | Festival Particle Effect Overlay 20 | `store:9052172` | overlays_particles | yes |  | url | 35 | browsed, popular_or_highlight |
| 9 | Fireworks Night Overlay 01 | `store:30951946` | overlays_particles | yes |  | url | 35 | browsed, popular_or_highlight |
| 10 | Glitter Wedding Overlay 02 | `store:1659270` | light_flash | yes |  | url | 35 | browsed, popular_or_highlight |
| 11 | Horror Pack Overlay 4 | `store:42337` | film_burn_leak | yes |  | url | 35 | browsed, popular_or_highlight |
| 12 | Light Effects Overlay 03 | `store:15893310` | light_flash | yes |  | url | 35 | browsed, popular_or_highlight |
| 13 | Light Leak Effect Filter Vol 02 Overlay 05 | `store:1569940` | film_burn_leak | yes |  | url | 35 | browsed, popular_or_highlight |
| 14 | Love Daily Record Overlay 02 | `store:1395558` | light_flash | yes |  | url | 35 | browsed, popular_or_highlight |
| 15 | Love Particles Overlay 06 | `store:6315410` | overlays_particles | yes |  | url | 35 | browsed, popular_or_highlight |

### Titles

| # | Name | Key | Group | Premium | Downloaded | Preview | Score | Hints |
|---:|---|---|---|---|---|---|---:|---|
| 1 | Basic 1 | `store:203568` | text_animations |  | yes | url | 1096 | browsed, downloaded, recently_used, used_in_user_projects:16 |
| 2 | Art Title 26 | `store:1266226` | text_animations |  |  |  | 95 | browsed, popular_or_highlight, trending |
| 3 | Art Title 30 | `store:1291942` | text_animations |  |  |  | 95 | browsed, popular_or_highlight, trending |
| 4 | Big Titles Pack Title 08 | `store:123873` | text_animations |  |  | url | 95 | browsed, popular_or_highlight, trending |
| 5 | Game UI Set Pack Title 02 | `store:1309012` | text_animations |  |  | url | 95 | browsed, popular_or_highlight, trending |
| 6 | Memory Life Record Title 05 | `store:1346108` | text_animations |  |  | url | 95 | browsed, popular_or_highlight, trending |
| 7 | Fire Particle Pack Title 01 | `store:1569332` | text_animations |  |  | url | 70 | browsed, trending |
| 8 | Fire Particle Pack Title 07 | `store:1569356` | text_animations |  |  | url | 70 | browsed, trending |
| 9 | Fire Particle Pack Title 11 | `store:1569372` | text_animations |  |  | url | 70 | browsed, trending |
| 10 | Opener 02 | `store:125392` | text_animations |  |  | url | 70 | browsed, trending |

## Quick wins: look-alikes already in the registry

Same name and kind as an existing Jibaru effect. Each needs a visual check
against its preview (and the Filmora GUID, available once downloaded) before
an alias is added to `crates/jb-render/src/aliases.rs`.

| Name | Kind | Key | Look-alike | Premium |
|---|---|---|---|---|
| Fade In | animation | `store:59854706` | `jb.anim.fade_in` |  |
| Fade Out | animation | `store:59854854` | `jb.anim.fade_out` | yes |
| Zoom In | animation | `store:59859082` | `jb.anim.zoom_in` |  |
| Boom | animation | `store:59854678` | `jb.anim.boom` |  |
| Zoom Out | animation | `store:59859094` | `jb.anim.zoom_out` |  |
| Boom | animation | `store:59854806` | `jb.anim.boom` |  |
| Fade Out | animation | `store:59854818` | `jb.anim.fade_out` |  |
| Boom! | animation | `textanim:Boom!` | `jb.anim.boom` |  |
| Fade In | animation | `preset_anim:Fade In` | `jb.anim.fade_in` |  |
| Fade Out | animation | `preset_anim:Fade Out` | `jb.anim.fade_out` |  |
| Zoom In | animation | `preset_anim:Zoom In` | `jb.anim.zoom_in` |  |
| Zoom Out | animation | `preset_anim:Zoom Out` | `jb.anim.zoom_out` |  |
| Zoom in | animation | `textanim:Zoom in` | `jb.anim.zoom_in` |  |
| Zoom out | animation | `textanim:Zoom out` | `jb.anim.zoom_out` |  |
| Mosaic | effect | `store:45293` | `jb.pixel_mosaic` |  |
| Slow Zoom | effect | `store:1854066` | `jb.slow_zoom` | yes |
| Camera Shake | effect | `store:34591602` | `jb.handheld_shake` | yes |
| Chromatic Aberration | effect | `store:38888` | `jb.chromatic_aberration` |  |
| Radial Blur | effect | `store:38876` | `jb.radial_blur` |  |
| ColorCorrection | effect | `builtin:8AC32793-7F54-46B1-8008-86CE5FFF614D` | `jb.color.adjust` |  |
| Emboss | effect | `builtin:83BECB59-BA1F-4A4D-9CE1-B918E7F026B0` | `jb.emboss` |  |
| FilmGrain | effect | `builtin:6EEFFABB-4894-446C-8187-3D9D9922E3A9` | `jb.grano_pelicula` |  |
| Fisheye | effect | `builtin:A0DFCB73-9C3D-4D18-813E-8CC29D76D406` | `jb.fisheye` |  |
| Gaussian Blur | effect | `builtin:BCBE3824-BB66-4C29-A5DB-DFE4381FAA9F` | `jb.gaussian_blur` |  |
| Heart Mask | effect | `builtin:138113E7-6F74-4C2E-884C-F19F5B251817` | `jb.mask.heart` |  |
| Kaleidoscope | effect | `builtin:A5394086-2AE7-4DF3-B85B-0D69E4594338` | `jb.kaleidoscope` |  |
| MotionBlur | effect | `builtin:1CB976CB-FDD7-4669-A06F-D3AFB3A58456` | `jb.motion_blur` |  |
| Radial Blur | effect | `builtin:A75765C5-7A80-4044-A532-51EB2E15024E` | `jb.radial_blur` |  |
| Sketch | effect | `builtin:04E1E803-CB09-49CF-91C9-071B1BF67315` | `jb.sketch` |  |
| Teal Orange | filter | `store:11685020` | `jb.teal_orange` |  |
| Polaroid | filter | `color_preset:Polaroid` | `jb.polaroid` |  |
| Vivid | filter | `builtin:9E2DC6CF-BDC2-4368-99DD-DBB8A42A8298` | `jb.vivido` |  |
| Old Film | overlay | `store:39094` | `jb.pelicula_antigua` |  |
| Flash | transition | `store:39164` | `jb.white_flash` |  |
| Push Right | transition | `store:39200` | `jb.push_right` |  |
| Push Left | transition | `store:1650274` | `jb.push_left` |  |
| Push Up | transition | `store:1650278` | `jb.push_up` | yes |
| Push Down | transition | `store:1653462` | `jb.push_down` | yes |
| Fast Wipe Up | transition | `store:63902798` | `jb.fast_wipe_up` | yes |
| Push Left | transition | `store:63912154` | `jb.push_left` | yes |
| Push Up | transition | `store:63912162` | `jb.push_up` | yes |
| Fast Zoom | transition | `store:63902802` | `jb.fast_zoom` | yes |
| Fast Wipe Up | transition | `store:1668918` | `jb.fast_wipe_up` | yes |
| Fast Wipe Down | transition | `store:1650254` | `jb.fast_wipe_down` |  |
| Blur Dissolve | transition | `store:31011821` | `jb.blur_dissolve` |  |
| Bounce | transition | `store:59032746` | `jb.bounce_in` | yes |
| Heart | transition | `slug:Heart` | `jb.wipe_heart` |  |
| Pixelate | transition | `builtin:0FBDABAD-9BC5-42E7-9596-EC20AC763806` | `jb.pixelate` |  |

## Grouping proposal for parallel implementation

Each group shares visual mechanics, so one agent can build the common machinery
(e.g. a parametric zoom-blur pass, a mask/matte generator, a particle system)
and then fit each item's parameters to its preview. Suggested lanes:

| Lane | Groups | Items | Premium | Shared building blocks |
|---|---|---:|---:|---|
| A | zoom_punch_shake, blur | 176 | 113 | radial/zoom blur, directional blur, eased transform curves, shake noise |
| B | slide_push_whip | 126 | 94 | rigid translation with motion blur, easing presets, edge mirroring/clamping |
| C | shape_luma_wipes, fade_dissolve | 155 | 95 | procedural masks (SDF shapes, lines, clock, ink/brush noise mattes), soft edges |
| D | 3d_flip_cube | 126 | 98 | perspective quad renderer with lighting/shadow, page-curl mesh |
| E | distortion_warp, glitch_digital | 137 | 78 | UV displacement fields, block noise, channel split, scanlines |
| F | film_burn_leak, light_flash, overlays_particles | 244 | 164 | procedural burn/leak/flare generators and a GPU particle system (replaces Filmora's overlay videos) |
| G | color_looks | 165 | 29 | grade-to-LUT fitting from preview frames, look presets |
| H | text_animations | 265 | 19 | per-glyph animation engine, title template layout |

Deferred: `ai_generative` (needs segmentation / generative models),
`audio_reactive` (needs the audio analysis pipeline), `other` (triage first).

### zoom_punch_shake

Scale/translate the frame over time (push-in, punch, zoom-through, bounce, shake), usually with radial or directional motion blur and a cut at peak speed.

137 items (transition 56, effect 27, animation 54); 96 premium; 2 implemented; 21 measured.
Implemented: Fast Zoom (`jb.fast_zoom`), Cross Zoom (`jb.cross_zoom`).
Next up: Warp Zoom 4 [P], Zoom, Lens Zoom, Round Zoom Out, Slow Zoom [P], Extreme, Fast Zoom [P], Slow Zoom [P], Zoom Bounce [P], Zoom In.

### slide_push_whip

Rigid translation of A and/or B (push, slide, whip pan, drag, fly) with directional motion blur; easing curves differ per item.

126 items (transition 45, effect 6, animation 75); 94 premium; 3 implemented; 7 measured.
Implemented: Fast Wipe Left (`jb.wipe_left`), Slide (`jb.pan_slide`), Fast Wipe Right (`jb.fast_wipe_right`).
Next up: Motion Parallax Effect Out [P], Push Right, Push Left, Push Up [P], Push Down [P], Arcane Close [P], Fast Wipe Up [P], Push Left [P], Push Up [P], Drag Down Right [P].

### shape_luma_wipes

A reveals B through a moving mask: lines, shapes, clock, iris, ink/brush or paper-tear luma mattes, mosaics, shatter/slice.

107 items (transition 46, effect 29, animation 32); 71 premium; 0 implemented; 4 measured.
Next up: Mosaic, Evaporate 2, Erase, Shape Transition 60, Erase Line Left [P], Clockwise Wipe 02 [P], Linear Wipe Left [P], Erase Slide Left [P], Glass Eraser Transition [P], Modern Transparent Transition 10 [P].

### 3d_flip_cube

Perspective transforms of the frame: cube, flip, page curl, door, roll, cards, swing, 3D slideshow planes.

126 items (transition 33, effect 24, animation 69); 98 premium; 0 implemented; 9 measured.
Next up: Corner Pin [P], Page Curl, Spin4, Spin2 [P], Paper Page Curl, Swing Left [P], 3D Slideshow Style 3 [P], 3D Slideshow Style 4 [P], 3D Slideshow Style 6 [P], Parallel Photobook [P].

### distortion_warp

Per-pixel UV displacement: ripple, wave, swirl/whirl, liquid, kaleidoscope, fisheye, mirror/tunnel tiling.

52 items (transition 11, effect 31, animation 10); 20 premium; 0 implemented; 0 measured.
Next up: Morph, Tropical Waves Transition 3 [P], Shock Wave [P], Shockwave 3 [P], Tunnel Effect 1 [P], Ripple Smash [P], Twirl Ripple Right [P], Twirl Ripple Left [P], Glowing Swirling 02 [P], Ripple Surge [P].

### glitch_digital

Block displacement, RGB channel offsets, scanlines, noise, pixelation, VHS/TV artifacts, digital scan bars.

85 items (transition 15, effect 28, overlay 8, animation 34); 58 premium; 0 implemented; 3 measured.
Next up: Glitch Intro Transition 09 [P], Glitch Effect 20 [P], Noise Texture [P], Glitch Dissolve [P], Cinematic Digital Slideshow Transition 07, RGB Stroke, Strong Glitch, Old TV [P], Cyber Ray Scanning 02 [P], Flicker Glitch [P].

### film_burn_leak

Film looks with overlay imagery: burns, light leaks, sprockets, scratches, grain, projector flicker; needs procedural burn/leak generators.

53 items (transition 9, effect 9, overlay 21, animation 14); 37 premium; 1 implemented; 2 measured.
Implemented: Film Burn Transition 02 (`jb.film_burn_02`).
Next up: Cool Film, Gold Sand [P], Film Roll Transition 01 [P], Film Erase [P], Film Rolling [P], Film Slides 2 [P], Filming Shake [P], Horror Pack Overlay 4 [P], Light Leak Effect Filter Vol 02 Overlay 05 [P], Retro DVD 06 [P].

### light_flash

Additive light: white/colored flashes, flares, glows, light sweeps, bokeh, neon edges, strobe/flicker.

117 items (transition 20, effect 34, overlay 24, animation 39); 75 premium; 0 implemented; 5 measured.
Next up: Flash, Cinematic Transition 02 [P], Cinematic Transition 04, Glow Ink Up [P], Sparkle Motion Transition 07 [P], Sparkle Motion Transition 11 [P], Sparkle Motion Transition 15 [P], Blink Block [P], Edge Glow 01 [P], Edge Glow 04 [P].

### overlays_particles

Composited elements: particles, sparkles, smoke, snow, fire, confetti, seasonal stickers; needs procedural particle systems.

74 items (transition 9, effect 14, overlay 36, animation 15); 52 premium; 0 implemented; 1 measured.
Next up: My_Daily_Vlog_Vol2_Pack_Transition_01, Train Left [P], Explosion Fire [P], Realistic Firefly 01 [P], Basic Particle Effects Overlay 03 [P], Comic Speedlines Overlay 01 [P], Festival Particle Effect Overlay 20 [P], Fireworks Night Overlay 01 [P], Love Particles Overlay 06 [P], Movie Atmosphere Overlay 05 [P].

### blur

Gaussian/directional/radial blur effects and blur-based dissolves.

39 items (transition 8, effect 26, animation 5); 17 premium; 1 implemented; 2 measured.
Implemented: Blur Dissolve (`jb.blur_dissolve`).
Next up: Halo Blur [P], Basic Blur [P], Blur Dissolve 2 [P], Blur Dissolve 1, Blur Expose [P], Direction Blur [P], Surface Blur [P], Hexagon Dissolve Blur [P], Out Blur out [P], Blur Dissolve.

### fade_dissolve

Opacity blends: dissolves, dips to color, blend-mode mixes.

48 items (transition 26, effect 3, animation 19); 24 premium; 2 implemented; 10 measured.
Implemented: Fade (`jb.fade_black`), Dissolve (`jb.dissolve`).
Next up: Dissolve 02 [P], Fade In, Fade Slide1, Fade Out [P], Dissolve White [P], BlendMode, Dissolve Left [P], Fade Dissolve [P], Fade Dissolve Up [P], Fade Slide3.

### color_looks

Color grades (Filmora Filters, built-in looks, color presets, LUTs): per-pixel color transforms, implementable as generated 3D LUTs or shader grades.

165 items (filter 146, lut 19); 29 premium; 0 implemented; 1 measured.
Next up: AI Color Enhance [P], Sunny Days, AI White Balance [P], B&W Style [P], Aura Contrast [P], Bright Contrast [P], Clean Exposure [P], Clear Radiance [P], Defined Glow [P], Natural Enhance [P].

### text_animations

Title templates and text animations (per-character/line motion, typewriter, caption styles).

265 items (effect 7, animation 168, title 90); 19 premium; 1 implemented; 0 measured.
Implemented: Crosszoom (`jb.cross_zoom`).
Next up: Basic 1, Art Title 26, Art Title 30, Big Titles Pack Title 08, Game UI Set Pack Title 02, Memory Life Record Title 05, Fire Particle Pack Title 01, Fire Particle Pack Title 07, Fire Particle Pack Title 11, Opener 02.

### audio_reactive

Audio visualizers and audio-driven effects (spectrum, waveform, beat-driven geometry).

44 items (effect 44); 0 premium; 0 implemented; 0 measured.
Next up: Audio Visual Geometry, Audio Visual Halo Geometry, Audio Visual Halo Spectrum, Audio Visualization 01, Audio Visualization 02, Audio Visualization 03, Audio Visualization 04, Audio Visualization 05, Audio Visualization 06, Audio Visualization 07.

### ai_generative

Needs segmentation/AI (body effects, AI relight, AI transitions, AI portrait borders); defer or replace with non-AI approximations.

56 items (transition 3, effect 53); 17 premium; 0 implemented; 0 measured.
Next up: None, Atmosphere Exposure, Translucent Water Light, Morning Sunshine, Soft Facial Light, Explosion Background [P], Golden beam [P], Haunt Effect 04 [P], Neon Rush [P], Ring Ripple [P].

### other

Unclassified by name; triage by preview before assignment.

85 items (transition 7, effect 70, animation 8); 11 premium; 1 implemented; 0 measured.
Implemented: Auto Enhance (`jb.auto_enhance`).
Next up: Low FPS [P], Smart Vignette [P], Expand Out [P], Cut It Off [P], Electronic Device Transition 01 [P], Floral Transition 4, My Daily Vlog Vol2 Pack Transition 02, The ending black, Aibao 56, Baltan.

## Preview measurements

Every item with a local `previewVideo.mp4` (downloaded, bundled, or previewed in
the store) was decoded with ffmpeg at 128x72 and measured per frame: mean
luma, saturation, Laplacian sharpness, mean absolute frame difference
(motion), global translation by phase correlation and zoom by a scale search
against the previous frame. For transitions each frame is also fitted as a
blend of the first (clip A) and last (clip B) frame; the remaining residual,
the luma above that blend (flash) or below it (dip), and three overlay scores
(rise of warm-hue pixels, of isolated bright spots, of local near-white areas
over the clean ends of the preview) flag overlay imagery that needs a
procedural recreation. The full per-frame series live in the builder's scratch
directory; the JSON keeps a compact summary per item (`measure`: duration,
fps, motion peak time, active span, cumulative zoom/translation and an
11-point curve of luma, motion, blend alpha, residual and relative
sharpness). These are heuristics for orienting an implementation, not ground
truth: source footage in the previews also moves, so small values are noise.

| Name | Kind | Dur (s) | Motion peak (s) | Net zoom | Net shift (x, y) | Overlay imagery | Flags |
|---|---|---:|---:|---:|---|---|---|
| Clockwise Zoom1 | animation | 1.0 | 0.36 | 1.853 | 0.133, 0.236 |  | zoom_in |
| Fade In | animation | 1.0 | 0.2 | 1.0 | 0.0, 0.0 |  | motion_blur_or_defocus |
| Fade Out | animation | 1.0 | 0.48 | 1.0 | -0.062, 0.0 |  | luma_peak_mid, motion_blur_or_defocus |
| Fade Slide1 | animation | 1.0 | 0.08 | 0.745 | 0.0, -0.278 |  | luma_peak_mid, zoom_out, motion_blur_or_defocus, flicker |
| Left Zoom | animation | 1.0 | 0.4 | 1.638 | -0.641, 0.0 |  | translation_horizontal, zoom_in, motion_blur_or_defocus |
| Zoom In | animation | 1.0 | 0.2 | 3.293 | -0.102, 0.139 |  | zoom_in, motion_blur_or_defocus |
| Mosaic | effect | 11.72 | 5.0 | 1.0 | 0.305, 0.153 |  | luma_peak_mid, motion_blur_or_defocus, flicker |
| AI Color Enhance | filter | 2.007 | 0.04 | 1.0 | 0.0, 0.0 |  |  |
| 3D Card | transition | 3.0 | 2.04 | 1.268 | -0.555, 0.097 |  | luma_peak_mid, translation_horizontal, zoom_in |
| 3D Slideshow Style 2 | transition | 3.0 | 0.56 | 0.946 | -0.078, 0.111 | bright_spots (sparkles / particles), local_white (flare / light sweep) | luma_peak_mid, motion_blur_or_defocus |
| Basic Zoom In | transition | 3.0 | 1.52 | 1.714 | 0.055, -0.014 |  | zoom_in |
| Basic Zoom In | transition | 2.0 | 1.96 | 1.697 | 0.055, -0.014 |  | luma_peak_mid, zoom_in, motion_blur_or_defocus, flicker |
| Basic Zoom Out | transition | 2.0 | 1.0 | 0.529 | 0.031, -0.083 |  | zoom_out, motion_blur_or_defocus |
| Blue Cyber Pulse | transition | 3.0 | 1.4 | 1.0 | 0.0, 0.0 |  |  |
| Blur Dissolve | transition | 2.007 | 0.84 | 1.0 | 0.0, 0.0 |  | plain_blend |
| Blur Expose | transition | 2.007 | 1.0 | 1.0 | 0.0, 0.042 |  | flash_bright, luma_peak_mid, motion_blur_or_defocus, flicker |
| Box Zoom In | transition | 3.0 | 1.56 | 3.621 | -0.039, 0.014 | local_white (flare / light sweep) | flash_bright, zoom_in |
| Cinematic Transition 02 | transition | 3.006 | 2.48 | 1.18 | 0.0, 0.0 | warm_overlay (film burn / light leak) | dip_dark, zoom_in, motion_blur_or_defocus |
| Cinematic Transition 04 | transition | 3.006 | 2.48 | 0.722 | 0.0, 0.0 |  | flash_bright, dip_dark, zoom_out, motion_blur_or_defocus |
| Clockwise Wipe 02 | transition | 3.0 | 1.64 | 1.0 | 0.0, 0.0 |  |  |
| Clockwise Wipe 02 | transition | 2.007 | 1.12 | 1.0 | 0.0, 0.0 |  |  |
| Cross Zoom | transition | 3.006 | 1.48 | 1.16 | 0.0, 0.0 |  | motion_blur_or_defocus |
| Dissolve | transition | 3.0 | 2.48 | 1.0 | 0.0, 0.0 |  | plain_blend |
| Erase Line Left | transition | 3.0 | 0.56 | 1.0 | 0.0, 0.0 |  |  |
| Fade | transition | 3.006 | 1.8 | 1.18 | 0.0, 0.0 |  | dip_dark, luma_peak_mid, zoom_in, motion_blur_or_defocus |
| Fade Dissolve | transition | 1.96 | 1.16 | 1.0 | 0.0, 0.0 |  |  |
| Fade Dissolve | transition | 1.96 | 1.16 | 1.0 | 0.0, 0.0 |  |  |
| Fade Dissolve Left | transition | 1.96 | 1.16 | 1.0 | 0.0, 0.0 |  |  |
| Fade Dissolve Up | transition | 1.96 | 1.16 | 1.0 | 0.0, 0.0 |  |  |
| Fast Wipe Left | transition | 3.006 | 1.48 | 0.85 | -0.273, 0.0 |  | zoom_out, motion_blur_or_defocus, flicker |
| Fast Wipe Right | transition | 3.006 | 1.48 | 1.0 | 0.312, 0.056 |  | motion_blur_or_defocus, flicker |
| Fast Zoom | transition | 3.006 | 1.48 | 0.855 | 0.0, 0.014 |  | zoom_out |
| Film Burn Light 04 | transition | 3.006 | 1.32 | 1.615 | -0.18, 0.042 | warm_overlay (film burn / light leak) | flash_bright, luma_peak_mid, zoom_in, motion_blur_or_defocus, flicker |
| Film Burn Transition 02 | transition | 3.0 | 1.48 | 0.716 | -0.008, 0.0 |  | flash_bright, dip_dark, luma_peak_mid, zoom_out, motion_blur_or_defocus, flicker |
| Flash | transition | 3.006 | 1.0 | 1.0 | 0.0, 0.0 |  | flash_bright, luma_peak_mid, motion_blur_or_defocus |
| Flash Zoom In | transition | 2.0 | 0.8 | 0.673 | -0.359, -0.347 |  | flash_bright, zoom_out, motion_blur_or_defocus |
| Flash Zoom Out | transition | 2.0 | 1.96 | 1.091 | -0.258, -0.111 |  | flash_bright, luma_peak_mid, motion_blur_or_defocus, flicker |
| Flip Roll Left | transition | 3.006 | 1.48 | 1.0 | -3.031, -0.042 |  | translation_horizontal, motion_blur_or_defocus |
| Glitch Intro Transition 09 | transition | 3.006 | 1.44 | 1.0 | 0.0, 0.0 | bright_spots (sparkles / particles) | flash_bright, luma_peak_mid, motion_blur_or_defocus, flicker |
| Glow Ink Up | transition | 5.0 | 2.36 | 0.98 | 0.0, 0.0 |  |  |
| Left Eye Zoom | transition | 3.0 | 1.56 | 0.927 | 0.172, 0.514 |  | dip_dark, luma_peak_mid, translation_vertical, motion_blur_or_defocus, flicker |
| Lens Zoom | transition | 3.006 | 1.52 | 1.472 | 0.0, 0.056 |  | zoom_in, motion_blur_or_defocus |
| Lightning Zoom | transition | 2.007 | 0.92 | 3.388 | 1.234, -0.028 |  | flash_bright, dip_dark, luma_peak_mid, translation_horizontal, zoom_in, motion_blur_or_defocus, flicker |
| Luminous Page Turning | transition | 2.0 | 1.96 | 0.703 | 0.25, -0.042 |  | flash_bright, luma_peak_mid, zoom_out, motion_blur_or_defocus, flicker |
| Modern Seamless 1 | transition | 2.0 | 0.84 | 1.737 | -0.023, 0.653 | warm_overlay (film burn / light leak), local_white (flare / light sweep) | flash_bright, translation_vertical, zoom_in, motion_blur_or_defocus |
| Page | transition | 2.007 | 1.04 | 1.0 | 0.0, 0.0 |  |  |
| Page Flip SFX 01 | transition | 2.007 | 1.04 | 1.0 | 0.0, 0.0 |  |  |
| Parallax Smooth Zoom In | transition | 3.006 | 1.72 | 1.869 | -0.164, -0.139 |  | flash_bright, luma_peak_mid, zoom_in, motion_blur_or_defocus |
| Photo Freeze | transition | 3.006 | 0.6 | 1.414 | -0.312, -0.333 | bright_spots (sparkles / particles) | flash_bright, luma_peak_mid, zoom_in, motion_blur_or_defocus, flicker |
| Push Left | transition | 3.006 | 1.56 | 0.739 | -1.008, 0.0 |  | translation_horizontal, zoom_out |
| Push Left | transition | 3.0 | 1.6 | 0.763 | -1.0, 0.0 |  | translation_horizontal, zoom_out |
| Push Up | transition | 3.006 | 1.32 | 1.033 | 0.0, -0.958 | bright_spots (sparkles / particles) | translation_vertical |
| Seamless 01 | transition | 2.007 | 1.08 | 1.715 | 0.0, 0.0 |  | zoom_in |
| Simple Sliding | transition | 2.0 | 0.84 | 1.662 | -0.016, -0.958 |  | translation_vertical, zoom_in |
| Slide | transition | 3.006 | 1.48 | 1.0 | 0.164, -0.125 |  |  |
| Slow Zoom | transition | 3.006 | 1.48 | 1.126 | 0.0, 0.0 |  |  |
| Space Rotation Left | transition | 2.0 | 1.96 | 1.215 | -0.414, -0.014 |  | flash_bright, luma_peak_mid, zoom_in, motion_blur_or_defocus, flicker |
| Sparkle Motion Transition 15 | transition | 3.006 | 1.44 | 0.919 | 0.586, -0.097 | bright_spots (sparkles / particles) | flash_bright, luma_peak_mid, translation_horizontal, motion_blur_or_defocus |
| Swap Left | transition | 3.006 | 2.48 | 1.112 | -0.641, 0.0 | bright_spots (sparkles / particles) | dip_dark, luma_peak_mid, translation_horizontal |
| Warp Zoom 4 | transition | 3.0 | 0.92 | 0.083 | -0.047, -0.111 |  | flash_bright, zoom_out, motion_blur_or_defocus, flicker |
| Warp Zoom 6 | transition | 3.006 | 0.88 | 0.092 | -0.047, -0.208 |  | flash_bright, zoom_out, motion_blur_or_defocus, flicker |
| Yellow Matrix Scan | transition | 3.0 | 1.8 | 1.0 | 0.0, 0.0 |  |  |
| Zoom | transition | 3.006 | 0.64 | 10.669 | -0.047, 0.014 |  | zoom_in |
| Zoom Ghost | transition | 2.0 | 1.04 | 0.665 | 0.008, -0.264 |  | flash_bright, luma_peak_mid, zoom_out, motion_blur_or_defocus, flicker |
| Zoom Whirl 1 | transition | 2.0 | 1.96 | 0.85 | 0.039, -0.167 |  | flash_bright, luma_peak_mid, zoom_out, motion_blur_or_defocus, flicker |

## Tracking progress

```sh
jb fx catalog-status                    # per kind and implementation group, top 3 missing each
jb fx catalog-status --by category      # per Filmora store category
jb fx catalog-status --premium --kind transition --top 10
jb --json fx catalog-status             # machine-readable
```

An item counts as implemented when its `jb` id is registered or when one of
its Filmora GUIDs resolves through a `filmora:<GUID>` alias in
`crates/jb-render/src/aliases.rs`, so adding an alias updates the status
without regenerating the catalog. Mappings recorded in the catalog that no
longer resolve are reported as stale.

## Regenerating

The catalog was produced by a throw-away script (kept outside the repository
so no Filmora material ever lands here) that reads the sources listed above
read-only, joins store rows, downloaded `resinfo.json`/`*.conf`/`filter.xml`
metadata and built-in registries by store id, slug and GUID, maps GUIDs to
`jb.*` ids through the live alias table, classifies items into groups by
name/tags/category keywords, and attaches the preview measurements. Field
meanings are in the JSON's `fields` object.
