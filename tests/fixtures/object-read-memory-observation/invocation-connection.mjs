// Fixed host connection for the admitted diagnostic; no native admission authority.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import os from 'node:os';
import {spawn} from 'node:child_process';
const ROOT_CID='cbd8cf8ac7240f703c9492d11b4a442474cd303adfb147a82648a2e5cb6bcfdf';
const ROOT_STAGE='/tmp/r4-object-read-complete-admitted-positive-owned-20261025';
const ROOT_WRAPPER="set -f\nnonce=$1\nroot=$2\nshift 2\n[ \"$#\" -eq 5 ] && [ \"$1\" = lexlean ] && [ \"$2\" = --diagnostic-format ] && [ \"$3\" = json ] && [ \"$4\" = verify ] && [ \"$5\" = tests/lexlean/ObjectAdmittedRangeResult.lex.tex ] || exit 74\n[ \"${#nonce}\" -eq 64 ] || exit 74\ncase \"$nonce\" in *[!a-f0-9]*) exit 74;; esac\n[ \"$root\" = \"/tmp/r4-object-read-root-$nonce\" ] || exit 74\nold_locale=${LC_ALL-}\nhad_locale=${LC_ALL+x}\nLC_ALL=C\nexport LC_ALL\nold_mask=$(umask)\numask 077\nset -C\n/bin/mkdir \"$root\" || exit 74\nexec 3< /proc/$$/stat || exit 74\nIFS= read -r raw <&3 || exit 74\nextra=\nif IFS= read -r extra <&3; then exit 74; fi\n[ -z \"$extra\" ] || exit 74\nexec 3<&-\ncr=$(printf '\\r')\ncase \"$raw\" in *\"$cr\"*) exit 74;; esac\ncase \"$raw\" in \"$$ (\"*\") \"*) ;; *) exit 74;; esac\npid=$$\n[ $((${#raw}+${#nonce}+${#pid}+12)) -le 4096 ] || exit 74\nprintf 'R4ROOT/1\\n%s\\n%s\\n%s\\n' \"$nonce\" \"$pid\" \"$raw\" > \"$root/root.marker\" || exit 74\ncheck_zero() {\n old_ifs=$IFS\n IFS=$(printf '\\011\\012\\013\\014\\015 ')\n seen=' '\n count=0\n size=0\n while IFS= read -r line; do\n  size=$((size+${#line}+1))\n  count=$((count+1))\n  [ \"$size\" -le 4096 ] && [ \"$count\" -le 64 ] || return 1\n  set -- $line\n  [ \"$#\" -eq 2 ] && [ \"${#1}\" -ge 1 ] && [ \"${#1}\" -le 64 ] && [ \"$2\" = 0 ] || return 1\n  case \"$1\" in *[!\\!-\\~]*) return 1;; esac\n  case \"$seen\" in *\" $1 \"*) return 1;; esac\n  seen=\"$seen$1 \"\n done\n [ -z \"$line\" ] || return 1\n for key in low high max oom oom_kill oom_group_kill; do\n  case \"$seen\" in *\" $key \"*) ;; *) return 1;; esac\n done\n IFS=$old_ifs\n}\ncheck_zero < /sys/fs/cgroup/memory.events || exit 74\nif [ \"$had_locale\" = x ]; then LC_ALL=$old_locale; export LC_ALL; else unset LC_ALL; fi\numask \"$old_mask\"\nexec \"$@\"\n";
const ROOT_READ='test -f "$1" && test ! -L "$1" && exec /usr/bin/head -c 4097 -- "$1"';
const ROOT_ENV={...process.env,DOCKER_CONFIG:'/tmp/lexlean-r4-docker-config-20261021',DOCKER_HOST:'unix:///Users/casey.allard/.colima/lexlean-r4/docker.sock'};
const LABELS=['staging-before','staging-after','lock-before','lock-after','check-before','check-after','before-verify','verify-return-before-copy','copy-start','copy-end','postguard-complete'];
const LIMIT=1012736, U64=2n**64n-1n;
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const fail=reason=>{throw Error('connection_'+reason);};
function requireValue(value,reason){if(!value)fail(reason);}
function encoded(value){return Buffer.from(JSON.stringify(value,(_,v)=>typeof v==='bigint'?JSON.rawJSON(String(v)):v).replace(/[\u007f-\uffff]/g,c=>'\\u'+c.charCodeAt(0).toString(16).padStart(4,'0'))+'\n');}
function decode(raw,cap){
 requireValue(Buffer.isBuffer(raw)&&raw.length>0&&raw.length<=cap,'size');
 let v;try{v=JSON.parse(raw.toString('utf8'),(k,x,c)=>typeof x==='number'&&!Number.isSafeInteger(x)&&c?.source&&/^-?\d+$/.test(c.source)?BigInt(c.source):x);}catch{fail('json');}
 requireValue(encoded(v).equals(raw),'canonical');return v;
}
function decimal(v){requireValue(typeof v==='string'&&/^(0|[1-9][0-9]*)$/.test(v)&&BigInt(v)<=U64,'decimal');return BigInt(v);}
function integer(v,min,max){requireValue(Number.isSafeInteger(v)&&v>=min&&v<=max,'integer');return v;}
function hex(v){requireValue(typeof v==='string'&&/^[a-f0-9]{64}$/.test(v),'hex');return v;}
function control(raw,tag){
 const v=decode(raw,512);requireValue(Array.isArray(v)&&v.length===(tag==='q'?5:9)&&v[0]===tag&&v[1]===1,'wire');
 hex(v[2]);integer(v[3],1,11);requireValue(v[4]===LABELS[v[3]-1],'label');
 if(tag==='a'){integer(v[5],1,LIMIT);requireValue(decimal(v[6])>0n&&BigInt(v[6])<=BigInt(LIMIT)&&v[7]===v[6],'offset');hex(v[8]);}return v;
}
const REQUIRED=['low','high','max','oom','oom_kill','oom_group_kill'];
const HEADER_LIMITS={quotaProfile:'owner-retention/1',pidFlags:{rssAbsent:1,hwmAbsent:2,namespaceAbsent:4,parentUnknown:8},sampleFlags:{peakUnsupported:1},slowFlags:{pressureUnsupported:1,localUnsupported:2},maxPids:64,maxIdentities:1024,maxKeysets:32};
function header(row,nonce,identity){
 requireValue(Array.isArray(row)&&row.length===10&&row[0]==='h'&&row[1]===3&&row[2]===nonce&&Array.isArray(row[4])&&row[4].length===3&&same([...row[4],row[3],row[5],row[6]],identity)&&row[8]==='relative_us'&&same(row[9],HEADER_LIMITS),'header');decimal(row[7]);
}
function keyed(text,cap,count,events=false){
 requireValue(typeof text==='string'&&!/[^\x00-\x7f]/.test(text)&&Buffer.byteLength(text)<=cap&&text.endsWith('\n'),'keyed');
 const lines=text.slice(0,-1).split('\n');requireValue(lines.length>=1&&lines.length<=count,'key_count');const result=new Map();
 for(const line of lines){const parts=line.split(/[\x09-\x0d\x20]+/).filter(Boolean);requireValue(parts.length===2&&/^[!-~]{1,64}$/.test(parts[0])&&!result.has(parts[0]),'key');result.set(parts[0],decimal(parts[1]));}
 if(events)requireValue(REQUIRED.every(k=>result.has(k)),'required_events');return result;
}
function pressure(text){
 requireValue(typeof text==='string'&&!/[^\x00-\x7f]/.test(text)&&Buffer.byteLength(text)<=4096&&text.endsWith('\n'),'pressure');const seen=new Set();
 for(const line of text.slice(0,-1).split('\n')){const p=line.split(/[\x09-\x0d\x20]+/).filter(Boolean),kind=p.shift();requireValue(['some','full'].includes(kind)&&!seen.has(kind)&&p.length===4,'pressure');seen.add(kind);const fields=new Map();
  for(const x of p){const parts=x.split('=');requireValue(parts.length===2&&!fields.has(parts[0]),'pressure');fields.set(...parts);}
  requireValue(fields.size===4&&['avg10','avg60','avg300','total'].every(k=>fields.has(k)),'pressure');decimal(fields.get('total'));for(const k of ['avg10','avg60','avg300'])requireValue(/^[0-9]{1,20}\.[0-9]{1,20}$/.test(fields.get(k)),'pressure');
 }
}
function raw64(value){requireValue(typeof value==='string','base64');const b=Buffer.from(value,'base64');requireValue(b.length<=4096&&b.toString('base64')===value,'base64');return b;}
function statIdentity(raw){
 requireValue(Buffer.isBuffer(raw)&&raw.length<=4096&&raw.at(-1)===10,'incomplete_stat');
 const first=raw.indexOf(' ('),last=raw.lastIndexOf(') ');
 requireValue(first>=1&&last>=first+2,'stat_delimiters');
 const tail=raw.subarray(last+2,-1);requireValue(!tail.includes(10)&&!tail.includes(13),'stat_tail');
 // Latin1 preserves every byte; Python bytes.split uses only ASCII whitespace.
 const fields=tail.toString('latin1').split(/[\x09-\x0d\x20]+/).filter(Boolean);
 requireValue(fields.length>=20&&/^[A-Za-z]$/.test(fields[0]),'stat_fields');
 const value=[raw.subarray(0,first).toString('latin1'),fields[1],fields[19]];
 value.forEach(decimal);requireValue(value[0]!=='0','pid');return value;
}
function checkIdentity(row,target){
 requireValue(row.length===14&&row[6]==='owned_unattributed'&&row[7]===null,'identity');integer(row[1],1,1024);[row[2],row[3],row[4]].forEach(decimal);requireValue(row[2]!=='0','pid');if(row[5]!==null)decimal(row[5]);
 const before=statIdentity(raw64(row[11])),after=statIdentity(raw64(row[12]));requireValue(encoded(before).equals(encoded(after))&&encoded(before).equals(encoded([row[2],row[4],row[3]])),'raw_stat');
 requireValue(raw64(row[10]).equals(Buffer.from('0::'+target+'\n')),'membership');raw64(row[8]);
 if(row[9]!==null){requireValue(Array.isArray(row[9])&&row[9].length>=1&&row[9].length<=64&&row[9][0]===row[2],'namespace');row[9].forEach(x=>requireValue(decimal(x)>0n,'namespace'));}
 requireValue(Array.isArray(row[13])&&['observed','not_observed','unavailable'].includes(row[13][0]),'parent');return row[2];
}
function admitBoundaryPrefix({journal,request,ack,identity,previousPrefix}){
 const q=control(request,'q'),a=control(ack,'a');
 requireValue(encoded(q.slice(2)).equals(encoded(a.slice(2,5)))&&a[8]===hash(encoded(identity)),'binding');
 requireValue(Buffer.isBuffer(journal)&&journal.length<=LIMIT,'journal');
 if(previousPrefix)requireValue(journal.length>=previousPrefix.copyEnd&&hash(journal.subarray(0,previousPrefix.copyEnd))===previousPrefix.hash&&a[3]===previousPrefix.sequence+1&&a[5]>previousPrefix.sample&&BigInt(a[6])>BigInt(previousPrefix.declaredOffset),'previous');
 const declaredOffset=Number(a[6]);let offset=0,copyEnd=0, pending=null,expected=1,sample=0,interval,slow=false,lastEnd=0n,global=null,local=null,ever=false;
 const dictionaries=new Map(),identities=new Map();
 function history(previous,current){if(previous)requireValue(current&&[...previous].every(([k,v])=>current.has(k)&&current.get(k)>=v),'counter_history');if(current)ever ||= [...current.values()].some(v=>v!==0n);return current;}
 while(offset<journal.length){
  const end=journal.indexOf(10,offset);requireValue(end>=0,'partial');const raw=journal.subarray(offset,end+1),row=decode(raw,32768);
  const caps={h:8192,e:8192,i:8192,s:16384,m:32768,g:8192,b:8192};
  requireValue(Array.isArray(row)&&caps[row[0]]&&raw.length<=caps[row[0]],'row');
  if(offset===0)header(row,q[2],identity);
  else if(row[0]==='h'||row[0]==='g')fail('header_gap');
  if(pending)requireValue(row[0]==='b','adjacency');
  if(row[0]==='e'){requireValue(row.length===3&&row[1]===dictionaries.size+1&&row[1]<=32&&Array.isArray(row[2])&&row[2].length<=64&&REQUIRED.every(k=>row[2].includes(k)),'dictionary');requireValue(row[2].every((k,i)=>typeof k==='string'&&/^[!-~]{1,64}$/.test(k)&&(i===0||row[2][i-1]<k)),'dictionary');dictionaries.set(row[1],row[2]);}
  if(row[0]==='i'){requireValue(row[1]===identities.size+1,'identity_order');identities.set(row[1],checkIdentity(row,identity[4][0]));}
  if(row[0]==='s'){
   requireValue(row.length===12&&row[1]===sample+1&&dictionaries.has(row[4]),'sample');sample=row[1];interval=row.slice(2,4);slow=false;const start=decimal(row[2]),endTime=decimal(row[3]);requireValue(start>=lastEnd&&endTime>=start,'sample_time');lastEnd=endTime;
   const keys=dictionaries.get(row[4]);requireValue(Array.isArray(row[5])&&row[5].length===keys.length,'event_values');global=history(global,new Map(keys.map((k,i)=>[k,decimal(row[5][i])])));
   decimal(row[6]);decimal(row[7]);integer(row[11],0,1);requireValue((row[8]===null)===(row[11]===1),'peak');if(row[8]!==null)decimal(row[8]);integer(row[10],0,Number.MAX_SAFE_INTEGER);
   requireValue(Array.isArray(row[9])&&row[9].length<=64,'processes');const seen=new Set();for(const p of row[9]){requireValue(Array.isArray(p)&&p.length===4&&identities.has(p[0])&&!seen.has(identities.get(p[0])),'process_reference');seen.add(identities.get(p[0]));integer(p[3],0,15);for(const [i,mask] of [[1,1],[2,2]]){requireValue((p[i]===null)===Boolean(p[3]&mask),'rss_flag');if(p[i]!==null)decimal(p[i]);}}
  }
  if(row[0]==='m'){requireValue(row.length===8&&!slow&&row[1]===sample&&encoded(row.slice(2,4)).equals(encoded(interval)),'slow');slow=true;keyed(row[4],16384,1024);integer(row[7],0,3);requireValue((row[5]===null)===Boolean(row[7]&1)&&(row[6]===null)===Boolean(row[7]&2),'slow_flags');if(row[5]!==null)pressure(row[5]);local=history(local,row[6]===null?null:keyed(row[6],4096,64,true));}
  if(row[0]==='b'){
   requireValue(row.length===8&&row[1]===expected&&row[2]===LABELS[expected-1]&&row[3]===interval?.[0]&&row[4]===sample&&row[5]===String(offset),'boundary');
   if(!pending){
    requireValue(slow,'forced_slow');if(expected===7)requireValue(!ever,'preverify_nonzero');
    const wanted=encoded(['q',1,q[2],expected,LABELS[expected-1]]);
    requireValue((row[6]==='synced'||row[6]==='unsynced')&&encoded(row[7]).equals(encoded([0,wanted.toString('base64'),null])),'request_copy');pending={offset:end+1,sample};
   }else{
    const wanted=encoded(['a',1,q[2],expected,LABELS[expected-1],sample,String(pending.offset),String(pending.offset),a[8]]);
    requireValue(row[6]==='synced'&&encoded(row[7]).equals(encoded([0,null,wanted.toString('base64')])),'ack_copy');
    if(expected===q[3]){requireValue(pending.offset===declaredOffset&&sample===a[5]&&wanted.equals(ack),'ack_prefix');copyEnd=end+1;break;}
    pending=null;expected++;
   }
  }
  offset=end+1;
 }
 requireValue(copyEnd>0,'missing_boundary');
 return {sequence:q[3],sample:a[5],declaredOffset,copyEnd,hash:hash(journal.subarray(0,copyEnd))};
}
const pauseWord=new Int32Array(new SharedArrayBuffer(4));
const now=()=>process.hrtime.bigint();
function sleepSync(remaining){if(remaining>0n)Atomics.wait(pauseWord,0,0,Number(remaining>10000000n?10000000n:remaining)/1e6);}
const filePin=s=>[s.dev,s.ino,s.uid,s.mode,s.nlink].map(String);
const ackPin=s=>[...filePin(s),String(s.size)];
function ackTime(state){const time=now();requireValue(time>=state.last&&time-state.started<=2000000000n,'deadline');state.last=time;return time;}
function same(a,b){return encoded(a).equals(encoded(b));}
function usingFD(fd,action){let result;const errors=[];try{result=action(fd);}catch(e){errors.push(e);}try{fs.closeSync(fd);}catch(e){errors.push(e);}if(errors.length)throw new AggregateError(errors,'connection_io');return result;}
class HostConnection{
 constructor({directory,pin,nonce,identity}){
  requireValue(typeof directory==='string'&&path.isAbsolute(directory)&&path.normalize(directory)===directory&&!/[\0\r\n]/.test(directory),'directory');
  requireValue(Array.isArray(pin)&&pin.length===4,'pin');pin.forEach(decimal);hex(nonce);
  this.directory=directory;this.pin=pin;this.nonce=nonce;this.identity=identity;this.sequence=1;this.failed=false;this.previous=null;this.published=null;this.lastAck=null;this.closed=false;
  this.guard();
 }
 guard(){
  requireValue(!this.closed,'closed');let cursor=path.parse(this.directory).root;
  for(const part of this.directory.slice(cursor.length).split('/')){cursor=path.join(cursor,part);const item=fs.lstatSync(cursor,{bigint:true});requireValue(item.isDirectory()&&!item.isSymbolicLink(),'directory_type');}
  const s=fs.lstatSync(this.directory,{bigint:true});requireValue(same([s.dev,s.ino,s.uid,s.mode].map(String),this.pin)&&s.uid===BigInt(process.getuid())&&(s.mode&511n)===448n,'directory_pin');
 }
 read(name,cap,missing=false,append=false){
  this.guard();let fd;
  try{fd=fs.openSync(path.join(this.directory,name),fs.constants.O_RDONLY|fs.constants.O_NOFOLLOW|fs.constants.O_NONBLOCK);}catch(e){if(missing&&e.code==='ENOENT'){this.guard();return null;}throw e;}
  return usingFD(fd,()=>{
   const before=fs.fstatSync(fd,{bigint:true});requireValue(before.isFile()&&before.uid===BigInt(process.getuid())&&(before.mode&511n)===384n&&before.nlink===1n&&before.size<=BigInt(cap),'file_pin');
   const raw=Buffer.alloc(Number(before.size));let done=0;
   while(done<raw.length){const n=fs.readSync(fd,raw,done,raw.length-done,done);requireValue(Number.isInteger(n)&&n>0&&n<=raw.length-done,'read');done+=n;}
   const after=fs.fstatSync(fd,{bigint:true});requireValue(same(filePin(before),filePin(after))&&(append?after.size>=before.size:after.size===before.size),'file_changed');
   const current=fs.lstatSync(path.join(this.directory,name),{bigint:true});requireValue(same(filePin(before),filePin(current)),'file_replaced');this.guard();return {raw,pin:[...filePin(before),String(before.size)]};
  });
 }
 readAck(state){
  if(state.used){
   const peer=this.read('ack',512);requireValue(same(peer.pin,state.pin)&&same(ackPin(fs.lstatSync(path.join(this.directory,'ack'),{bigint:true})),state.pin),'ack_generation');this.guard();ackTime(state);return peer;
  }
  this.guard();let fd;
  try{fd=fs.openSync(path.join(this.directory,'ack'),fs.constants.O_RDONLY|fs.constants.O_NOFOLLOW|fs.constants.O_NONBLOCK);}catch(e){if(e.code==='ENOENT'){this.guard();return null;}throw e;}
  const prior=this.lastAckPin;
  const known=s=>Array.isArray(prior)&&prior.length===6&&prior[4]==='1'&&same([...filePin(s).slice(0,4),'1',String(s.size)],prior);
  const valid=s=>s.isFile()&&s.uid===BigInt(process.getuid())&&(s.mode&511n)===384n&&s.size<=512n;
  const value=usingFD(fd,()=>{
   const before=fs.fstatSync(fd,{bigint:true});requireValue(valid(before)&&(before.nlink===1n||(before.nlink===0n&&known(before))),'file_pin');
   const raw=Buffer.alloc(Number(before.size));let done=0;
   while(done<raw.length){const n=fs.readSync(fd,raw,done,raw.length-done,done);requireValue(Number.isInteger(n)&&n>0&&n<=raw.length-done,'read');done+=n;}
   const after=fs.fstatSync(fd,{bigint:true});
   const accepted=known(before)&&known(after)&&this.lastAck&&raw.equals(this.lastAck)&&after.isFile()&&(after.nlink===0n||after.nlink===1n)&&!(before.nlink===0n&&after.nlink===1n);
   requireValue((same(ackPin(before),ackPin(after))&&before.nlink===1n)||accepted,'file_changed');
   // A producer generation is a new inode, never in-place rewriting of an accepted ACK.
   if(prior&&String(before.dev)===prior[0]&&String(before.ino)===prior[1])requireValue(accepted,'ack_old_mutated');
   const current=fs.lstatSync(path.join(this.directory,'ack'),{bigint:true});this.guard();
   if(same(ackPin(before),ackPin(current))&&before.nlink===1n)return {raw,pin:ackPin(before)};
   requireValue(accepted&&valid(current)&&current.nlink===1n&&current.dev===before.dev&&current.ino!==before.ino,'file_replaced');
   ackTime(state);return {replacement:ackPin(current)};
  });
  if(!value.replacement)return value;
  // usingFD has closed the accepted-old descriptor successfully before this grant.
  state.used=true;state.pin=value.replacement;
  const peer=this.read('ack',512);requireValue(same(peer.pin,state.pin)&&same(ackPin(fs.lstatSync(path.join(this.directory,'ack'),{bigint:true})),state.pin),'ack_generation');this.guard();ackTime(state);return peer;
 }
 publish(raw){
  control(raw,'q');const current=this.read('request',512,true);requireValue(same(current?.pin??null,this.published?.pin??null)&&(!current||current.raw.equals(this.published.raw)),'destination');
  const file=path.join(this.directory,'request_temp'),fd=fs.openSync(file,fs.constants.O_RDWR|fs.constants.O_CREAT|fs.constants.O_EXCL|fs.constants.O_NOFOLLOW,0o600);
  usingFD(fd,()=>{let offset=0;while(offset<raw.length){const n=fs.writeSync(fd,raw,offset,raw.length-offset);requireValue(Number.isInteger(n)&&n>0&&n<=raw.length-offset,'write');offset+=n;}fs.fsyncSync(fd);});
  const prepared=this.read('request_temp',512);requireValue(prepared.raw.equals(raw),'readback');
  const again=this.read('request',512,true);requireValue(same(again?.pin??null,current?.pin??null)&&(!again||again.raw.equals(current.raw)),'destination_changed');
  requireValue(same(this.read('request_temp',512).pin,prepared.pin),'temp_changed');this.guard();fs.renameSync(file,path.join(this.directory,'request'));this.published=prepared;
 }
 boundary(sequence){
  try{
   requireValue(!this.failed&&sequence===this.sequence&&sequence<=11,'sequence');
   const started=now(),request=encoded(['q',1,this.nonce,sequence,LABELS[sequence-1]]),waiting={started,last:started,used:false,pin:null};
   this.publish(request);
   while(true){
    ackTime(waiting);
    const peer=this.readAck(waiting);
    if(peer&&(waiting.used||!this.lastAck||!peer.raw.equals(this.lastAck))){
     const ack=control(peer.raw,'a');requireValue(ack[2]===this.nonce&&ack[3]===sequence&&ack[8]===hash(encoded(this.identity)),'ack_identity');
     const journal=this.read('journal.jsonl',LIMIT,false,true),declared=Number(ack[6]);
     const end=journal.raw.indexOf(10,declared);
     if(journal.raw.length>=declared&&end>=0){
      requireValue(end+1-declared<=8192,'adjacent_size');
      const result=admitBoundaryPrefix({journal:journal.raw.subarray(0,end+1),request,ack:peer.raw,identity:this.identity,previousPrefix:this.previous});
      const confirm=this.read('journal.jsonl',LIMIT,false,true);requireValue(confirm.raw.length>=result.copyEnd&&hash(confirm.raw.subarray(0,result.copyEnd))===result.hash,'prefix_changed');
      const accepted=ackTime(waiting);
      this.lastAck=peer.raw;this.lastAckPin=peer.pin;this.previous=result;this.sequence++;
      if(sequence===7||sequence===8){this.rootControls??=new Map();this.rootControls.set(sequence,{request,ack:peer.raw,receivedNs:String(accepted)});}
      return {...result,receivedNs:String(accepted)};
     }
    }
    sleepSync(2000000000n-(ackTime(waiting)-started));
   }
  }catch(e){this.failed=true;throw e;}
 }
 verifyArguments(stage){
  requireValue(stage===ROOT_STAGE&&this.identity[0]===ROOT_CID,'root_invocation');hex(this.nonce);
  return ['exec','-w',stage,ROOT_CID,'/bin/sh','-c',ROOT_WRAPPER,'r4-root-v1',this.nonce,'/tmp/r4-object-read-root-'+this.nonce,'lexlean','--diagnostic-format','json','verify','tests/lexlean/ObjectAdmittedRangeResult.lex.tex'];
 }
 writeRootSlot(name,raw,cap){
  requireValue((name==='root.marker'&&cap===4096)||(name==='root-retrieval.json'&&cap===16384),'root_slot');
  requireValue(Buffer.isBuffer(raw)&&raw.length<=cap,'root_size');this.guard();
  const fd=fs.openSync(path.join(this.directory,name),fs.constants.O_RDWR|fs.constants.O_CREAT|fs.constants.O_EXCL|fs.constants.O_NOFOLLOW,0o600);
  usingFD(fd,()=>{let done=0;while(done<raw.length){const n=fs.writeSync(fd,raw,done,raw.length-done);requireValue(Number.isInteger(n)&&n>0&&n<=raw.length-done,'root_write');done+=n;}fs.fsyncSync(fd);});
  requireValue(this.read(name,cap).raw.equals(raw),'root_readback');
 }
 async retainRoot(){
  requireValue(!this.rootAttempted,'root_once');this.rootAttempted=true;
  requireValue(this.identity[0]===ROOT_CID,'root_invocation');hex(this.nonce);
  const capture=await captureRoot(this);this.rootCapture=capture;
  const errors=[...capture.errors];
  if(capture.row[7])try{this.writeRootSlot('root.marker',capture.stdout,4096);}catch(e){errors.push(e);}
  try{this.writeRootSlot('root-retrieval.json',encoded(capture.row),16384);}catch(e){errors.push(e);}
  if(errors.length)throw new AggregateError(errors,'root_acquisition');
  return capture.row;
 }
 closeFiles(){this.closed=true;}
 ownTransport(child,digest){
  requireValue(!this.child,'transport_once');hex(digest);this.contextDigest=digest;this.child=child;this.transportClosed=false;this.transportErrors=[];
  child.on('error',e=>this.transportErrors.push(e));child.stdin.on('error',e=>this.transportErrors.push(e));
  child.once('close',(code,signal)=>{this.transportClosed=true;this.transportResult={code,signal};});
 }
 ownerRecord(name){
  const value=this.read(name,512,true);if(!value||value.raw.length===0)return null;
  // Owner writes once rather than replacing; an incomplete final LF is pending.
  if(value.raw.at(-1)!==10)return null;
  const row=decodeOwner(value.raw);requireValue(row[2]===this.nonce&&row[3]===this.contextDigest,'owner_binding');return row;
 }
 ready(){
  const deadline=now()+2000000000n;
  while(true){
   requireValue(now()<=deadline&&!this.transportErrors.length,'ready_timeout');
   const launch=this.ownerRecord('observer-launch.json');
   if(launch){requireValue(launch[0]==='ol'&&same(launch[5],this.identity[5].slice(0,2))&&launch[6]===this.identity[3],'launch_identity');this.launch=launch;
    const journal=this.read('journal.jsonl',LIMIT,true,true);
    if(journal){const end=journal.raw.indexOf(10);if(end>=0){header(decode(journal.raw.subarray(0,end+1),8192),this.nonce,this.identity);return;}}
   }
   sleepSync(deadline-now());
  }
 }
 async close(){
  if(this.closeResult)return this.closeResult;
  if(this.closed)fail('closed_without_receipt');
  const errors=[];let exit,receiptFailed=false;
  try{
   const deadline=now()+6000000000n;
   try{requireValue(this.child,'transport_missing');this.child.stdin.end();}catch(e){errors.push(e);}
   while(now()<=deadline){
    if(!exit&&!receiptFailed)try{
     exit=this.ownerRecord('observer-exit.json');
     if(exit){requireValue(exit[0]==='oe','exit_tag');if(this.launch)requireValue(same(exit.slice(4,6),this.launch[5]),'exit_identity');requireValue(['exited','signaled'].includes(exit[6][0]),'exit_unknown');}
    }catch(e){errors.push(e);receiptFailed=true;exit=undefined;}
    if(this.transportClosed&&(exit||receiptFailed))break;
    await new Promise(resolve=>setTimeout(resolve,Math.max(0,Math.min(10,Number(deadline-now())/1e6))));
   }
   if(!this.transportClosed||(!exit&&!receiptFailed))errors.push(Error('connection_cleanup_timeout'));
   if(this.transportErrors)errors.push(...this.transportErrors);
   if(this.transportClosed&&(this.transportResult.code!==0||this.transportResult.signal!==null))errors.push(Error('connection_transport_failure'));
  }catch(e){errors.push(e);}finally{try{this.closeFiles();}catch(e){errors.push(e);}}
  if(errors.length===1)throw errors[0];
  if(errors.length)throw new AggregateError(errors,'connection_cleanup');
  this.closeResult=exit;return exit;
 }
}
function captureRoot(owner){
 const errors=[],out=[Buffer.alloc(0),Buffer.alloc(0)],ended=[false,false],overflow=[false,false],readFailed=[false,false];
 let started,end,child,exited=false,closed=false,noChild=false,stopped=false,finished=false,signalLost=false,kill=false,processState=['not_run',null,null];
 let normalTimer,finalTimer,killTimer;
 const numberSignal=signal=>typeof signal==='string'&&Number.isInteger(os.constants.signals[signal])&&os.constants.signals[signal]>=1&&os.constants.signals[signal]<=64?os.constants.signals[signal]:null;
 const status=(code,signal)=>{const n=numberSignal(signal);return Number.isInteger(code)&&code>=0&&code<=255&&signal===null?['exited',code,null]:code===null&&n!==null?['signaled',null,n]:['unknown',null,null];};
 return new Promise(resolve=>{
  const finish=()=>{
   if(finished)return;finished=true;clearTimeout(normalTimer);clearTimeout(finalTimer);clearTimeout(killTimer);
   try{end=now();if(started!==undefined&&end<started)throw Error('root_clock_regression');if(started!==undefined&&end-started>2000000000n)errors.push(Error('root_capture_deadline'));}catch(e){errors.push(e);end=undefined;}
   if(!closed&&child){processState=['unknown',null,null];for(const pipe of [child.stdout,child.stderr])try{pipe.destroy();}catch(e){errors.push(e);}}
   const complete=ended.map((v,i)=>v&&!overflow[i]&&!readFailed[i]);
   let reason='unknown';
   if(noChild)reason='spawn_error';
   else if(overflow[0]||overflow[1])reason=!complete[0]&&!complete[1]?'truncated_both':!complete[0]?'truncated_stdout':'truncated_stderr';
   else if(!stopped&&processState[0]==='exited'&&processState[1]===0&&complete.every(Boolean)&&errors.length===0)reason='none';
   else if(processState[0]==='signaled')reason='signaled';
   else if(processState[0]==='exited'&&processState[1]!==0)reason='exit_nonzero';
   if(reason!=='none'&&errors.length===0)errors.push(Error('root_'+reason));
   const timed=started!==undefined&&end!==undefined&&end>=started;
   resolve({stdout:out[0],row:['r',1,owner.nonce,hash(encoded(owner.identity)),timed?String(started):null,timed?String(end):null,processState,...complete,out[0].toString('base64'),out[1].toString('base64'),reason],errors});
  };
  const signal=kind=>{
   if(finished||exited||closed||noChild||signalLost||!child||!Number.isInteger(child.pid)||child.pid<=0)return;
   try{const time=now();if(time<started||time>=started+4000000000n){signalLost=true;errors.push(Error('root_signal_clock'));return;}if(!child.kill(kind)){signalLost=true;errors.push(Error('root_signal_not_sent'));}}catch(e){signalLost=true;errors.push(e);}
  };
  const stop=reason=>{
   if(stopped||finished)return;stopped=true;errors.push(Error('root_'+reason));
   let remaining;
   try{const time=now();if(time<started)throw Error('root_clock_regression');remaining=started+4000000000n-time;}catch(e){errors.push(e);return;}
   if(remaining<=0n){finish();return;}
   signal('SIGTERM');
   killTimer=setTimeout(()=>{if(!kill){kill=true;signal('SIGKILL');}},Math.min(250,Number(remaining)/1e6));
  };
  try{started=now();}catch(e){errors.push(e);noChild=true;finish();return;}
  normalTimer=setTimeout(()=>stop('timeout'),2000);
  finalTimer=setTimeout(()=>{if(!closed)errors.push(Error('root_completion_timeout'));finish();},4000);
  try{
   child=spawn('docker',['exec',ROOT_CID,'/bin/sh','-c',ROOT_READ,'r4-root-read-v1','/tmp/r4-object-read-root-'+owner.nonce+'/root.marker'],{env:ROOT_ENV,stdio:['ignore','pipe','pipe']});
   owner.rootChild=child;
   child.on('error',e=>{errors.push(e);if(!Number.isInteger(child.pid)||child.pid<=0){noChild=true;processState=['not_run',null,null];finish();}else stop('process_error');});
   child.once('exit',(code,signalName)=>{exited=true;processState=status(code,signalName);});
   child.once('close',(code,signalName)=>{closed=true;processState=status(code,signalName);finish();});
   [child.stdout,child.stderr].forEach((pipe,i)=>{
    pipe.on('data',part=>{if(finished)return;const bytes=Buffer.from(part),room=4096-out[i].length;out[i]=Buffer.concat([out[i],bytes.subarray(0,room)]);if(bytes.length>room){overflow[i]=true;stop(i?'stderr_overflow':'stdout_overflow');}});
    pipe.on('end',()=>{ended[i]=true;});pipe.on('error',e=>{readFailed[i]=true;errors.push(e);stop('read_error');});
   });
  }catch(e){errors.push(e);noChild=!child;processState=noChild?['not_run',null,null]:['unknown',null,null];if(child)stop('setup_error');else finish();}
 });
}
function ownerIdentity(pair,optional=false){requireValue(Array.isArray(pair)&&pair.length===2&&decimal(pair[0])>0n,'owner_identity');if(pair[1]!==null||!optional)decimal(pair[1]);}
function decodeOwner(raw){
 const v=decode(raw,512);requireValue(Array.isArray(v)&&v[1]===1&&((v[0]==='ol'&&v.length===7)||(v[0]==='oe'&&v.length===8)),'owner_shape');hex(v[2]);hex(v[3]);
 if(v[0]==='ol'){ownerIdentity(v[4]);ownerIdentity(v[5]);requireValue(typeof v[6]==='string'&&/^[a-f0-9]{8}-(?:[a-f0-9]{4}-){3}[a-f0-9]{12}$/.test(v[6]),'boot');}
 else{
  const p=v[6];requireValue(Array.isArray(p)&&p.length===3,'owner_process');const reasons={not_run:['not_started','fork_failed'],exited:['natural','host_eof','owner_error'],signaled:['natural','host_eof','owner_error'],unknown:['lost_custody','reap_timeout','owner_error']};
  requireValue(Object.hasOwn(reasons,p[0])&&reasons[p[0]].includes(v[7]),'owner_reason');
  if(p[0]==='not_run')requireValue(v[4]===null&&v[5]===null,'not_run');else ownerIdentity(v.slice(4,6),true);
  if(p[0]==='exited'){integer(p[1],0,255);requireValue(p[2]===null,'exit_signal');}
  else if(p[0]==='signaled'){requireValue(p[1]===null,'signal_code');integer(p[2],1,64);}
  else requireValue(p[1]===null&&p[2]===null,'unknown_status');
 }
 return v;
}
function keys(object,names){requireValue(object!==null&&typeof object==='object'&&!Array.isArray(object)&&same(Object.keys(object).sort(),[...names].sort()),'config_keys');}
function absolute(value){requireValue(typeof value==='string'&&Buffer.byteLength(value)<=4092&&path.isAbsolute(value)&&path.normalize(value)===value&&!/[\0\r\n]/.test(value)&&!value.split('/').some(p=>p==='.'||p==='..'),'path');return value;}
function directoryPin(pin,privateMode=false){requireValue(Array.isArray(pin)&&pin.length===4,'config_pin');pin.forEach(decimal);const mode=BigInt(pin[3]);requireValue((mode&61440n)===16384n&&(!privateMode||(mode&511n)===448n),'config_directory_mode');return pin.map(BigInt);}
function decodeConfiguration(raw){
 const d=decode(raw,16384);keys(d,['schema','guestConfigDirectory','guestParentPin','hostOutputDirectory','hostOutputPin','identity','config']);requireValue(d.schema==='lexlean-r4/invocation-connection/1','config_schema');
 absolute(d.guestConfigDirectory);absolute(d.hostOutputDirectory);directoryPin(d.guestParentPin,true);directoryPin(d.hostOutputPin,true);
 const c=d.config;keys(c,['schema','nonce','context','output_directory']);requireValue(c.schema==='lexlean-r4/observer-config/1','guest_schema');hex(c.nonce);
 const x=c.context;requireValue(x?.docker?.[0]===ROOT_CID,'root_container');keys(x,['docker','boot','init_start','cgroup','proc','boot_directory','target']);
 requireValue(Array.isArray(x.docker)&&x.docker.length===4,'docker');hex(x.docker[0]);requireValue(typeof x.docker[1]==='string'&&x.docker[1].startsWith('sha256:'),'image');hex(x.docker[1].slice(7));
 requireValue(typeof x.docker[2]==='string'&&x.docker[2].length<=64&&/^[\x20-\x7e]+T.*Z$/.test(x.docker[2])&&Number.isFinite(Date.parse(x.docker[2])),'started');requireValue(decimal(x.docker[3])>0n,'init_pid');decimal(x.init_start);
 requireValue(typeof x.boot==='string'&&/^[a-f0-9]{8}-(?:[a-f0-9]{4}-){3}[a-f0-9]{12}$/.test(x.boot),'boot');absolute(x.target);
 const pair=(p,owned=false)=>{requireValue(Array.isArray(p)&&p.length===2,'directory_pair');return [absolute(p[0]),directoryPin(p[1],owned)];};
 const cg=pair(x.cgroup),proc=pair(x.proc),boot=pair(x.boot_directory);pair(c.output_directory,true);
 const id=d.identity;requireValue(Array.isArray(id)&&id.length===6&&same(id.slice(0,4),[...x.docker.slice(0,3),x.boot])&&same(id[4],[x.target,cg[1][0],cg[1][1],cg[1][3]])&&Array.isArray(id[5])&&id[5].length===3,'observed_identity');
 ownerIdentity(id[5].slice(0,2));absolute(id[5][2]);requireValue(id[5][2]!==x.target&&!id[5][2].startsWith(x.target.replace(/\/$/,'')+'/'),'observer_placement');
 return {directory:d.hostOutputDirectory,pin:d.hostOutputPin,nonce:c.nonce,identity:id,contextDigest:hash(encoded([x.docker,x.boot,x.init_start,cg,proc,boot,x.target])),arguments:['--config-directory',d.guestConfigDirectory,'--parent-pin',...d.guestParentPin]};
}

const OWNER_BOOTSTRAP="ROOT = '/Users/casey.allard/lexlean-r4-worktrees/model-port/tests/fixtures/object-read-memory-observation'\nMODULES = (('observer_entry', 6189, 'ab6f4c8ef96e297dce0b8a3acd24ddae6d3833cfc2ef393347d583e5340b5d1e'), ('observer_session', 8169, 'eb84e076c2809003406bdc9665aa12a7d6482c335b4c67914f3e9b0736b669c5'), ('live_context', 7977, 'df74150b428a0ad72f3ec4ac1a3638dacd786fc1e4220adc9c69bcfd77508a55'), ('boundary', 26535, 'b457b8e290158a45766ddec34a2518e8215a1c9fed35e31b1353bd7e5b76dddf'), ('family', 4288, '7a5f1135d1cea44c1fb02dd721a8eff2422708442fd69c272c1e4aa685a072b0'), ('observe', 32926, '341fdc5f4def127af11954b2cbfb0251932923b06eb6e227fc9a132553eb0e13'), ('cadence', 4264, 'aefa62d3c242f53013299ab362b2895c9c9e7ed6fc2e72c09307952542823c46'), ('control_slots', 5571, 'dfa6ef3ea8c12c60c5b37e48bb944f819bfa387afe2302c8b8ca2cf3cac1d79a'))\nimport sys, os, stat, io, json, base64, errno, datetime, hashlib, uuid, time\nimport dataclasses, typing, importlib.util\ndef stable(item):\n    return (item.st_dev, item.st_ino, item.st_uid, item.st_mode, item.st_nlink,\n            item.st_size, item.st_mtime_ns, item.st_ctime_ns)\ndef root_fd():\n    fd = os.open('/', os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)\n    try:\n        for part in ROOT.split('/')[1:]:\n            new = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,\n                          dir_fd=fd)\n            os.close(fd); fd = new\n        info = os.fstat(fd)\n        if info.st_uid != os.getuid() or info.st_mode & 0o022:\n            raise ValueError('source custody')\n        return fd\n    except BaseException:\n        os.close(fd)\n        raise\ndef verify(fd):\n    for name, length, digest in MODULES:\n        path = ROOT+'/'+name+'.py'\n        for cache in (importlib.util.cache_from_source(path), ROOT+'/'+name+'.pyc'):\n            if os.path.lexists(cache): raise ValueError('source cache')\n        src = os.open(name+'.py', os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC,\n                      dir_fd=fd)\n        try:\n            before = os.fstat(src)\n            if (not stat.S_ISREG(before.st_mode) or before.st_uid != os.getuid()\n                    or before.st_mode & 0o022 or before.st_nlink != 1 or before.st_size != length):\n                raise ValueError('source identity')\n            raw = os.read(src, length + 1)\n            if (len(raw) != length or hashlib.sha256(raw).hexdigest() != digest\n                    or stable(os.fstat(src)) != stable(before)\n                    or stable(os.stat(name+'.py', dir_fd=fd, follow_symlinks=False)) != stable(before)):\n                raise ValueError('source bytes')\n        finally:\n            os.close(src)\n\nimport select, signal, threading\nMODULES = MODULES + (('observer_owner', 15897, 'a0e6d5cd2c45600f034aea083dbeaec7361519708b68d4debb99cd31c2e725bc'),)\ntry:\n    source = root_fd()\n    try:\n        initial = stable(os.fstat(source))\n        verify(source)\n        sys.path.insert(0, ROOT)\n        import observer_owner\n        names = {row[0] for row in MODULES}\n        for name in names:\n            if getattr(sys.modules.get(name), '__file__', None) != ROOT+'/'+name+'.py':\n                raise ValueError('module path')\n        for name, module in tuple(sys.modules.items()):\n            path = getattr(module, '__file__', None)\n            if isinstance(path, str) and path.startswith(ROOT+'/') and name not in names:\n                raise ValueError('extra dependency')\n        verify(source)\n        again = root_fd()\n        try:\n            if stable(os.fstat(again)) != initial: raise ValueError('source replacement')\n        finally:\n            os.close(again)\n    finally:\n        os.close(source)\nexcept Exception:\n    sys.stderr.write('observer-owner: bootstrap\\n')\n    sys.exit(74)\nsys.exit(observer_owner.cli(sys.argv[1:]))\n";
const CONFIG_DIRECTORY='/Users/casey.allard/lexlean-r4-worktrees/model-port/tests/fixtures/object-read-memory-observation/invocation-control';
function loadConfiguration(){
 const s=fs.lstatSync(CONFIG_DIRECTORY,{bigint:true});
 const reader=new HostConnection({directory:CONFIG_DIRECTORY,pin:[s.dev,s.ino,s.uid,s.mode].map(String),nonce:'0'.repeat(64),identity:[]});
 try{return decodeConfiguration(reader.read('connection.json',16384).raw);}finally{reader.closeFiles();}
}
export async function openConnection(){
 const config=loadConfiguration(),connection=new HostConnection(config);
 try{
  const child=spawn('colima',['ssh','--profile','lexlean-r4','--','/usr/bin/python3.12','-I','-B','-c',OWNER_BOOTSTRAP,...config.arguments],{stdio:['pipe','ignore','ignore']});
  connection.ownTransport(child,config.contextDigest);connection.ready();return connection;
 }catch(primary){
  try{if(connection.child)await connection.close();else connection.closeFiles();}catch(cleanup){throw new AggregateError([primary,cleanup],'connection_setup_cleanup');}
  throw primary;
 }
}
