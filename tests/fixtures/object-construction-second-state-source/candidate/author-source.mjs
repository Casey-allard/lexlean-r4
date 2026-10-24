// Source-only construction of one reviewed finite candidate. No native execution.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import zlib from 'node:zlib';
import assert from 'node:assert/strict';
const root=path.dirname(new URL(import.meta.url).pathname);
const repo='/Users/casey.allard/lexlean-r4-worktrees/model-port';
const ev='/Users/casey.allard/lexlean-r4-evidence/2026-10-24';
const canonical=x=>JSON.stringify(sort(x));
function sort(x){return Array.isArray(x)?x.map(sort):x&&typeof x==='object'?Object.fromEntries(Object.keys(x).sort().map(k=>[k,sort(x[k])])):x;}
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const pin=p=>{const b=fs.readFileSync(p);return {path:p,bytes:b.length,sha256:sha(b)};};
const read=p=>JSON.parse(fs.readFileSync(p));
const dependencies=new Map();
function dep(p,expected){const q=pin(p);if(expected){assert.equal(q.sha256,expected.sha256);assert.equal(q.bytes,expected.bytes);}dependencies.set(p,q);return fs.readFileSync(p);}
const cap=1048576, reserve=10737418240n;
function alloc(){let total=0;function walk(p){for(const e of fs.readdirSync(p,{withFileTypes:true})){const f=path.join(p,e.name);if(e.isDirectory())walk(f);else total+=fs.statSync(f).blocks*512;}}walk(root);return total;}
const observations=[];
function budget(phase,extra=0){const s=fs.statfsSync(root,{bigint:true});const free=s.bavail*s.bsize;assert(free>=reserve+BigInt(cap));assert(alloc()+extra<=cap);observations.push({phase,freeBytes:String(free),allocatedBefore:alloc(),reservedAdditional:extra});}
function put(p,b){b=Buffer.isBuffer(b)?b:Buffer.from(b);budget('write '+p,Math.ceil(b.length/4096)*4096);const f=path.join(root,p);fs.mkdirSync(path.dirname(f),{recursive:true});fs.writeFileSync(f,b,{flag:'wx'});return pin(f);}
const json=(p,v)=>put(p,JSON.stringify(v,null,2)+'\n');
budget('initial');
const pp=path.join(ev,'object-construction-second-state-proof-plan.json');
const plan=JSON.parse(dep(pp,{bytes:13730,sha256:'87f56ec9e0b61c3920b2f5d2a2480c876fbeab6bc413974a99d9aa8f805e336a'}));
dep(path.join(ev,'object-construction-second-state-proof-spec-review.json'),{bytes:10965,sha256:'6fa82fc7106aea0cb23338bd8ae9d1b5591aecccb0cf52328781b6f3b17482d1'});
const ab=zlib.gunzipSync(dep(plan.astPayload.path,{bytes:plan.astPayload.compressedBytes,sha256:plan.astPayload.compressedSha256}));
assert.equal(ab.length,plan.astPayload.decodedBytes);assert.equal(sha(ab),plan.astPayload.decodedSha256);
const a=JSON.parse(ab);
for(const p of plan.sourcePins)dep(p.path,p);
for(const p of Object.values(plan.lineage))dep(p.path,p);
const pendingPath=path.join(repo,'docs/continuation/2026-10-23/pending-manifest.json');
const pending=JSON.parse(dep(pendingPath));
const current17=pending.files.map(x=>pin(path.join(repo,x.originalPath)));
const snapshots17=pending.files.map(x=>{const p=path.join(repo,'docs/continuation/2026-10-23',x.snapshotPath);dep(p,x);return pin(p);});
const parseSemantic=b=>JSON.parse(b.toString().match(/^\\semanticdata\{(.*)\}$/m)[1]);
function frame(name,imports,obj,native=false){const tag=native?'core':'semantic';return `\\begin{lexlean}{${name}}\n\\useglossary{lexlean.std.bool@1.2.0}\n\\useglossary{lexlean.std.nat@1.2.0}\n${imports.map(m=>`\\importmodule{${m}}\n`).join('')}\\title{Boolean}\n\\begin{${tag}module}\n\\${tag}data{${canonical(obj)}}\n\\end{${tag}module}\n\\end{lexlean}\n`;}
const sem=declarations=>({spec:'lexlean/semantic-module/2',declarations});
const states={};
// Negative source is written first; actual intended failure remains NOT_RUN.
states.negative=put('negative/tests/lexlean/Object257SecondState.lex.tex',frame('Object257SecondState',plan.representation.imports.Object257SecondState,sem([a.declarations.stateNegative])));
states.positive=put('positive/tests/lexlean/Object257SecondState.lex.tex',frame('Object257SecondState',plan.representation.imports.Object257SecondState,sem([a.declarations.statePositive])));
const context=put('shared/tests/lexlean/Object257SecondContext.lex.tex',frame('Object257SecondContext',plan.representation.imports.Object257SecondContext,sem([a.context.definition,a.context.family])));
const transport=put('shared/tests/lexlean/Object257SecondTransport.lex.tex',frame('Object257SecondTransport',plan.representation.imports.Object257SecondTransport,sem([a.declarations.lift,a.declarations.generic,a.declarations.suffix])));
// Closed endpoint lowering only: original A/M/E use calls, records, nat and lists.
// Signatures/field order are read from exact production semantic declarations.
const signatures=new Map();
for(const mod of ['R4.ObjectTypes','R4.ObjectStore']){
 const p=path.join(repo,'src',...mod.split('.'))+'.lex.tex';
 const doc=parseSemantic(dep(p));
 for(const d of doc.declarations)signatures.set(mod+'.'+d.name,{mod,d});
}
const qualify=(t,mod)=>{if(Array.isArray(t))return t.map(x=>qualify(x,mod));if(!t||typeof t!=='object')return t;const o=Object.fromEntries(Object.entries(t).map(([k,v])=>[k,qualify(v,mod)]));if(o.kind==='named'&&!o.member.module)o.member.module=mod;return o;};
const nodes=[],index=new Map(),mapping=[];
function intern(n){const k=canonical(n);if(index.has(k))return index.get(k);const id=nodes.length;nodes.push(n);index.set(k,id);return id;}
const zero={k:'z'},one={k:'s',a:zero};
const c=(n,u=[])=>intern({k:'c',n,u});
const app=(f,...xs)=>xs.reduce((f,x)=>intern({k:'a',f,x}),f);
function ty(t){switch(t.kind){case'nat':return c('Nat');case'named':assert.deepEqual(t.arguments,[]);return c('LexLeanR4.'+t.member.module+'.'+t.member.name);case'list':return app(c('List',[zero]),ty(t.element));case'result':return app(c('Except',[zero,zero]),ty(t.error),ty(t.ok));default:throw Error('type '+t.kind);}}
function lower(x,t,loc){let id;
 switch(x.kind){
 case'nat':assert.equal(t.kind,'nat');id=intern({k:'n',v:x.value});break;
 case'nil':assert.equal(t.kind,'list');assert.equal(canonical(x.element),canonical(t.element));id=app(c('List.nil',[zero]),ty(t.element));break;
 case'cons':assert.equal(t.kind,'list');id=app(c('List.cons',[zero]),ty(t.element),lower(x.head,t.element,loc+'/head'),lower(x.tail,t,loc+'/tail'));break;
 case'call':{const key=x.function.module+'.'+x.function.name;const s=signatures.get(key);assert(s,key);assert.equal(canonical(qualify(s.d.result,s.mod)),canonical(t));assert.equal(s.d.parameters.length,x.arguments.length);id=app(c('LexLeanR4.'+key),...x.arguments.map((v,i)=>lower(v,qualify(s.d.parameters[i].type,s.mod),loc+'/arguments/'+i)));break;}
 case'record':{assert.equal(t.kind,'named');assert.equal(canonical(t.member),canonical(x.type));const key=x.type.module+'.'+x.type.name;const s=signatures.get(key);assert(s&&s.d.kind==='structure',key);assert.deepEqual(s.d.type_parameters,[]);assert.deepEqual(s.d.parameters,[]);assert.equal(x.fields.length,s.d.fields.length);id=app(c('LexLeanR4.'+key+'.mk'),...s.d.fields.map((f,i)=>{assert.equal(x.fields[i].field,f.name);return lower(x.fields[i].value,qualify(f.type,s.mod),loc+'/fields/'+i+'/value');}));break;}
 default:throw Error('unsupported closed endpoint '+x.kind);
 }
 mapping.push({path:loc,kind:x.kind,semanticSha256:sha(canonical(x)),expectedType:t,node:id});return id;
}
const endpoints=Object.fromEntries(['A','M','E'].map(k=>[k,lower(a.endpoints[k],a.types.resultT,'/endpoints/'+k)]));
function expanded(t){if(t.endpoint){assert(Object.hasOwn(endpoints,t.endpoint));return endpoints[t.endpoint];}if(t.k==='a')return app(expanded(t.f),expanded(t.x));assert.equal(t.k,'c');return c(t.n,t.u);}
const dtype=expanded(a.nativeComposition.type),value=expanded(a.nativeComposition.value);
const core={spec:'lexlean/core-module/1',imports:plan.representation.imports.Object257SecondPrefixConsumer,nodes,proof_nodes:[],declarations:[{...plan.representation.nativeDeclaration,type:dtype,value}]};
const consumer=put('shared/tests/lexlean/Object257SecondPrefixConsumer.lex.tex',frame('Object257SecondPrefixConsumer',core.imports.filter(x=>x!=='Init').map(x=>x.replace(/^LexLeanR4\./,'')),core,true));
put('native-core-module.json',canonical(core)+'\n');
put('typed-endpoint-mapping.json.gz',zlib.gzipSync(canonical({schema:'lexlean-r4-second-prefix-exact-endpoint-map/v1',sourceAst:plan.astPayload,endpoints,mapping}),{level:9}));
const semanticCorrespondence=[['negative/tests/lexlean/Object257SecondState.lex.tex',['/declarations/stateNegative']],['positive/tests/lexlean/Object257SecondState.lex.tex',['/declarations/statePositive']],['shared/tests/lexlean/Object257SecondContext.lex.tex',['/context/definition','/context/family']],['shared/tests/lexlean/Object257SecondTransport.lex.tex',['/declarations/lift','/declarations/generic','/declarations/suffix']]].map(([file,pointers])=>{const doc=parseSemantic(fs.readFileSync(path.join(root,file)));const expected=pointers.map(p=>p.slice(1).split('/').reduce((v,k)=>v[k],a));assert.equal(canonical(doc.declarations),canonical(expected));return {file,pointers,declarationsSha256:sha(canonical(doc.declarations))};});
json('semantic-correspondence.json',semanticCorrespondence);
const differences=[];
function diff(x,y,p=''){if(canonical(x)===canonical(y))return;if(x&&y&&typeof x==='object'&&typeof y==='object'){assert.deepEqual(Object.keys(x),Object.keys(y));for(const k of Object.keys(x))diff(x[k],y[k],p+'/'+k);}else differences.push({path:p,before:x,after:y});}
diff(a.declarations.statePositive,a.declarations.stateNegative);assert.deepEqual(differences,plan.negative.exactDiff);
json('negative-diff.json',{status:'SOURCE_ONLY_NATIVE_NOT_RUN',sourceWriteOrder:['negative','positive'],differences,consumerUnchanged:true,frontierUnchanged:true,intendedFailure:plan.negative.expectedRejection,notAcceptedFailures:plan.negative.notAccepted});
const oldMapPath=path.join(repo,'tests/fixtures/object257-symbolic-continuation/evidence/symbolic-concrete/expected-inputs.json');
const oldMap=JSON.parse(dep(oldMapPath));
// Keep exact accepted shared closure by reference. Remove only config; no copied runtime.
const sharedInputs=oldMap.filter(x=>x.destination!=='lexlean.toml').map(x=>{const p=path.join(repo,x.path);dep(p,x);return {...x,source:p};});
const configSource=path.join(repo,'tests/fixtures/object257-symbolic-continuation/phases/symbolic-concrete/lexlean.toml');
const configBefore=dep(configSource).toString();
const configAfter=configBefore.replace('entrypoints = ["tests/lexlean/Object257SymbolicConsumer.lex.tex"]','entrypoints = ["tests/lexlean/Object257SecondPrefixConsumer.lex.tex"]');assert.notEqual(configAfter,configBefore);assert.equal(configAfter.replace('Object257SecondPrefixConsumer','Object257SymbolicConsumer'),configBefore);
const config=put('lexlean.toml',configAfter);
const sharedNew=[context,transport,consumer].map(p=>({...p,source:p.path,destination:'tests/lexlean/'+path.basename(p.path)}));
const phaseMaps={};
for(const phase of ['second-prefix-false','second-prefix-positive']){const state=states[phase.endsWith('false')?'negative':'positive'];const inputs=[...sharedInputs,...sharedNew,{...state,source:state.path,destination:'tests/lexlean/Object257SecondState.lex.tex'},{...config,source:config.path,destination:'lexlean.toml'}];assert.equal(new Set(inputs.map(x=>x.destination)).size,inputs.length);phaseMaps[phase]=inputs;json('inputs/'+phase+'.json',inputs);}
// Statically check graph selection, including native core imports and semantic framing.
const graph=[];for(const [phase,inputs]of Object.entries(phaseMaps)){const modules=new Map();for(const x of inputs.filter(x=>x.destination.endsWith('.lex.tex'))){const text=fs.readFileSync(x.source,'utf8');const name=text.match(/\\begin\{lexlean\}\{([^}]+)\}/)[1];const imports=[...text.matchAll(/\\importmodule\{([^}]+)\}/g)].map(x=>x[1]);assert(!modules.has(name));modules.set(name,{name,imports,destination:x.destination});const m=text.match(/^\\(?:semantic|core)data\{(.*)\}$/m);assert(m);assert.equal(canonical(JSON.parse(m[1])),m[1]);}const seen=new Set();function visit(name){assert(modules.has(name),'missing '+name);if(seen.has(name))return;seen.add(name);for(const m of modules.get(name).imports)visit(m);}visit('Object257SecondPrefixConsumer');graph.push({phase,suppliedInputs:inputs.length,suppliedModules:modules.size,reachableModules:[...seen],unreachableSuppliedModules:[...modules.keys()].filter(x=>!seen.has(x)),target:'Object257SecondPrefixConsumer',nativeOutcome:'NOT_RUN'});}
for(let i=0;i<nodes.length;i++){const n=nodes[i];assert(['a','c','n'].includes(n.k));if(n.k==='a'){assert(n.f<i&&n.x<i);}}
json('source-audit.json',{schema:'lexlean-r4-second-prefix-source-audit/v1',status:'HOST_STRUCTURAL_ONLY_NOT_NATIVE_PROOF',graphs:graph,nativeNodeCount:nodes.length,endpointMappingCount:mapping.length,endpoints,nativeDeclaration:core.declarations[0],limits:plan.limits,configChange:'Only entrypoint symbolic consumer -> second prefix consumer',noLock:true,semanticASTsExact:true,negativeExact:true,actualConsumer:'A=E with Eq.trans alpha A M E hFirst hSecond',remaining:plan.remaining,inspectorHistory:['Read-only initial signature inspector used an overbroad regular expression and failed JSON parsing; corrected to exact single semanticdata line before construction. No source written by that failed inspector.']});
for(const p of current17)assert.deepEqual(pin(p.path),p);
for(const p of snapshots17)assert.deepEqual(pin(p.path),p);
json('preservation.json',{current17,snapshots17,allCurrentUnchanged:true,allSnapshotsUnchanged:true,repoWrites:0,nativeCalls:0});
put('README.md',`# Second original atom and original257 prefix consumer\n\nSOURCE CANDIDATE ONLY. Every native admission, negative, positive, axiom inventory and kernel replay is NOT_RUN. Independent source SPEC/CODE and separate invocation review are required. The sole current restoration job and pending Read33 retain native priority. No runner is supplied.\n\nThe negative source was written first and changes only expected second inventory ID byte 76 to 77. Frontier ID76 and the mandatory actual original257 A=E final consumer are identical. Execute negative before positive only after parent fresh resource/idle/source/tool gates and successful Read33 sequencing. Admission, timeout, OOM or axiom failure is not the intended negative.\n\nMaterialize a fresh owned stage using inputs/second-prefix-false.json, verifying every source and destination hash. Generate lexlean.lock only with the official compiler. Run official check and verify using lexlean.toml; inspect the dependency-level state reflexivity diagnostic before considering the corresponding positive phase in a separate fresh stage. The positive must include state, lift, generic prefix, exact suffix and native original257 consumer, empty observed axioms and all kernel replays. Helpers alone do not accept the bridge. Parent must provide a separately reviewed invocation; these instructions grant no execution authority.\n\nAll shared closure files are pinned references, not copied. The accepted symbolic closure includes one unselected negative module, preserved as an input but excluded by the selected import graph. Config changes only the entrypoint. No production lock, SDK substitute or runtime definition change is supplied. The native DAG contains ordinary theorem Eq.trans with Except error-first type and Type0 Eq universe1; typed-endpoint-mapping.json.gz records every lowered A/M/E subtree for independent checking against the pinned plan. Semantic correspondence is byte-canonical parsed AST identity. author-source.mjs is an inert reproducible source authoring record; it uses exclusive writes and must never be rerun inside this frozen directory.\n\nThe remaining 193 bytes/seven atoms and all original39 control propositions remain open. Accepted count331/403 is unchanged. All17 current files and original17 snapshots remain byte-preserved. No cache, speed or full257 claim.\n`);
json('budget.json',{capBytes:cap,reserveBytes:String(reserve),observations,allocatedBeforeFinalRecords:alloc(),temporaryDiskCopies:0,compressionInMemory:true,nativeCalls:0,qualification:'Write reservations are conservative rounded file sizes with current allocated census; directories and final manifest are separately covered by remaining cap. Not a cross-process filesystem guarantee.'});
const payloads=[];function collect(p){for(const e of fs.readdirSync(p,{withFileTypes:true})){const f=path.join(p,e.name);if(e.isDirectory())collect(f);else payloads.push({...pin(f),path:path.relative(root,f)});}}collect(root);
budget('manifest reserve',32768);
json('manifest.json',{schema:'lexlean-r4-second-state-source-candidate/v1',status:'FROZEN_SOURCE_ONLY_PENDING_INDEPENDENT_SPEC_CODE_ALL_NATIVE_NOT_RUN',createdAt:new Date().toISOString(),payloads,dependencies:[...dependencies.values()],newNativeOutcomes:'NOT_RUN',limits:plan.limits,source17Unchanged:true,snapshot17Unchanged:true,allocatedPayloadBytes:alloc(),additionalAllocationCap:cap,freeze:'Manifest excludes itself; exact manifest hash reported separately.'});
assert(alloc()<=cap);console.log(JSON.stringify({manifest:pin(path.join(root,'manifest.json')),payloadCount:payloads.length,dependencies:dependencies.size,allocatedBytes:alloc(),nodes:nodes.length,mapping:mapping.length,phaseGraphs:graph},null,2));
