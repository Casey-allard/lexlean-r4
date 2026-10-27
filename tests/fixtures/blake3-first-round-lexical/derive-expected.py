"""Fixed first-round production-AST authoring derivation; not a kernel proof.
Reads pinned committed Nat/Core/Blake3 semantic source, evaluates actual
BlockWords, InitialRaw and one RoundRaw. No SDK, external hash or trusted CV.
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
iv=call('R4.Blake3Core','b3CoreIV',[])
message=call('R4.Blake3','blake3BlockWords',[list(range(64))])
initial=call('R4.Blake3Core','b3CoreInitialRaw',[iv,0,64,1])
result=call('R4.Blake3Core','b3CoreRoundRaw',[initial,message])
assert list(result)==['s'+str(i) for i in range(16)]
assert all(0<=v<2**32 for v in result.values())
assert list(message.values())==[int.from_bytes(bytes(range(i,i+4)),'little') for i in range(0,64,4)]
print(json.dumps({'status':'UNTRUSTED_AUTHORING_DATA_NATIVE_NOT_RUN','sourcePins':pins,'messageBytes':list(range(64)),'message':message,'initial':initial,'firstRound':result,'definitionCalls':calls,'observedPrimitiveArities':sorted(arities),'limit':'One exact production round evaluated for expected literal authoring only; official Reflexivity and empty-axiom artifact remain mandatory.'},indent=2))
