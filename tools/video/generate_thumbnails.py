"""Generates high-CTR, professional 1280x720 YouTube Thumbnail variants
using Benjamin's actual Rust gameplay recordings and official brand logos.
"""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DOCS_ASSETS = ROOT / "docs" / "assets"
FRAMES_DIR = ROOT / "outputs" / "CTW-Rust" / "video" / "thumb_frames"
OUTPUT_DIR = ROOT / "outputs" / "CTW-Rust" / "video" / "thumbnails"
EDGE_EXE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"

OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

LOGO_VERT_URI = (DOCS_ASSETS / "logo-transparent.png").resolve().as_uri()
LOGO_HORIZ_URI = (DOCS_ASSETS / "logo-ctw-transparent.png").resolve().as_uri()

BG_MOTO_URI = (FRAMES_DIR / "moto_boulevard.jpg").resolve().as_uri()
BG_CAR_URI = (FRAMES_DIR / "car_hellenbach.jpg").resolve().as_uri()
BG_SPARK_URI = (FRAMES_DIR / "sprint_radar.jpg").resolve().as_uri()


def render_html_to_png(html_content: str, png_path: Path):
    temp_html = png_path.with_suffix(".html")
    temp_html.write_text(html_content, encoding="utf-8")
    cmd = [
        EDGE_EXE,
        "--headless",
        r"--user-data-dir=C:\Users\BENJAMIN\AppData\Local\Temp\edge_thumb_temp",
        "--default-background-color=ff08090d",
        f"--screenshot={png_path.resolve()}",
        "--window-size=1280,720",
        f"file:///{temp_html.resolve().as_posix()}",
    ]
    subprocess.run(cmd, check=True)
    if temp_html.exists():
        temp_html.unlink()
    print(f"Generated thumbnail: {png_path.name} ({png_path.stat().st_size // 1024} KB)")


def generate_style_1_action_fr():
    """Style 1 (Recommandé) : Action Sanchez, Logo officiel, Bandeau massif 'ENFIN SUR PC !'"""
    html = f"""<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{
    width: 1280px; height: 720px;
    background: #08090d url('{BG_MOTO_URI}') center/cover no-repeat;
    position: relative;
    display: flex; flex-direction: column; justify-content: space-between; align-items: center;
    padding: 30px 40px;
    font-family: 'Segoe UI', Arial Black, Impact, sans-serif;
    color: #fff;
    overflow: hidden;
}}
.vignette {{
    position: absolute; top: 0; left: 0; width: 100%; height: 100%;
    background: radial-gradient(circle at 50% 55%, rgba(0,0,0,0.25) 0%, rgba(8,9,13,0.88) 85%);
    z-index: 1;
}}
.content {{
    position: relative; z-index: 2; width: 100%; height: 100%;
    display: flex; flex-direction: column; justify-content: space-between; align-items: center;
}}
.top-row {{
    display: flex; justify-content: space-between; align-items: center; width: 100%;
}}
.logo-container img {{
    height: 180px;
    filter: drop-shadow(0 12px 28px rgba(0,0,0,0.98)) drop-shadow(0 0 25px rgba(255, 59, 48, 0.4));
}}
.tag-rust {{
    background: linear-gradient(135deg, #e11d48, #be123c);
    border: 3px solid #ffffff;
    border-radius: 12px;
    padding: 10px 24px;
    font-size: 24px; font-weight: 900;
    color: #fff;
    letter-spacing: 2px;
    text-transform: uppercase;
    box-shadow: 0 10px 30px rgba(225, 29, 72, 0.7);
    transform: rotate(2deg);
}}
.giant-banner {{
    background: linear-gradient(90deg, #ff1a1a 0%, #e11d48 40%, #ffd700 100%);
    padding: 16px 55px;
    border-radius: 16px;
    font-size: 56px; font-weight: 900; letter-spacing: 3px;
    color: #ffffff;
    box-shadow: 0 15px 45px rgba(0, 0, 0, 0.9), 0 0 35px rgba(225, 29, 72, 0.8);
    border: 4px solid #ffffff;
    text-transform: uppercase;
    transform: rotate(-2deg);
    text-shadow: 0 4px 10px rgba(0,0,0,0.8);
}}
.badge-row {{
    display: flex; gap: 16px; width: 100%; justify-content: center;
}}
.thumb-badge {{
    background: rgba(10, 15, 29, 0.94);
    border: 2px solid #ffd700;
    padding: 12px 26px; border-radius: 14px;
    font-size: 22px; font-weight: 900; color: #ffd700;
    box-shadow: 0 10px 25px rgba(0,0,0,0.85);
    letter-spacing: 1.5px;
    text-transform: uppercase;
}}
.badge-highlight {{
    background: #10b981; color: #ffffff; border: 3px solid #ffffff;
    box-shadow: 0 10px 30px rgba(16, 185, 129, 0.7);
}}
</style>
</head>
<body>
    <div class="vignette"></div>
    <div class="content">
        <div class="top-row">
            <div class="logo-container">
                <img src="{LOGO_HORIZ_URI}" alt="ReGTA CTW Logo">
            </div>
            <div class="tag-rust">
                🦀 100% RUST NATIF
            </div>
        </div>
        <div class="giant-banner">
            ENFIN SUR PC !
        </div>
        <div class="badge-row">
            <div class="thumb-badge badge-highlight">⚡ FPS ILLIMITÉS • 4K</div>
            <div class="thumb-badge">🛡️ SANS ÉMULATEUR</div>
            <div class="thumb-badge">🎮 GAMEPLAY RÉEL</div>
        </div>
    </div>
</body>
</html>"""
    out = OUTPUT_DIR / "thumbnail_style1_action_fr.png"
    render_html_to_png(html, out)
    return out


def generate_style_2_muscle_car_fr():
    """Style 2 : Bolide Hellenbach, gros logo vertical, badge 'PORTAGE PC EN RUST'"""
    html = f"""<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{
    width: 1280px; height: 720px;
    background: #08090d url('{BG_CAR_URI}') center/cover no-repeat;
    position: relative;
    display: flex; flex-direction: column; justify-content: space-between; align-items: center;
    padding: 35px 45px;
    font-family: 'Segoe UI', Arial Black, Impact, sans-serif;
    color: #fff;
    overflow: hidden;
}}
.vignette {{
    position: absolute; top: 0; left: 0; width: 100%; height: 100%;
    background: radial-gradient(circle at 60% 50%, rgba(0,0,0,0.3) 0%, rgba(8,9,13,0.92) 80%);
    z-index: 1;
}}
.content {{
    position: relative; z-index: 2; width: 100%; height: 100%;
    display: flex; flex-direction: column; justify-content: space-between;
}}
.header-box {{
    display: flex; align-items: center; justify-content: space-between;
}}
.logo-box img {{
    height: 220px;
    filter: drop-shadow(0 15px 30px rgba(0,0,0,0.95)) drop-shadow(0 0 20px rgba(255,215,0,0.3));
}}
.title-side {{
    text-align: right;
}}
.title-main {{
    font-size: 54px; font-weight: 900; line-height: 1.1;
    color: #ffd700;
    text-shadow: 0 4px 15px rgba(0,0,0,0.9), 0 0 25px rgba(255, 215, 0, 0.5);
    text-transform: uppercase;
}}
.title-sub {{
    font-size: 32px; font-weight: 900; color: #ffffff;
    background: rgba(225, 29, 72, 0.9);
    display: inline-block; padding: 6px 18px; border-radius: 8px; margin-top: 10px;
    border: 2px solid #fff;
}}
.mid-banner {{
    background: rgba(15, 23, 42, 0.92);
    border-left: 8px solid #10b981;
    padding: 15px 25px; border-radius: 10px;
    font-size: 26px; font-weight: 800; color: #fff;
    max-width: 700px;
    box-shadow: 0 10px 30px rgba(0,0,0,0.8);
}}
.badge-row {{
    display: flex; gap: 16px;
}}
.thumb-badge {{
    background: rgba(10, 15, 29, 0.94);
    border: 2px solid #ffd700;
    padding: 12px 24px; border-radius: 12px;
    font-size: 21px; font-weight: 900; color: #ffd700;
    box-shadow: 0 10px 25px rgba(0,0,0,0.85);
}}
.badge-highlight {{
    background: #10b981; color: #ffffff; border: 3px solid #ffffff;
}}
</style>
</head>
<body>
    <div class="vignette"></div>
    <div class="content">
        <div class="header-box">
            <div class="logo-box">
                <img src="{LOGO_VERT_URI}" alt="ReGTA CTW Logo">
            </div>
            <div class="title-side">
                <div class="title-main">PORTAGE PC NATIF</div>
                <div class="title-sub">🦀 CODE MACHINE RUST</div>
            </div>
        </div>
        <div class="mid-banner">
            🚀 15 ANS APRÈS : ADIEU L'ÉMULATEUR !
        </div>
        <div class="badge-row">
            <div class="thumb-badge badge-highlight">⚡ FPS ILLIMITÉS (144Hz+)</div>
            <div class="thumb-badge">🛡️ ZÉRO ÉMULATEUR</div>
            <div class="thumb-badge">🏎️ 16 COURSES & DEALERS</div>
        </div>
    </div>
</body>
</html>"""
    out = OUTPUT_DIR / "thumbnail_style2_muscle_car_fr.png"
    render_html_to_png(html, out)
    return out


def generate_style_3_international_en():
    """Style 3 : Version anglaise / internationale 'FINALLY ON PC !'"""
    html = f"""<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{
    width: 1280px; height: 720px;
    background: #08090d url('{BG_MOTO_URI}') center/cover no-repeat;
    position: relative;
    display: flex; flex-direction: column; justify-content: space-between; align-items: center;
    padding: 30px 40px;
    font-family: 'Segoe UI', Arial Black, Impact, sans-serif;
    color: #fff;
    overflow: hidden;
}}
.vignette {{
    position: absolute; top: 0; left: 0; width: 100%; height: 100%;
    background: radial-gradient(circle at 50% 50%, rgba(0,0,0,0.3) 0%, rgba(8,9,13,0.88) 85%);
    z-index: 1;
}}
.content {{
    position: relative; z-index: 2; width: 100%; height: 100%;
    display: flex; flex-direction: column; justify-content: space-between; align-items: center;
}}
.top-box {{
    display: flex; justify-content: space-between; align-items: center; width: 100%;
}}
.logo-container img {{
    height: 175px;
    filter: drop-shadow(0 12px 28px rgba(0,0,0,0.98)) drop-shadow(0 0 25px rgba(255, 59, 48, 0.4));
}}
.badge-devlog {{
    background: linear-gradient(135deg, #ffd700, #f59e0b);
    color: #08090d;
    padding: 10px 26px; border-radius: 12px;
    font-size: 24px; font-weight: 900; letter-spacing: 2px;
    box-shadow: 0 10px 30px rgba(255, 215, 0, 0.6);
    border: 3px solid #ffffff;
}}
.giant-banner {{
    background: linear-gradient(90deg, #ff1a1a 0%, #e11d48 40%, #ffd700 100%);
    padding: 16px 55px;
    border-radius: 16px;
    font-size: 58px; font-weight: 900; letter-spacing: 3px;
    color: #ffffff;
    box-shadow: 0 15px 45px rgba(0, 0, 0, 0.9), 0 0 35px rgba(225, 29, 72, 0.8);
    border: 4px solid #ffffff;
    text-transform: uppercase;
    transform: rotate(-1.5deg);
    text-shadow: 0 4px 10px rgba(0,0,0,0.8);
}}
.badge-row {{
    display: flex; gap: 16px; width: 100%; justify-content: center;
}}
.thumb-badge {{
    background: rgba(10, 15, 29, 0.94);
    border: 2px solid #ffd700;
    padding: 12px 26px; border-radius: 14px;
    font-size: 22px; font-weight: 900; color: #ffd700;
    box-shadow: 0 10px 25px rgba(0,0,0,0.85);
    letter-spacing: 1.5px;
    text-transform: uppercase;
}}
.badge-highlight {{
    background: #10b981; color: #ffffff; border: 3px solid #ffffff;
    box-shadow: 0 10px 30px rgba(16, 185, 129, 0.7);
}}
</style>
</head>
<body>
    <div class="vignette"></div>
    <div class="content">
        <div class="top-box">
            <div class="logo-container">
                <img src="{LOGO_HORIZ_URI}" alt="ReGTA CTW Logo">
            </div>
            <div class="badge-devlog">
                ⚡ DEVLOG #1
            </div>
        </div>
        <div class="giant-banner">
            FINALLY ON PC !
        </div>
        <div class="badge-row">
            <div class="thumb-badge badge-highlight">⚡ UNCAPPED FPS • 4K</div>
            <div class="thumb-badge">🦀 100% RUST PORT</div>
            <div class="thumb-badge">🛡️ NO EMULATOR</div>
        </div>
    </div>
</body>
</html>"""
    out = OUTPUT_DIR / "thumbnail_style3_international_en.png"
    render_html_to_png(html, out)
    return out


def main():
    print("--- Generating High-CTR Thumbnail Variants ---")
    s1 = generate_style_1_action_fr()
    s2 = generate_style_2_muscle_car_fr()
    s3 = generate_style_3_international_en()

    # Default thumbnail set to Style 1 (Action + ENFIN SUR PC !)
    dest_docs = DOCS_ASSETS / "thumbnail.png"
    dest_docs.write_bytes(s1.read_bytes())
    dest_video = ROOT / "outputs" / "CTW-Rust" / "video" / "thumbnail.png"
    dest_video.write_bytes(s1.read_bytes())
    dest_launch = ROOT / "outputs" / "CTW-Rust" / "video" / "launch-film" / "thumbnail.png"
    if dest_launch.parent.exists():
        dest_launch.write_bytes(s1.read_bytes())

    print(f"\nDefault thumbnail updated with Style 1 -> {dest_docs}")


if __name__ == "__main__":
    main()
