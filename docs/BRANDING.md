# Branding and typeface provenance

Project: **ReGTA CTW**. Full name: **ReGTA CTW PSP Recomp**. Credit: **by Benji Paname**.

The website and GitHub presentation use English as their primary language. The website also offers French.

## Requested final typography

- **GTA:** Pricedown, in the heavy/Black style associated with the series. [Raymond Larabie's history](https://typodermicfonts.com/between-the-lines/stories/pricedown/) confirms its use by Rockstar. [Publisher page](https://typodermicfonts.com/pricedown/) describes desktop licenses for logos and static graphics.
- **CHINATOWN WARS / CTW:** the font identified for the title is **Bruce Mikita**, not Chinese Rocks. [Historical identification](https://m.thegtaplace.com/downloads/f2974-chinatown-wars-font-bruce-mikita); [Harold Lohner's current publisher page](https://www.haroldsfonts.com/font/bruce-mikita) offers Regular and Solid variants.

## Implemented artwork — 24 October 2026

The project features two official SVG logo versions generated with outlined glyphs from the authentic typefaces:

1. **Version 1: Horizontal / Inline (`docs/assets/logo-ctw.svg`, 960 × 260)**
   - Single-row presentation: `ReGTA : CTW` with a stylized Chinatown gold diamond separator.
   - `ReGTA`: Large, integrated Pricedown Black typography. The "**Re**" uses a capital **R** and lowercase **e**, rendered in vibrant Recomp Red (`#FF3B30` → `#E52521` → `#7F1D1D`) with bold black contour and drop shadow; seamlessly continuing into "**GTA**" in classic pure white/silver (`#FFFFFF` → `#CBD5E1`).
   - `CTW`: Authentic Chinatown Wars red and yellow/gold dual-tone styling. Heavy black drop shadow, golden imperial outline, deep lacquer red face, and calligraphic golden accents.
   - Ideal for horizontal navigation bars, website headers, and repository banners.

2. **Version 2: Vertical / Stacked (`docs/assets/logo.svg`, 900 × 480)**
   - Stacked cover presentation: **`ReGTA` on top, `CTW` located directly underneath ("en bas du ReGTA")**.
   - `ReGTA`: Prominently centered on the top line in full Pricedown Black with capital **R** and lowercase **e**, red-and-white two-tone wordmark.
   - `CTW`: Centered directly below `ReGTA` with authentic Chinatown Wars lacquer red face and imperial golden yellow contour and highlights.
   - Subtitle: `PSP RECOMPILATION` letter-spaced in clean modern silver.
   - Ideal for the GitHub Pages showcase hero section, game box presentation, and social media cards.

Both SVGs were rasterized locally and visually inspected at their native dimensions using Microsoft Edge headless rendering. The fonts are not embedded as webfonts; each line is fixed vector artwork.

## Terms and publication status

The supplied Pricedown archive includes **Typodermic Desktop EULA v260817**, effective 17 August 2026. Section 2.2 permits fixed logos and static artwork, including commercial materials, without redistributing the font or providing a reusable font capability. The desktop terms are not a GPL license for the font software.

The supplied Bruce Mikita archive contains fonts and a wfonts.com download note, but no explicit license. Its font metadata identifies Harold Lohner and copyright 2000, with no license text or license URL. **Permission for public/commercial use of this supplied historical version has not been established.** Obtain the applicable terms or permission from the publisher before public distribution; do not interpret the third-party download note as a license. The current publisher's offer does not by itself establish the terms of this old copy.

The files remain private under ignored `target/private-fonts/`. No font software or license PDF is committed. No purchase or publication has been performed. This record documents the evidence, not a legal clearance.

## Reproduce the artwork

Install `fonttools` in a local Python environment, then run the generator:

```powershell
python tools/research/generate_brand_logos.py
```

Outputs: `docs/assets/logo.svg` (900 × 480) and `docs/assets/logo-ctw.svg` (960 × 260). The generator preserves glyph proportions and generates fixed paths. No font dependency is needed to view the GTA / CTW lettering.

## Input fingerprints

- `Pricedown Bl.otf` — SHA-256 `19f8cd90ce76992c565debe80d167f58e6e1e79a6e0b86f24bd9dce12052b256`
- `BRUCM___.TTF` — SHA-256 `6c6700ee98153ca00d58284b294f196aed291141ba1dad4915efb559cbf602f2`
