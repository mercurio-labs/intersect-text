// Host invocation preference and presentation. Rust owns catalog/parse semantics.
export function validateEntryRule(value){
 if(value!==null&&(typeof value!=='string'||value.length===0))throw new Error('Invalid parser entry selection.');
 return value;
}
export function createEntrySelection({host,results,choose}){
 host.innerHTML='<label>Parser entry <select id="parser-entry" aria-label="Parser entry"></select></label><span id="parser-entry-status" role="status">Compile a language to choose its parser entry.</span>';
 const picker=host.querySelector('select'),status=host.querySelector('[role="status"]');
 let desired=null,catalog=null,enabled=false,busy=false,serial=0,model=null,failure='';
 function render(){
  picker.replaceChildren();const option=(value,text)=>{const item=document.createElement('option');item.value=value;item.textContent=text;picker.append(item);};
  option('',catalog?`Resource default (${catalog.default_entry})`:'Resource default');
  for(const [name,rule]of Object.entries(catalog?.entries||{}))option(name,`${name}${rule.datatype?' · datatype':''}`);
  if(desired!==null&&(!catalog||!Object.hasOwn(catalog.entries,desired)))option(desired,`${desired} · requested, unavailable in active language`);
  picker.value=desired??'';picker.disabled=!enabled||busy||!catalog;
  const active=model?.entry_rule??null;
  status.textContent=failure||(!catalog?'Compile a language to choose its parser entry.':`Active: ${active??'resource default'}${active===desired?'':` · Requested: ${desired??'resource default'}`}`);
 }
 picker.onchange=async()=>{
  desired=validateEntryRule(picker.value===''?null:picker.value);failure='';busy=true;const ticket=++serial;render();
  try{await choose(desired);}catch(error){if(ticket===serial)failure=error.message;}
  finally{if(ticket===serial){busy=false;render();}}
 };
 return{
  rule:()=>desired,
  restore(value){desired=validateEntryRule(value);serial++;busy=false;failure='';render();},
  reset(){catalog=null;model=null;serial++;busy=false;failure='';results.replaceChildren();results.hidden=true;render();},
  catalog(value){catalog=value;failure='';render();},
  unavailable(message){catalog=null;failure=message;render();},
  enabled(value){enabled=value;render();},
  model(value){
   model=value;results.replaceChildren();results.hidden=!value.entry_values;
   if(value.entry_values){
    const title=document.createElement('h3');title.textContent='Entry results';results.append(title);
    for(const [uri,returned]of Object.entries(value.entry_values)){
     const item=document.createElement('section'),label=document.createElement('strong'),body=document.createElement('pre');label.textContent=uri;
     body.textContent=returned.kind==='datatype'?`Raw datatype text\n${returned.value}`:returned.kind==='empty'?'No object or datatype value':returned.kind==='object'?'Object result · inspect it in the model tree':`Unsupported entry result kind: ${returned.kind}`;item.append(label,body);results.append(item);
    }
   }
   render();
  },
 };
}
