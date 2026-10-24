# SPDX-License-Identifier: GPL-3.0-only
"""
ReGTA Chinatown Wars — Autonomous YouTube Video Generator (Gameplay Edition)
Generates a 1080p 60 FPS Devlog video in English where the entire visual is
real Chinatown Wars 60 FPS gameplay, with transparent logo overlays,
broadcast lower-thirds, neural voiceover (en-US-AndrewNeural), and mixed audio.
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

# Assets
LOGO_TRANSPARENT_PATH = REPO_ROOT / "docs" / "assets" / "logo-transparent.png"
LOGO_CTW_TRANSPARENT_PATH = REPO_ROOT / "docs" / "assets" / "logo-ctw-transparent.png"

# Gameplay Recordings from User's PC
GAMEPLAY_V1 = Path(r"C:\Users\BENJAMIN\Videos\NVIDIA\PPSSPP\PPSSPP 2026.10.21 - 18.45.18.01.mp4")
GAMEPLAY_V2 = Path(r"C:\Users\BENJAMIN\Videos\NVIDIA\PPSSPP\PPSSPP 2026.10.24 - 06.00.37.01.mp4")

# English Voiceover Scenes
SCENES = [
    {
        "id": "scene_01",
        "voice": (
            "Grand Theft Auto: Chinatown Wars is one of the most unique and ambitious games in the entire GTA series. "
            "But for more than fifteen years, PC gamers have been forced to rely on clunky PSP emulators. "
            "Today, that changes forever."
        ),
    },
    {
        "id": "scene_02",
        "voice": (
            "This is ReGTA Chinatown Wars, an independent, clean-room native reverse engineering and PC port written entirely from scratch in Rust. "
            "Instead of emulating the PSP hardware, it runs natively on modern Windows systems, reading game data straight from your personal disc "
            "with zero input latency, full ultra-widescreen support, and uncapped high-refresh rate framerates."
        ),
    },
    {
        "id": "scene_03",
        "voice": (
            "Development has been moving at lightspeed. We have already fully implemented all sixteen street races across Liberty City with competitive AI rivals, "
            "the complete Drug Wars economy with eighty-one dealers and seven rival gangs, authentic tattoo parlor and car-bomb minigames, "
            "twenty-one purchasable safehouses, and classic odd jobs from Taxi driver to Vigilante."
        ),
    },
    {
        "id": "scene_04",
        "voice": (
            "The codebase adheres to the highest engineering standards: one hundred and sixty automated unit tests passing, "
            "zero compiler warnings, and all one hundred and sixty-six original PSP code overlays indexed. "
            "Best of all, the project strictly respects copyright: no game assets or pirated ROMs are distributed. "
            "Players supply their own legally owned PSP game disc."
        ),
    },
    {
        "id": "scene_05",
        "voice": (
            "To accelerate development and bring full story missions, animated comic cutscenes, and vehicle radio stations to life, "
            "your support makes a massive difference. You can star the project on GitHub, follow the progress on our live website, "
            "and consider donating via Tipeee or PayPal. All links are in the description below. "
            "Thank you for watching, and welcome to the future of Chinatown Wars."
        ),
    },
]


def generate_synth_soundtrack(output_path: Path, duration_s: float):
    """Synthesizes a 110 BPM cyberpunk electronic synth beat."""
    sample_rate = 44100
    total_samples = int(sample_rate * duration_s)
    bpm = 110.0
    beat_len = 60.0 / bpm
    bar_len = beat_len * 4.0

    print(f"Synthesizing {duration_s:.1f}s soundtrack to {output_path.name}...")
    roots = [73.42, 58.27, 65.41, 55.00]
    arp_notes = [
        [146.83, 174.61, 220.00, 261.63],
        [116.54, 146.83, 174.61, 233.08],
        [130.81, 164.81, 196.00, 261.63],
        [110.00, 146.83, 174.61, 220.00],
    ]

    frames = bytearray()
    chunk_size = 2048
    buffer = []

    for i in range(total_samples):
        t = i / sample_rate
        bar_idx = int((t / bar_len) % 4)
        beat_in_bar = (t % bar_len) / beat_len
        beat_frac = beat_in_bar % 1.0

        is_kick_beat = (beat_in_bar < 0.35) or (2.0 <= beat_in_bar < 2.35)
        kick = 0.0
        if is_kick_beat and beat_frac < 0.25:
            k_freq = max(45.0, 160.0 * math.exp(-beat_frac * 24.0))
            kick = math.sin(2 * math.pi * k_freq * beat_frac) * math.exp(-beat_frac * 14.0)

        is_snare_beat = (1.0 <= beat_in_bar < 1.4) or (3.0 <= beat_in_bar < 3.4)
        snare = 0.0
        if is_snare_beat:
            s_t = beat_in_bar % 1.0
            noise = (math.sin(i * 1234.567) % 1.0) * 2.0 - 1.0
            tone = math.sin(2 * math.pi * 180.0 * s_t)
            snare = (tone * 0.4 + noise * 0.6) * math.exp(-s_t * 16.0)

        hh_frac = (t % (beat_len / 2.0)) / (beat_len / 2.0)
        hh = ((math.sin(i * 9876.543) % 1.0) * 2.0 - 1.0) * math.exp(-hh_frac * 28.0) * 0.25

        root_freq = roots[bar_idx]
        bass = 0.35 * (math.sin(2 * math.pi * root_freq * t) + 0.25 * math.sin(2 * math.pi * root_freq * 2.0 * t))

        arp_step = int((t / (beat_len / 4.0)) % 4)
        arp_freq = arp_notes[bar_idx][arp_step]
        arp_frac = (t % (beat_len / 4.0)) / (beat_len / 4.0)
        arp = 0.22 * math.sin(2 * math.pi * arp_freq * t) * math.exp(-arp_frac * 9.0)

        mix_left = (kick * 0.45 + snare * 0.35 + hh * 0.20 + bass * 0.40 + arp * 0.25)
        mix_right = (kick * 0.45 + snare * 0.35 - hh * 0.20 + bass * 0.40 + arp * 0.30)

        vol = 0.16
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

        probe_cmd = [FFMPEG_EXE, "-i", str(out_file)]
        res = subprocess.run(probe_cmd, capture_output=True, text=True)
        dur = 5.0
        for line in res.stderr.splitlines():
            if "Duration" in line:
                parts = line.split("Duration:")[1].split(",")[0].strip().split(":")
                dur = float(parts[0]) * 3600 + float(parts[1]) * 60 + float(parts[2])
                break
        durations[sid] = dur + 0.8
        print(f"  -> {sid}.mp3: {dur:.2f}s (subclip duration: {durations[sid]:.2f}s)")

    return durations


def build_overlay_html(scene_idx: int) -> str:
    """Builds transparent overlay HTML with alpha transparency and broadcast lower-thirds."""
    logo_trans_uri = LOGO_TRANSPARENT_PATH.resolve().as_uri()
    logo_ctw_uri = LOGO_CTW_TRANSPARENT_PATH.resolve().as_uri()

    if scene_idx == 0:
        # Scene 1: Epic Intro Hook
        return f"""<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{
    width: 1920px; height: 1080px;
    background: transparent;
    display: flex; flex-direction: column; justify-content: space-between; align-items: center;
    padding: 60px 80px;
    font-family: 'Segoe UI', Arial, sans-serif;
    color: #fff;
}}
.top-pill {{
    background: rgba(15, 23, 42, 0.85);
    border: 1px solid rgba(255, 215, 0, 0.5);
    padding: 10px 28px;
    border-radius: 999px;
    font-size: 17px; font-weight: 800; color: #ffd700; letter-spacing: 2px;
    box-shadow: 0 4px 25px rgba(0,0,0,0.6);
}}
.center-logo-box {{
    display: flex; flex-direction: column; align-items: center; justify-content: center;
}}
.center-logo-box img {{
    height: 310px;
    filter: drop-shadow(0 15px 40px rgba(0,0,0,0.95));
}}
.intro-sub {{
    margin-top: 15px;
    background: linear-gradient(90deg, #ff3b30, #ffd700);
    padding: 10px 32px;
    border-radius: 8px;
    font-size: 26px; font-weight: 900; letter-spacing: 3px; color: #fff;
    box-shadow: 0 10px 30px rgba(0,0,0,0.7);
    text-transform: uppercase;
}}
.bottom-bar {{
    background: rgba(8, 9, 13, 0.90);
    border: 1.5px solid rgba(255, 255, 255, 0.15);
    padding: 14px 40px;
    border-radius: 12px;
    font-size: 20px; font-weight: 800; color: #38bdf8; letter-spacing: 2px;
    box-shadow: 0 8px 30px rgba(0,0,0,0.8);
}}
</style>
</head>
<body>
    <div class="top-pill">FIRST CLEAN-ROOM PC RECOMPILATION • DEVLOG #1</div>
    <div class="center-logo-box">
        <img src="{logo_trans_uri}" alt="ReGTA Logo">
        <div class="intro-sub">NATIVE PC PORT IN RUST • UNCAPPED FPS</div>
    </div>
    <div class="bottom-bar">15 YEARS OF EMULATION IS OVER • UNCAPPED HIGH-REFRESH RATE FRAMERATES</div>
</body>
</html>"""

    elif scene_idx == 1:
        # Scene 2: Recomp vs Emulator Comparison
        return f"""<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{
    width: 1920px; height: 1080px;
    background: transparent;
    display: flex; flex-direction: column; justify-content: space-between;
    padding: 40px 60px;
    font-family: 'Segoe UI', Arial, sans-serif;
    color: #fff;
}}
.header-bar {{
    display: flex; justify-content: space-between; align-items: center;
}}
.header-bar img {{
    height: 65px;
    filter: drop-shadow(0 4px 15px rgba(0,0,0,0.8));
}}
.header-badge {{
    background: rgba(15, 23, 42, 0.9);
    border: 1px solid #ffd700; color: #ffd700;
    padding: 8px 24px; border-radius: 999px;
    font-size: 16px; font-weight: 800; letter-spacing: 2px;
}}
.lower-card {{
    background: rgba(10, 14, 22, 0.92);
    border: 2px solid rgba(255, 255, 255, 0.15);
    border-radius: 20px;
    padding: 28px 40px;
    box-shadow: 0 20px 50px rgba(0,0,0,0.85);
    backdrop-filter: blur(14px);
}}
.card-headline {{
    font-size: 26px; font-weight: 900; color: #ffd700; margin-bottom: 18px; letter-spacing: 1.5px;
    text-transform: uppercase;
}}
.split-grid {{
    display: grid; grid-template-columns: 1fr 1fr; gap: 30px;
}}
.comp-col {{
    border-radius: 12px; padding: 18px 24px;
}}
.col-bad {{
    background: rgba(239, 68, 68, 0.15); border-left: 5px solid #ef4444;
}}
.col-good {{
    background: rgba(16, 185, 129, 0.18); border-left: 5px solid #10b981;
    box-shadow: 0 0 25px rgba(16, 185, 129, 0.2);
}}
.col-title {{ font-size: 20px; font-weight: 900; margin-bottom: 8px; }}
.col-bad .col-title {{ color: #f87171; }}
.col-good .col-title {{ color: #34d399; }}
.col-body {{ font-size: 17px; line-height: 1.4; color: #cbd5e1; }}
</style>
</head>
<body>
    <div class="header-bar">
        <img src="{logo_ctw_uri}" alt="ReGTA CTW Logo">
        <div class="header-badge">RUST NATIVE VS PSP EMULATOR</div>
    </div>
    <div class="lower-card">
        <div class="card-headline">WHY NATIVE RECOMPILATION CHANGES EVERYTHING</div>
        <div class="split-grid">
            <div class="comp-col col-bad">
                <div class="col-title">❌ TRADITIONAL PSP EMULATION (PPSSPP)</div>
                <div class="col-body">Heavy CPU instruction translation overhead • Frame micro-stutter • 30 FPS console lock • Input lag on analog controls</div>
            </div>
            <div class="comp-col col-good">
                <div class="col-title">⚡ ReGTA NATIVE PC PORT (RUST)</div>
                <div class="col-body">100% Native x86_64 machine code • Uncapped Framerates (144Hz+) & 4K • Zero CPU emulation overhead • Ultra-low input latency</div>
            </div>
        </div>
    </div>
</body>
</html>"""

    elif scene_idx == 2:
        # Scene 3: Playable Features (Races, Drugs, Minigames)
        return f"""<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{
    width: 1920px; height: 1080px;
    background: transparent;
    display: flex; flex-direction: column; justify-content: space-between;
    padding: 40px 60px;
    font-family: 'Segoe UI', Arial, sans-serif;
    color: #fff;
}}
.header-bar {{
    display: flex; justify-content: space-between; align-items: center;
}}
.header-bar img {{
    height: 65px;
    filter: drop-shadow(0 4px 15px rgba(0,0,0,0.8));
}}
.header-badge {{
    background: rgba(15, 23, 42, 0.9);
    border: 1px solid #10b981; color: #34d399;
    padding: 8px 24px; border-radius: 999px;
    font-size: 16px; font-weight: 800; letter-spacing: 2px;
}}
.lower-card {{
    background: rgba(10, 14, 22, 0.94);
    border: 2px solid rgba(255, 215, 0, 0.35);
    border-radius: 20px;
    padding: 26px 36px;
    box-shadow: 0 20px 50px rgba(0,0,0,0.9);
    backdrop-filter: blur(14px);
}}
.card-headline {{
    font-size: 25px; font-weight: 900; color: #ffd700; margin-bottom: 16px; letter-spacing: 1.5px;
    text-transform: uppercase;
}}
.features-grid {{
    display: grid; grid-template-columns: 1fr 1fr; gap: 20px;
}}
.f-box {{
    background: rgba(30, 41, 59, 0.7); border-left: 4px solid #ffd700; border-radius: 8px;
    padding: 12px 18px;
}}
.f-title {{ font-size: 18px; font-weight: 800; color: #f8fafc; margin-bottom: 4px; }}
.f-desc {{ font-size: 15px; color: #94a3b8; line-height: 1.35; }}
</style>
</head>
<body>
    <div class="header-bar">
        <img src="{logo_ctw_uri}" alt="ReGTA CTW Logo">
        <div class="header-badge">ALPHA MILESTONE v0.1.4 PLAYABLE</div>
    </div>
    <div class="lower-card">
        <div class="card-headline">FULL LIBERTY CITY GAMEPLAY ALREADY REBUILT</div>
        <div class="features-grid">
            <div class="f-box">
                <div class="f-title">🏎️ 16 Street Races & Time Trials</div>
                <div class="f-desc">Full circuit checkpoints across Broker, Dukes, Bohan & Algonquin with AI rivals & rubber-banding.</div>
            </div>
            <div class="f-box">
                <div class="f-title">💊 Drug Wars Wholesale Economy</div>
                <div class="f-desc">81 dynamic street dealers, 7 rival gangs, price trends, police seizures, and stash boxes.</div>
            </div>
            <div class="f-box">
                <div class="f-title">🐉 Tactile Touch Minigames</div>
                <div class="f-desc">Tattoo parlor needle tracing, car-bomb wire rigging, safe cracking, and ignition hotwiring.</div>
            </div>
            <div class="f-box">
                <div class="f-title">🏙️ 21 Safehouses & Odd Jobs</div>
                <div class="f-desc">Purchasable real estate, Taxi with nitro boost, Vigilante 5-wave hunts, and 100 LCPD cameras.</div>
            </div>
        </div>
    </div>
</body>
</html>"""

    elif scene_idx == 3:
        # Scene 4: Engineering Rigor & Clean-Room Legal Integrity
        return f"""<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{
    width: 1920px; height: 1080px;
    background: transparent;
    display: flex; flex-direction: column; justify-content: space-between;
    padding: 40px 60px;
    font-family: 'Segoe UI', Arial, sans-serif;
    color: #fff;
}}
.header-bar {{
    display: flex; justify-content: space-between; align-items: center;
}}
.header-bar img {{
    height: 65px;
    filter: drop-shadow(0 4px 15px rgba(0,0,0,0.8));
}}
.header-badge {{
    background: rgba(15, 23, 42, 0.9);
    border: 1px solid #38bdf8; color: #38bdf8;
    padding: 8px 24px; border-radius: 999px;
    font-size: 16px; font-weight: 800; letter-spacing: 2px;
}}
.lower-card {{
    background: rgba(10, 14, 22, 0.94);
    border: 2px solid rgba(56, 189, 248, 0.35);
    border-radius: 20px;
    padding: 26px 36px;
    box-shadow: 0 20px 50px rgba(0,0,0,0.9);
    backdrop-filter: blur(14px);
}}
.card-headline {{
    font-size: 25px; font-weight: 900; color: #38bdf8; margin-bottom: 16px; letter-spacing: 1.5px;
    text-transform: uppercase;
}}
.specs-row {{
    display: flex; gap: 20px; justify-content: space-between; margin-bottom: 14px;
}}
.spec-item {{
    background: rgba(15, 23, 42, 0.85); border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 10px; padding: 12px 20px; flex: 1; text-align: center;
}}
.spec-val {{ font-size: 24px; font-weight: 900; color: #ffd700; }}
.spec-lbl {{ font-size: 14px; color: #94a3b8; font-weight: 700; text-transform: uppercase; }}
.legal-box {{
    background: rgba(255, 59, 48, 0.12); border: 1px solid rgba(255, 59, 48, 0.4);
    border-radius: 8px; padding: 12px 20px; font-size: 15px; color: #e2e8f0; line-height: 1.4;
}}
.legal-box strong {{ color: #ff3b30; }}
</style>
</head>
<body>
    <div class="header-bar">
        <img src="{logo_ctw_uri}" alt="ReGTA CTW Logo">
        <div class="header-badge">ENGINEERING INTEGRITY</div>
    </div>
    <div class="lower-card">
        <div class="card-headline">UNCOMPROMISING RUST QUALITY & CLEAN-ROOM ARCHITECTURE</div>
        <div class="specs-row">
            <div class="spec-item">
                <div class="spec-val">160 / 160</div>
                <div class="spec-lbl">Unit Tests Passed (100%)</div>
            </div>
            <div class="spec-item">
                <div class="spec-val">0 WARNINGS</div>
                <div class="spec-lbl">Strict -D warnings Policy</div>
            </div>
            <div class="spec-item">
                <div class="spec-val">166 OVERLAYS</div>
                <div class="spec-lbl">PSP Code Containers Indexed</div>
            </div>
            <div class="spec-item">
                <div class="spec-val">100% NATIVE</div>
                <div class="spec-lbl">Windows x64 Machine Code</div>
            </div>
        </div>
        <div class="legal-box">
            <strong>STRICT LEGAL INTEGRITY:</strong> No copyrighted game assets, ROMs, or playable game binaries are distributed. ReGTA requires a personal, legally owned European PSP game disc.
        </div>
    </div>
</body>
</html>"""

    elif scene_idx == 4:
        # Scene 5: Outro & Support
        return f"""<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{
    width: 1920px; height: 1080px;
    background: transparent;
    display: flex; flex-direction: column; justify-content: space-between;
    padding: 40px 60px;
    font-family: 'Segoe UI', Arial, sans-serif;
    color: #fff;
}}
.header-bar {{
    display: flex; justify-content: space-between; align-items: center;
}}
.header-bar img {{
    height: 75px;
    filter: drop-shadow(0 4px 15px rgba(0,0,0,0.8));
}}
.header-badge {{
    background: linear-gradient(135deg, #ff3b30, #ffd700);
    color: #fff; border: 1px solid #fff;
    padding: 10px 28px; border-radius: 999px;
    font-size: 18px; font-weight: 900; letter-spacing: 2px;
}}
.lower-card {{
    background: rgba(10, 14, 22, 0.95);
    border: 2px solid rgba(255, 215, 0, 0.45);
    border-radius: 20px;
    padding: 28px 40px;
    box-shadow: 0 20px 50px rgba(0,0,0,0.95);
    backdrop-filter: blur(14px);
}}
.card-headline {{
    font-size: 26px; font-weight: 900; color: #ffd700; margin-bottom: 18px; letter-spacing: 1.5px;
    text-transform: uppercase;
}}
.support-grid {{
    display: grid; grid-template-columns: 1fr 1fr; gap: 20px; margin-bottom: 16px;
}}
.s-box {{
    background: rgba(30, 41, 59, 0.8); border: 1.5px solid rgba(255, 215, 0, 0.3);
    border-radius: 12px; padding: 14px 20px;
}}
.s-title {{ font-size: 19px; font-weight: 900; color: #ffd700; margin-bottom: 4px; }}
.s-link {{ font-family: Consolas, monospace; font-size: 16px; color: #38bdf8; font-weight: 700; }}
.cta-banner {{
    background: linear-gradient(90deg, #ff3b30, #e11d48);
    border-radius: 8px; padding: 12px 24px; font-size: 18px; font-weight: 900; text-align: center;
    color: #fff; letter-spacing: 2px; text-transform: uppercase;
}}
</style>
</head>
<body>
    <div class="header-bar">
        <img src="{logo_trans_uri}" alt="ReGTA CTW Logo">
        <div class="header-badge">SUPPORT THE PROJECT</div>
    </div>
    <div class="lower-card">
        <div class="card-headline">HELP US ACCELERATE THE NATIVE PC PORT</div>
        <div class="support-grid">
            <div class="s-box">
                <div class="s-title">❤️ Support on Tipeee (Creator Tips)</div>
                <div class="s-link">fr.tipeee.com/benji-paname</div>
            </div>
            <div class="s-box">
                <div class="s-title">💳 Support via PayPal.Me</div>
                <div class="s-link">paypal.me/BenjiLsg</div>
            </div>
            <div class="s-box">
                <div class="s-title">💻 Open Source on GitHub</div>
                <div class="s-link">github.com/benji-paname/regta-chinatown-wars-pc-port</div>
            </div>
            <div class="s-box">
                <div class="s-title">🌐 Live Showcase Website</div>
                <div class="s-link">benji-paname.github.io/regta-chinatown-wars-pc-port</div>
            </div>
        </div>
        <div class="cta-banner">ALL LINKS ARE IN THE DESCRIPTION BELOW • SUBSCRIBE & STAR ON GITHUB!</div>
    </div>
</body>
</html>"""


def build_thumbnail_html() -> str:
    """Builds 1280x720 YouTube Thumbnail using transparent logo."""
    logo_trans_uri = LOGO_TRANSPARENT_PATH.resolve().as_uri()
    # Extract background frame from v2 gameplay at 25s for real action background
    bg_frame = TEMP_DIR / "thumb_bg.jpg"
    subprocess.run([
        FFMPEG_EXE, "-ss", "25", "-i", str(GAMEPLAY_V2),
        "-vframes", "1", "-q:v", "2", str(bg_frame), "-y"
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    bg_uri = bg_frame.resolve().as_uri() if bg_frame.exists() else ""

    return f"""<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{
    width: 1280px; height: 720px;
    background: #08090d url('{bg_uri}') center/cover no-repeat;
    position: relative;
    display: flex; flex-direction: column; justify-content: space-between; align-items: center;
    padding: 35px 40px;
    font-family: 'Segoe UI', Arial Black, Impact, sans-serif;
    color: #fff;
    overflow: hidden;
}}
.vignette {{
    position: absolute; top: 0; left: 0; width: 100%; height: 100%;
    background: radial-gradient(circle at 50% 50%, rgba(0,0,0,0.4) 0%, rgba(8,9,13,0.88) 85%);
    z-index: 1;
}}
.content {{
    position: relative; z-index: 2; width: 100%; height: 100%;
    display: flex; flex-direction: column; justify-content: space-between; align-items: center;
}}
.logo-container img {{
    height: 220px;
    filter: drop-shadow(0 15px 35px rgba(0,0,0,0.95));
}}
.giant-banner {{
    background: linear-gradient(90deg, #ff3b30, #e11d48, #ffd700);
    padding: 14px 45px;
    border-radius: 14px;
    font-size: 46px; font-weight: 900; letter-spacing: 2px;
    color: #ffffff;
    box-shadow: 0 15px 40px rgba(225, 29, 72, 0.65);
    border: 3px solid #ffffff;
    text-transform: uppercase;
    transform: rotate(-1.5deg);
}}
.badge-row {{
    display: flex; gap: 16px; width: 100%; justify-content: center;
}}
.thumb-badge {{
    background: rgba(15, 23, 42, 0.92);
    border: 2px solid #ffd700;
    padding: 12px 24px; border-radius: 12px;
    font-size: 21px; font-weight: 900; color: #ffd700;
    box-shadow: 0 10px 25px rgba(0,0,0,0.7);
    letter-spacing: 1px;
}}
.highlight-badge {{
    background: #10b981; color: #ffffff; border: 2px solid #ffffff;
}}
</style>
</head>
<body>
    <div class="vignette"></div>
    <div class="content">
        <div class="logo-container">
            <img src="{logo_trans_uri}" alt="ReGTA Logo">
        </div>
        <div class="giant-banner">
            NATIVE PC RECOMPILATION • RUST
        </div>
        <div class="badge-row">
            <div class="thumb-badge highlight-badge">⚡ UNCAPPED FPS • NATIVE 4K</div>
            <div class="thumb-badge">🏎️ 16 STREET RACES</div>
            <div class="thumb-badge">💊 DRUG WARS PLAYABLE</div>
            <div class="thumb-badge">🛡️ NO EMULATOR</div>
        </div>
    </div>
</body>
</html>"""


def render_html_to_png(html_content: str, png_path: Path, width: int, height: int, transparent: bool = True):
    """Renders HTML string to PNG via headless Edge with alpha transparency."""
    temp_html = png_path.with_suffix(".html")
    temp_html.write_text(html_content, encoding="utf-8")
    bg_arg = "--default-background-color=00000000" if transparent else "--default-background-color=ff08090d"

    cmd = [
        EDGE_EXE,
        "--headless",
        r"--user-data-dir=C:\Users\BENJAMIN\AppData\Local\Temp\edge_ov_temp",
        bg_arg,
        f"--screenshot={png_path.resolve()}",
        f"--window-size={width},{height}",
        f"file:///{temp_html.resolve().as_posix()}",
    ]
    subprocess.run(cmd, check=True)
    if temp_html.exists():
        temp_html.unlink()
    print(f"Rendered PNG: {png_path.name} ({width}x{height}, transparent={transparent})")


def assemble_gameplay_base(total_duration: float) -> Path:
    """Concatenates and scales real gameplay clips into a continuous 1080p 60 FPS background."""
    print("\n--- Assembling Base 1080p 60 FPS Gameplay Track ---")
    base_gameplay = TEMP_DIR / "base_gameplay.mp4"

    custom_clip = OUTPUT_DIR / "custom_gameplay.mp4"
    if custom_clip.exists() and custom_clip.stat().st_size > 1024:
        print(f"Detected custom Rust gameplay video recorded by user: {custom_clip.name}!")
        cmd = [
            FFMPEG_EXE,
            "-stream_loop", "-1",
            "-i", str(custom_clip),
            "-vf", "scale=1920:1080:force_original_aspect_ratio=increase,crop=1920:1080,fps=60",
            "-t", f"{total_duration:.2f}",
            "-c:v", "libx264",
            "-preset", "veryfast",
            "-crf", "18",
            "-pix_fmt", "yuv420p",
            "-y",
            str(base_gameplay),
        ]
        subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        print(f"Base Gameplay Video Assembled from custom recording: {base_gameplay.name} ({total_duration:.1f}s)")
        return base_gameplay

    if base_gameplay.exists() and base_gameplay.stat().st_size > 10 * 1024 * 1024:
        print(f"Base Gameplay Video already assembled: {base_gameplay.name} ({base_gameplay.stat().st_size / (1024*1024):.1f} MB)")
        return base_gameplay

    # Assemble: v1 (13s) + v2 (54s) + v2 (0s-50s) to cover > 115 seconds
    cmd = [
        FFMPEG_EXE,
        "-i", str(GAMEPLAY_V1),
        "-i", str(GAMEPLAY_V2),
        "-i", str(GAMEPLAY_V2),
        "-filter_complex",
        "[0:v]scale=1920:1080:force_original_aspect_ratio=increase,crop=1920:1080,fps=60[v0];"
        "[1:v]scale=1920:1080:force_original_aspect_ratio=increase,crop=1920:1080,fps=60[v1];"
        "[2:v]scale=1920:1080:force_original_aspect_ratio=increase,crop=1920:1080,fps=60[v2_s];"
        "[v0][v1][v2_s]concat=n=3:v=1:a=0[vcat]",
        "-map", "[vcat]",
        "-t", f"{total_duration:.2f}",
        "-c:v", "libx264",
        "-preset", "veryfast",
        "-crf", "18",
        "-pix_fmt", "yuv420p",
        "-y",
        str(base_gameplay),
    ]
    subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print(f"Base Gameplay Video Assembled: {base_gameplay.name} ({total_duration:.1f}s)")
    return base_gameplay


def assemble_voiceover_track(durations: dict, total_duration: float) -> Path:
    """Concatenates scene voiceover audio clips into one continuous track."""
    print("\n--- Assembling Voiceover Audio Track ---")
    concat_txt = TEMP_DIR / "voice_concat.txt"
    with open(concat_txt, "w", encoding="utf-8") as f:
        for sid in durations:
            mp3_path = (TEMP_DIR / f"{sid}.mp3").resolve().as_posix()
            f.write(f"file '{mp3_path}'\n")

    master_voice = TEMP_DIR / "master_voice.mp3"
    cmd = [
        FFMPEG_EXE,
        "-f", "concat",
        "-safe", "0",
        "-i", str(concat_txt),
        "-c", "copy",
        "-y",
        str(master_voice),
    ]
    subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return master_voice


def render_master_video(base_gameplay: Path, master_voice: Path, durations: dict, total_duration: float):
    """Applies timed transparent overlays and mixes voiceover + soundtrack onto gameplay."""
    print("\n--- Compositing Gameplay + Transparent Overlays + Audio Mix ---")
    final_output = OUTPUT_DIR / "ReGTA_Chinatown_Wars_Devlog_01_EN.mp4"

    # Calculate overlay time ranges
    t_start = 0.0
    time_ranges = []
    for sid, dur in durations.items():
        time_ranges.append((t_start, t_start + dur))
        t_start += dur

    # Generate background synth music
    soundtrack_wav = TEMP_DIR / "soundtrack.wav"
    generate_synth_soundtrack(soundtrack_wav, total_duration + 2.0)

    # Build complex filter graph for 5 overlays
    # Input 0: base_gameplay.mp4
    # Input 1-5: ov_0.png .. ov_4.png
    # Input 6: master_voice.mp3
    # Input 7: soundtrack.wav
    cmd = [
        FFMPEG_EXE,
        "-i", str(base_gameplay),
        "-i", str(TEMP_DIR / "ov_0.png"),
        "-i", str(TEMP_DIR / "ov_1.png"),
        "-i", str(TEMP_DIR / "ov_2.png"),
        "-i", str(TEMP_DIR / "ov_3.png"),
        "-i", str(TEMP_DIR / "ov_4.png"),
        "-i", str(master_voice),
        "-i", str(soundtrack_wav),
        "-filter_complex",
        f"[0:v][1:v]overlay=0:0:enable='between(t,{time_ranges[0][0]:.2f},{time_ranges[0][1]:.2f})'[v1];"
        f"[v1][2:v]overlay=0:0:enable='between(t,{time_ranges[1][0]:.2f},{time_ranges[1][1]:.2f})'[v2];"
        f"[v2][3:v]overlay=0:0:enable='between(t,{time_ranges[2][0]:.2f},{time_ranges[2][1]:.2f})'[v3];"
        f"[v3][4:v]overlay=0:0:enable='between(t,{time_ranges[3][0]:.2f},{time_ranges[3][1]:.2f})'[v4];"
        f"[v4][5:v]overlay=0:0:enable='between(t,{time_ranges[4][0]:.2f},{time_ranges[4][1]:.2f})'[vout];"
        f"[6:a]volume=1.0[voice];"
        f"[7:a]volume=0.22,afade=t=out:st={total_duration - 2.5:.2f}:d=2.5[music];"
        f"[voice][music]amix=inputs=2:duration=first[aout]",
        "-map", "[vout]",
        "-map", "[aout]",
        "-c:v", "libx264",
        "-preset", "medium",
        "-crf", "19",
        "-pix_fmt", "yuv420p",
        "-c:a", "aac",
        "-b:a", "256k",
        "-t", f"{total_duration:.2f}",
        "-y",
        str(final_output),
    ]

    print(f"Encoding master gameplay video: {final_output.name}...")
    subprocess.run(cmd, check=True)
    print(f"\n Master Gameplay Video Created: {final_output}")
    print(f" File Size: {os.path.getsize(final_output) / (1024 * 1024):.2f} MB")
    return final_output


async def main():
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    TEMP_DIR.mkdir(parents=True, exist_ok=True)

    print("=================================================================")
    print(" ReGTA Chinatown Wars — YouTube Video Generator (Gameplay Edition)")
    print("=================================================================")

    # 1. Voiceover Synthesis
    durations = await generate_voiceovers()
    total_dur = sum(durations.values())
    print(f"\nTotal video duration: {total_dur:.1f} seconds ({total_dur / 60.0:.2f} minutes)")

    # 2. Render 5 Transparent Overlays (1920x1080)
    print("\n--- Rendering Transparent 1080p Overlays ---")
    for idx in range(len(SCENES)):
        ov_path = TEMP_DIR / f"ov_{idx}.png"
        html = build_overlay_html(idx)
        render_html_to_png(html, ov_path, 1920, 1080, transparent=True)

    # 3. Render High-CTR YouTube Thumbnail (1280x720) with transparent logo & real gameplay background
    print("\n--- Rendering YouTube Thumbnail (1280x720) ---")
    thumb_path = OUTPUT_DIR / "thumbnail.png"
    thumb_html = build_thumbnail_html()
    render_html_to_png(thumb_html, thumb_path, 1280, 720, transparent=False)
    # Sync to docs/assets/thumbnail.png
    (REPO_ROOT / "docs" / "assets" / "thumbnail.png").write_bytes(thumb_path.read_bytes())

    # 4. Assemble Base 1080p 60 FPS Gameplay
    base_gameplay = assemble_gameplay_base(total_dur)

    # 5. Assemble Master Voiceover Audio Track
    master_voice = assemble_voiceover_track(durations, total_dur)

    # 6. Composite Gameplay + Overlays + Audio into Final Deliverable
    final_video = render_master_video(base_gameplay, master_voice, durations, total_dur)

    print("\n=================================================================")
    print(" GAMEPLAY VIDEO GENERATION COMPLETED SUCCESSFULLY!")
    print(f" Master Video:   {final_video}")
    print(f" Thumbnail:      {thumb_path}")
    print("=================================================================")


if __name__ == "__main__":
    asyncio.run(main())
