"""Fixed fifth-round pinned-production-AST authoring data; not a kernel proof.
Exactly one RoundRaw from accepted literal R4 and four Permutes of M0.
"""
from pathlib import Path
import gzip, hashlib, json
W=Path(__file__).resolve().parents[3]
pins={'Blake3Nat':'91df70496689c63732341fad46abd39a3c91f8d756db0665136f789b74e9a68b','Blake3Core':'64242d1e99d648b420f33ca190f949308a151b1e52eb6a58af559f3d8561c6a0','Blake3':'a940ced45024e3b3c73da5ec373c27a57afd9abdbd0f1e87ff52dbbd0878754e'}
defs={}
for module in ['Blake3Nat','Blake3Core','Blake3']:
    pin=pins[module]
    raw=gzip.decompress((W/'docs/continuation/earlier-history-2026-10-23/files'/f'{pin}.txt.gz').read_bytes())
    assert hashlib.sha256(raw).hexdigest()==pin
    ast=json.loads(raw.decode().split('\\semanticdata{',1)[1].split('\n\\end{semanticmodule}',1)[0][:-1])
    defs['R4.'+module]={d['name']:d for d in ast['declarations']}
calls=0
arities=set()
def call(module,name,args):
    global calls
    calls+=1
    d=defs[module][name]
    assert len(args)==len(d['parameters'])
    return evaluate(d['body'],dict(zip([p['name'] for p in d['parameters']],args)),module)
def evaluate(node,env,module):
    kind=node['kind']
    if kind=='nat': return int(node['value'])
    if kind=='var': return env[node['name']]
    if kind=='record': return {f['field']:evaluate(f['value'],env,module) for f in node['fields']}
    if kind=='project': return evaluate(node['value'],env,module)[node['field']]
    if kind=='let': return evaluate(node['body'],dict(env,**{node['binder']['name']:evaluate(node['value'],env,module)}),module)
    if kind=='add': return evaluate(node['left'],env,module)+evaluate(node['right'],env,module)
    if kind=='beq': return evaluate(node['left'],env,module)==evaluate(node['right'],env,module)
    if kind=='if': return evaluate(node['then_value'] if evaluate(node['condition'],env,module) else node['else_value'],env,module)
    if kind=='call': return call(node['function'].get('module',module),node['function']['name'],[evaluate(a,env,module) for a in node['arguments']])
    if kind=='primitive':
        args=[evaluate(a,env,module) for a in node['arguments']]
        op=node['operation']
        arities.add((op,len(args)))
        if op=='multiply':
            assert len(args)==2
            return args[0]*args[1]
        if op=='subtract':
            assert len(args)==2
            return max(0,args[0]-args[1])
        if op=='quotient':
            assert len(args)==3 and args[1]>0
            return args[0]//args[1]
        if op=='remainder':
            assert len(args)==3 and args[1]>0
            return args[0]%args[1]
        raise AssertionError(op)
    if kind=='match':
        value=evaluate(node['scrutinee'],env,module)
        if isinstance(value,list):
            tag='List.nil' if not value else 'List.cons'
            args=[] if not value else [value[0],value[1:]]
        else:
            assert isinstance(value,int) and value>=0
            tag='Nat.zero' if value==0 else 'Nat.succ'
            args=[] if value==0 else [value-1]
        branch=next(b for b in node['branches'] if b['constructor']['name']==tag)
        return evaluate(branch['body'],dict(env,**dict(zip(branch['binders'],args))),module)
    raise AssertionError(kind)

base_path=W/'tests/fixtures/blake3-fourth-round/project/tests/lexlean/Blake3FourthRound.lex.tex'
base_bytes=base_path.read_bytes()
assert hashlib.sha256(base_bytes).hexdigest()=='aaf5ab14ed5389a217936ff3766d56f2aa2447d92ae5f656d7f2fe6acdfd5285'
base_ast=json.loads(next(line for line in base_bytes.decode().splitlines() if line.startswith('\\semanticdata{'))[14:-1])
r4_ast=base_ast['declarations'][0]['statement']['right']
r4=evaluate(r4_ast,{},'R4.Blake3Core')
m0=call('R4.Blake3','blake3BlockWords',[list(range(64))])
m1=call('R4.Blake3Core','b3CorePermute',[m0])
m2=call('R4.Blake3Core','b3CorePermute',[m1])
m3=call('R4.Blake3Core','b3CorePermute',[m2])
m4=call('R4.Blake3Core','b3CorePermute',[m3])
r5=call('R4.Blake3Core','b3CoreRoundRaw',[r4,m4])
assert list(r5)==['s'+str(i) for i in range(16)]
assert all(0<=v<2**32 for v in r5.values())
# Separate fixed integer calculation, from the inspected production G schedule.
mask=2**32-1
def rotr(x,n): return ((x>>n)|(x<<(32-n)))&mask
def mix(a,b,c,d,x,y):
    a=(a+b+x)&mask;d=rotr(d^a,16);c=(c+d)&mask;b=rotr(b^c,12)
    a=(a+b+y)&mask;d=rotr(d^a,8);c=(c+d)&mask;b=rotr(b^c,7)
    return a,b,c,d
v=[r4['s'+str(i)] for i in range(16)]
m=[m4['s'+str(i)] for i in range(16)]
schedule=[(0,4,8,12),(1,5,9,13),(2,6,10,14),(3,7,11,15),(0,5,10,15),(1,6,11,12),(2,7,8,13),(3,4,9,14)]
for k,indices in enumerate(schedule):
    values=mix(*(v[i] for i in indices),m[2*k],m[2*k+1])
    for i,value in zip(indices,values):v[i]=value
assert v==list(r5.values())
print(json.dumps({'status':'UNTRUSTED_AUTHORING_DATA_NATIVE_NOT_RUN','sourcePins':pins,'acceptedBaseSha256':hashlib.sha256(base_bytes).hexdigest(),'acceptedR4Ast':r4_ast,'messageBytes':list(range(64)),'message0':m0,'message1':m1,'message2':m2,'message3':m3,'message4':m4,'fourthRound':r4,'fifthRound':r5,'independentIntegerSchedule':{'schedule':schedule,'words':v,'matchesPinnedAstEvaluation':True},'definitionCalls':calls,'observedPrimitiveArities':sorted(arities),'limit':'Both host computations are untrusted authoring data; official full16word reflexivity and mandatory original consumer remain NOT_RUN.'},indent=2))
