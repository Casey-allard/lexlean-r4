"""Exact bounded authoring-data derivation; not a theorem or SDK replacement.
Reads the two pinned production semantic ASTs and prints the first compression
CV. The actual full-state Reflexivity proof with empty axioms is decisive.
No fixture or runtime writes, external BLAKE3 package, native call or trusted CV.
"""
from pathlib import Path
import base64, gzip, hashlib, json
W=Path(__file__).resolve().parents[3]
F=Path(__file__).resolve().parent
proposal=json.loads((F/'proposal.json').read_text())
defs={}
for module in ['Blake3Nat','Blake3Core']:
    pin=proposal['runtimePins'][module]
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
        assert isinstance(value,int) and value>=0
        tag='Nat.zero' if value==0 else 'Nat.succ'
        branch=next(b for b in node['branches'] if b['constructor']['name']==tag)
        return evaluate(branch['body'],dict(env,**dict(zip(branch['binders'],[] if value==0 else [value-1]))),module)
    raise AssertionError(kind)
iv=call('R4.Blake3Core','b3CoreIV',[])
words=[int.from_bytes(bytes(range(i,i+4)),'little') for i in range(0,64,4)]
assert words==proposal['valueDerivation']['LEwords']
output=call('R4.Blake3Core','b3CoreCompressRaw',[iv,{'s'+str(i):v for i,v in enumerate(words)},0,64,1])
cv=call('R4.Blake3Core','b3CoreFirstEight',[output])
values=[cv['c'+str(i)] for i in range(8)]
assert values==proposal['expectedS128']['chain_c0_to_c7']
print(json.dumps({'status':'AUTHORING_DATA_CONSISTENCY_ONLY_NATIVE_NOT_RUN','sourcePins':{m:proposal['runtimePins'][m] for m in ['Blake3Nat','Blake3Core']},'messageBytes':list(range(64)),'littleEndianWords':words,'counter':0,'blockLength':64,'flags':1,'iv':iv,'fullCompressionOutput':output,'expectedChain':values,'definitionCalls':calls,'observedPrimitiveArities':sorted(arities),'limit':'Not a full-state or kernel proof; exact production F(Q,S64)=ok(literal S128) still requires official rfl with empty axioms.'},indent=2))
