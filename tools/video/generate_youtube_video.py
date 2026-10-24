# SPDX-License-Identifier: GPL-3.0-only
"""
ReGTA Chinatown Wars — Autonomous YouTube Video Generator
Generates a complete 1080p 60 FPS Devlog Announcement video in English
with neural voiceover (en-US-AndrewNeural), 1080p slides, synthesized
cyberpunk synth soundtrack, and high-CTR thumbnail.
"""

import asyncio
import os
import math
import struct
import subprocess
import wave
from pathlib import Path

import edge_tts
import imageio_ffmpeg

# Paths
REPO_ROOT = Path(__file__).resolve().parent.parent.parent
OUTPUT_DIR = REPO_ROOT / "outputs" / "CTW-Rust" / "video"
TEMP_DIR = REPO_ROOT / "outputs" / "CTW-Rust" / "video" / "temp"
EDGE_EXE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
FFMPEG_EXE = imageio_ffmpeg.get_ffmpeg_exe()
LOGO_PATH = REPO_ROOT / "docs" / "assets" / "logo.png"

# Script Data (English Narration & Slides)
SCENES = [
    {
        "id": "scene_01",
        "title": "THE DEFINITIVE PC RECOMPILATION",
        "voice": (
            "Grand Theft Auto: Chinatown Wars is one of the most unique and ambitious games in the entire GTA series. "
            "But for more than fifteen years, PC gamers have been forced to rely on clunky PSP emulators. "
            "Today, that changes forever."
        ),
        "headline": "GRAND THEFT AUTO: CHINATOWN WARS",
        "subheadline": "THE NATIVE PC RECOMPILATION & REVERSE-ENGINEERING PROJECT",
        "badge": "BUILT IN RUST • 100% NATIVE X86_64",
        "bullets": [
            ("⚡ 15-Year Milestone", "First clean-room native PC recompilation of Chinatown Wars PSP"),
            ("🚀 Zero CPU Emulation", "Compiles directly to native machine code without emulator overhead"),
            ("🎮 True PC Experience", "Native 60+ FPS, ultra-widescreen 4K rendering, and modern gamepad controls"),
        ],
    },
    {
        "id": "scene_02",
        "title": "NATIVE RUST VS TRADITIONAL EMULATION",
        "voice": (
            "This is ReGTA Chinatown Wars, an independent, clean-room native reverse engineering and PC port written entirely from scratch in Rust. "
            "Instead of emulating the PSP hardware, it runs natively on modern Windows systems, reading game data straight from your personal disc "
            "with zero input latency, full ultra-widescreen support, and a rock-solid sixty frames per second."
        ),
        "headline": "WHY NATIVE RECOMPILATION CHANGES EVERYTHING",
        "subheadline": "HOW RUST ACHIEVES UNPARALLELED SPEED AND RESPONSIVENESS",
        "badge": "RUST RUNTIME VS PSP EMULATOR OVERHEAD",
        "comparison": {
            "left_title": "❌ TRADITIONAL PSP EMULATOR (PPSSPP)",
            "left_items": [
                "Heavy MIPS CPU instruction translation overhead",
                "Frame pacing stutter and 30 FPS console lock",
                "Analog stick input latency and deadzone drift",
                "Imprecise touch-screen minigame mapping",
            ],
            "right_title": "⚡ ReGTA NATIVE PC PORT (RUST)",
            "right_items": [
                "100% Native x86_64 machine code execution",
                "Silky smooth 60+ FPS & unconstrained 4K resolutions",
                "Instant disk asset streaming in 1.18 seconds",
                "Native XInput gamepad and fluid mouse steering",
            ],
        },
    },
    {
        "id": "scene_03",
        "title": "CORE GAMEPLAY ENGINE ALREADY PLAYABLE",
        "voice": (
            "Development has been moving at lightspeed. We have already fully implemented all sixteen street races across Liberty City with competitive AI rivals, "
            "the complete Drug Wars economy with eighty-one dealers and seven rival gangs, authentic tattoo parlor and car-bomb minigames, "
            "twenty-one purchasable safehouses, and classic odd jobs from Taxi driver to Vigilante."
        ),
        "headline": "WHAT'S ALREADY PLAYABLE & IMPLEMENTED",
        "subheadline": "AUTHENTIC PSP SYSTEMS REBUILT IN PURE RUST",
        "badge": "ALPHA MILESTONE 0.1.4 — FULL CITY LIFE",
        "grid_cards": [
            ("🏎️ 16 Street Races & Time Trials", "16 circuits across Broker, Dukes, Bohan & Algonquin with AI rival racers and rubber-banding."),
            ("💊 Drug Wars Wholesale Economy", "81 street dealers, 7 rival gangs, price trends, police confiscation, and stash boxes."),
            ("🐉 Tactile Touch Minigames", "Tattoo parlor needle tracing, car-bomb wire rigging, safe cracking, and hotwiring."),
            ("🏙️ Living City & Emergency Jobs", "21 purchasable safehouses, Taxi nitro boost, Vigilante police waves, and Firefighter."),
        ],
    },
    {
        "id": "scene_04",
        "title": "ENGINEERING RIGOR & CLEAN-ROOM INTEGRITY",
        "voice": (
            "The codebase adheres to the highest engineering standards: one hundred and sixty automated unit tests passing, "
            "zero compiler warnings, and all one hundred and sixty-six original PSP code overlays indexed. "
            "Best of all, the project strictly respects copyright: no game assets or pirated ROMs are distributed. "
            "Players supply their own legally owned PSP game disc."
        ),
        "headline": "CLEAN-ROOM ENGINEERING & INTEGRITY",
        "subheadline": "UNCOMPROMISING CODE QUALITY AND STRICT COPYRIGHT RESPECT",
        "badge": "160 TESTS PASSED • 0 WARNINGS • 100% LEGAL",
        "bullets": [
            ("🛡️ 160 Automated Unit Tests", "Full coverage for 3D physics, collision meshes, drug economy, and memory safety"),
            ("⚡ Zero Compiler Warnings", "Strict -D warnings policy enforced across all 45 modular Rust source files"),
            ("📦 166 PSP Code Overlays", "Automatic extraction and decompression from original UMD OVERLAYS.PAK container"),
            ("⚖️ Strict Pre-Alpha Policy", "Zero copyrighted ROMs or assets distributed. Players supply their own legal disc"),
        ],
    },
    {
        "id": "scene_05",
        "title": "ACCELERATE DEVELOPMENT & JOIN THE JOURNEY",
        "voice": (
            "To accelerate development and bring full story missions, animated comic cutscenes, and vehicle radio stations to life, "
            "your support makes a massive difference. You can star the project on GitHub, follow the progress on our live website, "
            "and consider donating via Tipeee or PayPal. All links are in the description below. "
            "Thank you for watching, and welcome to the future of Chinatown Wars."
        ),
        "headline": "JOIN THE JOURNEY & ACCELERATE DEVELOPMENT",
        "subheadline": "HELP US BRING THE COMPLETE CHINATOWN WARS CAMPAIGN TO PC",
        "badge": "OPEN SOURCE ON GITHUB • SUPPORT BENJI PANAME",
        "support_cards": [
            ("❤️ Support on Tipeee", "fr.tipeee.com/benji-paname", "Monthly or one-time creator tips to fund dedicated development time."),
            ("💳 Support via PayPal.Me", "paypal.me/BenjiLsg", "Direct, international donations to fuel reverse engineering tools."),
            ("💻 Open Source GitHub Repository", "github.com/benji-paname/regta-chinatown-wars-pc-port", "Star, fork, and inspect the Rust source code and technical documentation."),
            ("🌐 Live Trilingual Showcase", "benji-paname.github.io/regta-chinatown-wars-pc-port", "Official project roadmap, milestones, and trilingual updates (EN / FR / ES)."),
        ],
    },
]


def generate_synth_soundtrack(output_path: Path, duration_s: float):
    """
    Synthesizes an authentic 110 BPM cyberpunk electronic synthwave track
    (D minor chord progression: Dm -> Bb -> C -> Am) with punchy 808 kick,
    snare, hi-hats, sub-bass, and plucked arpeggio.
    """
    sample_rate = 44100
    total_samples = int(sample_rate * duration_s)
    bpm = 110.0
    beat_len = 60.0 / bpm
    bar_len = beat_len * 4.0

    print(f"Synthesizing {duration_s:.1f}s soundtrack to {output_path.name}...")

    # Chord roots: D2, Bb1, C2, A1
    roots = [73.42, 58.27, 65.41, 55.00]
    arp_notes = [
        [146.83, 174.61, 220.00, 261.63],  # Dm: D3, F3, A3, C4
        [116.54, 146.83, 174.61, 233.08],  # Bb: Bb2, D3, F3, Bb3
        [130.81, 164.81, 196.00, 261.63],  # C:  C3, E3, G3, C4
        [110.00, 146.83, 174.61, 220.00],  # Am: A2, D3, F3, A3
    ]

    frames = bytearray()
    chunk_size = 2048
    buffer = []

    for i in range(total_samples):
        t = i / sample_rate
        bar_idx = int((t / bar_len) % 4)
        beat_in_bar = (t % bar_len) / beat_len
        beat_frac = beat_in_bar % 1.0

        # 1. Kick on beat 0 and beat 2
        is_kick_beat = (beat_in_bar < 0.35) or (2.0 <= beat_in_bar < 2.35)
        kick_t = (t % (beat_len * 2.0))
        kick = 0.0
        if is_kick_beat and beat_frac < 0.25:
            k_freq = max(45.0, 160.0 * math.exp(-beat_frac * 24.0))
            kick = math.sin(2 * math.pi * k_freq * beat_frac) * math.exp(-beat_frac * 14.0)

        # 2. Snare on beat 1 and beat 3
        is_snare_beat = (1.0 <= beat_in_bar < 1.4) or (3.0 <= beat_in_bar < 3.4)
        snare = 0.0
        if is_snare_beat:
            s_t = beat_in_bar % 1.0
            noise = (math.sin(i * 1234.567) % 1.0) * 2.0 - 1.0
            tone = math.sin(2 * math.pi * 180.0 * s_t)
            snare = (tone * 0.4 + noise * 0.6) * math.exp(-s_t * 16.0)

        # 3. Hi-hat on every 8th note
        hh_frac = (t % (beat_len / 2.0)) / (beat_len / 2.0)
        hh = ((math.sin(i * 9876.543) % 1.0) * 2.0 - 1.0) * math.exp(-hh_frac * 28.0) * 0.25

        # 4. Deep sub-bass (D minor chord roots)
        root_freq = roots[bar_idx]
        bass = 0.35 * (
            math.sin(2 * math.pi * root_freq * t)
            + 0.25 * math.sin(2 * math.pi * root_freq * 2.0 * t)
        )

        # 5. Plucked 16th-note Arpeggio
        arp_step = int((t / (beat_len / 4.0)) % 4)
        arp_freq = arp_notes[bar_idx][arp_step]
        arp_frac = (t % (beat_len / 4.0)) / (beat_len / 4.0)
        arp = 0.22 * math.sin(2 * math.pi * arp_freq * t) * math.exp(-arp_frac * 9.0)

        # 6. Smooth stereo mix (slight stereo spread)
        mix_left = (kick * 0.45 + snare * 0.35 + hh * 0.20 + bass * 0.40 + arp * 0.25)
        mix_right = (kick * 0.45 + snare * 0.35 - hh * 0.20 + bass * 0.40 + arp * 0.30)

        # Master volume attenuation (-18dB under voiceover)
        vol = 0.18
        s_left = int(max(-32767, min(32767, mix_left * vol * 32767)))
        s_right = int(max(-32767, min(32767, mix_right * vol * 32767)))

        buffer.append((s_left, s_right))
        if len(buffer) >= chunk_size:
            for sl, sr in buffer:
                frames.extend(struct.pack("<hh", sl, sr))
            buffer.clear()

    if buffer:
        for sl, sr in buffer:
            frames.extend(struct.pack("<hh", sl, sr))

    with wave.open(str(output_path), "wb") as wf:
        wf.setnchannels(2)
        wf.setsampwidth(2)
        wf.setframerate(sample_rate)
        wf.writeframes(frames)

    print(f"Soundtrack generated: {output_path} ({os.path.getsize(output_path)} bytes)")


async def generate_voiceovers():
    """Generates all 5 scene voiceover audio files using edge-tts (en-US-AndrewNeural)."""
    print("\n--- Generating English Neural Voiceovers (en-US-AndrewNeural) ---")
    voice = "en-US-AndrewNeural"
    durations = {}

    for scene in SCENES:
        sid = scene["id"]
        out_file = TEMP_DIR / f"{sid}.mp3"
        print(f"Synthesizing voice for {sid}...")
        clean_text = scene["voice"].replace("—", ", ").replace("–", "-")

        # Retry loop for robust network streaming
        saved = False
        for attempt in range(4):
            try:
                comm = edge_tts.Communicate(clean_text, voice)
                await comm.save(str(out_file))
                saved = True
                break
            except Exception as e:
                print(f"  Attempt {attempt + 1} failed ({e}), retrying in 1s...")
                await asyncio.sleep(1.0)

        if not saved:
            raise RuntimeError(f"Failed to synthesize voice for {sid}")

        # Query exact duration via ffmpeg
        probe_cmd = [FFMPEG_EXE, "-i", str(out_file)]
        res = subprocess.run(probe_cmd, capture_output=True, text=True)
        dur = 5.0
        for line in res.stderr.splitlines():
            if "Duration" in line:
                # Duration: 00:00:14.23, ...
                parts = line.split("Duration:")[1].split(",")[0].strip().split(":")
                dur = float(parts[0]) * 3600 + float(parts[1]) * 60 + float(parts[2])
                break
        # Add 0.8s padding for natural breathing and pacing
        durations[sid] = dur + 0.8
        print(f"  -> {sid}.mp3: {dur:.2f}s (subclip duration: {durations[sid]:.2f}s)")

    return durations


def build_slide_html(scene: dict) -> str:
    """Builds a cinematic 1920x1080 HTML5/CSS3 presentation slide."""
    logo_uri = LOGO_PATH.resolve().as_uri()

    # Content section generator
    content_html = ""
    if "comparison" in scene:
        cmp = scene["comparison"]
        left_items = "".join(f"<li>{item}</li>" for item in cmp["left_items"])
        right_items = "".join(f"<li>{item}</li>" for item in cmp["right_items"])
        content_html = f"""
        <div class="comparison-grid">
            <div class="comp-box comp-bad">
                <div class="comp-header">{cmp["left_title"]}</div>
                <ul>{left_items}</ul>
            </div>
            <div class="comp-box comp-good">
                <div class="comp-header">{cmp["right_title"]}</div>
                <ul>{right_items}</ul>
            </div>
        </div>
        """
    elif "grid_cards" in scene:
        cards_html = "".join(
            f"""
            <div class="feature-card">
                <div class="card-title">{title}</div>
                <div class="card-desc">{desc}</div>
            </div>
            """
            for title, desc in scene["grid_cards"]
        )
        content_html = f'<div class="feature-grid">{cards_html}</div>'
    elif "support_cards" in scene:
        cards_html = "".join(
            f"""
            <div class="support-card">
                <div class="support-name">{name}</div>
                <div class="support-url">{url}</div>
                <div class="support-desc">{desc}</div>
            </div>
            """
            for name, url, desc in scene["support_cards"]
        )
        content_html = f'<div class="support-grid">{cards_html}</div>'
    elif "bullets" in scene:
        bullets_html = "".join(
            f"""
            <div class="bullet-row">
                <div class="bullet-head">{head}</div>
                <div class="bullet-body">{body}</div>
            </div>
            """
            for head, body in scene["bullets"]
        )
        content_html = f'<div class="bullets-container">{bullets_html}</div>'

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{
    width: 1920px;
    height: 1080px;
    background: #08090d;
    background-image: 
        radial-gradient(circle at 15% 20%, rgba(220, 20, 20, 0.22) 0%, transparent 45%),
        radial-gradient(circle at 85% 80%, rgba(255, 215, 0, 0.18) 0%, transparent 45%),
        radial-gradient(rgba(255, 255, 255, 0.08) 1px, transparent 1px);
    background-size: 100% 100%, 100% 100%, 32px 32px;
    color: #f8fafc;
    font-family: 'Segoe UI', -apple-system, system-ui, sans-serif;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 50px 80px;
    overflow: hidden;
}}

/* Top Header Bar */
.header-bar {{
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 2px solid rgba(255, 255, 255, 0.12);
    padding-bottom: 24px;
}}
.logo-wrap img {{
    height: 75px;
    object-fit: contain;
    filter: drop-shadow(0 6px 20px rgba(0,0,0,0.8));
}}
.header-badge {{
    background: linear-gradient(135deg, rgba(255, 59, 48, 0.25), rgba(255, 215, 0, 0.20));
    border: 1px solid #ffd700;
    color: #ffd700;
    font-size: 16px;
    font-weight: 800;
    letter-spacing: 2.5px;
    padding: 10px 22px;
    border-radius: 999px;
    text-transform: uppercase;
    box-shadow: 0 0 25px rgba(255, 215, 0, 0.25);
}}

/* Main Title Section */
.title-wrap {{
    margin-top: 20px;
    margin-bottom: 25px;
}}
.main-headline {{
    font-size: 50px;
    font-weight: 900;
    letter-spacing: -0.5px;
    line-height: 1.15;
    background: linear-gradient(180deg, #ffffff 0%, #e2e8f0 70%, #94a3b8 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    text-transform: uppercase;
    text-shadow: 0 10px 30px rgba(0,0,0,0.5);
}}
.sub-headline {{
    color: #ffd700;
    font-size: 21px;
    font-weight: 700;
    letter-spacing: 2px;
    margin-top: 8px;
    text-transform: uppercase;
}}

/* Dynamic Center Body */
.center-content {{
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
}}

/* Comparison Layout (Slide 2) */
.comparison-grid {{
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 40px;
    width: 100%;
}}
.comp-box {{
    background: rgba(15, 20, 28, 0.85);
    border-radius: 18px;
    padding: 35px 40px;
    backdrop-filter: blur(12px);
    box-shadow: 0 20px 50px rgba(0,0,0,0.6);
}}
.comp-bad {{
    border: 2px solid rgba(239, 68, 68, 0.5);
    background: linear-gradient(180deg, rgba(239, 68, 68, 0.12), rgba(15, 20, 28, 0.95));
}}
.comp-good {{
    border: 2px solid rgba(16, 185, 129, 0.7);
    background: linear-gradient(180deg, rgba(16, 185, 129, 0.16), rgba(15, 20, 28, 0.95));
    box-shadow: 0 0 40px rgba(16, 185, 129, 0.25);
}}
.comp-header {{
    font-size: 24px;
    font-weight: 900;
    margin-bottom: 24px;
    letter-spacing: 1.5px;
}}
.comp-bad .comp-header {{ color: #f87171; }}
.comp-good .comp-header {{ color: #34d399; }}
.comp-box ul {{
    list-style: none;
}}
.comp-box li {{
    font-size: 20px;
    line-height: 1.6;
    margin-bottom: 14px;
    color: #e2e8f0;
}}

/* Feature Cards Grid (Slide 3) */
.feature-grid {{
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 28px;
    width: 100%;
}}
.feature-card {{
    background: linear-gradient(145deg, rgba(30, 41, 59, 0.7), rgba(15, 23, 42, 0.9));
    border: 1px solid rgba(255, 215, 0, 0.3);
    border-radius: 16px;
    padding: 26px 30px;
    box-shadow: 0 12px 35px rgba(0,0,0,0.4);
}}
.card-title {{
    font-size: 24px;
    font-weight: 800;
    color: #ffd700;
    margin-bottom: 10px;
}}
.card-desc {{
    font-size: 19px;
    color: #cbd5e1;
    line-height: 1.45;
}}

/* Bullet List (Slide 1 & 4) */
.bullets-container {{
    display: flex;
    flex-direction: column;
    gap: 20px;
}}
.bullet-row {{
    background: linear-gradient(90deg, rgba(255, 59, 48, 0.15), rgba(15, 23, 42, 0.7));
    border-left: 6px solid #ff3b30;
    border-radius: 0 14px 14px 0;
    padding: 20px 30px;
}}
.bullet-head {{
    font-size: 24px;
    font-weight: 800;
    color: #f8fafc;
    margin-bottom: 6px;
}}
.bullet-body {{
    font-size: 19px;
    color: #94a3b8;
    line-height: 1.4;
}}

/* Support Cards (Slide 5) */
.support-grid {{
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 26px;
}}
.support-card {{
    background: linear-gradient(135deg, rgba(30, 41, 59, 0.8), rgba(15, 23, 42, 0.95));
    border: 1.5px solid rgba(255, 215, 0, 0.4);
    border-radius: 16px;
    padding: 24px 28px;
    box-shadow: 0 15px 40px rgba(0,0,0,0.5);
}}
.support-name {{
    font-size: 22px;
    font-weight: 800;
    color: #ffd700;
    margin-bottom: 6px;
}}
.support-url {{
    font-family: Consolas, monospace;
    font-size: 17px;
    color: #38bdf8;
    margin-bottom: 8px;
    font-weight: 700;
}}
.support-desc {{
    font-size: 17px;
    color: #cbd5e1;
    line-height: 1.35;
}}

/* Bottom Status Footer */
.footer-bar {{
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid rgba(255, 255, 255, 0.12);
    padding-top: 20px;
    font-size: 17px;
    color: #94a3b8;
}}
.footer-item strong {{
    color: #f8fafc;
}}
.footer-right {{
    color: #ff3b30;
    font-weight: 800;
    letter-spacing: 1.5px;
}}
</style>
</head>
<body>
    <div class="header-bar">
        <div class="logo-wrap">
            <img src="{logo_uri}" alt="ReGTA CTW Logo">
        </div>
        <div class="header-badge">{scene["badge"]}</div>
    </div>

    <div class="title-wrap">
        <div class="main-headline">{scene["headline"]}</div>
        <div class="sub-headline">{scene["subheadline"]}</div>
    </div>

    <div class="center-content">
        {content_html}
    </div>

    <div class="footer-bar">
        <div class="footer-item">DEVELOPER: <strong>BENJI PANAME</strong></div>
        <div class="footer-item">STATUS: <strong>PRE-ALPHA v0.1.4 (160 TESTS PASSING)</strong></div>
        <div class="footer-item">CLEAN ROOM: <strong>NO COPYRIGHTED ASSETS DISTRIBUTED</strong></div>
        <div class="footer-right">YOUTUBE DEVLOG #1</div>
    </div>
</body>
</html>"""


def build_thumbnail_html() -> str:
    """Builds a high-impact 1280x720 YouTube Thumbnail."""
    logo_uri = LOGO_PATH.resolve().as_uri()
    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{
    width: 1280px;
    height: 720px;
    background: #08090d;
    background-image: 
        radial-gradient(circle at 50% 30%, rgba(220, 20, 20, 0.35) 0%, transparent 60%),
        radial-gradient(circle at 10% 90%, rgba(255, 215, 0, 0.25) 0%, transparent 50%),
        radial-gradient(rgba(255, 255, 255, 0.08) 1px, transparent 1px);
    background-size: 100% 100%, 100% 100%, 28px 28px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    align-items: center;
    padding: 40px;
    font-family: 'Segoe UI', Arial Black, Impact, sans-serif;
    color: #fff;
    overflow: hidden;
}}
.logo-container img {{
    height: 190px;
    filter: drop-shadow(0 15px 35px rgba(0,0,0,0.9));
}}
.giant-banner {{
    background: linear-gradient(90deg, #ff3b30, #e11d48, #ffd700);
    padding: 14px 45px;
    border-radius: 14px;
    font-size: 48px;
    font-weight: 900;
    letter-spacing: 2px;
    text-transform: uppercase;
    color: #ffffff;
    box-shadow: 0 15px 40px rgba(225, 29, 72, 0.6);
    border: 3px solid #ffffff;
    transform: rotate(-1.5deg);
}}
.badge-row {{
    display: flex;
    gap: 18px;
    width: 100%;
    justify-content: center;
}}
.thumb-badge {{
    background: rgba(15, 23, 42, 0.9);
    border: 2px solid #ffd700;
    padding: 14px 28px;
    border-radius: 14px;
    font-size: 22px;
    font-weight: 900;
    color: #ffd700;
    box-shadow: 0 10px 25px rgba(0,0,0,0.6);
    letter-spacing: 1px;
}}
.highlight-badge {{
    background: #10b981;
    color: #ffffff;
    border: 2px solid #ffffff;
}}
</style>
</head>
<body>
    <div class="logo-container">
        <img src="{logo_uri}" alt="ReGTA Logo">
    </div>

    <div class="giant-banner">
        NATIVE PC RECOMPILATION • RUST
    </div>

    <div class="badge-row">
        <div class="thumb-badge highlight-badge">⚡ 60 FPS • NATIVE 4K</div>
        <div class="thumb-badge">🏎️ 16 STREET RACES</div>
        <div class="thumb-badge">💊 DRUG WARS PLAYABLE</div>
        <div class="thumb-badge">🛡️ NO EMULATOR</div>
    </div>
</body>
</html>"""


def render_html_to_png(html_content: str, png_path: Path, width: int, height: int):
    """Renders HTML string to PNG via headless Microsoft Edge."""
    temp_html = png_path.with_suffix(".html")
    temp_html.write_text(html_content, encoding="utf-8")

    cmd = [
        EDGE_EXE,
        "--headless",
        r"--user-data-dir=C:\Users\BENJAMIN\AppData\Local\Temp\edge_video_temp",
        "--default-background-color=ff08090d",
        f"--screenshot={png_path.resolve()}",
        f"--window-size={width},{height}",
        f"file:///{temp_html.resolve().as_posix()}",
    ]
    subprocess.run(cmd, check=True)
    if temp_html.exists():
        temp_html.unlink()
    print(f"Rendered PNG: {png_path.name} ({width}x{height})")


def create_video_subclips(durations: dict):
    """Creates MP4 subclips for each scene image + audio voiceover."""
    print("\n--- Assembling Video Subclips with FFmpeg ---")
    subclips = []

    for scene in SCENES:
        sid = scene["id"]
        png_path = TEMP_DIR / f"{sid}.png"
        audio_path = TEMP_DIR / f"{sid}.mp3"
        subclip_path = TEMP_DIR / f"{sid}_clip.mp4"
        dur = durations[sid]

        print(f"Encoding {sid}_clip.mp4 (duration: {dur:.2f}s)...")
        # Loop image, encode 60 FPS H.264, attach voiceover
        cmd = [
            FFMPEG_EXE,
            "-loop", "1",
            "-i", str(png_path),
            "-i", str(audio_path),
            "-c:v", "libx264",
            "-tune", "stillimage",
            "-c:a", "aac",
            "-b:a", "192k",
            "-pix_fmt", "yuv420p",
            "-r", "60",
            "-t", str(dur),
            "-shortest",
            str(subclip_path),
            "-y",
        ]
        subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        subclips.append(subclip_path)

    return subclips


def assemble_final_video(subclips: list, total_duration: float):
    """Concatenates subclips and mixes background synth soundtrack."""
    print("\n--- Mixing Final Master Video (1080p 60 FPS) ---")
    concat_txt = TEMP_DIR / "concat_list.txt"
    with open(concat_txt, "w", encoding="utf-8") as f:
        for clip in subclips:
            # Escape path for ffmpeg concat
            p_str = clip.resolve().as_posix()
            f.write(f"file '{p_str}'\n")

    unmixed_video = TEMP_DIR / "unmixed_master.mp4"
    print("Concatenating subclips...")
    cmd_concat = [
        FFMPEG_EXE,
        "-f", "concat",
        "-safe", "0",
        "-i", str(concat_txt),
        "-c", "copy",
        str(unmixed_video),
        "-y",
    ]
    subprocess.run(cmd_concat, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    # Generate background soundtrack matching total video duration
    music_wav = TEMP_DIR / "soundtrack.wav"
    generate_synth_soundtrack(music_wav, total_duration + 2.0)

    # Final audio mix: voiceover + background music (-18dB)
    final_output = OUTPUT_DIR / "ReGTA_Chinatown_Wars_Devlog_01_EN.mp4"
    print(f"Mixing audio tracks into final deliverable: {final_output.name}...")

    # Complex filter: mix voiceover track with background music
    fade_start = max(0.0, total_duration - 2.5)
    filter_expr = f"[0:a]volume=1.0[voice];[1:a]volume=0.30,afade=t=out:st={fade_start:.2f}:d=2.5[music];[voice][music]amix=inputs=2:duration=first[aout]"
    cmd_mix = [
        FFMPEG_EXE,
        "-i", str(unmixed_video),
        "-i", str(music_wav),
        "-filter_complex", filter_expr,
        "-map", "0:v",
        "-map", "[aout]",
        "-c:v", "copy",
        "-c:a", "aac",
        "-b:a", "256k",
        str(final_output),
        "-y",
    ]
    subprocess.run(cmd_mix, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    print(f"\n Master Video Produced: {final_output}")
    print(f" File Size: {os.path.getsize(final_output) / (1024 * 1024):.2f} MB")
    return final_output


async def main():
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    TEMP_DIR.mkdir(parents=True, exist_ok=True)

    print("=================================================================")
    print(" ReGTA Chinatown Wars — YouTube Video Generator (English)")
    print("=================================================================")

    # 1. Voiceover Synthesis
    durations = await generate_voiceovers()
    total_dur = sum(durations.values())
    print(f"\nTotal video duration: {total_dur:.1f} seconds ({total_dur / 60.0:.2f} minutes)")

    # 2. Render Slide PNGs (1920x1080)
    print("\n--- Rendering 1080p Presentation Slides ---")
    for scene in SCENES:
        sid = scene["id"]
        png_path = TEMP_DIR / f"{sid}.png"
        html = build_slide_html(scene)
        render_html_to_png(html, png_path, 1920, 1080)

    # 3. Render High-CTR YouTube Thumbnail (1280x720)
    print("\n--- Rendering YouTube Thumbnail (1280x720) ---")
    thumb_path = OUTPUT_DIR / "thumbnail.png"
    thumb_html = build_thumbnail_html()
    render_html_to_png(thumb_html, thumb_path, 1280, 720)

    # 4. Create Video Subclips
    subclips = create_video_subclips(durations)

    # 5. Assemble Final Master Video with Mixed Soundtrack
    final_video = assemble_final_video(subclips, total_dur)

    print("\n=================================================================")
    print(" VIDEO GENERATION COMPLETED SUCCESSFULLY!")
    print(f" Video Path:     {final_video}")
    print(f" Thumbnail Path: {thumb_path}")
    print("=================================================================")


if __name__ == "__main__":
    asyncio.run(main())
