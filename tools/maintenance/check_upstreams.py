"""Refresh independent upstream mirrors and record a task-scoped evidence snapshot.
Never pull, merge, execute upstream code, or change our project's Git remotes.
"""
import argparse,json,subprocess
from pathlib import Path
from datetime import datetime,timezone
ROOT=Path(__file__).resolve().parents[2]

def git(path,*args):
    p=subprocess.run(['git','-C',str(path),*args],capture_output=True,text=True,encoding='utf-8',errors='replace',timeout=120)
    if p.returncode:raise RuntimeError(p.stderr.strip() or p.stdout.strip())
    return p.stdout.strip()

def verify_repository(path,url):
    if Path(git(path,'rev-parse','--show-toplevel')).resolve()!=path.resolve():
        raise RuntimeError('Not an independent Git repository; refusing parent-repository fallback')
    if git(path,'remote','get-url','origin').removesuffix('.git').rstrip('/')!=url.removesuffix('.git').rstrip('/'):
        raise RuntimeError('Unexpected origin URL; refusing fetch')

def inspect(config,previous):
    path=(ROOT/config['path']).resolve()
    if not path.is_relative_to(ROOT/'references'):raise RuntimeError('Mirror must be inside references/')
    if not path.exists():
        p=subprocess.run(['git','clone','--depth','30',config['url'],str(path)],capture_output=True,text=True,timeout=120)
        if p.returncode:raise RuntimeError(p.stderr.strip())
    verify_repository(path,config['url'])
    local_status=git(path,'status','--porcelain')
    git(path,'fetch','origin',config['branch'])
    revision=git(path,'rev-parse','FETCH_HEAD')
    lines=git(path,'log','-6','--format=%H%x09%cI%x09%s',revision).splitlines()
    commits=[dict(zip(['sha','date','subject'],line.split('\t',2))) for line in lines]
    old=previous.get('revision');changed=None
    if old:
        try:changed=git(path,'diff','--name-only',old,revision).splitlines()
        except RuntimeError:pass
    return {**{k:config[k] for k in ['id','url','platform','use']},'status':'checked','revision':revision,'previous_revision':old,'changed_files':changed,'local_changes_present':bool(local_status),'recent_commits':commits}

def main():
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--task',required=True);args=ap.parse_args()
    destination=ROOT/'docs/upstream-status.json'
    previous=json.loads(destination.read_text(encoding='utf-8')) if destination.exists() else {'repositories':[]}
    before={r['id']:r for r in previous['repositories']}
    result={'checked_at':datetime.now(timezone.utc).isoformat(timespec='seconds'),'task':args.task,'repositories':[]}
    for config in json.loads((ROOT/'tools/maintenance/upstreams.json').read_text(encoding='utf-8-sig'))['repositories']:
        try:record=inspect(config,before.get(config['id'],{}))
        except Exception as exc:record={'id':config['id'],'url':config['url'],'status':'unavailable','error':str(exc),'last_successful_revision':before.get(config['id'],{}).get('revision') or before.get(config['id'],{}).get('last_successful_revision')}
        result['repositories'].append(record);print(config['id'],record['status'],record.get('revision',''),flush=True)
    destination.write_text(json.dumps(result,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
    if any(r['status']!='checked' for r in result['repositories']):raise SystemExit(1)

if __name__=='__main__':main()
