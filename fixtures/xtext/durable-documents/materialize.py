"""Restore exact pinned original verification files to a NEW owned directory.

This is a storage adapter, not an oracle or language implementation. It never
interprets historical absolute paths as writable destinations.
"""
from pathlib import Path,PurePosixPath
import argparse,json
from verify import Corpus,require,pin

def main():
    p=argparse.ArgumentParser();p.add_argument('--prefix',required=True);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    require(a.prefix.endswith('/'),'prefix must end in slash')
    require(not a.output.exists(),'output must not exist')
    corpus=Corpus();selected=[(name,q)for name,q in corpus.origins.items()if name.startswith(a.prefix)]
    require(bool(selected),'empty original prefix');a.output.mkdir(parents=True)
    restored=[]
    for name,q in selected:
        relative=PurePosixPath(name[len(a.prefix):]);require(not relative.is_absolute()and'..'not in relative.parts,'unowned output path')
        dest=a.output.joinpath(*relative.parts);dest.parent.mkdir(parents=True,exist_ok=True)
        data=corpus.raw(name);dest.open('xb').write(data);require(pin(dest.read_bytes())=={k:q[k]for k in ['bytes','sha256']},'restored bytes differ')
        restored.append({'origin':name,'relative_path':relative.as_posix(),'bytes':q['bytes'],'sha256':q['sha256']})
    print(json.dumps({'status':'PASS byte-exact materialization only','prefix':a.prefix,'files':restored},ensure_ascii=True))
if __name__=='__main__':main()
