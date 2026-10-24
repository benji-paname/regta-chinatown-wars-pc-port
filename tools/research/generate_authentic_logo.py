# SPDX-License-Identifier: GPL-3.0-only
"""
Generates the draft, simple, and modern ReGTA CTW logo draft:
- 'GTA' in draft Pricedown Grand Theft Auto font typography with thick black contour and drop shadow.
- 'CHINATOWN WARS' in draft unverified angular lettering (not the cover font) in vibrant golden yellow.
- Sleek modern 'PC RECOMP' minimalist subtitle.
- Pure transparent/obsidian modern aesthetic, no clutter.
"""

def generate_logo_svg():
    svg = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 900 360" width="100%" height="100%">
  <defs>
    <!-- Filters for draft GTA & Chinatown Wars outlines and shadows -->
    <filter id="gtaShadow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="7" dy="9" stdDeviation="0" flood-color="#000000" flood-opacity="1.0" />
      <feDropShadow dx="0" dy="0" stdDeviation="12" flood-color="rgba(0,0,0,0.8)" />
    </filter>

    <filter id="ctwShadow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="5" dy="6" stdDeviation="0" flood-color="#000000" flood-opacity="1.0" />
      <feDropShadow dx="0" dy="0" stdDeviation="8" flood-color="rgba(229, 37, 33, 0.45)" />
    </filter>

    <!-- Gradients -->
    <linearGradient id="gtaWhiteGrad" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#ffffff" />
      <stop offset="70%" stop-color="#f8fafc" />
      <stop offset="100%" stop-color="#cbd5e1" />
    </linearGradient>

    <linearGradient id="ctwGoldGrad" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#fff066" />
      <stop offset="35%" stop-color="#ffcc00" />
      <stop offset="85%" stop-color="#ff9900" />
      <stop offset="100%" stop-color="#e67e00" />
    </linearGradient>

    <linearGradient id="recompCyanGrad" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#38bdf8" />
      <stop offset="50%" stop-color="#818cf8" />
      <stop offset="100%" stop-color="#38bdf8" />
    </linearGradient>
  </defs>

  <!-- Clean, Deep Background (with subtle ambient glow) -->
  <rect width="900" height="360" rx="20" fill="#090b10" />
  
  <!-- Subtle Red/Gold Atmospheric Glow -->
  <circle cx="450" cy="170" r="260" fill="radial-gradient(circle, rgba(229,37,33,0.12) 0%, rgba(255,204,0,0.04) 50%, transparent 70%)" />

  <!-- ================================================================= -->
  <!-- 1. "GTA" - AUTHENTIC PRICEDOWN GRAND THEFT AUTO TYPOGRAPHY       -->
  <!-- ================================================================= -->
  <g id="GTA_Pricedown" filter="url(#gtaShadow)" transform="translate(81, 0)">
    <!-- Letter 'G' in draft Pricedown -->
    <path d="M 235,160 
             C 175,160 145,130 145,86 
             C 145,42 175,12 235,12 
             C 275,12 298,28 308,46 
             L 278,64 
             C 270,52 256,42 235,42 
             C 195,42 180,62 180,86 
             C 180,110 195,130 235,130 
             C 255,130 270,122 276,114 
             L 276,96 
             L 230,96 
             L 230,72 
             L 310,72 
             L 310,136 
             C 294,152 268,160 235,160 Z"
          fill="url(#gtaWhiteGrad)" 
          stroke="#000000" 
          stroke-width="14" 
          stroke-linejoin="round" 
          paint-order="stroke fill" />

    <!-- Letter 'T' in draft Pricedown -->
    <path d="M 326,12 
             L 458,12 
             L 458,42 
             L 408,42 
             L 408,156 
             L 374,156 
             L 374,42 
             L 326,42 Z"
          fill="url(#gtaWhiteGrad)" 
          stroke="#000000" 
          stroke-width="14" 
          stroke-linejoin="round" 
          paint-order="stroke fill" />

    <!-- Letter 'A' in draft Pricedown (Iconic rounded arch top & low crossbar) -->
    <path d="M 522,12 
             C 555,12 580,26 592,48 
             L 592,156 
             L 558,156 
             L 558,122 
             L 486,122 
             L 486,156 
             L 452,156 
             L 452,48 
             C 464,26 489,12 522,12 Z
             M 522,40 
             C 498,40 486,52 486,72 
             L 486,96 
             L 558,96 
             L 558,72 
             C 558,52 546,40 522,40 Z"
          fill="url(#gtaWhiteGrad)" 
          stroke="#000000" 
          stroke-width="14" 
          stroke-linejoin="round" 
          paint-order="stroke fill" />
  </g>

  <!-- ================================================================= -->
  <!-- 2. "CHINATOWN WARS" - OFFICIAL CHINESE ROCKS CHISEL-BRUSH FONT    -->
  <!-- ================================================================= -->
  <g id="CHINATOWN_WARS" filter="url(#ctwShadow)" transform="translate(12, 182)">
    
    <!-- WORD: CHINATOWN -->
    <g transform="translate(68, 0)">
      <!-- 'C' with 45-deg chisel cuts -->
      <path d="M 54,12 L 20,12 L 8,24 L 8,62 L 20,74 L 54,74 L 54,58 L 24,58 L 20,54 L 20,32 L 24,28 L 54,28 Z" 
            fill="url(#ctwGoldGrad)" stroke="#000000" stroke-width="7" stroke-linejoin="round" paint-order="stroke fill"/>

      <!-- 'H' with calligraphic crossbar -->
      <path d="M 66,12 L 78,12 L 78,38 L 98,38 L 98,12 L 110,12 L 110,74 L 98,74 L 98,50 L 78,50 L 78,74 L 66,74 Z" 
            fill="url(#ctwGoldGrad)" stroke="#000000" stroke-width="7" stroke-linejoin="round" paint-order="stroke fill"/>

      <!-- 'I' with chisel notches -->
      <path d="M 122,12 L 134,12 L 134,74 L 122,74 Z" 
            fill="url(#ctwGoldGrad)" stroke="#000000" stroke-width="7" stroke-linejoin="round" paint-order="stroke fill"/>

      <!-- 'N' with sharp diagonal slash & flick -->
      <path d="M 146,12 L 158,12 L 176,52 L 176,12 L 188,12 L 188,78 L 176,78 L 158,38 L 158,74 L 146,74 Z" 
            fill="url(#ctwGoldGrad)" stroke="#000000" stroke-width="7" stroke-linejoin="round" paint-order="stroke fill"/>

      <!-- 'A' with sharp calligraphy apex -->
      <path d="M 212,12 L 224,12 L 238,74 L 225,74 L 222,58 L 202,58 L 199,74 L 187,74 Z M 205,46 L 219,46 L 212,24 Z" 
            fill="url(#ctwGoldGrad)" stroke="#000000" stroke-width="7" stroke-linejoin="round" paint-order="stroke fill"/>

      <!-- 'T' with beveled ends -->
      <path d="M 242,12 L 280,12 L 280,26 L 267,26 L 267,74 L 255,74 L 255,26 L 242,26 Z" 
            fill="url(#ctwGoldGrad)" stroke="#000000" stroke-width="7" stroke-linejoin="round" paint-order="stroke fill"/>

      <!-- 'O' blocky octagonal chisel cuts -->
      <path d="M 296,12 L 322,12 L 332,22 L 332,64 L 322,74 L 296,74 L 286,64 L 286,22 Z M 298,26 L 298,60 L 320,60 L 320,26 Z" 
            fill="url(#ctwGoldGrad)" stroke="#000000" stroke-width="7" stroke-linejoin="round" paint-order="stroke fill"/>

      <!-- 'W' iconic double-V katana cuts -->
      <path d="M 340,12 L 351,12 L 358,54 L 366,12 L 377,12 L 385,54 L 392,12 L 403,12 L 392,74 L 380,74 L 372,32 L 364,74 L 352,74 Z" 
            fill="url(#ctwGoldGrad)" stroke="#000000" stroke-width="7" stroke-linejoin="round" paint-order="stroke fill"/>

      <!-- 'N' ending Chinatown -->
      <path d="M 414,12 L 426,12 L 444,52 L 444,12 L 456,12 L 456,78 L 444,78 L 426,38 L 426,74 L 414,74 Z" 
            fill="url(#ctwGoldGrad)" stroke="#000000" stroke-width="7" stroke-linejoin="round" paint-order="stroke fill"/>
    </g>

    <!-- WORD: WARS (in larger bold chisel calligraphy) -->
    <g transform="translate(542, 0)">
      <!-- Red Japanese/Chinese Calligraphy Stamp (Square seal) -->
      <rect x="0" y="14" width="46" height="58" rx="6" fill="#b91c1c" stroke="#ef4444" stroke-width="2"/>
      <text x="23" y="54" font-family="'Segoe UI', sans-serif" font-size="28" font-weight="900" fill="#ffffff" text-anchor="middle">血</text>

      <!-- 'W' in WARS -->
      <path d="M 58,12 L 71,12 L 79,54 L 88,12 L 101,12 L 110,54 L 118,12 L 131,12 L 119,74 L 105,74 L 96,32 L 87,74 L 73,74 Z" 
            fill="url(#ctwGoldGrad)" stroke="#000000" stroke-width="7" stroke-linejoin="round" paint-order="stroke fill"/>

      <!-- 'A' in WARS -->
      <path d="M 145,12 L 159,12 L 175,74 L 161,74 L 157,58 L 137,58 L 133,74 L 119,74 Z M 140,46 L 154,46 L 147,24 Z" 
            fill="url(#ctwGoldGrad)" stroke="#000000" stroke-width="7" stroke-linejoin="round" paint-order="stroke fill"/>

      <!-- 'R' in WARS with outward kick -->
      <path d="M 183,12 L 210,12 L 219,22 L 219,42 L 210,50 L 222,74 L 208,74 L 198,52 L 195,52 L 195,74 L 183,74 Z M 195,24 L 195,40 L 206,40 L 206,24 Z" 
            fill="url(#ctwGoldGrad)" stroke="#000000" stroke-width="7" stroke-linejoin="round" paint-order="stroke fill"/>

      <!-- 'S' in WARS with 45-deg angled terminals -->
      <path d="M 251,14 L 230,14 L 226,26 L 243,30 L 249,36 L 249,46 L 241,52 L 224,52 L 224,36 L 236,36 L 236,26 L 220,26 L 220,58 L 228,68 L 247,68 L 257,58 L 257,38 L 249,30 L 234,26 L 234,22 L 251,22 Z" 
            fill="url(#ctwGoldGrad)" stroke="#000000" stroke-width="7" stroke-linejoin="round" paint-order="stroke fill"/>
    </g>
  </g>

  <!-- ================================================================= -->
  <!-- 3. SLEEK MODERN SUBTITLE: "PC RECOMP"                     -->
  <!-- ================================================================= -->
  <g transform="translate(450, 310)">
    <!-- Thin Sleek Center Divider -->
    <line x1="-320" y1="0" x2="-140" y2="0" stroke="#334155" stroke-width="1.5" />
    <line x1="140" y1="0" x2="320" y2="0" stroke="#334155" stroke-width="1.5" />

    <!-- Diamond Center Accents -->
    <polygon points="-135,-3 -130,0 -135,3 -140,0" fill="#38bdf8" />
    <polygon points="135,-3 140,0 135,3 130,0" fill="#38bdf8" />

    <!-- Clean Modern Typography -->
    <text x="0" y="5" 
          font-family="'Segoe UI', -apple-system, sans-serif" 
          font-size="14" 
          font-weight="800" 
          letter-spacing="9" 
          fill="url(#recompCyanGrad)" 
          text-anchor="middle">PC RECOMP</text>
  </g>
</svg>'''
    return svg

if __name__ == '__main__':
    raise SystemExit("Historical hand-drawn draft only. See docs/BRANDING.md; this script no longer overwrites public logos.")
