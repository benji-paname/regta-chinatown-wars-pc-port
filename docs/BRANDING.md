# Branding and typeface provenance

Project: **ReGTA CTW**. Full name: **ReGTA CTW PSP Recomp**. Credit: **by Benji Paname**.

The website and GitHub presentation use English as their primary language. The website also offers French.

## Requested final typography

- **GTA:** Pricedown, in the heavy/Black style associated with the series. [Raymond Larabie's history](https://typodermicfonts.com/between-the-lines/stories/pricedown/) confirms its use by Rockstar. [Publisher page](https://typodermicfonts.com/pricedown/) describes desktop licenses for logos and static graphics. Exact cover lettering may include custom spacing or modifications beyond typing the letters.
- **CHINATOWN WARS / CTW:** the font identified for the title is **Bruce Mikita**, not Chinese Rocks. [Historical identification](https://m.thegtaplace.com/downloads/f2974-chinatown-wars-font-bruce-mikita); [Harold Lohner's current publisher page](https://www.haroldsfonts.com/font/bruce-mikita) offers Regular and Solid variants. Confirm the correct variant against the cover when creating the artwork.

## Implemented artwork — 24 October 2026

The main and compact SVGs now contain actual outlined glyphs from the font files provided by Benjamin:

- `Pricedown Bl.otf`: **Pricedown Black**, version 5.200, Typodermic Fonts. GTA lettering.
- `BRUCM___.TTF`: **Bruce Mikita Regular**, v1.0 (Fontographer 4.1.3, 23 July 2000), Harold Lohner. CHINATOWN WARS and CTW lettering, with the font's inset highlight.
- The supplied Bruce Mikita Two / BRUCMT__ variants were inspected but not selected. Regular retains the inset detail.

These are new project wordmarks built with the identified typefaces, not copies of the original Rockstar logo composition. Exact cover spacing and any custom alterations are not claimed. The red Re prefix and PSP RECOMP subtitle use ordinary sans-serif text.

Both SVGs were rasterized locally and visually inspected at their native dimensions. The fonts are not embedded as webfonts. Each line is fixed vector artwork, not a reusable glyph library.

## Terms and publication status

The supplied Pricedown archive includes **Typodermic Desktop EULA v260817**, effective 17 August 2026. Section 2.2 permits fixed logos and static artwork, including commercial materials, without redistributing the font or providing a reusable font capability. The desktop terms are not a GPL license for the font software.

The supplied Bruce Mikita archive contains fonts and a wfonts.com download note, but no explicit license. Its font metadata identifies Harold Lohner and copyright 2000, with no license text or license URL. **Permission for public/commercial use of this supplied historical version has not been established.** Obtain the applicable terms or permission from the publisher before public distribution; do not interpret the third-party download note as a license. The current publisher's offer does not by itself establish the terms of this old copy.

The files remain private under ignored `target/private-fonts/`. No font software or license PDF is committed. No purchase or publication has been performed. This record documents the evidence, not a legal clearance.

## Reproduce the artwork

Install `fonttools` in a local Python environment, then supply your own font paths:

```powershell
python tools/research/generate_brand_logos.py --pricedown "PATH/Pricedown Bl.otf" --bruce-mikita "PATH/BRUCM___.TTF"
```

Outputs: `docs/assets/logo.svg` (900 × 480) and `docs/assets/logo-ctw.svg` (900 × 260). The generator preserves glyph proportions and generates fixed paths. No font dependency is needed to view the GTA / CTW lettering.

The old `generate_authentic_logo.py` and `generate_compact_logo.py` scripts are preserved as historical drafts but no longer overwrite public logos when run directly.

## Input fingerprints

- `Pricedown Bl.otf` — SHA-256 `19f8cd90ce76992c565debe80d167f58e6e1e79a6e0b86f24bd9dce12052b256`
- `BRUCM___.TTF` — SHA-256 `6c6700ee98153ca00d58284b294f196aed291141ba1dad4915efb559cbf602f2`
