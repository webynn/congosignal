// AI decisions are data, never executable code or arbitrary tool URLs.
export function supportedAction(action){
  if(!action||!['none','navigate','search','collect','analyze'].includes(action.type))return false;
  if(action.type==='navigate'&&!['briefing','situation','reports','minerals','flights','radio','actors','agreements','sources','social','costs'].includes(action.target))return false;
  if(['navigate','search'].includes(action.type)&&(typeof action.query!=='string'||action.query.length>160))return false;
  if(action.type==='search'&&!action.query.trim())return false;
  return true;
}
export function createVoiceCenter({api,L,execute,currentView}) {
  let panel,previousFocus,stream,recorder,ws,timer,closeTimer,connecting=false,running=false,epoch=0,finalText='',reply='',history=[],pending=null,ttsSocket=null,ttsTimer,audio=null,audioUrl=null;
  const $=s=>panel.querySelector(s);
  function message(t){if(panel)$('[data-status]').textContent=t;}
  function release(){clearTimeout(timer);clearTimeout(closeTimer);if(recorder?.state==='recording'){recorder.onstop=null;recorder.stop();}recorder=null;stream?.getTracks().forEach(t=>t.stop());stream=null;if(ws){ws.onclose=null;ws.close();ws=null;}connecting=false;if(panel){$('[data-mic]').disabled=false;$('[data-stop]').disabled=true;$('textarea').readOnly=false;}}
  function cancel(){epoch++;release();stopSpeaking();}
  function close(){cancel();panel?.remove();panel=null;previousFocus?.focus();}
  function stopping(){if(!recorder)return;message(L('Finishing transcription…','Finalisation de la transcription…'));if(recorder.state==='recording')recorder.stop();stream?.getTracks().forEach(t=>t.stop());clearTimeout(timer);closeTimer=setTimeout(()=>{release();message(L('Transcription timed out. Review the text before running it.','Transcription expirée. Vérifiez le texte avant exécution.'));},8000);}
  async function record(){
    if(connecting||stream||running)return;
    connecting=true;const session=++epoch;
    stopSpeaking();$('[data-mic]').disabled=true;message(L('Requesting microphone…','Accès au microphone…'));
    try{
      if(!navigator.mediaDevices?.getUserMedia||!window.MediaRecorder)throw Error(L('Microphone recording requires HTTPS or localhost and a supported browser.','Le microphone nécessite HTTPS ou localhost et un navigateur compatible.'));
      const config=await api('/v1/voice/status');
      if(session!==epoch)return;
      if(!config.configured)throw Error(L('Soniox key is not configured.','Clé Soniox non configurée.'));
      if(config.paused)throw Error(L('Voice spending is paused. Set Soniox limits in Costs & budgets. Text questions use the AI budget.','Budget vocal en pause. Configurez Soniox dans Coûts et budgets. Les questions écrites utilisent le budget IA.'));
      const acquired=await navigator.mediaDevices.getUserMedia({audio:{echoCancellation:true,noiseSuppression:true},video:false});
      if(session!==epoch){acquired.getTracks().forEach(t=>t.stop());return;}stream=acquired;
      message(L('Connecting to Soniox…','Connexion à Soniox…'));
      const credential=await api('/v1/voice/token',{});if(session!==epoch)return;
      const socket=new WebSocket('wss://stt-rt.soniox.com/transcribe-websocket',['soniox-api-key',credential.api_key]);ws=socket;
      timer=setTimeout(()=>{if(session===epoch){release();message(L('Connection timed out. Try again.','Connexion expirée. Réessayez.'));}},15000);
      finalText='';$('textarea').value='';$('textarea').readOnly=true;
      socket.onopen=()=>{
        if(session!==epoch)return;
        clearTimeout(timer);
        socket.send(JSON.stringify({model:credential.model,audio_format:'auto',language_hints:['en','fr'],enable_endpoint_detection:true}));
        const mime=['audio/webm;codecs=opus','audio/webm','audio/mp4'].find(v=>MediaRecorder.isTypeSupported(v));
        if(!mime){release();message(L('No supported recording format. Use Chrome.','Aucun format compatible. Utilisez Chrome.'));return;}
        try{
          recorder=new MediaRecorder(stream,{mimeType:mime});
          recorder.ondataavailable=e=>{if(e.data.size&&socket.readyState===WebSocket.OPEN)socket.send(e.data);};
          recorder.onstop=()=>{if(socket.readyState===WebSocket.OPEN)socket.send('');};
          recorder.onerror=()=>{release();message(L('Microphone recording failed.','Échec de l’enregistrement.'));};
          recorder.start(250);connecting=false;$('[data-stop]').disabled=false;message(L('Listening · maximum 60 seconds. Press Stop when done.','Écoute · 60 secondes maximum. Appuyez sur Arrêter.'));timer=setTimeout(stopping,55000);
        }catch(e){release();message(e.message);}
      };
      socket.onmessage=e=>{
        if(session!==epoch||!panel)return;
        try{
          const data=JSON.parse(e.data);
          if(data.error_code){release();message(L('Soniox ended the session. Review your transcript or retry.','Soniox a interrompu la session. Vérifiez le texte ou réessayez.'));return;}
          let partial='';for(const t of data.tokens||[]){const value=String(t.text||'').replace(/<[^>]*>/g,'');if(t.is_final)finalText+=value;else partial+=value;}
          $('textarea').value=(finalText+partial).slice(0,1000);
          if((data.tokens||[]).some(t=>t.text==='<end>')&&recorder?.state==='recording')stopping();
          if(data.finished){release();message(L('Transcript ready.','Transcription prête.'));if($('[data-auto-send]').checked&&$('textarea').value.trim())run();}
        }catch{release();message(L('Invalid transcription response.','Réponse de transcription invalide.'));}
      };
      socket.onerror=()=>{if(session===epoch){release();message(L('Voice connection failed. Check your Soniox settings.','Connexion vocale échouée. Vérifiez Soniox.'));}};
      socket.onclose=()=>{if(session===epoch){release();message(L('Session ended. Review the transcript before running.','Session terminée. Vérifiez le texte avant exécution.'));}};
    }catch(e){if(session===epoch){release();message(e.message);}}
  }
  function showSources(citations){const list=$('[data-citations]');list.replaceChildren();for(const s of citations||[]){const li=document.createElement('li'),link=document.createElement('a');let u;if(typeof s.url!=='string'||!s.url)continue;try{u=new URL(s.url,location.origin);}catch{continue;}if(!['http:','https:'].includes(u.protocol))continue;link.href=u.href;link.target='_blank';link.rel='noopener noreferrer';link.textContent=typeof s.title==='string'?s.title:s.id;li.append(link);if(s.date)li.append(document.createTextNode(' · '+s.date));list.append(li);}}
  async function run(){
    if(running||connecting||stream)return;
    const question=$('textarea').value.trim();if(!question)return;
    stopSpeaking();pending=null;$('[data-confirm]').hidden=true;running=true;$('[data-run]').disabled=true;
    const session=epoch;message(L('Okapi is reading your workspace…','Okapi consulte vos données…'));
    try{
      const response=await api('/v1/voice/chat',{message:question,history:history.slice(0,6).reverse().map(h=>({user:h.text,assistant:h.result})),view:currentView()});
      if(!panel||session!==epoch)return;
      if(!supportedAction(response.action)||typeof response.answer!=='string')throw Error(L('Unsupported assistant response. No action taken.','Réponse non prise en charge. Aucune action exécutée.'));
      reply=response.answer;showSources(response.citations);
      if(['navigate','search'].includes(response.action?.type)){await execute(response.action);}
      if(['collect','analyze'].includes(response.action.type)){pending=response.action;$('[data-confirm]').hidden=false;$('[data-confirm]').textContent=response.action.type==='collect'?L('Launch collection','Lancer la collecte'):L('Queue up to five AI analyses','Lancer jusqu’à cinq analyses IA');}
      message(reply);history.unshift({text:question,result:reply});history=history.slice(0,8);const list=$('[data-history]');list.replaceChildren();for(const item of history){const li=document.createElement('li');li.textContent=item.text+' — '+item.result;list.append(li);}
      if($('[data-auto-speak]').checked)await speak();
    }catch(e){if(panel&&session===epoch)message(e.message);}
    finally{running=false;if(panel)$('[data-run]').disabled=false;}
  }
  function stopSpeaking(){clearTimeout(ttsTimer);ttsSocket?.close();ttsSocket=null;if(audio){audio.pause();audio=null;}if(audioUrl){URL.revokeObjectURL(audioUrl);audioUrl=null;}window.speechSynthesis?.cancel();}
  async function speak(){
    if(!reply||stream||connecting)return;stopSpeaking();const session=epoch;
    try{
      const credential=await api('/v1/voice/token',{usage:'tts'});if(!panel||session!==epoch)return;
      const socket=new WebSocket('wss://tts-rt.soniox.com/tts-websocket',['soniox-api-key',credential.api_key]);ttsSocket=socket;
      const chunks=[];let bytes=0,ended=false;
      ttsTimer=setTimeout(()=>{stopSpeaking();message(reply+'\n'+L('Speech timed out; the text answer is available.','Délai vocal dépassé ; la réponse écrite reste disponible.'));},60000);
      socket.onopen=()=>{socket.send(JSON.stringify({model:credential.model,voice:credential.voice,language:(document.documentElement.lang||'en').slice(0,2),audio_format:'mp3',stream_id:'okapi'}));socket.send(JSON.stringify({stream_id:'okapi',text:reply.slice(0,2400),text_end:true}));};
      socket.onmessage=async event=>{
        if(!panel||session!==epoch)return;
        try{
          const data=JSON.parse(event.data);if(data.error_code)throw Error(L('Soniox speech generation failed.','Échec de la synthèse Soniox.'));
          if(data.audio){const raw=atob(data.audio);bytes+=raw.length;if(bytes>6000000)throw Error('Audio response too large');chunks.push(Uint8Array.from(raw,c=>c.charCodeAt(0)));}
          if(data.audio_end)ended=true;
          if(data.terminated){clearTimeout(ttsTimer);socket.onclose=null;socket.close();ttsSocket=null;if(!ended||!chunks.length)throw Error('Incomplete audio response');audioUrl=URL.createObjectURL(new Blob(chunks,{type:'audio/mpeg'}));audio=new Audio(audioUrl);audio.onended=stopSpeaking;await audio.play();}
        }catch(e){stopSpeaking();message(reply+'\n'+e.message);}
      };
      socket.onerror=()=>{stopSpeaking();message(reply+'\n'+L('Speech connection failed.','Connexion vocale échouée.'));};
      socket.onclose=()=>{if(!ended){stopSpeaking();message(reply+'\n'+L('Speech interrupted. Text answer remains available.','Lecture interrompue. Réponse écrite disponible.'));}};
    }catch(e){if(panel&&session===epoch)message(reply+'\n'+L('Voice reply unavailable: ','Réponse vocale indisponible : ')+e.message);}
  }
  function open(prompt){
    if(panel){if(typeof prompt==='string')$('textarea').value=prompt;$('textarea').focus();return;}
    previousFocus=document.activeElement;panel=document.createElement('section');panel.className='voice-center';panel.setAttribute('role','dialog');panel.setAttribute('aria-label',L('Okapi assistant','Assistant Okapi'));
    panel.innerHTML=`<header><div><small>CONGO SIGNAL · OKAPI</small><h2>${L('Talk to Okapi','Parlez à Okapi')}</h2></div><button class="btn" data-close aria-label="${L('Close voice center','Fermer le centre vocal')}">×</button></header><p>${L('Ask about your news, social intelligence, mineral sites or agreements. Okapi answers with sources and can navigate for you.','Interrogez vos actualités, réseaux sociaux, sites miniers ou accords. Okapi répond avec ses sources et peut naviguer pour vous.')}</p><div class="actions"><button class="btn primary" data-mic>${L('Start microphone','Activer le micro')}</button><button class="btn" data-stop disabled>${L('Stop','Arrêter')}</button></div><p class="voice-status" data-status role="status" aria-live="polite">${L('Ready · microphone off','Prêt · microphone désactivé')}</p><textarea class="input" rows="3" maxlength="1000" aria-label="${L('Voice command','Commande vocale')}" placeholder="${L('Open minerals','Ouvre les minerais')}"></textarea><div class="actions"><button class="btn primary" data-run>${L('Ask Okapi','Demander à Okapi')}</button><button class="btn" data-speak>${L('Speak with Soniox','Lire avec Soniox')}</button><button class="btn" data-silence>${L('Silence','Silence')}</button></div><div class="voice-options"><label><input type="checkbox" data-auto-send checked> ${L('Send when I finish speaking','Envoyer à la fin de ma phrase')}</label><label><input type="checkbox" data-auto-speak> ${L('Speak answers with Soniox','Lire les réponses avec Soniox')}</label></div><button class="btn primary" data-confirm hidden></button><ul data-citations></ul><details open><summary>${L('Try asking','Exemples de questions')}</summary><ul><li>${L('Can you show me the mineral sites?','Peux-tu me montrer les sites miniers ?')}</li><li>${L('What is happening across social intelligence?','Que se passe-t-il sur les réseaux sociaux ?')}</li><li>${L('What are the main news stories, and what sources support them?','Quelles sont les principales actualités et leurs sources ?')}</li></ul></details><p class="muted">${L('Audio goes to Soniox only while recording. Nothing is recorded in the background. Conversation stays in page memory and is sent with retrieved evidence to the configured AI provider. Collection and analysis use existing workers, permissions and budgets.','L’audio est transmis à Soniox pendant l’enregistrement uniquement. Aucune écoute en arrière-plan. La conversation reste en mémoire et est transmise avec les sources au fournisseur IA configuré. Collecte et analyse respectent les permissions et budgets existants.')}</p><ol data-history></ol>`;
    document.body.append(panel);$('[data-close]').onclick=close;$('[data-mic]').onclick=record;$('[data-stop]').onclick=stopping;$('[data-run]').onclick=run;$('[data-speak]').onclick=speak;$('[data-silence]').onclick=stopSpeaking;$('[data-confirm]').onclick=async()=>{if(!pending||running)return;const task=pending;pending=null;running=true;$('[data-confirm]').hidden=true;try{message(await execute(task));}catch(e){message(e.message);}finally{running=false;}};
    panel.addEventListener('keydown',e=>{e.stopPropagation();if(e.key==='Escape')close();if((e.ctrlKey||e.metaKey)&&e.key==='Enter'){e.preventDefault();run();}});if(typeof prompt==='string')$('textarea').value=prompt;$('textarea').focus();
    api('/v1/voice/status').then(s=>{if(panel&&s.paused)message(L('Voice spending paused · set Soniox limits in Costs & budgets. Text questions use your AI budget.','Budget vocal en pause · configurez Soniox dans Coûts et budgets. Questions écrites soumises au budget IA.'));else if(panel&&!s.configured)message(L('Soniox key missing. Typed commands still work.','Clé Soniox absente. Les commandes écrites restent disponibles.'));}).catch(e=>message(e.message));
  }
  document.addEventListener('visibilitychange',()=>{if(document.hidden){cancel();message(L('Microphone stopped because this tab was hidden.','Microphone arrêté : onglet masqué.'));}});
  window.addEventListener('pagehide',cancel);
  return {open,close};
}
