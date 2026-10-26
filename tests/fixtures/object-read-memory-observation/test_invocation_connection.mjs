import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import crypto from 'node:crypto';
import os from 'node:os';
import path from 'node:path';
import {registerHooks,syncBuiltinESMExports} from 'node:module';
import childProcess from 'node:child_process';
import {EventEmitter} from 'node:events';
const url=new URL('./invocation-connection.mjs',import.meta.url);
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const wire=x=>Buffer.from(JSON.stringify(x).replace(/[\u007f-\uffff]/g,c=>'\\u'+c.charCodeAt(0).toString(16).padStart(4,'0'))+'\n');
let module;
async function candidate(){
 assert.ok(fs.existsSync(url),'direct connection implementation missing');
 if(!module){
  const source=fs.readFileSync(url,'utf8');
  const suffix='\nexport {admitBoundaryPrefix,OWNER_BOOTSTRAP};\nexport const testHostClass=typeof HostConnection===\'undefined\'?undefined:HostConnection;\nexport const testDecodeOwner=typeof decodeOwner===\'undefined\'?undefined:decodeOwner;\nexport const testConfig=typeof decodeConfiguration===\'undefined\'?undefined:decodeConfiguration;\nexport function testObservationTick(s){return ackTime(s); }\nexport function testObservationBindings(o){const a=ackTime,s=sleepSync,p=admitBoundaryPrefix;const restore=[];try{ackTime=o.ack(a);restore.push(()=>{ackTime=a;});sleepSync=o.sleep(s);restore.push(()=>{sleepSync=s;});admitBoundaryPrefix=o.prefix(p);restore.push(()=>{admitBoundaryPrefix=p;});}catch(e){o.errors[3].push(e);}return ()=>{for(const f of restore.reverse())try{f();}catch(e){o.errors[3].push(e);}};}\n';
  const hook=registerHooks({load(u,c,next){return u===url.href?{format:'module',source:source+suffix,shortCircuit:true}:next(u,c);}});
  try{module=await import(url.href);}finally{hook.deregister();}
 }
 return module;
}
const identity=['a'.repeat(64),'sha256:'+'b'.repeat(64),'2026-10-26T00:00:00Z','12345678-1234-1234-1234-123456789abc',['/owned',1,2,16832],['99','100','/observer']];
const nonce='c'.repeat(64), binding=sha(wire(identity));
function records(sequence=1, mutate=()=>{},headerMutation=()=>{},statPair=null){
 const keys=['high','low','max','oom','oom_group_kill','oom_kill'];
 let rows=[['h',3,nonce,identity[3],identity.slice(0,3),identity[4],identity[5],'1000','relative_us',{quotaProfile:'owner-retention/1',pidFlags:{rssAbsent:1,hwmAbsent:2,namespaceAbsent:4,parentUnknown:8},sampleFlags:{peakUnsupported:1},slowFlags:{pressureUnsupported:1,localUnsupported:2},maxPids:64,maxIdentities:1024,maxKeysets:32}],['e',1,keys]];
 if(statPair)rows.push(['i',1,'123','222','41',null,'owned_unattributed',null,Buffer.from('lexlean\0').toString('base64'),['123'],Buffer.from('0::/owned\n').toString('base64'),...statPair.map(b=>b.toString('base64')),['not_observed']]);
 headerMutation(rows[0]);let journal=Buffer.concat(rows.map(wire)), request,ack;
 for(let n=1;n<=sequence;n++){
  const s=['s',n,String(n),String(n),1,keys.map(()=>'0'),'123','0','456',[],0,0];
  const m=['m',n,String(n),String(n),'anon 100\n',null,null,3];
  if(statPair)s[9]=[[1,'0','0',0]];
  mutate(s,m,n,rows);
  journal=Buffer.concat([journal,wire(s),wire(m)]);
  const labels=['staging-before','staging-after','lock-before','lock-after','check-before','check-after','before-verify','verify-return-before-copy','copy-start','copy-end','postguard-complete'];
  request=wire(['q',1,nonce,n,labels[n-1]]);
  const first=wire(['b',n,labels[n-1],String(n),n,String(journal.length),'unsynced',[0,request.toString('base64'),null]]);
  const offset=journal.length+first.length;
  ack=wire(['a',1,nonce,n,labels[n-1],n,String(offset),String(offset),binding]);
  const second=wire(['b',n,labels[n-1],String(n),n,String(offset),'synced',[0,null,ack.toString('base64')]]);
  journal=Buffer.concat([journal,first,second]);
 }
 return {journal,request,ack,identity,previousPrefix:null};
}
test('admit actual request-ending prefix and adjacent ACK copy',async()=>{
 const {admitBoundaryPrefix}=await candidate();const data=records();
 const got=admitBoundaryPrefix(data);
 assert.equal(got.sequence,1);assert.equal(got.copyEnd,data.journal.length);
 assert.equal(got.declaredOffset,Number(JSON.parse(data.ack)[6]));
 assert.equal(got.hash,sha(data.journal));
});
test('before7 admits complete all-zero history, rejects semantic forgeries',async()=>{
 const {admitBoundaryPrefix}=await candidate();
 assert.equal(admitBoundaryPrefix(records(7)).sequence,7);
 for(const mutate of [
  (s,m,n)=>{if(n===2)s[5][2]='1';},
  s=>{s[5].pop();},s=>{s[4]=2;},s=>{s[6]=null;},
  (s,m)=>{m[4]='broken\n';},(s,m)=>{m[1]=90;},
  (s,m)=>{m[6]='low 0\n';m[7]=1;},
  s=>{s[9]=[[99,'0','0',0]];},s=>{s[2]='00';}
 ]) assert.throws(()=>admitBoundaryPrefix(records(7,mutate)));
});
test('prior prefix includes ACK-copy and permits only unchanged append',async()=>{
 const {admitBoundaryPrefix}=await candidate();const first=records(1),last=records(2);
 last.previousPrefix=admitBoundaryPrefix(first);
 assert.equal(admitBoundaryPrefix(last).sequence,2);
 last.journal=Buffer.from(last.journal);last.journal[20]^=1;
 assert.throws(()=>admitBoundaryPrefix(last));
});
test('actual owned host files publish once and semantically admit ACK',async()=>{
 const {testHostClass:HostConnection}=await candidate();assert.equal(typeof HostConnection,'function');
 const dir=fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()),'r4-node-'));fs.chmodSync(dir,0o700);
 try{
  const pin=fs.statSync(dir,{bigint:true});const data=records();
  fs.writeFileSync(path.join(dir,'journal.jsonl'),data.journal,{mode:0o600});
  fs.writeFileSync(path.join(dir,'ack'),data.ack,{mode:0o600});
  const connection=new HostConnection({directory:dir,pin:[pin.dev,pin.ino,pin.uid,pin.mode].map(String),nonce,identity});
  try{
   assert.equal(connection.boundary(1).sequence,1);
   assert.deepEqual(fs.readFileSync(path.join(dir,'request')),data.request);
   assert.throws(()=>connection.boundary(1));
   assert.throws(()=>connection.boundary(2));
  }finally{connection.closeFiles();}
 }finally{fs.rmSync(dir,{recursive:true});}
});
test('owner receipt decoder rejects unknown/type/canonical and status combinations',async()=>{
 const {testDecodeOwner:decodeOwner}=await candidate();assert.equal(typeof decodeOwner,'function');
 const v=['oe',1,nonce,'d'.repeat(64),'99','100',['exited',0,null],'natural'];
 assert.deepEqual(decodeOwner(wire(v)),v);
 for(const mutate of [x=>x[1]=true,x=>x[4]='0',x=>x[6]=['running',null,null],x=>x[7]='lost_custody',x=>x[6]=['exited',true,null],x=>x.push(null)]){
  const bad=structuredClone(v);mutate(bad);assert.throws(()=>decodeOwner(wire(bad)));
 }
 assert.throws(()=>decodeOwner(Buffer.from(JSON.stringify(v)+' \n')));
});
async function finiteExchange(mode='normal'){
 const fixture=new URL('./finite_owner_connection_fixture.py',import.meta.url);
 assert.ok(fs.existsSync(fixture),'finite owner/reader fixture missing');
 const {testHostClass:HostConnection}=await candidate();
 const {spawn}=await import('node:child_process');
 const root=fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()),'r4-owner-node-'));
 let child,connection,done;const observation=mode==='after-exit'?deadlineObserver():null,state={awaited:false,settled:false,result:null};let restoreObservation=()=>{},afterAwait=false;
 try{
  child=spawn('/opt/homebrew/bin/python3',['-B',fixture.pathname,root],{env:{...process.env,PYTHONDONTWRITEBYTECODE:'1'},stdio:['pipe','pipe','pipe']});
  done=new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('finite watchdog transport deadline')),12000);child.once('error',e=>{clearTimeout(timer);reject(e);});child.once('close',(code,signal)=>{clearTimeout(timer);resolve({code,signal});});});
  done.then(result=>{state.settled=true;state.result=result;},()=>{state.settled=true;});
  done.catch(()=>{});
  const data=await new Promise((resolve,reject)=>{
   let raw=Buffer.alloc(0),err=Buffer.alloc(0);const timer=setTimeout(()=>reject(Error('finite fixture readiness timeout')),2000);
   child.stdout.on('data',part=>{raw=Buffer.concat([raw,part]);if(raw.length>16384){clearTimeout(timer);reject(Error('fixture output cap'));}else if(raw.includes(10)){clearTimeout(timer);try{resolve(JSON.parse(raw.subarray(0,raw.indexOf(10))));}catch(e){reject(e);}}});
   child.stderr.on('data',part=>{err=Buffer.concat([err,part]).subarray(0,8192);});
   child.once('close',()=>{clearTimeout(timer);reject(Error('fixture early exit '+err));});child.once('error',e=>{clearTimeout(timer);reject(e);});
  });
  connection=new HostConnection(data);connection.ownTransport(child,data.contextDigest);connection.ready();
  if(observation)restoreObservation=observation.install(connection,(await candidate()).testObservationBindings);
  if(mode==='consumed'){
   const publish=connection.publish.bind(connection);
   connection.publish=raw=>{publish(raw);Atomics.wait(new Int32Array(new SharedArrayBuffer(4)),0,0,2100);assert.equal(JSON.parse(fs.readFileSync(path.join(data.directory,'ack')))[3],1);};
   assert.throws(()=>connection.boundary(1),/deadline/);assert.equal(connection.failed,true);
   assert.throws(()=>connection.boundary(2));assert.equal(JSON.parse(fs.readFileSync(path.join(data.directory,'request')))[3],1);
  }else for(let n=1;n<=11;n++)assert.equal(connection.boundary(n).sequence,n);
  if(mode==='after-exit'){state.awaited=true;afterAwait=true;state.result=await done;}
  if(!observation){
  const exit=await connection.close();
  if(mode==='consumed'){assert.equal(exit[7],'host_eof');assert.ok(['exited','signaled'].includes(exit[6][0]));}
  else{assert.deepEqual(exit[6],['exited',0,null]);assert.equal(exit[7],'natural');}
  assert.deepEqual(await done,{code:0,signal:null});
  if(mode!=='consumed'){const guest=JSON.parse(fs.readFileSync(path.join(data.directory,'guest-terminal.json')));assert.deepEqual(guest.slice(5,7),['ended','ended']);}
  }
 }catch(e){if(!observation)throw e;observation.errors[0].push(e);
 }finally{
  if(observation){
   try{restoreObservation();}catch(e){observation.errors[3].push(e);}
   const exit=await finishDeadlineFixture(observation,connection,child,done,state,afterAwait);
   if(!observation.errors.some(x=>x.length))try{assert.deepEqual(exit[6],['exited',0,null]);assert.equal(exit[7],'natural');assert.deepEqual(state.result,{code:0,signal:null});const guest=JSON.parse(fs.readFileSync(path.join(connection.directory,'guest-terminal.json')));assert.deepEqual(guest.slice(5,7),['ended','ended']);}catch(e){observation.errors[0].push(e);try{observation.snapshot(1,connection,child,state.settled,afterAwait);}catch(err){observation.errors[1].push(err);}}
   try{if(!child||child.exitCode!==null||child.signalCode!==null)fs.rmSync(root,{recursive:true});}catch(e){observation.errors[2].push(e);}
   observation.failure();if(observation.errors.some(group=>group.length>0)){try{console.log('DEADLINE_OBSERVATION '+observation.encode().trimEnd());}catch(e){observation.errors[1].push(e);}throw observation.failure();}
  }else{
   if(connection&&!connection.closed)await connection.close();else if(child)child.stdin.destroy();
   if(done)await done;
   if(!child||child.exitCode!==null||child.signalCode!==null)fs.rmSync(root,{recursive:true});
  }
 }
 assert.equal(fs.existsSync(root),false,'owned finite fixture removed only after transport wait');
}
test('real Node files exchange eleven boundaries with finite delivered owner and reader',()=>finiteExchange());
test('consumed real ACK timeout poisons request and EOF cleans owned observer',()=>finiteExchange('consumed'));
test('EOF after actual exit preserves natural receipt',()=>finiteExchange('after-exit'));
test('independent Python identity vector includes LF; old stripped digest rejects',async()=>{
 const {admitBoundaryPrefix}=await candidate();
 assert.equal(wire(identity).length,251);assert.equal(wire(identity).at(-1),10);
 assert.equal(binding,'e5d2226ebf02bd56986552749a8109823b6a32ab5eda890b974659822bc4eb41');
 const data=records(),old=JSON.parse(data.ack);old[8]='d435e38748f24705477643dc2c8b11bf3219b26eaf96a0205bfc5ae2f432a4db';data.ack=wire(old);
 assert.throws(()=>admitBoundaryPrefix(data),/binding/);
});
function configuration(){
 const pin=['1','2','501','16832'];
 return {schema:'lexlean-r4/invocation-connection/1',guestConfigDirectory:'/guest/config',guestParentPin:pin,
  hostOutputDirectory:'/host/output',hostOutputPin:pin,identity,
  config:{schema:'lexlean-r4/observer-config/1',nonce,context:{docker:[...identity.slice(0,3),'41'],boot:identity[3],init_start:'900',cgroup:['/guest/cgroup',pin],proc:['/guest/proc',pin],boot_directory:['/guest/boot',pin],target:'/owned'},output_directory:['/guest/output',pin]}};
}
test('explicit configuration keeps host and guest pins distinct and rejects malformed data',async()=>{
 const {testConfig:decodeConfiguration}=await candidate();assert.equal(typeof decodeConfiguration,'function');
 const record=configuration();record.identity=structuredClone(record.identity);record.identity[0]=fixedCid;record.config.context.docker[0]=fixedCid;const got=decodeConfiguration(wire(record));
 assert.deepEqual(got.arguments,['--config-directory','/guest/config','--parent-pin','1','2','501','16832']);
 assert.equal(got.directory,'/host/output');assert.equal(got.nonce,nonce);
 for(const mutate of [r=>r.extra=1,r=>r.guestParentPin[0]=true,r=>r.config.context.extra=0,r=>r.config.nonce='d'.repeat(64),r=>r.identity[4][1]=3,r=>r.guestConfigDirectory='../guest',r=>r.config.context.docker[3]='00',r=>r.hostOutputPin=[]]){
  const bad=structuredClone(record);mutate(bad);
  if(bad.config.nonce!==nonce){assert.equal(decodeConfiguration(wire(bad)).nonce,bad.config.nonce);continue;}
  assert.throws(()=>decodeConfiguration(wire(bad)));
 }
 assert.throws(()=>decodeConfiguration(Buffer.from('{"schema":1,"schema":1}\n')));
});
test('fixed production connection rejects absent reviewed descriptor before launch',async()=>{
 const m=await candidate();assert.equal(typeof m.openConnection,'function');
 await assert.rejects(m.openConnection(),error=>error.code==='ENOENT');
});
test('fixed parent bootstrap pins owner closure and reaches real CLI in isolation',async()=>{
 const {OWNER_BOOTSTRAP}=await candidate(),{spawnSync}=await import('node:child_process');
 assert.equal(Buffer.byteLength(OWNER_BOOTSTRAP),3818);
 assert.equal(sha(OWNER_BOOTSTRAP),'57f9d411b178f8c42bf8668935247160eed790b93d6aebe144dac94fa164f3cd');
 assert.equal(sha(fs.readFileSync(new URL('./observer_owner.py',import.meta.url))),'a0e6d5cd2c45600f034aea083dbeaec7361519708b68d4debb99cd31c2e725bc');
 for(const [script,status] of [[OWNER_BOOTSTRAP,64],[OWNER_BOOTSTRAP.replace('a0e6d5cd2c45600f034aea083dbeaec7361519708b68d4debb99cd31c2e725bc','0'.repeat(64)),74]]){
  const result=spawnSync('/opt/homebrew/bin/python3',['-I','-B','-c','import signal; signal.alarm(5)\n'+script],{env:{PYTHONDONTWRITEBYTECODE:'1'},maxBuffer:8192,timeout:6000});
  assert.equal(result.error,undefined);assert.equal(result.status,status,result.stderr.toString());assert.equal(result.stdout.length,0);
 }
});
test('owned slot failures poison attempts and preserve partial files',async()=>{
 const {testHostClass:HostConnection}=await candidate();
 for(const mode of ['oversize','symlink','hardlink','collision','sync','zero-write','wrong-binding']){
  const root=fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()),'r4-slot-negative-'));
  let connection;const sync=fs.fsyncSync,write=fs.writeSync;
  try{
   const s=fs.statSync(root,{bigint:true}),data=records();
   fs.writeFileSync(path.join(root,'journal.jsonl'),data.journal,{mode:0o600});
   fs.writeFileSync(path.join(root,'ack'),mode==='oversize'?Buffer.alloc(513):data.ack,{mode:0o600});
   if(mode==='symlink'){fs.renameSync(path.join(root,'ack'),path.join(root,'target'));fs.symlinkSync(path.join(root,'target'),path.join(root,'ack'));}
   if(mode==='hardlink')fs.linkSync(path.join(root,'ack'),path.join(root,'alias'));
   if(mode==='collision')fs.writeFileSync(path.join(root,'request_temp'),'owned-collision',{mode:0o600});
   if(mode==='wrong-binding'){const a=JSON.parse(data.ack);a[8]='d435e38748f24705477643dc2c8b11bf3219b26eaf96a0205bfc5ae2f432a4db';fs.writeFileSync(path.join(root,'ack'),wire(a));}
   connection=new HostConnection({directory:root,pin:[s.dev,s.ino,s.uid,s.mode].map(String),nonce,identity});
   if(mode==='sync')fs.fsyncSync=()=>{throw Error('fixture fsync');};
   if(mode==='zero-write')fs.writeSync=()=>0;
   assert.throws(()=>connection.boundary(1));assert.equal(connection.failed,true);
   assert.throws(()=>connection.boundary(2));
   if(['sync','zero-write','collision'].includes(mode)){assert.equal(fs.existsSync(path.join(root,'request')),false);assert.equal(fs.existsSync(path.join(root,'request_temp')),true);}
  }finally{fs.fsyncSync=sync;fs.writeSync=write;connection?.closeFiles();fs.rmSync(root,{recursive:true});}
 }
});
test('retained positive counters and missing slow sample block seventh boundary',async()=>{
 const {admitBoundaryPrefix}=await candidate();
 assert.throws(()=>admitBoundaryPrefix(records(7,(s,m,n)=>{if(n>=2)s[5][2]='1';})),/preverify_nonzero/);
 const d=records();const lines=d.journal.toString().split('\n').filter(Boolean).map(JSON.parse);
 // Rebuild the wire offsets after removing the slow row: not merely a broken offset.
 const sampleBytes=Buffer.concat(lines.filter(row=>!['m','b'].includes(row[0])).map(wire));
 const first=wire(['b',1,'staging-before','1',1,String(sampleBytes.length),'unsynced',[0,d.request.toString('base64'),null]]);
 const offset=sampleBytes.length+first.length;d.ack=wire(['a',1,nonce,1,'staging-before',1,String(offset),String(offset),binding]);
 d.journal=Buffer.concat([sampleBytes,first,wire(['b',1,'staging-before','1',1,String(offset),'synced',[0,null,d.ack.toString('base64')]])]);
 assert.throws(()=>admitBoundaryPrefix(d),/forced_slow/);
});
test('header limits and flag meanings must match the delivered profile',async()=>{
 const {admitBoundaryPrefix}=await candidate();
 for(const change of [h=>h[9].maxPids=65,h=>h[9].pidFlags.rssAbsent=2,h=>h[9].extra=1,h=>h[4].push('extra')])assert.throws(()=>admitBoundaryPrefix(records(1,()=>{},change)));
});
test('transport zero never substitutes for missing, unknown or mismatched owner exit',async()=>{
 const {testHostClass:HostConnection}=await candidate(),{EventEmitter}=await import('node:events');
 for(const mode of ['missing','unknown','mismatch','transport-failure']){
  const dir=fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()),'r4-exit-negative-'));let connection;const clock=process.hrtime.bigint;
  try{
   const s=fs.statSync(dir,{bigint:true});connection=new HostConnection({directory:dir,pin:[s.dev,s.ino,s.uid,s.mode].map(String),nonce,identity});
   const child=new EventEmitter();child.stdin=new EventEmitter();child.stdin.end=()=>child.emit('close',mode==='transport-failure'?1:0,null);
   connection.ownTransport(child,'d'.repeat(64));connection.launch=['ol',1,nonce,'d'.repeat(64),['88','90'],['99','100'],identity[3]];
   if(mode!=='missing')fs.writeFileSync(path.join(dir,'observer-exit.json'),wire(['oe',1,nonce,'d'.repeat(64),'99',mode==='mismatch'?'101':'100',mode==='unknown'?['unknown',null,null]:['exited',0,null],mode==='unknown'?'owner_error':'natural']),{mode:0o600});
   if(mode==='missing'){let calls=0;process.hrtime.bigint=()=>calls++===0?0n:7000000000n;}
   await assert.rejects(connection.close(),/connection_(cleanup_timeout|exit_unknown|exit_identity|transport_failure)/);
   assert.equal(connection.closed,true);
  }finally{process.hrtime.bigint=clock;connection?.closeFiles();fs.rmSync(dir,{recursive:true});}
 }
});

test('definitive receipt and writer errors cannot skip owned transport wait',async()=>{
 const {testHostClass:H}=await candidate();
 for(const writerError of [false,true]){
  const c=Object.create(H.prototype);c.closed=false;c.transportClosed=false;c.transportErrors=[];
  c.child={stdin:{end(){setTimeout(()=>{c.transportClosed=true;c.transportResult={code:0,signal:null};},5);if(writerError)throw Error('writer broken');}}};
  let reads=0;c.ownerRecord=()=>{reads++;throw Error('malformed receipt');};
  await assert.rejects(c.close(),error=>{const text=[error,...(error.errors||[])].map(String).join(' ');assert.match(text,/malformed receipt/);if(writerError)assert.match(text,/writer broken/);return true;});
  assert.equal(c.transportClosed,true,'independent wait must finish');assert.equal(reads,1,'definitive receipt error is not retried');assert.equal(c.closed,true);
 }
});

test('full seventh prefixes match Python rawstat bytes for both snapshots',async()=>{
 const {admitBoundaryPrefix}=await candidate(),{spawnSync}=await import('node:child_process');
 const valid=Buffer.from('123 (x) S 41 '+'0 '.repeat(17)+'222\n');
 const forms=[valid,Buffer.concat([Buffer.from([0xb1]),valid.subarray(1)]),valid.subarray(0,-1),Buffer.from(valid.toString().replace(') S ',') 9 ')),Buffer.from(valid.toString().replace(' S ',' !! ')),Buffer.from(valid.toString().replace('41 ','41\n')),Buffer.from(valid.toString().replace('41 ','41\r')),Buffer.from(valid.toString().replace('123 ','0123 ')),Buffer.from(valid.toString().replace('222','18446744073709551616')),Buffer.concat([Buffer.from('123 ('),Buffer.from([0xff]),Buffer.from(' spaces (x)) S 41 '+'0 '.repeat(17)+'222\n')])];
 const code="import signal,sys,json,base64;signal.alarm(5);sys.path.insert(0,sys.argv[1]);import observe\nr=[]\nfor x in json.load(sys.stdin):\n try: observe.parse_stat(base64.b64decode(x));r.append(True)\n except observe.ObservationError:r.append(False)\nprint(json.dumps(r))";
 const p=spawnSync('/opt/homebrew/bin/python3',['-I','-B','-c',code,new URL('.',url).pathname],{input:JSON.stringify(forms.map(b=>b.toString('base64'))),encoding:'utf8',timeout:6000,maxBuffer:8192});
 assert.equal(p.error,undefined);assert.equal(p.status,0,p.stderr);const expected=JSON.parse(p.stdout);assert.deepEqual(expected,[true,false,false,false,false,false,false,false,false,true]);
 for(let i=0;i<forms.length;i++)for(const side of [0,1]){
  const pair=[valid,valid];pair[side]=forms[i];const data=records(7,()=>{},()=>{},pair);
  if(expected[i])assert.equal(admitBoundaryPrefix(data).sequence,7,'positive '+i);
  else assert.throws(()=>admitBoundaryPrefix(data),undefined,'negative '+i+' side '+side);
 }
});

test('fixed root wrapper and retrieval are connected methods with derived paths',async()=>{
 const {testHostClass:H}=await candidate();
 assert.equal(typeof H.prototype.verifyArguments,'function','missing actual wrapper');
 assert.equal(typeof H.prototype.retainRoot,'function','missing bounded root acquisition');
 const c=Object.create(H.prototype);c.nonce=nonce;c.identity=[fixedCid];
 const args=c.verifyArguments('/tmp/r4-object-read-complete-admitted-positive-owned-20261025');
 assert.deepEqual(args.slice(0,6),['exec','-w','/tmp/r4-object-read-complete-admitted-positive-owned-20261025',fixedCid,'/bin/sh','-c']);
 assert.deepEqual(args.slice(7),['r4-root-v1',nonce,'/tmp/r4-object-read-root-'+nonce,'lexlean','--diagnostic-format','json','verify','tests/lexlean/ObjectAdmittedRangeResult.lex.tex']);
 assert.match(args[6],/exec "\$@"/);assert.match(args[6],/\/proc\/\$\$\/stat/);
 c.identity=['a'.repeat(64)];assert.throws(()=>c.verifyArguments('/tmp/x'));
});
const fixedCid='cbd8cf8ac7240f703c9492d11b4a442474cd303adfb147a82648a2e5cb6bcfdf';

async function rootFixture(mode='normal',stream=0,size=32){
 const {testHostClass:H}=await candidate();const dir=fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()),'r4-root-test-'));
 const st=fs.statSync(dir,{bigint:true});const c=new H({directory:dir,pin:[st.dev,st.ino,st.uid,st.mode].map(String),nonce,identity:[fixedCid,...identity.slice(1)]});
 const old={spawn:childProcess.spawn,clock:process.hrtime.bigint,set:globalThis.setTimeout,clear:globalThis.clearTimeout};
 const timers=new Map(),signals=[],calls=[];let time=0n,id=0,launched=false,settled=false,error,closed=false;
 const child=new EventEmitter();child.pid=12345;child.stdout=new EventEmitter();child.stderr=new EventEmitter();
 child.stdout.destroy=()=>calls.push('destroy-out');child.stderr.destroy=()=>calls.push('destroy-err');
 const end=(code=0,signal=null)=>{child.stdout.emit('end');child.stderr.emit('end');child.emit('exit',code,signal);child.emit('close',code,signal);closed=true;};
 child.kill=kind=>{signals.push([kind,time]);if(mode==='signal-error')throw Error('fixture signal');if(mode==='signal-false')return false;if(mode!=='hang'&&mode!=='signal-error'&&mode!=='clock-stop'&&mode!=='late-zero')queueMicrotask(()=>end(null,kind));return true;};
 try{
  globalThis.setTimeout=(fn,ms)=>{const key=++id;timers.set(key,{at:time+BigInt(Math.round(ms*1e6)),fn});return key;};globalThis.clearTimeout=key=>timers.delete(key);
  process.hrtime.bigint=()=>{if(mode==='clock-start'||((mode==='clock-end'||mode==='clock-stop')&&launched))throw Error('fixture clock');return time;};
  childProcess.spawn=(exe,args,options)=>{
   calls.push([exe,args,options]);assert.equal(exe,'docker');assert.deepEqual(args,['exec',fixedCid,'/bin/sh','-c','test -f "$1" && test ! -L "$1" && exec /usr/bin/head -c 4097 -- "$1"','r4-root-read-v1','/tmp/r4-object-read-root-'+nonce+'/root.marker']);
   assert.deepEqual(options.stdio,['ignore','pipe','pipe']);launched=true;
   if(mode==='spawn-throw')throw Error('fixture spawn');
   queueMicrotask(()=>{
    if(mode==='spawn-event'){child.pid=undefined;child.emit('error',Error('fixture spawn'));return;}
    if(mode==='hang'||mode==='signal-error'||mode==='signal-false'||mode==='late-zero'){
     if(mode==='late-zero')setTimeout(()=>end(),3000);return;
    }
    if(mode==='read-error'){child.stdout.emit('error',Error('fixture read'));return;}
    [child.stdout,child.stderr][stream].emit('data',Buffer.alloc(size,65));
    if(mode==='clock-stop'){child.stdout.emit('error',Error('fixture read'));return;}
    if(mode==='exit-before-close'){child.emit('exit',0,null);setTimeout(()=>end(),3000);return;}
    if(mode==='late-clock')time=3000000000n;
    if(size<=4096)end(mode==='nonzero'?7:0,mode==='unknown-signal'?'WRONG':null);
   });return child;
  };syncBuiltinESMExports();
  if(mode==='root-write')fs.writeFileSync(path.join(dir,'root.marker'),'collision',{mode:0o600});
  if(mode==='receipt-write')fs.writeFileSync(path.join(dir,'root-retrieval.json'),'collision',{mode:0o600});
  const pending=c.retainRoot().catch(e=>{error=e;}).finally(()=>settled=true);
  for(let steps=0;!settled&&steps<50;steps++){
   await Promise.resolve();await Promise.resolve();
   if(!settled&&timers.size){const [key,event]=[...timers].sort((a,b)=>a[1].at<b[1].at?-1:1)[0];timers.delete(key);time=event.at;event.fn();}
  }
  assert.equal(settled,true,'bounded scheduler');await pending;
  const receipt=mode==='receipt-write'?null:JSON.parse(fs.readFileSync(path.join(dir,'root-retrieval.json'),'utf8'));
  assert.equal(c.rootAttempted,true);
  await assert.rejects(c.retainRoot(),/root_once/);
  return {receipt,error,signals,time,calls,launched,closed,child:c.rootChild};
 }finally{childProcess.spawn=old.spawn;process.hrtime.bigint=old.clock;globalThis.setTimeout=old.set;globalThis.clearTimeout=old.clear;syncBuiltinESMExports();c.closeFiles();fs.rmSync(dir,{recursive:true});}
}
test('actual root acquisition bounds both streams and retains independent receipt',async()=>{
 const normal=await rootFixture();assert.equal(normal.error,undefined);assert.deepEqual(normal.receipt.slice(6,9),[['exited',0,null],true,true]);assert.equal(normal.receipt[11],'none');
 for(const stream of [0,1])for(const size of [4096,4097]){
  const r=await rootFixture('normal',stream,size);assert.equal(Buffer.from(r.receipt[9+stream],'base64').length,Math.min(size,4096));
  assert.equal(r.receipt[7+stream],size===4096);assert.equal(Boolean(r.error),size===4097);
 }
 const failed=await rootFixture('root-write');assert.match(String(failed.error),/root_acquisition/);assert.equal(failed.receipt[11],'none');
 const receipt=await rootFixture('receipt-write');assert.ok(receipt.error);
});
test('root lifetime clocks errors and exits retain finite original-handle authority',async()=>{
 for(const mode of ['spawn-throw','spawn-event','nonzero','read-error','unknown-signal','clock-start','clock-end','clock-stop','hang','signal-error','late-zero','exit-before-close']){
  const r=await rootFixture(mode);assert.ok(r.error,mode);
  if(mode==='clock-start')assert.equal(r.launched,false);
  if(mode==='clock-stop'){assert.equal(r.time,4000000000n,'keep existing final timer after clock failure');assert.equal(r.signals.length,0);}
  if(['spawn-throw','spawn-event','clock-start','exit-before-close'].includes(mode))assert.equal(r.signals.length,0,mode);
  assert.ok(r.time<=4000000000n,mode+' deadline');
  assert.equal(new Set(r.signals.map(x=>x[0])).size,r.signals.length,'each signal once');
  if(mode==='hang'){assert.deepEqual(r.signals.map(x=>x[0]),['SIGTERM','SIGKILL']);assert.deepEqual(r.signals.map(x=>x[1]),[2000000000n,2250000000n]);assert.equal(r.receipt[6][0],'unknown');assert.ok(r.child);}
  if(mode==='late-zero')assert.notEqual(r.receipt[11],'none');
 }
});

test('finite isolated wrapper preserves marker and exec argv with keyed zero parity',async()=>{
 const {testHostClass:H}=await candidate(),c=Object.create(H.prototype);c.nonce=nonce;c.identity=[fixedCid];
 const literal=c.verifyArguments('/tmp/r4-object-read-complete-admitted-positive-owned-20261025')[6];
 const normal='low 0\nhigh 0\nmax 0\noom 0\noom_kill 0\noom_group_kill 0\n';
 const cases=[[normal,true],[normal+'future! 0\n',true],[normal.replace('low 0','\tlow\v0\f'),true],[normal+'max 0\n',false],[normal+'future 1\n',false],[normal.replace('low 0\n',''),false],[normal.replace('max 0','max 00'),false],[normal.slice(0,-1),false],[normal+'bad key 0\n',false],[normal+'[x]*? 0\n',true],[normal,false,'collision'],[normal,false,'exec'],[normal,false,'stat-cr'],[normal,false,'stat-extra']];
 const parityCode="import signal,sys,json;signal.alarm(5);sys.path.insert(0,sys.argv[1]);import observe\nr=[]\nfor x in json.load(sys.stdin):\n try:r.append(all(v=='0' for v in observe.parse_events(x.encode()).values()))\n except observe.ObservationError:r.append(False)\nprint(json.dumps(r))";
 const parity=childProcess.spawnSync('/opt/homebrew/bin/python3',['-I','-B','-c',parityCode,new URL('.',url).pathname],{input:JSON.stringify(cases.filter(x=>!x[2]).map(x=>x[0])),encoding:'utf8',timeout:6000,killSignal:'SIGKILL',maxBuffer:8192});
 assert.equal(parity.error,undefined);assert.equal(parity.status,0,parity.stderr);assert.deepEqual(JSON.parse(parity.stdout),cases.filter(x=>!x[2]).map(x=>x[1]));
 const quote=s=>"'"+s.replaceAll("'","'\\''")+"'";
 for(const [events,okay,fault] of cases){
  const root=fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()),'r4-shell-finite-')),n=crypto.randomBytes(32).toString('hex'),markerDir='/tmp/r4-object-read-root-'+n;
  try{
   fs.writeFileSync(path.join(root,'events'),events);fs.writeFileSync(path.join(root,'lexlean'),'#!/bin/sh\nprintf \'%s\\n\' "$@"\n',{mode:0o700});
   if(fault==='collision')fs.mkdirSync(markerDir,{mode:0o700});
   if(fault==='exec')fs.chmodSync(path.join(root,'lexlean'),0o600);
   const statFile=quote(path.join(root,'stat'));
   const prelude="printf '%s (fixture) S 41 "+'0 '.repeat(17)+"222\\n' \"$$\" > "+statFile+'\n';
   const extra=fault==='stat-cr'?"printf '\\r' >> "+statFile+'\n':fault==='stat-extra'?"printf 'extra\\n' >> "+statFile+'\n':'';
   const script=prelude+extra+literal.replace('/proc/$$/stat',statFile).replace('/sys/fs/cgroup/memory.events',quote(path.join(root,'events')));
   const code='import signal,os,sys;signal.alarm(5);os.execve("/bin/sh",["sh","-c",sys.argv[1],*sys.argv[2:]],os.environ)';
   const args=['-I','-B','-c',code,script,'r4-root-v1',n,markerDir,'lexlean','--diagnostic-format','json','verify','tests/lexlean/ObjectAdmittedRangeResult.lex.tex'];
   const r=childProcess.spawnSync('/opt/homebrew/bin/python3',args,{env:{...process.env,PATH:root},encoding:'utf8',timeout:6000,killSignal:'SIGKILL',maxBuffer:8192});
   assert.equal(r.error,undefined);assert.equal(r.status,okay?0:fault==='exec'?126:74,events+' fault='+fault+' stderr='+r.stderr);
   assert.equal(r.stdout,okay?'--diagnostic-format\njson\nverify\ntests/lexlean/ObjectAdmittedRangeResult.lex.tex\n':'');
   if(['collision','stat-cr','stat-extra'].includes(fault)){assert.equal(fs.existsSync(path.join(markerDir,'root.marker')),false);continue;}
   const marker=fs.readFileSync(path.join(markerDir,'root.marker'),'utf8');assert.equal(marker.split('\n')[2],String(r.pid));assert.equal(marker.split('\n').length,5);assert.equal(fs.statSync(path.join(markerDir,'root.marker')).mode&511,384);
  }finally{fs.rmSync(root,{recursive:true});if(fs.existsSync(markerDir))fs.rmSync(markerDir,{recursive:true});}
 }
});

test('late completion cannot win timer race and failed signal is not retried',async()=>{
 const late=await rootFixture('late-clock');assert.ok(late.error,'elapsed2s cannot become success before timer callback');assert.notEqual(late.receipt[11],'none');
 const lost=await rootFixture('signal-false');assert.ok(lost.error);assert.deepEqual(lost.signals.map(x=>x[0]),['SIGTERM']);
});

test('configuration rejects a different compiler container before any launch',async()=>{
 const {testConfig:decodeConfiguration}=await candidate();
 assert.throws(()=>decodeConfiguration(wire(configuration())),/root_container/);
 const good=configuration();good.identity=structuredClone(good.identity);good.identity[0]=fixedCid;good.config.context.docker[0]=fixedCid;
 assert.equal(decodeConfiguration(wire(good)).identity[0],fixedCid);
});

test('retrieval receipts round-trip the unchanged Python codec',async()=>{
 const rows=[];
 for(const [mode,stream,size] of [['normal',0,32],['normal',0,4097],['normal',1,4097],['spawn-throw',0,0],['nonzero',0,32],['hang',0,0],['late-zero',0,0],['clock-start',0,0],['unknown-signal',0,0]])rows.push((await rootFixture(mode,stream,size)).receipt);
 const code="import signal,sys,json,base64;signal.alarm(5);sys.path.insert(0,sys.argv[1]);import observe\nfor r in json.load(sys.stdin):\n b=observe.encode_retrieval(nonce=r[2],binding=r[3],start=r[4],end=r[5],process=r[6],stdout=base64.b64decode(r[9]),stderr=base64.b64decode(r[10]),stdout_complete=r[7],stderr_complete=r[8],reason=r[11]);assert json.loads(b)==r\nprint('PASS')";
 const r=childProcess.spawnSync('/opt/homebrew/bin/python3',['-I','-B','-c',code,new URL('.',url).pathname],{input:JSON.stringify(rows),encoding:'utf8',timeout:6000,killSignal:'SIGKILL',maxBuffer:8192});
 assert.equal(r.error,undefined);assert.equal(r.status,0,r.stderr);assert.equal(r.stdout,'PASS\n');
});
test('actual owner receipt errors wait independently and aggregate transport deadline',async()=>{
 const {testHostClass:H}=await candidate();
 for(const mode of ['malformed','binding','identity','read']){
  const c=Object.create(H.prototype);c.closed=false;c.nonce=nonce;c.contextDigest='d'.repeat(64);c.transportErrors=[];c.transportClosed=false;c.launch=['ol',1,nonce,c.contextDigest,['88','90'],['99','100'],identity[3]];
  const receipt=['oe',1,nonce,mode==='binding'?'e'.repeat(64):c.contextDigest,'99',mode==='identity'?'101':'100',['exited',0,null],'natural'];let reads=0;
  c.read=()=>{reads++;if(mode==='read')throw Error('fixture read');return {raw:mode==='malformed'?Buffer.from('bad\n'):wire(receipt)};};
  c.child={stdin:{end(){setTimeout(()=>{c.transportClosed=true;c.transportResult={code:0,signal:null};},5);}}};
  await assert.rejects(c.close());assert.equal(c.transportClosed,true);assert.equal(reads,1);
 }
 const c=Object.create(H.prototype);c.closed=false;c.transportErrors=[];c.child={stdin:{end(){throw Error('writer');}}};c.ownerRecord=()=>{throw Error('receipt');};
 const oldClock=process.hrtime.bigint,oldTimer=globalThis.setTimeout;let time=0n,waits=0;
 try{
  process.hrtime.bigint=()=>time;globalThis.setTimeout=fn=>{time+=1000000000n;waits++;queueMicrotask(fn);};
  await assert.rejects(c.close(),e=>{const text=e.errors.map(String).join(' ');for(const name of ['writer','receipt','cleanup_timeout'])assert.match(text,new RegExp(name));return true;});
  assert.equal(waits,7);assert.equal(c.closed,true);
 }finally{process.hrtime.bigint=oldClock;globalThis.setTimeout=oldTimer;}
});
test('fixed read literal rejects missing and symlink marker with bounded host fixture',async()=>{
 const script='test -f "$1" && test ! -L "$1" && exec /usr/bin/head -c 4097 -- "$1"';
 const root=fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()),'r4-read-finite-'));
 try{
  const regular=path.join(root,'root.marker'),link=path.join(root,'alias');fs.writeFileSync(regular,Buffer.alloc(4097,65));fs.symlinkSync(regular,link);
  for(const [file,status,length] of [[regular,0,4097],[link,1,0],[path.join(root,'missing'),1,0]]){
   const code='import signal,os,sys;signal.alarm(5);os.execve("/bin/sh",["sh","-c",sys.argv[1],"r4-root-read-v1",sys.argv[2]],os.environ)';
   const r=childProcess.spawnSync('/opt/homebrew/bin/python3',['-I','-B','-c',code,script,file],{timeout:6000,killSignal:'SIGKILL',maxBuffer:8192});
   assert.equal(r.error,undefined);assert.equal(r.status,status);assert.equal(r.stdout.length,length);
  }
 }finally{fs.rmSync(root,{recursive:true});}
});

function textGrammarVectors(){
 const events='low 0\nhigh 0\nmax 0\noom 0\noom_kill 0\noom_group_kill 0\n';
 const pressure='some avg10=0.00 avg60=0.00 avg300=0.00 total=0\n';
 const bases={stat:'anon 100\n',events,pressure},vectors=[];
 const add=(leaf,text,valid,label,semantic='valid')=>vectors.push({leaf,text,valid,label,semantic});
 // Every non-ASCII ECMAScript whitespace/line-terminator, plus lone-surrogate input.
 const unicode=[0xa0,0x1680,...Array.from({length:11},(_,i)=>0x2000+i),0x2028,0x2029,0x202f,0x205f,0x3000,0xfeff,0xd800].map(code=>String.fromCodePoint(code));
 for(const [leaf,base] of Object.entries(bases)){
  add(leaf,base,true,'baseline');
  for(const char of unicode){
   for(const [position,text] of [['between',base.replace(' ',char)],['leading',char+base],['trailing',base.replace('\n',char+'\n')],['blank',base+char+'\n'],['key',base[0]+char+base.slice(1)]])add(leaf,text,false,'unicode-'+char.charCodeAt(0).toString(16)+'-'+position);
  }
  for(const ws of ['\t','\v','\f','\r',' \t\v\f\r '])add(leaf,ws+base.replaceAll(' ',ws).replaceAll('\n',ws+'\n'),true,'ASCII '+JSON.stringify(ws));
  add(leaf,base.replaceAll('\n','\r\n'),true,'CRLF');
  for(const [label,text] of [['noLF',base.slice(0,-1)],['empty',''],['blank','\n'+base],['extraLF',base+'\n'],['spaceLine',base+' \t\v\f\r\n'],['NUL',base.replace(' ','\0')]])add(leaf,text,false,label);
  const cap=leaf==='stat'?16384:4096;
  const exact=base.slice(0,-1)+' '.repeat(cap-base.length)+'\n';add(leaf,exact,true,'exactByteCap');add(leaf,exact.slice(0,-1)+' \n',false,'overByteCap');
 }
 for(const leaf of ['stat','events']){
  const base=bases[leaf];add(leaf,base+'future!? 0\n',true,'unknownKey');add(leaf,base+base.split('\n')[0]+'\n',false,'duplicate');
  for(const value of ['00','18446744073709551616','-1','+1','1e0','1.0'])add(leaf,base.replace(/ \d+\n/,' '+value+'\n'),false,'badU64 '+value);
  add(leaf,base.replace(/ \d+\n/,' 18446744073709551615\n'),true,'maxU64',leaf==='events'?'nonzero':'valid');
 }
 add('events',events.replace('low 0\n',''),false,'missingRequired');
 const event64=events+Array.from({length:58},(_,i)=>'future'+i+' 0\n').join('');add('events',event64,true,'64keys');add('events',event64+'overflow 0\n',false,'65keys');
 const stat1024=Array.from({length:1024},(_,i)=>'k'+i+' 0\n').join('');add('stat',stat1024,true,'1024keys');add('stat',stat1024+'extra 0\n',false,'1025keys');
 add('pressure',pressure.replace('some','full'),true,'fullOnly');add('pressure',pressure+pressure.replace('some','full'),true,'bothCategories');add('pressure',pressure+pressure,false,'duplicateCategory');
 add('pressure',pressure.replace('0.00','00000000000000000000.99999999999999999999'),true,'20digitAverageNotU64');
 add('pressure',pressure.replace('total=0','total=18446744073709551615'),true,'maxTotal');
 for(const [label,text] of [['avgOverflowDigits',pressure.replace('0.00','0'.repeat(21)+'.0')],['totalOverflow',pressure.replace('total=0','total=18446744073709551616')],['totalLeadingZero',pressure.replace('total=0','total=00')],['extraEquals',pressure.replace('avg10=','avg10==')],['duplicateField',pressure.replace('avg60=','avg10=')],['unknownField',pressure.replace('avg60=','future=')],['unknownCategory',pressure.replace('some','other')],['extraField',pressure.replace('\n',' extra=0\n')]])add('pressure',text,false,label);
 for(const avg of ['1','1.',' .1','1.2.3','+1.0','-1.0','1e0.0'])add('pressure',pressure.replace('0.00',avg),false,'badAverage '+avg);
 return {bases,vectors};
}
test('three retained text leaves match Python bytes through complete seventh prefixes',async()=>{
 const {admitBoundaryPrefix}=await candidate(),{bases,vectors}=textGrammarVectors();
 const code="import signal,sys,json,base64;signal.alarm(5);sys.path.insert(0,sys.argv[1]);import observe,boundary\nr=[]\nfor leaf,raw in json.load(sys.stdin):\n try:{'stat':boundary._slow_stat,'events':observe.parse_events,'pressure':boundary._pressure}[leaf](base64.b64decode(raw));r.append(True)\n except (observe.ObservationError,UnicodeError):r.append(False)\nprint(json.dumps(r))";
 const r=childProcess.spawnSync('/opt/homebrew/bin/python3',['-I','-B','-c',code,new URL('.',url).pathname],{input:JSON.stringify(vectors.map(v=>[v.leaf,Buffer.from(v.text).toString('base64')])),encoding:'utf8',timeout:6000,killSignal:'SIGKILL',maxBuffer:16384});
 assert.equal(r.error,undefined);assert.equal(r.status,0,r.stderr);const oracle=JSON.parse(r.stdout);assert.equal(oracle.length,vectors.length);
 const apply=(m,leaf,text)=>{if(leaf==='stat')m[4]=text;else if(leaf==='events'){m[6]=text;m[7]&=~2;}else{m[5]=text;m[7]&=~1;}};
 const mismatches=[];
 for(let i=0;i<vectors.length;i++){
  const v=vectors[i];assert.equal(oracle[i],v.valid,v.leaf+' '+v.label+' independent oracle');
  for(const earlier of [false,true]){
   const data=records(7,(s,m,n)=>apply(m,v.leaf,earlier&&n!==2?bases[v.leaf]:v.text));
   let error,admitted;try{admitted=admitBoundaryPrefix(data);}catch(e){error=e;}
   // Positive local key additions must persist: do not confuse disappearance with lexical parity.
   if(earlier&&v.valid)continue;
   const expected=v.valid&&v.semantic==='valid';
   if(Boolean(admitted)!==expected)mismatches.push(v.leaf+':'+v.label+':earlier='+earlier+':'+String(error||'ADMITTED'));
   if(v.valid&&v.semantic==='nonzero'&&!earlier)assert.match(String(error),/preverify_nonzero/);
  }
 }
 assert.deepEqual(mismatches,[],vectors.length+' vectors, full canonical envelopes; earlier malformed history remains rejected');
});

test('deterministic ACK rename interleavings poison before semantic admission',async()=>{
 const {testHostClass:H,admitBoundaryPrefix}=await candidate(),evidence=[];
 for(const variant of ['expected-next','foreign-nonce','future-sequence','tampered-open-bytes','unaccepted-open-pin']){
  const prior=records(1),next=records(2),nextRow=JSON.parse(next.ack);
  if(variant==='foreign-nonce')nextRow[2]='e'.repeat(64);
  if(variant==='future-sequence'){nextRow[3]=3;nextRow[4]='lock-before';}
  const nextBytes=wire(nextRow),acceptedPrior=prior.ack;
  const oldRow=JSON.parse(prior.ack);if(variant==='tampered-open-bytes')oldRow[2]='f'.repeat(64);
  const openedBytes=wire(oldRow),oldInode=variant==='unaccepted-open-pin'?12n:10n;
  const info=(ino,size)=>({dev:1n,ino,uid:BigInt(process.getuid()),mode:33152n,nlink:1n,size:BigInt(size),isFile:()=>true});
  const acceptedPin=info(10n,acceptedPrior.length),opened=info(oldInode,openedBytes.length),replacement=info(11n,nextBytes.length);
  let current=opened,fstats=0,publications=0,reads=0,closed=0;const clocks=[];
  const c=Object.create(H.prototype);Object.assign(c,{directory:'/owned-mock',nonce,identity,sequence:2,failed:false,lastAck:acceptedPrior,previous:admitBoundaryPrefix(prior),closed:false});
  c.guard=()=>{};c.publish=raw=>{publications++;assert.equal(JSON.parse(raw)[3],2);};
  const originals={open:fs.openSync,fstat:fs.fstatSync,read:fs.readSync,lstat:fs.lstatSync,close:fs.closeSync,clock:process.hrtime.bigint};
  try{
   fs.openSync=(file,flags)=>{assert.equal(file,'/owned-mock/ack');assert.ok(flags&fs.constants.O_NOFOLLOW);return 99;};
   fs.fstatSync=fd=>{assert.equal(fd,99);fstats++;if(fstats===2)current=replacement;return opened;};
   fs.readSync=(fd,buffer,offset,length,position)=>{assert.equal(fd,99);reads++;openedBytes.copy(buffer,offset,position,position+length);return length;};
   fs.lstatSync=file=>{assert.equal(file,'/owned-mock/ack');assert.equal(fstats,2);return current;};
   fs.closeSync=fd=>{assert.equal(fd,99);closed++;};
   process.hrtime.bigint=()=>{const value=BigInt(clocks.length)*100n;clocks.push(String(value));return value;};
   assert.throws(()=>c.boundary(2),error=>error instanceof AggregateError&&error.errors.some(e=>/connection_file_replaced/.test(String(e))),variant);
   assert.equal(c.failed,true);assert.equal(c.sequence,2);assert.equal(c.previous.sequence,1);assert.equal(publications,1);assert.equal(closed,1);
   assert.throws(()=>c.boundary(3),/connection_sequence/);assert.equal(publications,1);assert.deepEqual(clocks,['0','100']);
   const pin=x=>[x.dev,x.ino,x.uid,x.mode,x.nlink,x.size].map(String);
   evidence.push({variant,acceptedPriorPin:pin(acceptedPin),openedBeforeAndAfterPin:pin(opened),currentPathPin:pin(replacement),openedEqualsAccepted:openedBytes.equals(acceptedPrior),replacementNonce:nextRow[2],replacementSequence:nextRow[3],oldHash:sha(openedBytes),replacementHash:sha(nextBytes),fstats,reads,closed,publications,clockReads:clocks,unchangedAbsoluteDeadline:'2000000000',outcome:'connection_file_replaced; poisoned; no next publication'});
  }finally{fs.openSync=originals.open;fs.fstatSync=originals.fstat;fs.readSync=originals.read;fs.lstatSync=originals.lstat;fs.closeSync=originals.close;process.hrtime.bigint=originals.clock;}
 }
 console.log('ACK_INTERLEAVING '+JSON.stringify(evidence));
});

async function admittedAckInterleaving({when='after-second',fault='',partial=false}={}){
 const {testHostClass:H,admitBoundaryPrefix}=await candidate(),prior=records(1),next=records(2);
 const uid=BigInt(process.getuid()),pin=x=>[x.dev,x.ino,x.uid,x.mode,x.nlink,BigInt(x.raw.length)].map(String);
 const make=(ino,raw)=>({dev:1n,ino,uid,mode:33152n,nlink:1n,raw,type:true});
 const old=make(10n,prior.ack),replacement=make(11n,next.ack),acceptedPin=pin(old);
 const changed=JSON.parse(replacement.raw);
 if(fault==='foreign')changed[2]='e'.repeat(64);
 if(fault==='future'){changed[3]=3;changed[4]='lock-before';}
 if(fault==='replay'){changed.splice(0,changed.length,...JSON.parse(prior.ack));}
 if(fault==='binding')changed[8]='f'.repeat(64);
 replacement.raw=wire(changed);
 if(fault==='old-bytes'){const x=JSON.parse(old.raw);x[2]='d'.repeat(64);old.raw=wire(x);}
 if(fault==='unknown-old')old.ino=12n;
 if(fault==='old-dev')old.dev=2n;
 if(fault==='old-uid')old.uid=uid+1n;
 if(fault==='old-mode')old.mode=33088n;
 if(fault==='old-type')old.type=false;
 if(fault==='old-size')old.raw=Buffer.concat([old.raw,Buffer.from(' ')]);
 if(fault==='old-link2')old.nlink=2n;
 if(fault==='candidate-mode')replacement.mode=33088n;
 if(fault==='candidate-link2')replacement.nlink=2n;
 if(fault==='candidate-dev')replacement.dev=2n;
 if(fault==='same-inode')replacement.ino=10n;
 let current=old,rotated=false,firstStats=0,opens=0,reads=0,publications=0,journals=0,clockCalls=0,pathStats=0;
 const fds=new Map(),closed=[],trace=[],clockValues=[];
 const rotate=()=>{if(rotated)return;rotated=true;old.nlink=0n;current=replacement;trace.push(['rename',pin(old),pin(replacement)]);};
 const c=Object.create(H.prototype);Object.assign(c,{directory:'/owned-ack',nonce,identity,sequence:2,failed:false,lastAck:prior.ack,lastAckPin:fault==='no-prior'?undefined:acceptedPin,previous:admitBoundaryPrefix(prior),closed:false});
 c.guard=()=>{};c.publish=raw=>{publications++;assert.equal(JSON.parse(raw)[3],2);};
 c.read=(name,...args)=>{
  if(name!=='journal.jsonl')return H.prototype.read.call(c,name,...args);
  journals++;let raw=next.journal;
  if(fault==='bad-prefix'){raw=Buffer.from(raw);raw[20]^=1;}
  if(partial&&journals===1){raw=raw.subarray(0,Number(JSON.parse(next.ack)[6]));if(fault==='between-poll-rotation')current=make(13n,replacement.raw);}
  return {raw,pin:[]};
 };
 const originals={open:fs.openSync,fstat:fs.fstatSync,read:fs.readSync,lstat:fs.lstatSync,close:fs.closeSync,clock:process.hrtime.bigint,wait:Atomics.wait};let result,error;
 const stat=file=>({dev:file.dev,ino:file.ino,uid:file.uid,mode:file.mode,nlink:file.nlink,size:BigInt(file.raw.length),isFile:()=>file.type});
 try{
  fs.openSync=(file,flags)=>{assert.equal(file,'/owned-ack/ack');assert.ok(flags&fs.constants.O_NOFOLLOW);opens++;assert.ok(opens<=6,'fixture open bound');if(opens===2&&fault==='missing-candidate')throw Object.assign(Error('gone'),{code:'ENOENT'});if(opens===2&&fault==='second-rotation')current=make(13n,replacement.raw);const fd=100+opens;fds.set(fd,current);trace.push(['open',fd,pin(current)]);if(opens===1&&when==='before-first')rotate();return fd;};
  fs.fstatSync=fd=>{const file=fds.get(fd);assert.ok(file);if(fd===101){firstStats++;if(firstStats===2&&when==='between')rotate();if(firstStats===2&&fault==='relink')old.nlink=1n;if(firstStats===2&&fault==='after-link2')old.nlink=2n;const value=stat(file);if(firstStats===2&&when==='after-second')rotate();return value;}return stat(file);};
  fs.readSync=(fd,buffer,offset,length,position)=>{reads++;if(fd!==101&&fault==='short-read')return 0;return fds.get(fd).raw.copy(buffer,offset,position,position+length);};
  fs.lstatSync=file=>{assert.equal(file,'/owned-ack/ack');pathStats++;if(fault==='candidate-path-size'&&pathStats===2)current.raw=Buffer.concat([current.raw,Buffer.from(' ')]);return stat(current);};
  fs.closeSync=fd=>{assert.ok(fds.has(fd));assert.equal(closed.includes(fd),false);closed.push(fd);trace.push(['close',fd]);if((fd===101&&fault==='old-close')||(fd===102&&fault==='candidate-close'))throw Error('fixture close '+fd);};
  process.hrtime.bigint=()=>{const n=clockCalls++;assert.ok(n<40,'fixture clock bound');let value=BigInt(n)*100n;if((fault==='entry-timeout'&&n>=2)||(fault==='read-timeout'&&n>=3)||(fault==='admission-timeout'&&n>=4))value=2000000001n;if(fault==='clock-regression'&&n>=3)value=0n;if(fault==='inclusive-edge'&&n>=2)value=2000000000n;clockValues.push(String(value));return value;};Atomics.wait=()=> 'timed-out';
  try{result=c.boundary(2);}catch(e){error=e;}
  if(error){assert.equal(c.failed,true);const before=publications;assert.throws(()=>c.boundary(3),/connection_sequence/);assert.equal(publications,before);assert.equal(c.sequence,2);assert.equal(c.lastAck,prior.ack);assert.deepEqual(c.lastAckPin,fault==='no-prior'?undefined:acceptedPin);}
  return {result,error,opens,reads,closed,publications,journals,trace,clockValues,acceptedPin,currentPin:pin(current),committedPin:c.lastAckPin};
 }finally{fs.openSync=originals.open;fs.fstatSync=originals.fstat;fs.readSync=originals.read;fs.lstatSync=originals.lstat;fs.closeSync=originals.close;process.hrtime.bigint=originals.clock;Atomics.wait=originals.wait;}
}
test('accepted prior ACK rotation admits the expected next complete prefix',async()=>{
 const r=await admittedAckInterleaving();assert.equal(r.error,undefined,'expected-next must be admitted');assert.equal(r.result.sequence,2);assert.equal(r.publications,1);assert.equal(r.opens,2);assert.deepEqual(r.closed,[101,102]);assert.deepEqual(r.committedPin,r.currentPin);
});

function ackErrors(error){return [String(error),...(error instanceof AggregateError?error.errors.flatMap(ackErrors):[])].join(' ');}
test('transition candidate replay must reject immediately rather than poll',async()=>{
 const r=await admittedAckInterleaving({fault:'replay'});assert.match(ackErrors(r.error),/connection_ack_identity/);assert.equal(r.opens,2);assert.equal(r.publications,1);
});

test('ACK transition covers all rename positions and bounded partial-prefix wait',async()=>{
 const proof=[];
 for(const when of ['before-first','between','after-second']){
  const r=await admittedAckInterleaving({when});assert.equal(r.error,undefined,when);assert.equal(r.result.sequence,2);assert.equal(r.opens,2);assert.deepEqual(r.closed,[101,102]);assert.equal(r.publications,1);assert.deepEqual(r.committedPin,r.currentPin);
  assert.ok(r.trace.findIndex(x=>x[0]==='close'&&x[1]===101)<r.trace.findIndex(x=>x[0]==='open'&&x[1]===102));
  proof.push({when,accepted:r.acceptedPin,committed:r.committedPin,trace:r.trace,clocks:r.clockValues,opens:r.opens,reads:r.reads,closed:r.closed,publications:r.publications});
 }
 const p=await admittedAckInterleaving({partial:true});assert.equal(p.error,undefined);assert.equal(p.result.sequence,2);assert.equal(p.opens,3);assert.equal(p.publications,1);assert.deepEqual(p.closed,[101,102,103]);assert.equal(p.journals,3);assert.deepEqual(p.committedPin,p.currentPin);
 const edge=await admittedAckInterleaving({fault:'inclusive-edge'});assert.equal(edge.error,undefined);assert.equal(edge.result.receivedNs,'2000000000');
 console.log('ACK_ACCEPTED_TRANSITIONS '+JSON.stringify(proof));
});
test('ACK transition preserves strict identity custody deadline and one allowance',async()=>{
 const faults=['no-prior','unknown-old','old-bytes','old-dev','old-uid','old-mode','old-type','old-size','old-link2','after-link2','candidate-mode','candidate-link2','candidate-dev','same-inode','foreign','future','binding','bad-prefix','missing-candidate','second-rotation','short-read','old-close','candidate-close','entry-timeout','read-timeout','admission-timeout','clock-regression'];
 for(const fault of faults){
  const r=await admittedAckInterleaving({fault});assert.ok(r.error,fault+' must reject');assert.doesNotMatch(ackErrors(r.error),/fixture (open|clock) bound/,fault);assert.equal(r.publications,1);assert.equal(r.closed.length,r.trace.filter(x=>x[0]==='open').length,fault+' closes each opened FD');
  if(fault==='old-close')assert.equal(r.opens,1,'old close must precede candidate open');
  if(fault.endsWith('timeout'))assert.match(ackErrors(r.error),/connection_deadline/);
 }
 const relink=await admittedAckInterleaving({when:'before-first',fault:'relink'});assert.match(ackErrors(relink.error),/file_changed/);
 const second=await admittedAckInterleaving({partial:true,fault:'between-poll-rotation'});assert.match(ackErrors(second.error),/ack_generation/);assert.equal(second.journals,1);assert.equal(second.publications,1);assert.equal(second.opens,3);
});

test('strict replacement includes final pathname size in full candidate pin',async()=>{
 const r=await admittedAckInterleaving({fault:'candidate-path-size'});assert.ok(r.error,'size change after replacement second fstat must reject');assert.match(ackErrors(r.error),/ack_generation/);
});

// Reviewed test-local diagnostic: these controls never launch the finite fixture.
const DEADLINE_ENUMS=Object.freeze({stage:{unknown:0,boundary:1,publish:2,loop:3,ack:4,transition:5,journal:6,prefix:7,confirm:8,final:9,sleepCheck:10,sleep:11,returned:12},outcome:{unknown:0,ok:1,throw:2},fact:{unknown:0,no:1,yes:2},ack:{unknown:0,missing:1,old:2,candidate:3},read:{unknown:0,ok:1,missing:2,partial:3,oversize:4,changed:5,invalid:6,error:7},exit:{unknown:0,exited:1,signaled:2,ended:3},reason:{unknown:0,natural:1,host_eof:2,ended:3,quota:4},signal:{unknown:0,none:1,SIGTERM:2,SIGKILL:3,SIGALRM:4},error:{none:0,deadline:1,io:2,assertion:3,other:4,multiple:5}});
test('deadline observation construction has exact finite scalar and packet bounds',()=>{
 assert.equal(typeof deadlineObserver,'function','reviewed observer missing');
 const o=deadlineObserver();assert.deepEqual(o.enums,DEADLINE_ENUMS);
 const u='18446744073709551615',d='f'.repeat(64),pin=Array(6).fill(u),t=[99,u,d,99,u,99,99],snap=[u,99,99,99,t,t];
 const maximum=['do1',[999999,99,999999,u,u,u,u,99,99,Array(8).fill(999999)],[99,99,pin,pin,u,d,99,999999,99,u],[u,u,u,u,u,d,d,99],[999999,Array.from({length:8},()=>[99,999999,u,99])],[snap,snap],Array.from({length:4},()=>[99,99])];
 assert.equal(Buffer.byteLength(JSON.stringify(maximum)+'\n'),1778);
 assert.equal(o.encode(maximum),JSON.stringify(maximum)+'\n');
 for(const v of [-1n,18446744073709551616n,1.2,'123',{},null])assert.equal(o.u(v),v===null?null:'u');
 assert.equal(o.u(18446744073709551615n),u);assert.equal(o.increment(999999),'x');assert.equal(o.increment('x'),'x');
 for(const bad of [()=>{const x=structuredClone(maximum);x[1][1]=100;return x;},()=>{const x=structuredClone(maximum);x[3][5]='z'.repeat(64);return x;},()=>{const x=structuredClone(maximum);x[4][1].push([0,0,null,0]);return x;},()=>{const x=structuredClone(maximum);x.push('extra');return x;}])assert.throws(()=>o.encode(bad()));
 for(let n=0;n<30;n++)o.event(3,n,BigInt(n),1);assert.equal(o.packet[4][1].length,8);assert.equal(o.packet[4][0],22);assert.ok(Buffer.byteLength(o.encode())<=1778);
});
test('deadline observation actual-body stages preserve clocks deadlines delegation and IO',async()=>{
 assert.equal(typeof deadlineScenario,'function','actual-body observation control missing');
 for(const [mode,stage,backwards] of [['publish',3,false],['missing',3,false],['old',3,false],['partial',3,false],['semantic',9,false],['transition',5,false],['backwards',3,true]]){
  const x=await deadlineScenario(mode);assert.match(String(x.error),/connection_deadline/,mode);if(mode==='backwards')assert.equal(x.calls.filter(v=>v==='sleep').length,1);assert.equal(x.packet[1][1],stage,mode);assert.equal(x.packet[1][7],backwards?2:1,mode);assert.equal(x.packet[1][8],backwards?1:2,mode);assert.equal(x.packet[1][0],2);assert.equal(x.restored,true);assert.equal(x.publications,1);assert.deepEqual(x.calls,x.unobservedCalls,mode+' no extra production IO/delegation');assert.ok(x.clockIdentity);assert.ok(Buffer.byteLength(x.encoded)<=1778);
 }
});
test('deadline observation preserves exception identity and independent restoration',async()=>{
 assert.equal(typeof deadlineScenario,'function');const error=Error('delegate original');const x=await deadlineScenario('throw',error);assert.equal(x.error,error);assert.equal(x.restored,true);
 const o=deadlineObserver(),original=Error('primary'),cleanup=Error('cleanup'),snapshot=Error('snapshot');o.errors[0].push(original);o.errors[2].push(cleanup);o.errors[1].push(snapshot);const result=o.failure();assert.ok(result instanceof AggregateError);assert.deepEqual(result.errors,[original,snapshot,cleanup]);assert.equal(o.errors[0][0],original);assert.ok(!o.encode().includes('primary'));
});
test('deadline observation terminal projections are fixed and snapshot reads are bounded',()=>{
 assert.equal(typeof deadlineObserver,'function');const o=deadlineObserver();
 const huge={toString(){throw Error('must not stringify');},nested:Array(1000).fill({secret:'opaque'})};assert.deepEqual(o.projectTerminal(huge,'owner',null).slice(3),[0,null,0,0]);
 assert.equal(typeof deadlineSnapshotControl,'function');
 for(const mode of ['missing','partial','oversize','changed','symlink','close']){const x=deadlineSnapshotControl(mode);assert.ok(x.status!==1,mode);assert.ok(x.reads<=1);assert.equal(x.closes,x.opens);assert.ok(x.bytes<=4097);}
 const x=deadlineSnapshotControl('valid');assert.equal(x.status,1);assert.equal(x.closes,1);assert.equal(x.reads,1);
});

function deadlineObserver(){
 const enums=DEADLINE_ENUMS,E=enums.stage,Umax=18446744073709551615n;
 const u=x=>x===null?null:typeof x==='bigint'&&x>=0n&&x<=Umax?String(x):'u';
 const increment=x=>x==='x'||x===999999?'x':Number.isInteger(x)&&x>=0&&x<999999?x+1:'u';
 const terminal=()=>[0,null,null,0,null,0,0],snapshot=()=>[null,0,0,0,terminal(),terminal()];
 const packet=['do1',[0,0,0,null,null,null,null,0,0,Array(8).fill(0)],[0,0,Array(6).fill(null),Array(6).fill(null),null,null,0,0,0,null],[null,null,null,null,null,null,null,0],[0,[]],[snapshot(),snapshot()],Array.from({length:4},()=>[0,0])];
 const errors=Array.from({length:4},()=>[]);let phase=E.loop,nested=0,current=null,active=false,connection;
 const safe=f=>{try{return f();}catch(e){errors[1].push(e);}};
 const count=i=>packet[1][9][i]=increment(packet[1][9][i]);
 const number=x=>Number.isSafeInteger(x)&&x>=0?u(BigInt(x)):'u';
 const pin=x=>Array.isArray(x)&&x.length===6?x.map(v=>typeof v==='string'&&/^(0|[1-9][0-9]{0,19})$/.test(v)?u(BigInt(v)):'u'):Array(6).fill(null);
 function event(stage,poll,time,outcome){if(packet[4][1].length===8){packet[4][1].shift();packet[4][0]=increment(packet[4][0]);}packet[4][1].push([stage,poll,u(time),outcome]);}
 function encode(value=packet){
  const scalar=(v,t)=>t==='E'?Number.isInteger(v)&&v>=0&&v<=99:t==='N'?(Number.isInteger(v)&&v>=0&&v<=999999)||v==='x'||v==='u'||v===null:t==='U'?v===null||v==='u'||(typeof v==='string'&&/^(0|[1-9][0-9]{0,19})$/.test(v)&&BigInt(v)<=Umax):v===null||v==='u'||(typeof v==='string'&&/^[a-f0-9]{64}$/.test(v));
  const check=(v,t)=>{if(Array.isArray(t)){assert.ok(Array.isArray(v)&&v.length===t.length);t.forEach((k,i)=>check(v[i],k));}else assert.ok(scalar(v,t),'observation scalar '+t);};
  const T=['E','U','D','E','U','E','E'],S=['U','E','E','E',T,T];
  assert.ok(Array.isArray(value)&&value.length===7&&value[0]==='do1');check(value[1],['N','E','N','U','U','U','U','E','E',Array(8).fill('N')]);check(value[2],['E','E',Array(6).fill('U'),Array(6).fill('U'),'U','D','E','N','E','U']);check(value[3],['U','U','U','U','U','D','D','E']);assert.ok(Array.isArray(value[4])&&value[4].length===2);check(value[4][0],'N');assert.ok(Array.isArray(value[4][1])&&value[4][1].length<=8);for(const e of value[4][1])check(e,['E','N','U','E']);check(value[5],[S,S]);check(value[6],Array.from({length:4},()=>['E','E']));const raw=JSON.stringify(value)+'\n';assert.ok(/^[\x00-\x7f]*$/.test(raw)&&Buffer.byteLength(raw)<=1778);return raw;
 }
 const api={enums,packet,errors,u,increment,event,encode,
  failure(){for(let i=0;i<4;i++){packet[6][i]=[errors[i].length?2:1,errors[i].length>1?5:errors[i].length?4:0];}const all=errors.flat();return all.length>1?new AggregateError(all,'finite diagnostic failure'):all[0];},
  ack(fn){return function(state){const priorLast=state.last;const stage=nested?E.transition:phase===E.loop?E.loop:phase===E.confirm?E.final:E.sleepCheck;safe(()=>{packet[1][1]=stage;if(stage===E.loop){packet[1][2]=increment(packet[1][2]);count(6);}if(stage===E.transition)count(7);packet[1][3]=u(state.started);packet[1][4]=u(state.last);packet[2][1]=state.used?2:1;packet[2][3]=pin(state.pin);});let outcome=1;try{return Reflect.apply(fn,this,arguments);}catch(e){outcome=2;throw e;}finally{safe(()=>{packet[1][5]=u(current);packet[1][7]=typeof current==='bigint'&&typeof state.last==='bigint'?(current<priorLast?2:1):0;packet[1][8]=typeof current==='bigint'&&typeof state.started==='bigint'?(current-state.started>2000000000n?2:1):0;event(stage,packet[1][2],current,outcome);});}};},
  sleep(fn){return function(){safe(()=>{phase=E.sleep;packet[1][1]=E.sleep;});try{return Reflect.apply(fn,this,arguments);}finally{phase=E.loop;}};},
  prefix(fn){return function(){safe(()=>{phase=E.prefix;packet[1][1]=E.prefix;count(4);});const result=Reflect.apply(fn,this,arguments);safe(()=>{packet[3][4]=number(result.copyEnd);packet[3][6]=result.hash;});return result;};},
  install(c,bindings){connection=c;const restores=[],save=(name,wrap)=>{const original=c[name];c[name]=wrap(original);restores.push(()=>{c[name]=original;});};let undo=()=>{};
   try{undo=bindings(api);
    save('publish',fn=>function(){safe(()=>{phase=E.publish;packet[1][1]=E.publish;});const result=Reflect.apply(fn,this,arguments);safe(()=>{packet[1][6]=u(api.clockOriginal());});phase=E.loop;return result;});
    save('readAck',fn=>function(){nested++;safe(()=>{phase=E.ack;packet[1][1]=E.ack;});try{const result=Reflect.apply(fn,this,arguments);safe(()=>{const a=packet[2];a[2]=pin(c.lastAckPin);a[1]=arguments[0].used?2:1;a[3]=pin(arguments[0].pin);if(!result){a[0]=1;count(0);return;}a[0]=c.lastAck&&result.raw.equals(c.lastAck)?2:3;count(a[0]===2?1:2);a[4]=number(result.raw.length);a[5]=sha(result.raw);if(result.raw.length<=512){const row=JSON.parse(result.raw);a[6]=row[2]===c.nonce?2:1;a[7]=Number.isInteger(row[3])&&row[3]>=0&&row[3]<=999999?row[3]:'u';a[8]=a[7]>=1&&a[7]<=11?a[7]:0;a[9]=typeof row[6]==='string'&&/^[0-9]{1,20}$/.test(row[6])?u(BigInt(row[6])):'u';}});return result;}finally{nested--;phase=E.ack;}});
    save('read',fn=>function(name){const confirm=phase===E.prefix;if(name==='journal.jsonl')safe(()=>{phase=confirm?E.confirm:E.journal;packet[1][1]=phase;});const result=Reflect.apply(fn,this,arguments);if(name==='journal.jsonl')safe(()=>{if(confirm)count(5);const p=packet[3];p[0]=number(result.raw.length);p[1]=packet[2][9];p[3]=c.previous?number(c.previous.copyEnd):null;p[5]=c.previous?.hash??null;p[7]=confirm?2:1;const offset=p[1]!==null&&p[1]!=='u'?Number(p[1]):-1;const end=offset>=0&&offset<=result.raw.length?result.raw.indexOf(10,offset):-1;p[2]=end>=0?number(end+1):null;if(end<0)count(3);});return result;});
    save('boundary',fn=>function(seq){const clock=process.hrtime.bigint;api.clockOriginal=clock;phase=E.loop;let first=true;packet[1]=[seq,E.boundary,0,null,null,null,null,0,0,Array(8).fill(0)];packet[2]=[0,0,pin(c.lastAckPin),Array(6).fill(null),null,null,0,0,0,null];packet[3]=[null,null,null,null,null,null,null,0];packet[4]=[0,[]];active=true;process.hrtime.bigint=function(){const value=Reflect.apply(clock,this,arguments);current=value;if(first){first=false;safe(()=>{packet[1][3]=u(value);});}return value;};try{return Reflect.apply(fn,this,arguments);}finally{active=false;try{process.hrtime.bigint=clock;}catch(e){errors[3].push(e);}};});
   }catch(e){errors[3].push(e);}
   return ()=>{for(const f of [...restores.reverse(),undo])try{f();}catch(e){errors[3].push(e);}};
  },
  projectTerminal(row,role,known){const t=terminal();if(!Array.isArray(row))return t;const owner=role==='owner';const valid=owner?row.length===8&&row[0]==='oe'&&row[1]===1&&Array.isArray(row[6])&&row[6].length===3:row.length===15&&row[0]==='t'&&row[1]===1&&row[2]==='guest';if(!valid)return t;t[0]=1;const kind=owner?row[6][0]:row[5];t[3]=kind==='exited'?1:kind==='signaled'?2:kind==='ended'?3:0;t[4]=owner&&Number.isSafeInteger(row[6][1])?number(row[6][1]):null;const reason=owner?row[7]:row[6];t[5]=typeof reason==='string'&&Object.hasOwn(enums.reason,reason)?enums.reason[reason]:0;if(known)t[6]=owner?(row[2]===known.nonce&&row[3]===known.contextDigest&&row[4]===known.launch?.[5]?.[0]&&row[5]===known.launch?.[5]?.[1]?2:1):(row[3]===known.nonce&&row[4]===known.binding?2:1);return t;},
  readTerminal(directory,directoryPin,name,known){const role=name==='observer-exit.json'?'owner':'guest';assert.ok(['observer-exit.json','guest-terminal.json'].includes(name));let fd,t=terminal();const stable=s=>[s.dev,s.ino,s.uid,s.mode,s.nlink,s.size].map(String);try{const dir=fs.lstatSync(directory,{bigint:true});assert.ok(dir.isDirectory()&&!dir.isSymbolicLink());assert.deepEqual([dir.dev,dir.ino,dir.uid,dir.mode].map(String),directoryPin);fd=fs.openSync(path.join(directory,name),fs.constants.O_RDONLY|fs.constants.O_NOFOLLOW|fs.constants.O_NONBLOCK);const before=fs.fstatSync(fd,{bigint:true});if(!before.isFile()||before.uid!==BigInt(process.getuid())||before.nlink!==1n||(before.mode&511n)!==384n)throw Error('terminal identity');const raw=Buffer.alloc(4097),n=fs.readSync(fd,raw,0,4097,0);assert.ok(Number.isInteger(n)&&n>=0&&n<=4097);t[1]=number(n);t[2]=sha(raw.subarray(0,n));if(n>4096){t[0]=4;return t;}const after=fs.fstatSync(fd,{bigint:true}),currentPath=fs.lstatSync(path.join(directory,name),{bigint:true});if(JSON.stringify(stable(before))!==JSON.stringify(stable(after))||JSON.stringify(stable(before))!==JSON.stringify(stable(currentPath))){t[0]=5;return t;}if(n!==Number(before.size)||!n||raw[n-1]!==10){t[0]=3;return t;}try{const projected=api.projectTerminal(JSON.parse(raw.subarray(0,n)),role,known);projected[1]=t[1];projected[2]=t[2];if(projected[0]!==1)projected[0]=6;t=projected;}catch(e){t[0]=6;errors[1].push(e);}return t;}catch(e){t[0]=e.code==='ENOENT'?2:7;errors[1].push(e);return t;}finally{if(fd!==undefined)try{fs.closeSync(fd);}catch(e){t[0]=7;errors[1].push(e);}}},
  snapshot(index,c,child,settled,afterAwait){const s=packet[5][index];s[0]=Number.isSafeInteger(child?.exitCode)?number(child.exitCode):null;s[1]=child?.signalCode===null?1:enums.signal[child?.signalCode]??0;s[2]=settled?2:1;s[3]=afterAwait?2:1;for(const [i,name] of [[4,'observer-exit.json'],[5,'guest-terminal.json']])safe(()=>{s[i]=api.readTerminal(c.directory,c.pin,name,{nonce:c.nonce,contextDigest:c.contextDigest,launch:c.launch,binding:sha(wire(c.identity))});});}
 };return api;
}
async function deadlineScenario(mode,suppliedError){
 const m=await candidate(),H=m.testHostClass,first=records(1),next=records(2),originalClock=process.hrtime.bigint,originalWait=Atomics.wait;
 async function run(observed){let time=mode==='backwards'?100n:0n,publications=0;const calls=[],clockReturns=[],o=deadlineObserver(),c=Object.create(H.prototype);
  Object.assign(c,{nonce,identity,sequence:2,failed:false,lastAck:first.ack,lastAckPin:['1','2','3','33152','1',String(first.ack.length)],previous:m.admitBoundaryPrefix(first)});
  c.publish=()=>{calls.push('publish');publications++;if(mode==='publish')time=2000000001n;};
  c.readAck=state=>{calls.push('ack');if(mode==='throw')throw suppliedError;if(mode==='transition'){time=2000000001n;m.testObservationTick(state);}return (mode==='missing'||mode==='backwards')?null:{raw:mode==='old'?first.ack:next.ack,pin:['1','4','3','33152','1',String(next.ack.length)]};};
  c.read=()=>{calls.push('journal');if(mode==='semantic'&&calls.filter(x=>x==='journal').length===2)time=2000000001n;return {raw:mode==='partial'?next.journal.subarray(0,Number(JSON.parse(next.ack)[6])):next.journal,pin:[]};};
  const boundary=c.boundary,publish=c.publish,read=c.read,readAck=c.readAck;let restore=()=>{},error;
  const clock=()=>{clockReturns.push(time);return time;};process.hrtime.bigint=clock;Atomics.wait=()=>{calls.push('sleep');time=mode==='backwards'?0n:2000000001n;return 'timed-out';};
  try{if(observed)restore=o.install(c,m.testObservationBindings);try{c.boundary(2);}catch(e){error=e;}finally{restore();}return {error,packet:o.packet,encoded:o.encode(),calls,publications,clockIdentity:clockReturns.every(v=>typeof v==='bigint')&&process.hrtime.bigint===clock,restored:c.boundary===boundary&&c.publish===publish&&c.read===read&&c.readAck===readAck&&process.hrtime.bigint===clock};}
  finally{process.hrtime.bigint=originalClock;Atomics.wait=originalWait;}
 }
 const plain=await run(false),observed=await run(true);return {...observed,unobservedCalls:plain.calls};
}
function deadlineSnapshotControl(mode){
 const o=deadlineObserver(),dir='/fixed-snapshot',p=['1','2',String(process.getuid()),'16832'],raw=wire(['oe',1,nonce,'d'.repeat(64),'99','100',['exited',0,null],'natural']);let reads=0,opens=0,closes=0,bytes=0;
 const original={lstat:fs.lstatSync,open:fs.openSync,fstat:fs.fstatSync,read:fs.readSync,close:fs.closeSync};const file={dev:1n,ino:3n,uid:BigInt(process.getuid()),mode:33152n,nlink:1n,size:BigInt(raw.length),isFile:()=>true};
 try{fs.lstatSync=n=>n===dir?{dev:1n,ino:2n,uid:BigInt(process.getuid()),mode:16832n,isDirectory:()=>true,isSymbolicLink:()=>false}:{...file,ino:mode==='changed'?4n:3n};fs.openSync=()=>{if(mode==='missing')throw Object.assign(Error('absent'),{code:'ENOENT'});if(mode==='symlink')throw Object.assign(Error('link'),{code:'ELOOP'});opens++;return 5;};fs.fstatSync=()=>file;fs.readSync=(fd,b,offset,length)=>{reads++;bytes=length;if(mode==='oversize'){b.fill(65);return length;}return raw.copy(b,offset,0,mode==='partial'?raw.length-1:raw.length);};fs.closeSync=()=>{closes++;if(mode==='close')throw Error('close');};const t=o.readTerminal(dir,p,'observer-exit.json',null);return {status:t[0],reads,opens,closes,bytes};}
 finally{fs.lstatSync=original.lstat;fs.openSync=original.open;fs.fstatSync=original.fstat;fs.readSync=original.read;fs.closeSync=original.close;}
}
test('deadline observation cleanup preserves all errors and waits exactly once',async()=>{
 assert.equal(typeof finishDeadlineFixture,'function','single cleanup path missing');
 const o=deadlineObserver(),primary=Error('primary'),close=Error('close'),wait=Error('wait'),snapshot=Error('snapshot');o.errors[0].push(primary);const calls=[];
 o.snapshot=()=>{calls.push('snapshot');throw snapshot;};const c={closed:false,close:async()=>{calls.push('close');throw close;}};const state={awaited:false,settled:false,result:null};const done=Promise.reject(wait);done.catch(()=>{});
 await finishDeadlineFixture(o,c,{},done,state,false);assert.deepEqual(calls,['snapshot','close','snapshot']);assert.equal(state.awaited,true);assert.equal(o.errors[0][0],primary);assert.ok(o.errors[2].includes(close)&&o.errors[2].includes(wait));assert.equal(o.errors[1].length,2);
 const next=deadlineObserver();let n=0;await finishDeadlineFixture(next,{closed:false,close:async()=>{n++;return ['receipt'];}},{},Promise.resolve({code:0,signal:null}),{awaited:true,settled:true,result:{code:0,signal:null}},true);assert.equal(n,1);assert.equal(next.failure(),undefined);
});
test('deadline observation partial installation and restoration continue independently',async()=>{
 const m=await candidate(),o=deadlineObserver(),c={publish(){},readAck(){},read(){},boundary(){}};const before={...c};Object.defineProperty(c,'readAck',{value:c.readAck,writable:false,configurable:true});const undo=o.install(c,m.testObservationBindings);undo();assert.equal(c.publish,before.publish);assert.equal(c.read,before.read);assert.ok(o.errors[3].length);const e=Error('original');assert.throws(()=>o.prefix(()=>{throw e;})(),x=>x===e);
 const o2=deadlineObserver(),c2={...before},undo2=o2.install(c2,m.testObservationBindings);Object.defineProperty(c2,'read',{set(){throw Error('restore');},configurable:true});undo2();assert.equal(c2.publish,before.publish);assert.equal(c2.boundary,before.boundary);assert.ok(o2.errors[3].length);
});
async function finishDeadlineFixture(o,c,child,done,state,afterAwait){
 const snapshot=index=>{try{if(c)o.snapshot(index,c,child,state.settled,afterAwait);}catch(e){o.errors[1].push(e);}};
 if(o.errors.some(x=>x.length))snapshot(0);
 let exit;
 try{if(c&&!c.closed)exit=await c.close();else if(c)exit=c.closeResult;else if(child)child.stdin.destroy();}catch(e){o.errors[2].push(e);}
 if(done&&!state.awaited){state.awaited=true;try{state.result=await done;state.settled=true;}catch(e){state.settled=true;o.errors[2].push(e);}}
 if(o.errors.some(x=>x.length))snapshot(1);
 return exit;
}
test('deadline observation resets only current boundary and retains actual first clock',async()=>{
 const m=await candidate(),o=deadlineObserver(),c={publish(){},readAck(){},read(){},boundary(){this.publish();return marker;}},marker={},clock=process.hrtime.bigint;let values=0;process.hrtime.bigint=()=>{values++;return 123n;};let undo;
 try{c.boundary=function(){process.hrtime.bigint();this.publish();return marker;};undo=o.install(c,m.testObservationBindings);assert.equal(c.boundary(1),marker);o.packet[1][9][0]=9;o.event(3,1,123n,1);assert.equal(c.boundary(2),marker);assert.equal(o.packet[1][3],'123');assert.equal(o.packet[1][9][0],0);assert.deepEqual(o.packet[4],[0,[]]);assert.equal(values,4,'one production and one labeled publication clock per call');}
 finally{undo?.();process.hrtime.bigint=clock;}
});
test('deadline observation out-of-domain clock is typed unknown with actual comparisons retained',async()=>{
 const m=await candidate(),o=deadlineObserver(),clock=process.hrtime.bigint,c={publish(){},readAck(){},read(){},boundary(){process.hrtime.bigint();return m.testObservationTick({started:18446744073709551616n,last:18446744073709551616n,used:false});}};let undo;
 try{process.hrtime.bigint=()=>18446744073709551616n;undo=o.install(c,m.testObservationBindings);assert.equal(c.boundary(1),18446744073709551616n);assert.deepEqual(o.packet[1].slice(3,6),['u','u','u']);assert.deepEqual(o.packet[1].slice(7,9),[1,1]);assert.deepEqual(o.errors,[[],[],[],[]]);}
 finally{undo?.();process.hrtime.bigint=clock;}
});

function actualDeadlineConsumer(observation,log){
 const lines=finiteExchange.toString().split('\n').filter(line=>line.includes("console.log('DEADLINE_OBSERVATION '"));assert.equal(lines.length,1,'one actual final-consumer source statement');
 return Function('observation','console',lines[0])(observation,{log});
}
test('deadline observation final consumer preserves falsey presence and ordered compound errors',async()=>{
 for(const value of [undefined,null,false,0,-0,NaN,0n,'']){
  const o=deadlineObserver();o.errors[0].push(value);let thrown=false,caught,emits=0;
  try{actualDeadlineConsumer(o,raw=>{emits++;assert.ok(Buffer.byteLength(raw)<8192);});}catch(e){thrown=true;caught=e;}
  assert.equal(thrown,true,'actual final consumer must throw '+String(value));assert.ok(Object.is(caught,value));assert.equal(emits,1);
 }
 const clean=deadlineObserver();let emitted=0;actualDeadlineConsumer(clean,()=>emitted++);assert.equal(emitted,0);
 const encoding=deadlineObserver();encoding.errors[0].push(undefined);encoding.encode=()=>{throw false;};let encodeThrown=false,encodeError;try{actualDeadlineConsumer(encoding,()=>{throw Error('encode must precede log');});}catch(e){encodeThrown=true;encodeError=e;}assert.equal(encodeThrown,true);assert.ok(encodeError instanceof AggregateError);assert.equal(encodeError.errors.length,2);assert.ok(Object.is(encodeError.errors[0],undefined)&&Object.is(encodeError.errors[1],false));
 const o=deadlineObserver(),primary=undefined,snapshot=null,cleanup=false,restore=0,emission=NaN;let closes=0,waits=0,restores=0,emits=0;
 o.errors[0].push(primary);o.snapshot=()=>{throw snapshot;};try{restores++;throw restore;}catch(e){o.errors[3].push(e);}
 await finishDeadlineFixture(o,{closed:false,close:async()=>{closes++;throw cleanup;}},{},{then(resolve){waits++;resolve({code:0,signal:null});}},{awaited:false,settled:false,result:null},false);
 let caught;try{actualDeadlineConsumer(o,()=>{emits++;throw emission;});}catch(e){caught=e;}
 assert.ok(caught instanceof AggregateError);const expected=[primary,snapshot,snapshot,emission,cleanup,restore];assert.equal(caught.errors.length,expected.length);expected.forEach((x,i)=>assert.ok(Object.is(caught.errors[i],x)));assert.deepEqual([closes,waits,restores,emits],[1,1,1,1]);
});
async function deadlineDelegateScenario(mode){
 const m=await candidate(),H=m.testHostClass,o=deadlineObserver(),first=records(1),next=records(2),marker=Object.freeze({mode}),calls=[],clock=process.hrtime.bigint,wait=Atomics.wait;let inside=false,confirmed=false,reads=0,clocks=0;
 const c=Object.create(H.prototype);Object.assign(c,{nonce,identity,sequence:2,failed:false,lastAck:first.ack,previous:m.admitBoundaryPrefix(first)});
 c.publish=()=>{calls.push('publish');if(mode==='publish')throw marker;};
 c.readAck=s=>{calls.push('ack');if(mode==='ack')throw marker;if(mode==='transition'){inside=true;try{m.testObservationTick(s);}finally{inside=false;}}return mode==='sleep'?null:{raw:next.ack,pin:[]};};
 c.read=()=>{reads++;calls.push(reads===1?'journal':'confirm');if(mode===(reads===1?'journal':'confirm'))throw marker;if(reads===2)confirmed=true;return {raw:next.journal,pin:[]};};
 const old={publish:c.publish,readAck:c.readAck,read:c.read,boundary:c.boundary},ack=o.ack,prefix=o.prefix;
 o.ack=fn=>ack(function(s){if((mode==='transition'&&inside)||(mode==='final'&&confirmed)){calls.push('nested');throw marker;}return Reflect.apply(fn,this,arguments);});
 o.prefix=fn=>prefix(function(){calls.push('prefix');if(mode==='prefix')throw marker;return Reflect.apply(fn,this,arguments);});
 let undo=()=>{},error,thrown=false;
 try{process.hrtime.bigint=()=>{clocks++;return 100n;};Atomics.wait=()=>{calls.push('sleep');throw marker;};undo=o.install(c,m.testObservationBindings);try{c.boundary(2);}catch(e){thrown=true;error=e;}finally{undo();}return {marker,error,thrown,stage:o.packet[1][1],calls,clocks,restored:Object.keys(old).every(k=>c[k]===old[k])};}
 finally{process.hrtime.bigint=clock;Atomics.wait=wait;}
}
test('deadline observation delegate failures publish exact deepest stage before calls',async()=>{
 const expected={publish:[2,['publish']],ack:[4,['publish','ack']],journal:[6,['publish','ack','journal']],prefix:[7,['publish','ack','journal','prefix']],confirm:[8,['publish','ack','journal','prefix','confirm']],sleep:[11,['publish','ack','sleep']],transition:[5,['publish','ack','nested']],final:[9,['publish','ack','journal','prefix','confirm','nested']]};const failures=[];
 for(const [mode,[stage,calls]] of Object.entries(expected)){const x=await deadlineDelegateScenario(mode);assert.ok(x.thrown&&x.error===x.marker,mode);assert.deepEqual(x.calls,calls,mode);assert.equal(x.restored,true,mode);assert.equal(x.clocks,mode==='publish'?1:mode==='sleep'?4:3,mode+' original clock calls plus one labeled publication sample');if(x.stage!==stage)failures.push([mode,x.stage,stage]);}
 assert.deepEqual(failures,[],'every actual failing delegate must publish its stage');
});
