"""English launch film. Real supplied native gameplay only; no emulator fallback.
Usage: python tools/video/build_launch_film.py voice|assets|render|verify
Dependencies: Pillow, edge-tts (voice only), imageio-ffmpeg. Fonts stay private.
"""
import asyncio, json, math, re, subprocess, sys, wave, array, shutil
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
import imageio_ffmpeg
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'outputs/CTW-Rust/video/launch-film'
TMP=OUT/'work';TMP.mkdir(parents=True,exist_ok=True)
CONFIG=json.loads((ROOT/'tools/video/launch_film.json').read_text(encoding='utf-8-sig'))
SOURCE=ROOT/'outputs/CTW-Rust/video/custom_gameplay.mp4'
FF=imageio_ffmpeg.get_ffmpeg_exe()
BOLD='C:/Windows/Fonts/arialbd.ttf';REGULAR='C:/Windows/Fonts/arial.ttf'
W,H=1920,1080

def run(args):
    p=subprocess.run([FF,'-hide_banner','-loglevel','error',*map(str,args)],capture_output=True,text=True)
    if p.returncode:raise RuntimeError(p.stderr)

def duration(path):
    r=subprocess.run([FF,'-i',str(path)],capture_output=True,text=True)
    m=re.search(r'Duration: (\d+):(\d+):([\d.]+)',r.stderr)
    if not m:raise RuntimeError('No duration: '+str(path))
    return int(m[1])*3600+int(m[2])*60+float(m[3])

async def voice():
    import edge_tts
    for s in CONFIG['scenes']:
        mp3=TMP/f"voice-{s['id']}.mp3";wordfile=TMP/f"words-{s['id']}.json"
        if mp3.exists() and mp3.stat().st_size>1000 and wordfile.exists():
            print('Reusing voice',s['id'],flush=True);continue
        for attempt in range(3):
            words=[]
            try:
                comm=edge_tts.Communicate(s['voice'],CONFIG['voice'],rate='-3%',boundary='WordBoundary')
                with mp3.open('wb') as f:
                    async for chunk in comm.stream():
                        if chunk['type']=='audio':f.write(chunk['data'])
                        elif chunk['type']=='WordBoundary':words.append({k:chunk[k] for k in ['offset','duration','text']})
                wordfile.write_text(json.dumps(words),encoding='utf-8')
                print('Voice',s['id'],duration(mp3),flush=True);break
            except Exception:
                if attempt==2:raise
                await asyncio.sleep(2)

def timeline():
    start=0;scenes=[]
    for s in CONFIG['scenes']:
        q=dict(s);q['start']=start;q['speech']=duration(TMP/f"voice-{s['id']}.mp3")
        q['duration']=max(9 if s.get('end_card') else 6,q['speech']+1.0);start+=q['duration'];scenes.append(q)
    return scenes,start

def txt(draw,xy,text,size=32,color='#fffdf5',bold=False):
    draw.text(xy,text,font=ImageFont.truetype(BOLD if bold else REGULAR,size),fill=color)

def logo(im,stem,box):
    asset=Image.open(ROOT/f'docs/assets/{stem}.png').convert('RGBA');asset.thumbnail(box[2:],Image.Resampling.LANCZOS);im.alpha_composite(asset,box[:2])

def assets():
    scenes,total=timeline()
    for s in scenes:
        im=Image.new('RGBA',(W,H),(0,0,0,0));d=ImageDraw.Draw(im)
        if s.get('end_card'):
            d.rectangle((0,0,W,H),fill=(10,12,15,245));d.polygon([(0,0),(730,0),(550,H),(0,H)],fill=(26,20,18,255))
            logo(im,'logo',(60,240,710,430))
            txt(d,(840,205),'HELP BUILD',76,bold=True);txt(d,(840,290),"WHAT'S NEXT.",76,'#ffda22',True)
            txt(d,(845,425),'by Benji Paname',35,bold=True)
            d.rectangle((845,500,1750,504),fill='#d90000')
            txt(d,(845,548),'SUPPORT ON TIPEEE',29,'#ffda22',True)
            txt(d,(845,595),'fr.tipeee.com/benji-paname',32)
            txt(d,(845,670),'OR PAYPAL',26,'#ffda22',True)
            txt(d,(845,715),'paypal.me/BenjiLsg',32)
            txt(d,(845,815),'Follow the project on GitHub. Links below.',27)
            txt(d,(80,985),'Voluntary support. No game purchase, early access or release-date promise.',25,'#bfc0c4')
        else:
            # Compact status mark; the centre and original minimap remain visible.
            d.rounded_rectangle((710,28,1210,84),radius=5,fill=(8,9,12,215))
            d.ellipse((734,48,748,62),fill='#d90000');txt(d,(765,43),'PRE-ALPHA  /  NATIVE PC',24,bold=True)
            # Localized lower-third scrim, never a full screen feature dashboard.
            for y in range(815,1025):
                alpha=int(200*min(1,(y-815)/60));d.line((390,y,1850,y),fill=(7,9,12,alpha))
            d.rectangle((430,866,436,970),fill='#d90000')
            txt(d,(460,858),s['label'],23,'#ffda22',True)
            size=44
            while d.textlength(s['headline'],font=ImageFont.truetype(BOLD,size))>1320:size-=1
            txt(d,(460,910),s['headline'],size,bold=True)
        im.save(TMP/f"overlay-{s['id']}.png")
    # Thumbnail uses an actual frame, typography and vector branding, no invented scene.
    run(['-ss',99,'-i',SOURCE,'-frames:v',1,'-vf','scale=1280:720','-y',TMP/'thumbnail-frame.png'])
    im=Image.open(TMP/'thumbnail-frame.png').convert('RGBA');shade=Image.new('RGBA',im.size);d=ImageDraw.Draw(shade)
    for x in range(1280):
        a=int(240*max(0,1-x/1100));d.line((x,0,x,720),fill=(5,7,10,a))
    im=Image.alpha_composite(im,shade);d=ImageDraw.Draw(im)
    d.rectangle((40,44,250,83),fill='#d90000');txt(d,(54,51),'FIRST LOOK',22,bold=True)
    logo(im,'logo',(30,105,590,354))
    txt(d,(55,480),'A PC FUTURE',63,'#fffdf5',True)
    txt(d,(59,573),'REAL PROTOTYPE GAMEPLAY',22,'#ffda22',True)
    txt(d,(59,630),'by Benji Paname',25,bold=True)
    im.convert('RGB').save(OUT/'thumbnail.jpg',quality=95)
    im.save(OUT/'thumbnail.png');shutil.copy2(OUT/'thumbnail.png',ROOT/'docs/assets/thumbnail.png')
    (OUT/'timeline.json').write_text(json.dumps({'duration':total,'scenes':scenes},indent=2),encoding='utf-8')
    # YouTube subtitles from the new narration's word timings, not guessed scene lengths.
    subs=[]
    def stamp(t):
        ms=round(t*1000);return f'{ms//3600000:02}:{ms//60000%60:02}:{ms//1000%60:02},{ms%1000:03}'
    for s in scenes:
        words=json.loads((TMP/f"words-{s['id']}.json").read_text())
        for i in range(0,len(words),8):
            group=words[i:i+8];a=s['start']+.35+group[0]['offset']/1e7;b=s['start']+.35+(group[-1]['offset']+group[-1]['duration'])/1e7
            subs.append(f"{len(subs)+1}\n{stamp(a)} --> {stamp(b)}\n{' '.join(w['text'] for w in group)}\n")
    (OUT/'ReGTA_CTW_First_Look_EN.srt').write_text('\n'.join(subs),encoding='utf-8')
    print('Assets ready;',round(total,2),'seconds',flush=True)

def music(seconds):
    # Original restrained pulse; no commercial or original game music is reused.
    rate=24000;pcm=array.array('h');roots=[55,65.406,73.416,49];beat=60/96
    for i in range(int(seconds*rate)):
        t=i/rate;bar=int(t/(beat*4));root=roots[bar%4];p=t%beat
        kick=math.sin(2*math.pi*(48*p+4*(1-math.exp(-30*p))))*math.exp(-22*p)*.26
        bass=math.sin(2*math.pi*root*t)*(.5+.5*math.cos(2*math.pi*p/beat))*.12
        arp_step=int(t/(beat/2));ratio=[2,2.5,3,4][arp_step%4];u=t%(beat/2)
        bell=(math.sin(2*math.pi*root*ratio*t)+.16*math.sin(2*math.pi*root*ratio*2.01*t))*math.exp(-8*u)*.055
        fade=min(1,t/2,max(0,(seconds-t)/3));pcm.append(int(32767*(kick+bass+bell)*fade))
    with wave.open(str(TMP/'score.wav'),'wb') as w:w.setnchannels(1);w.setsampwidth(2);w.setframerate(rate);w.writeframes(pcm.tobytes())

def render():
    if not SOURCE.is_file():raise FileNotFoundError('Native gameplay is required. No emulator fallback is permitted.')
    scenes,total=timeline();music(total);segments=[]
    for s in scenes:
        partlen=s['duration']/len(s['shots']);shotfiles=[]
        for j,seek in enumerate(s['shots']):
            p=TMP/f"shot-{s['id']}-{j}.mp4"
            run(['-ss',seek,'-i',SOURCE,'-t',partlen,'-an','-vf','scale=1920:1080:flags=lanczos,setsar=1,fps=60','-c:v','libx264','-preset','fast','-crf',18,'-pix_fmt','yuv420p','-threads',6,'-y',p]);shotfiles.append(p)
        listing=TMP/f"shots-{s['id']}.txt";listing.write_text(''.join("file '"+p.as_posix()+"'\n" for p in shotfiles))
        bg=TMP/f"background-{s['id']}.mp4";run(['-f','concat','-safe',0,'-i',listing,'-c','copy','-y',bg])
        part=TMP/f"scene-{s['id']}.mp4"
        filters="[0:v][1:v]overlay=0:0:format=auto,format=yuv420p[v];[2:a]adelay=350:all=1,apad,aresample=48000[a]"
        run(['-i',bg,'-i',TMP/f"overlay-{s['id']}.png",'-i',TMP/f"voice-{s['id']}.mp3",'-filter_complex',filters,'-map','[v]','-map','[a]','-t',s['duration'],'-c:v','libx264','-preset','fast','-crf',18,'-threads',6,'-c:a','aac','-ar',48000,'-b:a','192k','-y',part]);segments.append(part);print('Rendered scene',s['id'],flush=True)
    listing=TMP/'scenes.txt';listing.write_text(''.join("file '"+p.as_posix()+"'\n" for p in segments));assembled=TMP/'assembled.mp4'
    run(['-f','concat','-safe',0,'-i',listing,'-c','copy','-y',assembled])
    final=OUT/'ReGTA_CTW_First_Look_EN.mp4'
    # Speech foreground, low music bed, final limiter. 60fps describes the export, not a performance guarantee.
    af=f'[0:a]loudnorm=I=-16:TP=-1.5:LRA=8,aresample=48000[voice];[1:a]volume=0.15,aresample=48000[music];[voice][music]amix=inputs=2:duration=first:normalize=0,alimiter=limit=0.94[a]'
    run(['-i',assembled,'-i',TMP/'score.wav','-filter_complex',af,'-map','0:v','-map','[a]','-c:v','copy','-c:a','aac','-b:a','256k','-ar',48000,'-ac',2,'-t',total,'-movflags','+faststart','-y',final]);print('FINAL',final,flush=True)

def verify():
    p=OUT/'ReGTA_CTW_First_Look_EN.mp4';run(['-i',p,'-f','null','-']);print('Decode OK',duration(p))
    r=subprocess.run([FF,'-i',str(p),'-af','volumedetect','-vn','-f','null','-'],capture_output=True,text=True)
    (OUT/'audio-check.txt').write_text(r.stderr,encoding='utf-8')
    times=[2,9,20,33,48,61,max(0,duration(p)-5)];sheet=Image.new('RGB',(1280,4*205),'#111');d=ImageDraw.Draw(sheet)
    for i,t in enumerate(times):
        f=TMP/f'check-{i}.png';run(['-ss',t,'-i',p,'-frames:v',1,'-vf','scale=426:240','-y',f]);im=Image.open(f);im.thumbnail((426,180));x=i%3*426;y=i//3*205;sheet.paste(im,(x,y));d.text((x,y+182),f'{t:.1f}s',fill='white')
    sheet.save(OUT/'review-contact.jpg')

if __name__=='__main__':
    mode=sys.argv[1]
    if mode=='voice':asyncio.run(voice())
    elif mode=='assets':assets()
    elif mode=='render':render()
    elif mode=='verify':verify()
    else:raise SystemExit('Choose voice, assets, render or verify.')
