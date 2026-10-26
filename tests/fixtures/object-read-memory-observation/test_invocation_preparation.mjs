import assert from 'node:assert/strict';
import test from 'node:test';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import childProcess from 'node:child_process';
import {registerHooks, syncBuiltinESMExports} from 'node:module';

const candidate = new URL('./invocation-preparation.mjs', import.meta.url);
const retained = JSON.parse(fs.readFileSync(new URL('./design/original-invocation.json', import.meta.url)));
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const oldGate = "if(freshEpochBinding===null)throw Error('NO_REVIEWED_FRESH_EPOCH: preparation only; no output or process invocation');\n";
const oldArgs = 'if(process.argv.length!==3||!["child-accounting-negative","original-admitted-negative","complete-admitted-positive"].includes(phase))throw Error(\'select exactly one reviewed admitted product phase; no automatic progression\');\n';
const newMain = '\nexport function main(argv){\n if(!Array.isArray(argv)||argv.length!==1||argv[0]!==phase)throw Error(\'select exactly complete-admitted-positive; preparation only\');\n '+oldGate+' fs.mkdirSync(out);\n runOriginal();\n}\n';

function expected(record = retained) {
  assert.equal(record.bytes, 12797);
  assert.equal(Buffer.byteLength(record.text), 12797);
  assert.equal(sha(record.text), '3cc48e7aef25481a08f64bb9e2ea2f2b6b32a2e48109659b0322efd90d099b27');
  let source = record.text;
  const once = (oldText, newText) => {
    assert.equal(source.split(oldText).length, 2, 'unique pinned edit');
    source = source.replace(oldText, newText);
  };
  once('const phase=process.argv[2];', "const phase='complete-admitted-positive';");
  once(oldArgs, '');
  once('// Binding below records the actual reviewed restart snapshot; it does not assert present storage or readiness.\n', '// Frozen source preparation has no live resource epoch or execution authority.\n');
  const epochLine = source.split('\n').filter(line => line.startsWith('const freshEpochBinding='));
  assert.equal(epochLine.length, 1);
  once(epochLine[0], 'const freshEpochBinding=null;');
  once(oldGate, '');
  once('fs.mkdirSync(out);\n', '');
  once('\ntry{\n', '\nfunction runOriginal(){\ntry{\n');
  return repaired(integrated(source+'}\n'+newMain));
}

function integrated(source){
 const once=(before,after)=>{assert.equal(source.split(before).length,2,'one direct integration edit');source=source.replace(before,after);};
 once("import {spawnSync} from 'node:child_process';", "import {spawnSync} from 'node:child_process';\nimport {openConnection} from './invocation-connection.mjs';");
 once('function runOriginal(){\ntry{',`async function runOriginal(connection){
 const observationErrors=[],completionErrors=[];let completed=0;
 const checkpoint=(sequence,required=true)=>{
  if(connection.failed){if(required)throw Error('observation stopped');return;}
  try{connection.boundary(sequence);completed=sequence;}
  catch(error){observationErrors.push({sequence,error:String(error)});if(required)throw error;}
 };
try{
try{`);
 once(" save('expected-inputs',inputs);", " save('expected-inputs',inputs);\n checkpoint(1);");
 once(" sources('source-before');", " sources('source-before');\n checkpoint(2);");
 once(" const lockResult=cli('lock',['lock'],false);", " checkpoint(3);\n const lockResult=cli('lock',['lock'],false);\n checkpoint(4);");
 once("  const checked=cli('check',['check',target],false);const checkParsed=JSON.parse(checked.stdout||'null');", "  checkpoint(5);\n  const checked=cli('check',['check',target],false);checkpoint(6);const checkParsed=JSON.parse(checked.stdout||'null');");
 once("   const result=cli('verify',['verify',target],false);const parsed=JSON.parse(result.stdout||'null');", "   checkpoint(7);\n   const result=cli('verify',['verify',target],false);checkpoint(8,false);\n   checkpoint(9,false);\n   try{\n   const parsed=JSON.parse(result.stdout||'null');");
 once("path.join(out,'artifact')]);}\n", "path.join(out,'artifact')]);}\n   }finally{checkpoint(10,false);}\n");
 once(" save('postcapture-errors',errors);", " if(completed===10)checkpoint(11,false);\n save('postcapture-errors',errors);");
 once(" save('postcapture-errors',errors);", " try{save('postcapture-errors',errors);}catch(error){completionErrors.push(error);}");
 once("}\n}\n\nexport function main", `}
}catch(error){completionErrors.unshift(error);
}finally{
 try{await connection.close();}catch(error){completionErrors.push(error);observationErrors.push({cleanup:String(error)});}
 if(observationErrors.length){process.exitCode=1;try{save('observation-errors',observationErrors);}catch(error){completionErrors.push(error);}}
}
 if(completionErrors.length===1)throw completionErrors[0];
 if(completionErrors.length>1)throw new AggregateError(completionErrors,'invocation_completion');
}

export function main`);
 once(' runOriginal();', ' return openConnection().then(connection=>runOriginal(connection));');
 return source;
}

function guarded(callback) {
  const calls = [], restored = [];
  for (const [owner, names] of [[fs, ['mkdirSync', 'writeFileSync', 'statfsSync', 'readFileSync']],
                               [childProcess, ['spawnSync','spawn']]]) {
    for (const name of names) {
      const original = owner[name];
      restored.push(() => { owner[name] = original; });
      owner[name] = () => { calls.push(name); throw Error('forbidden side effect '+name); };
    }
  }
  syncBuiltinESMExports();
  return Promise.resolve().then(callback).then(value => {
    assert.deepEqual(calls, []);
    return value;
  }).finally(() => { for (const restore of restored) restore(); syncBuiltinESMExports(); });
}

test('retained original has independent fixed byte identity; corrupt data rejects', () => {
  expected();
  assert.throws(() => expected({...retained, bytes: 1}));
  assert.throws(() => expected({...retained, text: retained.text+' '}));
  const corrupted = retained.text.replace('const phase=process.argv[2];', 'const phase=process.argv[3];');
  assert.notEqual(corrupted, retained.text);
  assert.throws(() => expected({...retained, text: corrupted}));
});

test('prepared source is the exact allowlisted adaptation, with no other edits', () => {
  assert.equal(fs.existsSync(candidate), true, 'preparation source missing');
  assert.equal(fs.readFileSync(candidate, 'utf8'), expected());
});

test('import is inert and exports only the closed main entry', async () => {
  assert.equal(fs.existsSync(candidate), true, 'preparation source missing');
  const source = fs.readFileSync(candidate, 'utf8');
  assert.equal(source, expected());
  const helper=new URL('./invocation-connection.mjs',import.meta.url);
  const helperSource=fs.readFileSync(helper,'utf8'),helperHash=sha(helperSource);
  const sources=new Map([[candidate.href,source],[helper.href,helperSource]]),loads=new Map();
  const hook = registerHooks({load(url, context, nextLoad) {
    if (!sources.has(url)) return nextLoad(url, context);
    loads.set(url,(loads.get(url)||0)+1);
    return {format: 'module', source:sources.get(url), shortCircuit: true};
  }});
  try {
    const module = await guarded(() => import(candidate.href));
    assert.equal(loads.get(candidate.href), 1);
    assert.equal(loads.get(helper.href),1);
    assert.equal(sha(helperSource),helperHash);
    assert.deepEqual(Object.keys(module), ['main']);
  } finally { hook.deregister(); }
});

test('positive-only arguments and immutable null gate reject before owned effects', async () => {
  assert.equal(fs.existsSync(candidate), true, 'preparation source missing');
  const {main} = await import(candidate.href);
  await guarded(() => {
    for (const argv of [undefined, null, {}, '', [], ['child-accounting-negative'],
                       ['original-admitted-negative'], ['complete-admitted-positive', 'extra']]) {
      assert.throws(() => main(argv), /select exactly complete-admitted-positive/);
    }
    assert.throws(() => main(['complete-admitted-positive']), /NO_REVIEWED_FRESH_EPOCH/);
  });
});

test('preservation oracle rejects representative changed obligations as text only', () => {
  const source = expected();
  const changes = [
    ['if(inputs.length!==50', 'if(inputs.length!==49'],
    ['ObjectAdmittedRangeResult.lex.tex', 'WrongResult.lex.tex'],
    ['107e02328a8052d1560d6e8addcfd1c70af6145bbf836650cac32dc044f550e5', '0'.repeat(64)],
    ['maxBuffer:20*1024*1024', 'maxBuffer:21*1024*1024'],
    ["['memory-after',()=>zeroMemory('memory-after')]", "['memory-after',()=>undefined]"],
    ["const result=cli('verify'", "cli('duplicate',['verify',target],false);const result=cli('verify'"],
  ];
  for (const [before, after] of changes) {
    assert.ok(source.includes(before));
    assert.throws(() => assert.equal(source.replace(before, after), expected()));
  }
  assert.throws(() => assert.equal(source+'fs.mkdirSync("unexpected");\n', expected()));
});

// Isolated-loader seam: only this test instance exposes the actual private body.
async function actualFlow({lock=0,check=0,verify=0,rejectBoundary=0,badParse='',guardFailure='',captureEscape=false,failSaves=[],cleanupFailure=false,rootFailure=false,dataFailure=''}={}){
 const originalRead=fs.readFileSync, source=originalRead(candidate,'utf8');
 const captures=new Map(),calls=[],boundaries=[];
 const flowURL=candidate.href+'?finite-flow='+Math.random();
 const hook=registerHooks({load(url,ctx,next){return url===flowURL?{format:'module',source:source+'\nexport {runOriginal, expectedSelectedModules};\nexport function testEpochLeaf(fn){epoch=fn;}\n',shortCircuit:true}:next(url,ctx);}});
 const changed=[];const replace=(obj,key,value)=>{const old=obj[key];changed.push(()=>obj[key]=old);obj[key]=value;};
 let connectionClosed=0;
 const connection={verifyArguments(stage){return ['exec','-w',stage,'cbd8cf8ac7240f703c9492d11b4a442474cd303adfb147a82648a2e5cb6bcfdf','lexlean','--diagnostic-format','json','verify','tests/lexlean/ObjectAdmittedRangeResult.lex.tex'];},failed:false,boundary(n){boundaries.push(n);calls.push(['boundary',n]);if(n===rejectBoundary){this.failed=true;throw Error('fixture ACK'+n);}return {sequence:n};},async close(){connectionClosed++;if(cleanupFailure)throw Error('fixture cleanup');},retainRoot(){calls.push('root-retention');if(rootFailure==='nested')throw new AggregateError([new AggregateError([Error('fixture inner IO')],'connection_io')],'root_acquisition');if(rootFailure)throw Error('fixture root retention');}};
 let module,toolReads=0;
 const inputRoot=new URL('../object-read-child-certificate-native/',import.meta.url);
 const manifestFile=new URL('manifest.json',inputRoot),manifestRecord=JSON.parse(originalRead(manifestFile));
 const repoRoot=candidate.pathname.split('/tests/fixtures/')[0];
 const corruptPaths={manifest:manifestFile.pathname,payload:new URL(manifestRecord.payloads[0].path,inputRoot).pathname,dependency:path.resolve(repoRoot,manifestRecord.dependencies[0].path)};
 try{
  module=await import(flowURL);hook.deregister();
  module.testEpochLeaf(name=>{calls.push(name);if(name===guardFailure)throw Error('fixture guard');});
  replace(fs,'readFileSync',(file,...args)=>{const value=originalRead(file,...args),name=file instanceof URL?file.pathname:String(file);return name===corruptPaths[dataFailure]?(typeof value==='string'?value+' ':Buffer.concat([value,Buffer.from(' ')])):value;});
  replace(fs,'statfsSync',()=>({bavail:100n*1024n**3n,bsize:1n}));
  replace(fs,'writeFileSync',(file,raw)=>{const name=String(file).split('/').at(-1);calls.push(['save',name]);if(name===guardFailure+'.json'||failSaves.includes(name))throw Error('fixture save '+name);captures.set(name,JSON.parse(raw));});
  const toolHashes=['107e02328a8052d1560d6e8addcfd1c70af6145bbf836650cac32dc044f550e5','79fb1d26fa5a39385d59fdc48a711a14b0710ca6480271acce99b4d177cea085','8229bc302b0b7d0e7679e28cc2b830c5aec995ffe5714833503360252f1490ba','8bc50bb93c2d510a366b879161ea5f0741e4039b6c43bd55c3b9b8a7b15f13de'];
  replace(childProcess,'spawnSync',(exe,args)=>{
   assert.ok(['docker','du','sysctl'].includes(exe),'unlisted executable');
   calls.push([exe,...args]);let stdout='',status=0;
   if(exe==='docker'){
    if(args[0]==='cp'){}
    else {assert.equal(args[0],'exec');const leaf=args[1]==='-w'?args.slice(4):args.slice(2);
     if(leaf[0]==='lexlean'){const action=leaf[3];status={lock,check,verify}[action];assert.notEqual(status,undefined,'unlisted SDK action');stdout=badParse===action?'{':JSON.stringify({success:status===0,modules:dataFailure==='selectedModules'&&action==='check'?module.expectedSelectedModules.slice(1):module.expectedSelectedModules,artifacts:['.lexlean/verified/'+'a'.repeat(64)]});}
     else if(leaf[0]==='mkdir'){}
     else if(leaf[0]==='ps')stdout='';
     else if(leaf[0]==='df')stdout='Filesystem 1024-blocks Used Available Capacity Mounted\nf 1 1 20000000 1% /tmp\n';
     else if(leaf[0]==='cat')stdout=leaf[1].endsWith('memory.events')?'low 0\nhigh 0\nmax 0\noom 0\noom_kill 0\noom_group_kill 0\n':'fixture-lock';
     else if(leaf[0]==='sha256sum'){
      const inputs=captures.get('expected-inputs.json')||[];
      stdout=leaf.slice(1).map((file,i)=>{const input=inputs.find(x=>file.endsWith('/'+x.destination));return (file==='/usr/local/bin/lexlean'||leaf[1]==='/usr/local/bin/lexlean'?toolHashes[i]:file.endsWith('/lexlean.lock')?sha('fixture-lock'):input?.sha256)+'  '+file;}).join('\n')+'\n';
      if(leaf[1]==='/usr/local/bin/lexlean'&&toolReads++===0){if(dataFailure==='toolHash')stdout='0'.repeat(64)+stdout.slice(64);if(dataFailure==='toolCount')stdout=stdout.split('\n').slice(1).join('\n');}
     }else assert.fail('unlisted leaf '+leaf[0]);
    }
   }
   return {status,signal:null,stdout,stderr:''};
  });
  syncBuiltinESMExports();let escaped;
  try{await module.runOriginal(connection);}catch(error){if(!captureEscape)throw error;escaped=error;}
  return {captures,calls,boundaries,connectionClosed,escaped};
 }finally{for(const restore of changed.reverse())restore();syncBuiltinESMExports();hook.deregister();process.exitCode=0;}
}

test('actual private invocation body connects all eleven boundaries',async()=>{
 const result=await actualFlow();
 assert.equal(result.captures.get('failure.json'),undefined,'finite transcript must reach real body result');
 assert.deepEqual(result.boundaries,[1,2,3,4,5,6,7,8,9,10,11]);
 assert.ok(result.captures.get('result.json'));
 const at=n=>result.calls.findIndex(c=>Array.isArray(c)&&c[0]==='boundary'&&c[1]===n);
 for(const [action,before,after] of [['lock',3,4],['check',5,6],['verify',7,8]]){
  const command=result.calls.findIndex(c=>Array.isArray(c)&&c[0]==='docker'&&c.includes(action));
  assert.ok(at(before)<command&&command<at(after),action+' bracket');
 }
 assert.ok(at(8)<result.calls.findIndex(c=>Array.isArray(c)&&c[0]==='save'&&c[1]==='result.json'));
});
test('actual early phase failures never skip controls or invoke verify',async()=>{
 for(const [options,prefix] of [[{lock:1},4],[{check:1},6],[{badParse:'lock'},4],[{badParse:'check'},6],[{rejectBoundary:7},7]]){
  const result=await actualFlow(options);
  assert.deepEqual(result.boundaries,Array.from({length:prefix},(_,i)=>i+1));
  assert.equal(result.calls.some(c=>Array.isArray(c)&&c.includes('verify')),false);
  assert.equal(result.connectionClosed,1);
  assert.ok(result.captures.has('postcapture-errors.json'));
  assert.ok(result.calls.includes('epoch-after'));
 }
});
test('ACK8 failure poisons publication but preserves SDK artifact and guards',async()=>{
 const result=await actualFlow({rejectBoundary:8});
 assert.deepEqual(result.boundaries,[1,2,3,4,5,6,7,8]);
 assert.equal(result.captures.get('result.json').verifyStatus,0);
 assert.ok(result.captures.has('artifact-copy.json'));
 assert.ok(result.captures.has('observation-errors.json'));
 for(const name of ['source-after','lock-after','tools-after','memory-after','idle-after','guest-after','host-after'])assert.ok(result.captures.has(name+'.json'),name);
 assert.ok(result.calls.includes('epoch-after'));assert.equal(result.connectionClosed,1);
});
test('verify failure skips artifact but records copy-decision boundaries',async()=>{
 const result=await actualFlow({verify:1});assert.deepEqual(result.boundaries,[1,2,3,4,5,6,7,8,9,10,11]);
 assert.equal(result.captures.has('artifact-copy.json'),false);assert.equal(result.captures.get('result.json').verifyStatus,1);
});
test('malformed verify and failed postguard preserve cleanup and other guards',async()=>{
 for(const options of [{badParse:'verify'},{guardFailure:'epoch-after'}]){
  const result=await actualFlow(options);assert.equal(result.connectionClosed,1);
  for(const name of ['memory-after','idle-after','guest-after','host-after'])assert.ok(result.captures.has(name+'.json'));
  assert.equal(result.boundaries.at(-1),11);
 }
});
test('each independent guard failure leaves all other guards and cleanup attempted',async()=>{
 const names=['source-after','lock-after','tools-after','epoch-after','memory-after','idle-after','guest-after','host-after'];
 for(const guardFailure of names){
  const result=await actualFlow({guardFailure});
  for(const name of names){
   if(name==='epoch-after')assert.ok(result.calls.includes(name));
   else assert.ok(result.calls.some(c=>Array.isArray(c)&&c[0]==='save'&&c[1]===name+'.json'),name+' attempted after '+guardFailure);
  }
  assert.ok(result.captures.get('postcapture-errors.json').some(e=>e.name===guardFailure));assert.equal(result.connectionClosed,1);
 }
});
test('SDK result/copy receipt failures still reach all guards and owned cleanup',async()=>{
 for(const guardFailure of ['result','artifact-copy','postcapture-errors','failure']){
  const result=await actualFlow({guardFailure,captureEscape:true,...(guardFailure==='failure'?{check:1}:{})});
  if(['failure','postcapture-errors'].includes(guardFailure))assert.match(String(result.escaped),/fixture save/);
  else{assert.equal(result.escaped,undefined);assert.ok(result.captures.has('failure.json'));}
  assert.equal(result.connectionClosed,1);assert.ok(result.calls.includes('epoch-after'));
  assert.ok(result.calls.some(c=>Array.isArray(c)&&c[0]==='save'&&c[1]==='host-after.json'));
 }
});
test('primary and cleanup receipt failures remain independently visible',async()=>{
 const result=await actualFlow({check:1,captureEscape:true,cleanupFailure:true,failSaves:['failure.json','observation-errors.json']});
 assert.ok(result.escaped instanceof AggregateError,'compound failures must not overwrite primary');
 const messages=result.escaped.errors.map(String).join('\n');
 assert.match(messages,/failure.json/);assert.match(messages,/observation-errors.json/);assert.match(messages,/fixture cleanup/);
 assert.equal(result.connectionClosed,1);assert.ok(result.calls.includes('epoch-after'));
});

test('return boundaries precede command persistence and retain root despite save failure',async()=>{
 const normal=await actualFlow();
 for(const [name,n] of [['lock',4],['check',6],['verify',8]]){
  const boundary=normal.calls.findIndex(c=>Array.isArray(c)&&c[0]==='boundary'&&c[1]===n);
  const saved=normal.calls.findIndex(c=>Array.isArray(c)&&c[0]==='save'&&c[1]===name+'.json');
  assert.ok(boundary<saved,name+' return boundary must precede capture save');
  const failed=await actualFlow({failSaves:[name+'.json'],captureEscape:true});
  assert.ok(failed.boundaries.includes(n));assert.equal(failed.connectionClosed,1);
  assert.equal(failed.captures.get('failure.json').returned.name,name,'actual returned tuple survives capture-save error');
  assert.equal(failed.captures.get('failure.json').returned.value.status,0);
  if(name==='verify'){assert.equal(failed.calls.filter(c=>c==='root-retention').length,1);assert.ok(failed.captures.has('artifact-copy.json'));}
  else assert.equal(failed.calls.includes('root-retention'),false);
 }
 assert.equal(normal.calls.filter(c=>c==='root-retention').length,1);
});

function repaired(source){
 const edits=[["const save=(name,value)=>fs.writeFileSync(path.join(out,name+'.json'),JSON.stringify(value,null,2)+'\\n',{flag:'wx'});\nfunction command(name,exe,args,required=true){\n const started=new Date().toISOString();\n const r=spawnSync(exe,args,{env,encoding:'utf8',maxBuffer:20*1024*1024});\n const value={args,started,ended:new Date().toISOString(),status:r.status,signal:r.signal,stdout:r.stdout,stderr:r.stderr,error:r.error?.message};\n save(name,value);if(required&&(r.error||r.status!==0))throw Error(name+' failed');return value;\n}\n", "const save=(name,value)=>fs.writeFileSync(path.join(out,name+'.json'),JSON.stringify(value,null,2)+'\\n',{flag:'wx'});\nfunction returned(exe,args){\n const started=new Date().toISOString();\n const r=spawnSync(exe,args,{env,encoding:'utf8',maxBuffer:20*1024*1024});\n return {args,started,ended:new Date().toISOString(),status:r.status,signal:r.signal,stdout:r.stdout,stderr:r.stderr,error:r.error?.message};\n}\nfunction command(name,exe,args,required=true){\n const value=returned(exe,args);\n save(name,value);if(required&&(value.error||value.status!==0))throw Error(name+' failed');return value;\n}\n"], ["function sources(name){const r=guest(name,['sha256sum',...inputs.map(x=>stage+'/'+x.destination)]);const lines=r.stdout.trim().split('\\n');if(lines.length!==inputs.length||lines.some((l,i)=>l.slice(0,64)!==inputs[i].sha256))throw Error('source mismatch');}\nasync function runOriginal(connection){\n const observationErrors=[],completionErrors=[];let completed=0;\n const checkpoint=(sequence,required=true)=>{\n", "function sources(name){const r=guest(name,['sha256sum',...inputs.map(x=>stage+'/'+x.destination)]);const lines=r.stdout.trim().split('\\n');if(lines.length!==inputs.length||lines.some((l,i)=>l.slice(0,64)!==inputs[i].sha256))throw Error('source mismatch');}\nfunction errorMessages(error){\n const seen=new Set(),pending=[error],messages=[];\n for(const item of pending){if(seen.has(item))continue;seen.add(item);messages.push(String(item));if(item instanceof AggregateError)for(const inner of item.errors)pending.push(inner);}\n return messages;\n}\nasync function runOriginal(connection){\n const observationErrors=[],completionErrors=[];let completed=0,returnedTuple=null;\n const checkpoint=(sequence,required=true)=>{\n"], [" checkpoint(2);\n const cli=(name,args,required=true)=>docker(name,['exec','-w',stage,cid,'lexlean','--diagnostic-format','json',...args],required);\n checkpoint(3);\n const lockResult=cli('lock',['lock'],false);\n checkpoint(4);\n const lockParsed=JSON.parse(lockResult.stdout||'null');\n", " checkpoint(2);\n const cli=(name,args,required=true)=>{const value=returned('docker',name==='verify'?connection.verifyArguments(stage):['exec','-w',stage,cid,'lexlean','--diagnostic-format','json',...args]);returnedTuple={name,value};return value;};\n checkpoint(3);\n const lockResult=cli('lock',['lock'],false);\n checkpoint(4);save('lock',lockResult);\n const lockParsed=JSON.parse(lockResult.stdout||'null');\n"], ["  checkpoint(5);\n  const checked=cli('check',['check',target],false);checkpoint(6);const checkParsed=JSON.parse(checked.stdout||'null');\n  {\n", "  checkpoint(5);\n  const checked=cli('check',['check',target],false);checkpoint(6);save('check',checked);const checkParsed=JSON.parse(checked.stdout||'null');\n  {\n"], ["   try{\n   const parsed=JSON.parse(result.stdout||'null');\n   save('result',{phase,stage,verifyStatus:result.status,result:parsed});\n   if(result.status===0&&parsed?.success===true){const artifact=parsed.artifacts?.[0];if(!/^\\.lexlean\\/verified\\/[a-f0-9]{64}$/.test(artifact))throw Error('artifact path');docker('artifact-copy',['cp',cid+':'+stage+'/'+artifact,path.join(out,'artifact')]);}\n   }finally{checkpoint(10,false);}\n", "   try{\n   const failures=[];\n   const attempt=async fn=>{try{return await fn();}catch(error){failures.push(error);}};\n   await attempt(()=>save('verify',result));\n   await attempt(()=>connection.retainRoot());\n   let parsed;\n   await attempt(()=>{parsed=JSON.parse(result.stdout||'null');});\n   await attempt(()=>save('result',{phase,stage,verifyStatus:result.status,result:parsed}));\n   await attempt(()=>{if(result.status===0&&parsed?.success===true){const artifact=parsed.artifacts?.[0];if(!/^\\.lexlean\\/verified\\/[a-f0-9]{64}$/.test(artifact))throw Error('artifact path');docker('artifact-copy',['cp',cid+':'+stage+'/'+artifact,path.join(out,'artifact')]);}});\n   if(failures.length)throw new AggregateError(failures,'postverify_obligations');\n   }finally{checkpoint(10,false);}\n"], ["\n}catch(e){save('failure',{message:String(e),stack:e?.stack});process.exitCode=1;}\nfinally{\n", "\n}catch(e){save('failure',{message:String(e),stack:e?.stack,returned:returnedTuple,errors:errorMessages(e)});process.exitCode=1;}\nfinally{\n"]];
 for(const [before,after] of edits){assert.equal(source.split(before).length,2,'one reviewed repair edit');source=source.replace(before,after);}
 return source;
}

test('every postverify obligation is independent under compound capture failures',async()=>{
 for(const options of [{verify:1},{rejectBoundary:8},{rootFailure:true},{failSaves:['verify.json']},{failSaves:['result.json']},{rootFailure:true,rejectBoundary:8,failSaves:['verify.json','result.json']}]){
  const r=await actualFlow(options);assert.equal(r.calls.filter(x=>x==='root-retention').length,1);
  assert.equal(r.captures.has('artifact-copy.json'),options.verify!==1);
  for(const name of ['source-after','lock-after','tools-after','memory-after','idle-after','guest-after','host-after'])assert.ok(r.captures.has(name+'.json'));
  assert.ok(r.calls.includes('epoch-after'));assert.equal(r.connectionClosed,1);
 }
});

test('root acquisition nested IO failures survive final retained failure report',async()=>{
 const r=await actualFlow({rootFailure:'nested'});assert.match(JSON.stringify(r.captures.get('failure.json')),/fixture inner IO/);assert.ok(r.captures.has('artifact-copy.json'));
});

test('actual corrupt inputs tools and selected closure fail before verify',async()=>{
 for(const [dataFailure,reason] of [['manifest','manifest drift'],['payload','candidate drift'],['dependency','authority drift'],['toolHash','tool drift'],['toolCount','tool drift'],['selectedModules','admitted selected closure mismatch']]){
  const r=await actualFlow({dataFailure});assert.ok(r.captures.has('failure.json'),dataFailure+' must fail');assert.match(r.captures.get('failure.json').message,new RegExp(reason),dataFailure);
  assert.equal(r.calls.some(c=>Array.isArray(c)&&c.includes('verify')),false,dataFailure+' must not invoke verify');
  assert.equal(r.calls.includes('root-retention'),false);assert.equal(r.connectionClosed,1);
  for(const name of ['tools-after','memory-after','idle-after','guest-after','host-after'])assert.ok(r.captures.has(name+'.json'),dataFailure+' '+name);
  assert.ok(r.calls.includes('epoch-after'));assert.ok(r.captures.has('postcapture-errors.json'));
  if(dataFailure==='selectedModules'){assert.ok(r.captures.has('source-after.json'));assert.ok(r.captures.has('lock-after.json'));assert.deepEqual(r.boundaries,[1,2,3,4,5,6]);}
 }
});
