// One-off child certificate proof phase. Preparation only until independent invocation CODE and fresh live gates pass.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {spawnSync} from 'node:child_process';
const repo='/Users/casey.allard/lexlean-r4-worktrees/model-port';
const candidateRoot=path.join(repo,'tests/fixtures/object-read-child-certificate-native');
const manifestPath=path.join(candidateRoot,'manifest.json');
const phase=process.argv[2];
if(process.argv.length!==3||!["child-accounting-negative","original-admitted-negative","complete-admitted-positive"].includes(phase))throw Error('select exactly one reviewed admitted product phase; no automatic progression');
const sourcePhase=phase;
const target=phase==='child-accounting-negative'?'tests/lexlean/ObjectReadParent49ConcreteChildEntry.lex.tex':'tests/lexlean/ObjectAdmittedRangeResult.lex.tex';
const expectedSelectedModules=phase==='child-accounting-negative'?["ObjectReadParent49","ObjectReadParent49AccountingState","ObjectReadParent49AuthenticatedFetch","ObjectReadParent49ConcreteChildEntry","ObjectReadParent49FetchContext","ObjectReadParent49FetchTransport","ObjectReadParent49VisitContext","ObjectReadParent49VisitTransition","ObjectReadParent49VisitTransport","R4.Hash","R4.HashNat","R4.HashNatCore","R4.HashWire","R4.Object","R4.ObjectCodec","R4.ObjectRead","R4.ObjectStore","R4.ObjectTypes","R4.Words"]:["ObjectAdmittedRangeResult","ObjectContextualFinishContext","ObjectContextualFinishTransport","ObjectContextualRangeFinish","ObjectManifestVersion","ObjectReadCompleteParent","ObjectReadConnectedRange","ObjectReadConnectedRangeContext","ObjectReadFinalChildState","ObjectReadFirstChildContinuation","ObjectReadFirstChildState","ObjectReadParent49","ObjectReadParent49AccountingState","ObjectReadParent49AuthenticatedFetch","ObjectReadParent49ConcreteChildEntry","ObjectReadParent49FetchContext","ObjectReadParent49FetchTransport","ObjectReadParent49VisitContext","ObjectReadParent49VisitTransition","ObjectReadParent49VisitTransport","ObjectReadRootTraversal","ObjectReadSecondChildContinuation","ObjectReadSecondChildState","R4.Hash","R4.HashNat","R4.HashNatCore","R4.HashWire","R4.Object","R4.ObjectCodec","R4.ObjectRead","R4.ObjectStore","R4.ObjectTypes","R4.Words"];
// Binding below records the actual reviewed restart snapshot; it does not assert present storage or readiness.
// The retained null gate precedes output creation; all original live resource guards remain unchanged.
const freshEpochBinding=null;
if(freshEpochBinding===null)throw Error('NO_REVIEWED_FRESH_EPOCH: preparation only; no output or process invocation');
const out=path.join(import.meta.dirname,phase);
fs.mkdirSync(out);
const stage=`/tmp/r4-object-read-${phase}-owned-20261025`;
const cid='cbd8cf8ac7240f703c9492d11b4a442474cd303adfb147a82648a2e5cb6bcfdf';
const env={...process.env,DOCKER_CONFIG:'/tmp/lexlean-r4-docker-config-20261021',DOCKER_HOST:'unix:///Users/casey.allard/.colima/lexlean-r4/docker.sock'};
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const save=(name,value)=>fs.writeFileSync(path.join(out,name+'.json'),JSON.stringify(value,null,2)+'\n',{flag:'wx'});
function command(name,exe,args,required=true){
 const started=new Date().toISOString();
 const r=spawnSync(exe,args,{env,encoding:'utf8',maxBuffer:20*1024*1024});
 const value={args,started,ended:new Date().toISOString(),status:r.status,signal:r.signal,stdout:r.stdout,stderr:r.stderr,error:r.error?.message};
 save(name,value);if(required&&(r.error||r.status!==0))throw Error(name+' failed');return value;
}
const docker=(name,args,required=true)=>command(name,'docker',args,required);
const guest=(name,args,required=true)=>docker(name,['exec',cid,...args],required);
const available=()=>{const s=fs.statfsSync(repo,{bigint:true});return s.bavail*s.bsize;};
const GiB=1024n**3n;
function host(name,pre){const a=available();save(name,{at:new Date().toISOString(),availableBytes:String(a),estimatedAdditionalBytes:String(pre?10n*GiB:0n),reserveBytes:String(10n*GiB)});if(a<(pre?20n:10n)*GiB)throw Error('host reserve');}
function zeroMemory(name){const r=guest(name,['cat','/sys/fs/cgroup/memory.events']);if(r.stdout.trim().split('\n').some(l=>l.split(/\s+/)[1]!=='0'))throw Error('nonzero memory events');}
function epoch(name){const r=docker(name,['inspect','--format','{{.State.StartedAt}} {{.RestartCount}} {{.HostConfig.Memory}} {{.HostConfig.NanoCpus}} {{.HostConfig.NetworkMode}} {{json .Mounts}}',cid]);if(r.stdout.trim()!==freshEpochBinding.startedAt+' 0 4294967296 2000000000 none []')throw Error('epoch/config drift');}
function idle(name){const r=guest(name,['ps','-eo','pid,comm,args']);if(r.stdout.split('\n').some(l=>/^\s*\d+\s+(lexlean|lean|lake|leanchecker)\s/.test(l)))throw Error('active compiler');}
function capacity(name,pre){const r=guest(name,['df','-Pk','/tmp']);const k=BigInt(r.stdout.trim().split('\n').at(-1).trim().split(/\s+/)[3]);if(k*1024n<(pre?11n:10n)*GiB)throw Error('guest reserve');}
function toolsIdentity(name){
 const tools=guest(name,['sha256sum','/usr/local/bin/lexlean','/usr/local/elan/toolchains/leanprover--lean4---v4.32.1/bin/lean','/usr/local/elan/toolchains/leanprover--lean4---v4.32.1/bin/lake','/usr/local/elan/toolchains/leanprover--lean4---v4.32.1/bin/leanchecker']);
 const expectedTools=['107e02328a8052d1560d6e8addcfd1c70af6145bbf836650cac32dc044f550e5','79fb1d26fa5a39385d59fdc48a711a14b0710ca6480271acce99b4d177cea085','8229bc302b0b7d0e7679e28cc2b830c5aec995ffe5714833503360252f1490ba','8bc50bb93c2d510a366b879161ea5f0741e4039b6c43bd55c3b9b8a7b15f13de'];
 const lines=tools.stdout.trim().split('\n');
 if(lines.length!==expectedTools.length||lines.some((l,i)=>l.slice(0,64)!==expectedTools[i]))throw Error('tool drift');
}
let inputs=[],lockHash;
function sources(name){const r=guest(name,['sha256sum',...inputs.map(x=>stage+'/'+x.destination)]);const lines=r.stdout.trim().split('\n');if(lines.length!==inputs.length||lines.some((l,i)=>l.slice(0,64)!==inputs[i].sha256))throw Error('source mismatch');}
try{
 const manifestBytes=fs.readFileSync(manifestPath);
 if(hash(manifestBytes)!=='abee135319c2ad9d4299a5dce83fe049da148be9716ccb65ae9c42e60c7de60b')throw Error('manifest drift');
 const manifest=JSON.parse(manifestBytes);
 for(const x of manifest.payloads){const b=fs.readFileSync(path.join(candidateRoot,x.path));if(b.length!==x.bytes||hash(b)!==x.sha256)throw Error('candidate drift '+x.path);}
 for(const x of manifest.dependencies){const b=fs.readFileSync(path.resolve(repo,x.path));if(b.length!==x.bytes||hash(b)!==x.sha256)throw Error('authority drift '+x.path);}
 const sourceReview=fs.readFileSync(path.join(repo,'docs/continuation/2026-10-24/read-child-certificate-source/source-code-review.json'));
 if(hash(sourceReview)!=='d196ca032fb3d7117e3e68dda5f65aa1b06f913ec650a522f36cf56e472e2b3a')throw Error('source CODE review drift');
 const bindings=JSON.parse(fs.readFileSync(path.join(candidateRoot,'source-bindings.json')));
 const selected=bindings.phases.find(x=>x.mode===sourcePhase);
 if(!selected||selected.entrypoint!==target||selected.inputCount!==50||selected.selectedCount!==expectedSelectedModules.length||JSON.stringify(selected.selectedModules)!==JSON.stringify(expectedSelectedModules))throw Error('phase binding');
 inputs=selected.inputs.map(x=>{const inputPath=path.resolve(repo,x.path);if(!inputPath.startsWith(repo+path.sep))throw Error('input repository root');return {...x,path:inputPath};});
 if(inputs.length!==50||new Set(inputs.map(x=>x.destination)).size!==50)throw Error('input census');
 for(const x of inputs){const b=fs.readFileSync(path.resolve(repo,x.path));if(b.length!==x.bytes||hash(b)!==x.sha256)throw Error('input drift '+x.path);}
 host('host-before',true);
 command('allocated-before','du',['-k','/Users/casey.allard/.colima/_lima/_disks/colima-lexlean-r4/datadisk','/Users/casey.allard/.colima/_lima/colima-lexlean-r4/disk']);
 command('swap-before','sysctl',['vm.swapusage']);
 epoch('epoch-before');idle('idle-before');zeroMemory('memory-before');capacity('guest-before',true);
 toolsIdentity('tools-before');
 save('expected-inputs',inputs);
 guest('new-stage',['mkdir',stage]);guest('directories',['mkdir','-p',stage+'/src/R4',stage+'/tests/lexlean']);
 for(let i=0;i<inputs.length;i++)docker('copy-'+i,['cp',path.resolve(repo,inputs[i].path),cid+':'+stage+'/'+inputs[i].destination]);
 sources('source-before');
 const cli=(name,args,required=true)=>docker(name,['exec','-w',stage,cid,'lexlean','--diagnostic-format','json',...args],required);
 const lockResult=cli('lock',['lock'],false);
 const lockParsed=JSON.parse(lockResult.stdout||'null');
 if(lockResult.status===0&&lockParsed?.success===true){
  const lock=guest('lock-bytes',['cat',stage+'/lexlean.lock']);lockHash=hash(Buffer.from(lock.stdout));
  const checked=cli('check',['check',target],false);const checkParsed=JSON.parse(checked.stdout||'null');
  {
   if(checked.status!==0||checkParsed?.success!==true)throw Error('check result');
   if(!Array.isArray(checkParsed.modules)||JSON.stringify([...checkParsed.modules].sort())!==JSON.stringify(expectedSelectedModules))throw Error('admitted selected closure mismatch');
   const result=cli('verify',['verify',target],false);const parsed=JSON.parse(result.stdout||'null');
   save('result',{phase,stage,verifyStatus:result.status,result:parsed});
   if(result.status===0&&parsed?.success===true){const artifact=parsed.artifacts?.[0];if(!/^\.lexlean\/verified\/[a-f0-9]{64}$/.test(artifact))throw Error('artifact path');docker('artifact-copy',['cp',cid+':'+stage+'/'+artifact,path.join(out,'artifact')]);}
  }
 }else{
  save('result',{phase,stage,classification:'lock-stage-result',lockStatus:lockResult.status,result:lockParsed,check:'NOT_RUN',verify:'NOT_RUN'});
  throw Error('lock result');
 }

}catch(e){save('failure',{message:String(e),stack:e?.stack});process.exitCode=1;}
finally{
 const errors=[];
 for(const [name,fn] of [['source-after',()=>inputs.length&&sources('source-after')],['lock-after',()=>{if(lockHash){const r=guest('lock-after',['sha256sum',stage+'/lexlean.lock']);if(r.stdout.slice(0,64)!==lockHash)throw Error('lock drift');}}],['tools-after',()=>toolsIdentity('tools-after')],['epoch-after',()=>epoch('epoch-after')],['memory-after',()=>zeroMemory('memory-after')],['idle-after',()=>idle('idle-after')],['guest-after',()=>capacity('guest-after',false)],['host-after',()=>host('host-after',false)]]){try{fn();}catch(e){errors.push({name,error:String(e)});}}
 save('postcapture-errors',errors);if(errors.length)process.exitCode=1;
}
