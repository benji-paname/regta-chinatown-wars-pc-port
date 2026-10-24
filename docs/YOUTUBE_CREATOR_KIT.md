# ReGTA CTW PC Recomp — YouTube first-look kit

New production from scratch, 25 October 2026. English primary. No upload has been performed.

## Deliverables

- [Film — 1080p, 60 fps, 1:18.51](../outputs/CTW-Rust/video/launch-film/ReGTA_CTW_First_Look_EN.mp4)
- [Thumbnail — 1280 × 720](../outputs/CTW-Rust/video/launch-film/thumbnail.png)
- [English subtitles — upload alongside the film](../outputs/CTW-Rust/video/launch-film/ReGTA_CTW_First_Look_EN.srt)
- [YouTube description](../outputs/CTW-Rust/video/launch-film/youtube-description-en.txt)
- [Narration](../outputs/CTW-Rust/video/launch-film/narration-en.txt)

These are local output links; video, narration audio and source recording are excluded from Git. The thumbnail and logo graphics are tracked. The older devlog and its obsolete production files were deleted at the project owner's request on 25 October 2026.

## Title

**GTA Chinatown Wars on Native PC? First Look at ReGTA CTW**

Alternative: **ReGTA CTW PC Recomp — Building a Native PC Future for Chinatown Wars**

Use the question/first-look framing rather than announcing a completed or downloadable game. No click-through rate is guaranteed.

## Ready-to-copy description

```text
Chinatown Wars has a native PC future. Here is an early look at ReGTA CTW PC Recomp, an independent Rust project by Benji Paname, built around the PSP version.

This video shows the current prototype, using real footage recorded from the native application. It is a work in progress, not a finished port. The full campaign, save compatibility and closer PSP gameplay fidelity still need work. There is no public playable download or release date.

Help the project move forward:
Tipeee: https://fr.tipeee.com/benji-paname/
PayPal: https://paypal.me/BenjiLsg

Follow the work, share your feedback, or star the repository:
Website: https://benji-paname.github.io/regta-chinatown-wars-pc-port/
GitHub: https://github.com/benji-paname/regta-chinatown-wars-pc-port

Support is voluntary. It supports ongoing research and development; it does not purchase the game, game assets, early access or a guaranteed release.

CHAPTERS
0:00 First Look
0:14 Real Prototype Footage
0:25 Built In Rust
0:34 The Vision
0:45 Work In Progress
0:58 Support The Development
1:09 By Benji Paname

ReGTA CTW is an independent fan research project, not affiliated with or endorsed by Rockstar Games or Take-Two Interactive. Game resources are not included with the source repository; a local build requires your own supported PSP game image.

Production: native prototype footage supplied by Benji Paname, synthetic English narration, original synthesized music. No generated gameplay. Export: 1080p60; this is an export specification, not a benchmark or guaranteed game framerate.

#ChinatownWars #GTA #PCGaming #Rust #Devlog
```

## Production design

See [the media review](PROMO_REVIEW.md). The new film uses only the supplied native gameplay recording: no emulator substitution, no AI-generated gameplay, no speed manipulation, and no performance comparison. Short lower-thirds preserve the central action and minimap. The final nine-second card presents the donation links and by Benji Paname credit.

Narration is synthetic English (Andrew). Subtitles use word timing from that same narration. The score is newly synthesized for this edit; it does not reuse game radio tracks. Visual changes are editorial overlays and cuts, not improvements to the game renderer.

The project wordmark is delivered horizontally and vertically, each with transparent and black-background SVG/PNG versions. See [branding provenance](BRANDING.md), including the remaining Bruce Mikita license documentation gap before public use.

## Reproduce locally

Requirements: Python, Pillow, edge-tts, imageio-ffmpeg, fontTools for logos; private fonts supplied by the project owner. Native footage must exist at `outputs/CTW-Rust/video/custom_gameplay.mp4`. The script fails rather than using footage from another game/runtime.

```powershell
python tools/research/generate_brand_logos.py
# Rasterize SVG logo assets with Sharp before composing video graphics.
python tools/video/build_launch_film.py voice
python tools/video/build_launch_film.py assets
python tools/video/build_launch_film.py render
python tools/video/build_launch_film.py verify
```

`voice` uses a network speech service. All video editing is local. Scene text and source timecodes are in `tools/video/launch_film.json`; the generated timeline records exact final starts/durations. If changing narration, invalidate its cached word/audio output before regenerating it. No source recording is uploaded by this pipeline.
