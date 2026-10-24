# Presentation review — 25 October 2026

Scope: latest logos, thumbnail, creator kit, generator, completed previous film, and the supplied native gameplay recording. This is a media review, not a certification of complete gameplay fidelity.

## Findings

- Previous logo: heavy gradients, additional interior strokes and a decorative colon weakened the cover-inspired colour hierarchy. The e was nominally lowercase but the same height as its neighbours.
- Previous 1:53 video: one continuous gameplay segment with long, dense feature dashboards. Central branding and repeated panels obscured the demonstration. The soundtrack had no breathing space allocated for its call to action.
- Previous voice/copy: unsupported zero-latency, clean-room/from-scratch, completed-feature and guaranteed-performance claims; disparaging emulator comparison. These were removed rather than replaced with other unverified promises.
- Previous generator could silently substitute PPSSPP recordings if native footage was absent. The new pipeline requires the native recording and fails if unavailable.
- Previous voice concatenation did not include the scene padding used by overlays. New scenes add explicit 350 ms speech lead-in and pad to their own durations, with subtitles derived from word timings.

## New direction

A 78.51-second English first-look film assembled from scratch: 13 selected shots from Benjamin's supplied native prototype recording, 7 concise sections, new narration, original restrained music, readable lower-thirds and a nine-second end card. No AI-generated gameplay or fabricated comparison is used. Gameplay runs at its recorded speed; a 60 fps export is not a claim that every game frame meets a performance target.

The final sequence explains optional support and points viewers to Tipeee / PayPal. It makes no purchase, early-access or delivery promise. English is primary; subtitles are supplied separately for YouTube accessibility. No upload or publication is performed by this editing task.

The four logo variants use Pricedown Black + Bruce Mikita Two, a visible lowercase e and a flat red/yellow cover-inspired palette. Public wording is now ReGTA CTW PC Recomp; technical PSP source references remain intact.

## Source and traceability

- Native source: `outputs/CTW-Rust/video/custom_gameplay.mp4` (2560 × 1440, 60 fps, 4:04.34).
- New film: `outputs/CTW-Rust/video/launch-film/ReGTA_CTW_First_Look_EN.mp4`.
- Exact selects, narration and timings: `tools/video/launch_film.json` and generated `launch-film/timeline.json`.
- Old finished film is preserved locally for comparison. Archived documents and third-party reference repository names are not rewritten to match a new brand.
- A current Bruce Mikita license was not supplied; see BRANDING.md. This edit does not add a rights clearance.
