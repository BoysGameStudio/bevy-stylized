// Local disposable-browser test driver; never attaches to a user profile.
import {spawn} from 'node:child_process';
import * as fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
const [chrome, url, output, flagsJson] = process.argv.slice(2);
if (!chrome || !url || !output) throw Error('expected chrome URL output flags-json');
const flags = JSON.parse(flagsJson || '[]');
const file = name => path.join(output, name);
const profile=await fs.mkdtemp(path.join(os.tmpdir(), 'stylized-browser-capture-'));
const log=await fs.open(file('browser-runtime.log'),'w');
const browser=spawn(chrome,[...flags,'--disable-background-networking','--enable-logging=stderr','--no-first-run','--no-default-browser-check',`--user-data-dir=${profile}`,'--remote-debugging-port=0',url],{stdio:['ignore',log.fd,log.fd]});
await fs.writeFile(file('browser-process.json'),JSON.stringify({pid:browser.pid,profile}));
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
const exited=new Promise(r=>browser.once('exit',r));
try {
 let port;
 for(let i=0;i<200;i++){try{port=(await fs.readFile(profile+'/DevToolsActivePort','utf8')).split('\n')[0];break;}catch{await sleep(100);}}
 if(!port)throw Error('browser endpoint absent');
 const version=await(await fetch(`http://127.0.0.1:${port}/json/version`)).json();
 const tabs=await(await fetch(`http://127.0.0.1:${port}/json/list`)).json();
 const ws=new WebSocket(tabs.find(x=>x.type==='page').webSocketDebuggerUrl);
 await new Promise((r,j)=>{ws.onopen=r;ws.onerror=j;});
 let id=0;const pending=new Map();const errors=[];
 ws.onmessage=e=>{const m=JSON.parse(e.data);if(m.id){const p=pending.get(m.id);if(p){pending.delete(m.id);clearTimeout(p.timer);m.error?p.reject(m.error):p.resolve(m.result);}}else if(m.method==='Runtime.exceptionThrown'){errors.push(m.params);}};
 const call=(method,params={})=>new Promise((resolve,reject)=>{const n=++id;const timer=setTimeout(()=>{pending.delete(n);reject(Error(method+' timed out'));},45000);pending.set(n,{resolve,reject,timer});ws.send(JSON.stringify({id:n,method,params}));});
 await call('Runtime.enable');await call('Page.enable');
 let result;
 for(let i=0;i<180;i++){
  const response=await call('Runtime.evaluate',{expression:'(()=>{const r=globalThis.stylizedResult;if(r){const {rgba,...meta}=r;return meta;}return globalThis.stylizedErrors?.length?{status:"FAIL",errors:globalThis.stylizedErrors}:null;})()',returnByValue:true});
  if(response.exceptionDetails)throw Error(JSON.stringify(response.exceptionDetails));result=response.result.value;if(result)break;if(i%20===0){const progress=await call('Runtime.evaluate',{expression:'globalThis.stylizedProgress || null',returnByValue:true});console.log('PROGRESS',JSON.stringify(progress.result.value));}await sleep(500);
 }
 if(!result){const progress=await call('Runtime.evaluate',{expression:'globalThis.stylizedProgress',returnByValue:true});throw Error('browser render readiness timeout; '+JSON.stringify({errors,progress}));}
 if(result.status==='PASS'){
  result.rgba=[];const length=result.width*result.height*4;if(length>16777216)throw Error('unexpected capture size');
  for(let offset=0;offset<length;offset+=16384){const part=await call('Runtime.evaluate',{expression:`globalThis.stylizedResult.rgba.slice(${offset},${offset+16384})`,returnByValue:true});result.rgba.push(...part.result.value);}
 }
 const late=await call('Runtime.evaluate',{expression:'globalThis.stylizedErrors || []',returnByValue:true});
 errors.push(...(late.result.value || []));
 const adapter=await call('Runtime.evaluate',{expression:'navigator.gpu.requestAdapter().then(a=>a?{vendor:a.info.vendor,architecture:a.info.architecture,device:a.info.device,description:a.info.description}:null)',awaitPromise:true,returnByValue:true});
 result.adapter=adapter.result.value;
 if(errors.length || result.status !== 'PASS') process.exitCode=1;
 const page=await call('Page.captureScreenshot',{format:'png'});
 await fs.writeFile(file('browser-window.png'),Buffer.from(page.data,'base64'));
 await fs.writeFile(file('browser-capture.json'),JSON.stringify({browser:version.Browser,result,errors}));
 console.log(JSON.stringify({browser:version.Browser,status:result.status,backend:result.backend,width:result.width,height:result.height,adopted:result.adopted_primitives,errors:result.errors?.map(x=>({kind:x.kind,messages:x.messages,message:x.message}))}));ws.close();for(const p of pending.values())clearTimeout(p.timer);
}finally{browser.kill('SIGTERM');await Promise.race([exited,sleep(3000)]);if(browser.exitCode===null && browser.signalCode===null){browser.kill('SIGKILL');await exited;}await fs.rm(profile,{recursive:true,force:true,maxRetries:10,retryDelay:100});await log.close();}
