// Verification-only prefix. The original served language worker follows verbatim.
(() => {
  const NativeWorker = self.Worker;
  const held = new Map(), latest = new Map(), counts = new Map(), probes = new Map();
  let configured = null, target = null, actor = null;
  const emit = (kind, params) => self.postMessage({jsonrpc:'2.0',method:'qualification/stringTrace',params:{kind,...params}});
  self.Worker = class extends NativeWorker {
    constructor(...args) {
      super(...args); actor = this;
      this.addEventListener('message', event => {
        const data = event.data, details = latest.get(data.token), request = details?.request;
        emit('response',{data,request,context:details?.context,resumes:counts.get(data.token)||0});
        if (probes.has(data.token)) {
          const id = probes.get(data.token); probes.delete(data.token);
          self.postMessage({jsonrpc:'2.0',id,result:data});
        }
        const configuredCount = configured?.counts?.[details?.context];
        if (configured?.enabled && request?.method==='rcore_behavior/resumeFunction' && data.response?.pending
            && counts.get(data.token)===configuredCount && !held.has(data.token)
            && (configured.all || target===data.token)) {
          event.stopImmediatePropagation();
          held.set(data.token,{worker:this,event});
          emit('held',{token:data.token,job:data.response.job,context:details.context,resumes:counts.get(data.token)});
        }
      });
    }
    postMessage(data,...rest) {
      if (data?.request) {
        const previous=latest.get(data.token), request=data.request;
        const context=request.arguments?.[0]?.data?.Model?.context ?? previous?.context;
        latest.set(data.token,{request,context});
        if (request.method==='rcore_behavior/startFunction' && configured?.enabled && !configured.all && target===null
            && Object.hasOwn(configured.counts,context)) target=data.token;
        if (request.method==='rcore_behavior/resumeFunction') counts.set(data.token,(counts.get(data.token)||0)+1);
        emit('request',{token:data.token,request,context,resumes:counts.get(data.token)||0});
      }
      return super.postMessage(data,...rest);
    }
  };
  self.addEventListener('message',event=>{
    const data=event.data;
    if(['qualification/configureString','qualification/releaseString','qualification/probeStringHost'].includes(data.method)) event.stopImmediatePropagation();
    if (data.method==='qualification/configureString') {
      configured=data.params;target=null;
      self.postMessage({jsonrpc:'2.0',id:data.id,result:{configured:true}});
    } else if (data.method==='qualification/releaseString') {
      for (const [token,record] of [...held]) if(data.params?.token===undefined || data.params.token===token) {
        held.delete(token);record.worker.onmessage(record.event);
      }
      self.postMessage({jsonrpc:'2.0',id:data.id,result:{released:true}});
    } else if (data.method==='qualification/probeStringHost') {
      if (!actor) throw new Error('No actual execution worker observed');
      const token=`qualification-probe:${data.id}`;probes.set(token,data.id);
      actor.postMessage({token,request:data.params});
    }
  });
})();
