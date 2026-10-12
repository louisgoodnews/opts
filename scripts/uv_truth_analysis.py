import sys,math,collections,csv
S=sys.argv[1]
rows=[]
for l in open(S+'/vertices.tsv'):
    f=l.rstrip('\n').split('\t')
    rows.append(f)
N=len(rows); print('vertices',N)
def fl(x):
    try: return float(x)
    except: return float('nan')
eps=1e-4
step_ok=[r[8]=='1' for r in rows]
dec_known=[r[7]!='-1' for r in rows]
dec_ok=[r[7]=='1' for r in rows]
print('UV step exact %.4f'%(sum(step_ok)/N))
dk=sum(dec_known); print('end-to-end: accepted-mesh vertices',dk,'exact %.4f'%(sum(dec_ok)/dk))
# error magnitudes of wrong (step)
def pct(a,p): a=sorted(a); return a[min(len(a)-1,int(p*len(a)))]
err=[math.hypot(fl(r[16]),fl(r[17])) for r,o in zip(rows,step_ok) if not o]
print('wrong (step) n=%d  err min %.4f median %.4f p95 %.4f p99 %.4f max %.3f'%(len(err),min(err),pct(err,.5),pct(err,.95),pct(err,.99),max(err)))
eerr=[math.hypot(fl(r[18]),fl(r[19])) for r,k,o in zip(rows,dec_known,dec_ok) if k and not o]
print('wrong (end-to-end) n=%d  err min %.4f median %.4f p95 %.4f p99 %.4f max %.3f'%(len(eerr),min(eerr),pct(eerr,.5),pct(eerr,.95),pct(eerr,.99),max(eerr)))
# truth jitter
jit=[(abs(fl(r[9]))>1e-6 or abs(fl(r[10]))>1e-6) for r in rows]
print('truth jittered %.4f'%(sum(jit)/N))
# wrong among jittered / not jittered; type of failure
miss=fa=wrongfix=0; nj=0; jj=0
for r,o,j in zip(rows,step_ok,jit):
    if j: jj+=1
    else: nj+=1
    if o: continue
    sdu,sdv=fl(r[16]),fl(r[17]); du,dv=fl(r[9]),fl(r[10])
    unchanged=abs(sdu-du)<1e-5 and abs(sdv-dv)<1e-5
    if j and unchanged: miss+=1          # jitter left in
    elif not j: fa+=1                    # clean vertex wrongly shifted
    else: wrongfix+=1                    # jittered, shifted by the wrong offset
print('jittered vertices %d: left jittered %d (%.3f), fixed wrongly %d (%.3f); clean vertices %d: wrongly shifted %d (%.4f)'%(jj,miss,miss/jj,wrongfix,wrongfix/jj,nj,fa,fa/nj))
# pattern: by n%5
for m in (0,1,2,3,4):
    sel=[o for r,o in zip(rows,step_ok) if int(r[2])%5==m]
    print('n mod 5 = %d: vertices %d exact %.4f'%(m,len(sel),sum(sel)/len(sel)))
# by jitter kind
kinds=collections.defaultdict(lambda:[0,0])
for r,o,j in zip(rows,step_ok,jit):
    du,dv=abs(fl(r[9]))>1e-6,abs(fl(r[10]))>1e-6
    k='none' if not(du or dv) else 'u' if du and not dv else 'v' if dv and not du else 'both'
    kinds[k][0]+=1; kinds[k][1]+=o
print('by jitter kind:',{k:(v[0],round(v[1]/v[0],4)) for k,v in kinds.items()})
# by plain uv outside [0,1]
inr=collections.defaultdict(lambda:[0,0])
for r,o in zip(rows,step_ok):
    u,v=fl(r[11]),fl(r[12]); k='in [0,1]' if 0<=u<=1 and 0<=v<=1 else 'outside'
    inr[k][0]+=1; inr[k][1]+=o
print('by plain uv range:',{k:(v[0],round(v[1]/v[0],4)) for k,v in inr.items()})
# by perm class
pc=collections.defaultdict(lambda:[0,0])
for r,o in zip(rows,step_ok): pc[r[6]][0]+=1; pc[r[6]][1]+=o
print('by position permutation class:',{k:(v[0],round(v[1]/v[0],4)) for k,v in sorted(pc.items())})
# by index
for name,fn in (('first 8 vertices',lambda i,n:i<8),('last 8',lambda i,n:i>=n-8),('i mod 5==0',lambda i,n:i%5==0),('i mod 2==0',lambda i,n:i%2==0)):
    sel=[o for r,o in zip(rows,step_ok) if fn(int(r[1]),int(r[2]))]
    print('index',name,len(sel),'exact %.4f'%(sum(sel)/len(sel)))
# per mesh distribution
per=collections.defaultdict(lambda:[0,0])
for r,o in zip(rows,step_ok): per[r[0]][0]+=1; per[r[0]][1]+=o
rates=sorted(v[1]/v[0] for v in per.values())
print('per mesh exact: p5 %.3f p25 %.3f median %.3f p75 %.3f; meshes with 100%% exact: %d of %d'%(rates[len(rates)//20],rates[len(rates)//4],rates[len(rates)//2],rates[3*len(rates)//4],sum(1 for x in rates if x>=0.9999),len(rates)))
# per mesh: share of distinct UV (constant-UV meshes)
byp=collections.defaultdict(list)
for r,o in zip(rows,step_ok): byp[r[0]].append((round(fl(r[11]),3),round(fl(r[12]),3),o))
buckets=collections.defaultdict(lambda:[0,0])
for p,vs in byp.items():
    d=len(set((a,b) for a,b,_ in vs))/len(vs)
    k='distinct uv share <5%' if d<0.05 else '5-20%' if d<0.2 else '20-50%' if d<0.5 else '>=50%'
    buckets[k][0]+=len(vs); buckets[k][1]+=sum(o for _,_,o in vs)
print('by distinct-UV share of the mesh:',{k:(v[0],round(v[1]/v[0],4)) for k,v in buckets.items()})
