#!/usr/bin/env python3
"""Summarises a directory of UV incident files (`<log dir>/incidents/uv-*.log`): quality totals, mesh size, family, stages, reasons, worst meshes.
Usage: python3 scripts/uv_incident_stats.py <incident dir>   (see docs/src/meta/uv-incidents-ng272-2026-10-13.md)"""
import re,os,glob,collections,statistics,sys
D=sys.argv[1]
rows=[]
for f in glob.glob(D+'/*.log'):
    t=open(f).read()
    g=lambda p:re.search(p,t)
    mesh=g(r'mesh: (.*)').group(1)
    v=int(g(r'vertices: (\d+)').group(1)); tri=int(g(r'triangles: (\d+)').group(1))
    cnt={k:int(g(k+r'\s+(\d+) vertices').group(1)) for k in ('Exact','Recovered','Unreliable')}
    sib=int(g(r'with this name in the installation: (\d+)').group(1))
    src=g(r'decoding source: (.*?)   ').group(1)
    stages={}
    for m in re.finditer(r'^(\S+(?: \(joint copies\))?)\s+(\d+)\s+([\d.]+) %\s+unreliable\s+(\d+)',t,re.M):
        stages[m.group(1)]=(int(m.group(2)),int(m.group(4)))
    reasons=re.findall(r'^\s*(\d+) vertices: (.*)$',t,re.M)
    # vertex rows
    vr=[]
    for m in re.finditer(r'^\s*(\d+)\s+\(\s*(-?[\d.]+)\s+(-?[\d.]+)\s+(-?[\d.]+)\)\s+\(\s*(-?[\d.]+)\s+(-?[\d.]+)\)\s+\(\s*(-?[\d.]+)\s+(-?[\d.]+)\)\s+du ([+-][\d.]+) dv ([+-][\d.]+)\s+(\S+)\s+([\d.]+)',t,re.M):
        vr.append((float(m.group(9)),float(m.group(10)),m.group(11),float(m.group(12))))
    rows.append(dict(file=os.path.basename(f),mesh=mesh.split('Vehicles/')[-1],v=v,tri=tri,cnt=cnt,sib=sib,src=src,stages=stages,reasons=reasons,vr=vr))
n=len(rows); print('incidents',n)
tv=sum(r['v'] for r in rows); tu=sum(r['cnt']['Unreliable'] for r in rows); te=sum(r['cnt']['Exact'] for r in rows); tr=sum(r['cnt']['Recovered'] for r in rows)
print('vertices in flagged meshes',tv,'exact %.1f%% recovered %.1f%% unreliable %.1f%%'%(100*te/tv,100*tr/tv,100*tu/tv))
pct=[100*r['cnt']['Unreliable']/r['v'] for r in rows]
print('unreliable share per mesh: min %.0f median %.0f max %.0f'%(min(pct),statistics.median(pct),max(pct)))
def bucket(r):
    v=r['v']; return '<=50' if v<=50 else '51-200' if v<=200 else '201-1000' if v<=1000 else '1001-5000' if v<=5000 else '>5000'
b=collections.defaultdict(lambda:[0,0,0])
for r in rows:
    k=bucket(r); b[k][0]+=1; b[k][1]+=r['v']; b[k][2]+=r['cnt']['Unreliable']
print('by mesh size:'); 
for k in ['<=50','51-200','201-1000','1001-5000','>5000']:
    if k in b: print('  %-10s meshes %3d vertices %6d unreliable %5.1f%%'%(k,b[k][0],b[k][1],100*b[k][2]/b[k][1]))
# by mod 5
m5=collections.defaultdict(lambda:[0,0,0])
for r in rows:
    k='5|n' if r['v']%5==0 else 'other'; m5[k][0]+=1; m5[k][1]+=r['v']; m5[k][2]+=r['cnt']['Unreliable']
print('by class:',{k:(v[0],v[1],round(100*v[2]/v[1],1)) for k,v in m5.items()})
# by family (file name prefix)
fam=collections.defaultdict(lambda:[0,0,0])
def family(r):
    name=os.path.basename(r['mesh']).lower(); d=os.path.basename(os.path.dirname(r['mesh'])).lower()
    if name.startswith('gn92_panel'): return 'GN92 cockpit panel parts (GN92_Panel_*)'
    if d=='generic': return 'Generic\\\\ shared parts'
    if 'fenster' in name or 'window' in name: return 'windows'
    if 'tuer' in name or 'door' in name: return 'doors'
    if name.startswith('gn92_trail'): return 'GN92_trail_*'
    if name.startswith('gn92_main'): return 'GN92_main_*'
    if name.startswith('gn92'): return 'GN92 other'
    return 'other'
for r in rows:
    k=family(r); fam[k][0]+=1; fam[k][1]+=r['v']; fam[k][2]+=r['cnt']['Unreliable']
print('by family:')
for k,v in sorted(fam.items(),key=lambda e:-e[1][1]): print('  %-42s meshes %3d vertices %6d unreliable %5.1f%%'%(k,v[0],v[1],100*v[2]/v[1]))
# stages overall
st=collections.defaultdict(lambda:[0,0])
for r in rows:
    for k,(a,u) in r['stages'].items(): st[k][0]+=a; st[k][1]+=u
print('stage totals (vertices, unreliable):',{k:tuple(v) for k,v in st.items()})
print('siblings: meshes with 0 siblings',sum(1 for r in rows if r['sib']==0),'of',n,'; with >=1:',[ (r['mesh'].split('/')[-1],r['sib']) for r in rows if r['sib']>0][:12])
print('sources',collections.Counter(r['src'] for r in rows))
# vertex rows: fraction where guess == stored (no offset removed)
allv=[x for r in rows for x in r['vr']]
zero=sum(1 for x in allv if abs(x[0])<1e-6 and abs(x[1])<1e-6)
print('listed unreliable vertices',len(allv),'best guess = "no jitter" for %.1f%%'%(100*zero/len(allv)))
marg=[x[3] for x in allv if x[2]=='smooth']
print('smooth margins: median %.2f p10 %.2f p90 %.2f ; below 0.5: %.1f%%'%(statistics.median(marg),sorted(marg)[len(marg)//10],sorted(marg)[9*len(marg)//10],100*sum(1 for m in marg if m<0.5)/len(marg)))
# worst meshes
print('worst 12 by unreliable vertices:')
for r in sorted(rows,key=lambda r:-r['cnt']['Unreliable'])[:12]:
    print('  %-52s v=%5d unreliable %5d (%2.0f%%) 5|n=%s'%(r['mesh'].split('Vehicles/')[-1][:52],r['v'],r['cnt']['Unreliable'],100*r['cnt']['Unreliable']/r['v'],r['v']%5==0))
print('highest share among larger meshes (>=200):')
for r in sorted([r for r in rows if r['v']>=200],key=lambda r:-r['cnt']['Unreliable']/r['v'])[:6]:
    print('  %-52s v=%5d %2.0f%%'%(r['mesh'].split('/')[-1][:52],r['v'],100*r['cnt']['Unreliable']/r['v']))
print('meshes by unreliable share:',collections.Counter('10-30%' if p<30 else '30-50%' if p<50 else '50-70%' if p<70 else '70-90%' if p<90 else '90-100%' for p in pct))
