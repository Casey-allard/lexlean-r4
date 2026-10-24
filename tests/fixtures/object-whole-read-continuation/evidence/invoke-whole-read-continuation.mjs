// One-off official-compiler invocation for one original whole-read hash/digest continuation; no Read33 acceptance.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {spawnSync} from 'node:child_process';
const repo='/Users/casey.allard/lexlean-r4-worktrees/model-port';
const manifestPath=path.join(import.meta.dirname,'manifest.json');
const phase=process.argv[2];
if(process.argv.length!==3||!['whole-false','whole-positive'].includes(phase))throw Error('select exactly one whole-read continuation phase');
const out=path.join(import.meta.dirname,phase);
fs.mkdirSync(out);
const stage=`/tmp/r4-object-whole-read-continuation-${phase}-owned-20261023`;
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
function epoch(name){const r=docker(name,['inspect','--format','{{.State.StartedAt}} {{.RestartCount}} {{.HostConfig.Memory}} {{.HostConfig.NanoCpus}} {{.HostConfig.NetworkMode}} {{json .Mounts}}',cid]);if(r.stdout.trim()!=='2026-10-23T20:22:35.225704499Z 0 4294967296 2000000000 none []')throw Error('epoch/config drift');}
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
 if(hash(manifestBytes)!=='9a65fc447b3c1785049539d6aafada4207238f0e758962d3371aa15758080cb8')throw Error('manifest drift');
 const manifest=JSON.parse(manifestBytes);
 for(const x of manifest.files){const b=fs.readFileSync(path.join(import.meta.dirname,x.path));if(b.length!==x.bytes||hash(b)!==x.sha256)throw Error('candidate drift '+x.path);}
 const candidate=JSON.parse(fs.readFileSync(path.join(import.meta.dirname,'candidate.json')));
 const selected=candidate.phases.find(x=>x.phase===(phase==='whole-false'?'negative':'positive'));
 inputs=selected.inputs.map(x=>{if(!['repository','candidate'].includes(x.base))throw Error('input base');return {...x,path:path.resolve(x.base==='repository'?repo:import.meta.dirname,x.path)};});
 if(inputs.length!==22||new Set(inputs.map(x=>x.destination)).size!==22)throw Error('input census');
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
  const checked=cli('check',['check'],false);const checkParsed=JSON.parse(checked.stdout||'null');
  {
   if(checked.status!==0||checkParsed?.success!==true)throw Error('check result');
   const result=cli('verify',['verify','tests/lexlean/ObjectWholeReadContinuation.lex.tex'],false);const parsed=JSON.parse(result.stdout||'null');
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
