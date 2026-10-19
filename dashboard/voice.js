// AI decisions are data, never executable code or arbitrary tool URLs.
export function supportedAction(action){
  if(!action||!['none','navigate','search','collect','analyze'].includes(action.type))return false;
  if(action.type==='navigate'&&!['briefing','situation','reports','minerals','flights','radio','actors','agreements','sources','social','costs'].includes(action.target))return false;
  if(['navigate','search'].includes(action.type)&&(typeof action.query!=='string'||action.query.length>160))return false;
  if(action.type==='search'&&!action.query.trim())return false;
  return true;
}
// Strip display-only markup; retain the words and numbers of the answer.
export function speechText(text){
  return String(text||'').replace(/\[(?:report|evidence|agreement|actor|social|mineral|source)[^\]]*\]/gi,'')
    .replace(/\[([^\]]+)\]\(https?:[^)]+\)/g,'$1')
    .replace(/https?:\/\/\S+/g,'').replace(/[*#`]+/g,'')
    .replace(/\s+/g,' ').trim().slice(0,2400);
}
export function pcm16Samples(bytes){
  if(bytes.length%2)throw Error('Incomplete PCM sample');
  const samples=new Float32Array(bytes.length/2);
  const view=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);
  for(let i=0;i<samples.length;i++)samples[i]=view.getInt16(i*2,true)/32768;
  return samples;
}
export function createVoiceCenter({api,L,execute,currentView}) {
  let panel,previousFocus,stream,recorder,ws,timer,closeTimer,connecting=false,running=false,epoch=0,finalText='',reply='',history=[],pending=null,ttsSocket=null,ttsTimer,audio=null,audioUrl=null,audioContext=null,speechEpoch=0,playhead=0,speechSources=new Set();
  // Voice mode: listen → answer → speak → listen again, until ended, closed or an error occurs.
  let voiceMode=false,resumeTimer;
  // Language and voice preferences (per browser). lang 'auto' detects FR/EN/SW from speech.
  const PREFS_KEY='okapi-voice-prefs';
  let prefs=(()=>{try{return {lang:'auto',voice:'',...JSON.parse(localStorage.getItem(PREFS_KEY)||'{}')}}catch{return {lang:'auto',voice:''}}})();
  let catalog=null,detectedLang='';
  const savePrefs=()=>{try{localStorage.setItem(PREFS_KEY,JSON.stringify(prefs))}catch{}};
  const pageLang=()=>(document.documentElement.lang||'en').slice(0,2);
  const langName=code=>{try{return new Intl.DisplayNames([pageLang()],{type:'language'}).of(code)||code}catch{return code}};
  /// Language for Okapi's reply and its spoken voice: chosen, else detected from speech, else the page language.
  const replyLang=()=>prefs.lang!=='auto'?prefs.lang:(detectedLang||pageLang());
  const ACCENT_FOR={fr:'french',sw:'african',es:'spanish',pt:'portuguese',ar:'arabic',de:'german',it:'italian',ja:'japanese',ko:'korean',zh:'chinese',hi:'indian',bn:'indian',ta:'indian',te:'indian',mr:'indian',gu:'indian',kn:'indian',ml:'indian',pa:'indian',ur:'indian',ru:'slavic',uk:'slavic',pl:'slavic',cs:'slavic',sk:'slavic',bg:'slavic',hr:'slavic',sr:'slavic',sl:'slavic',bs:'slavic',mk:'slavic',be:'slavic',id:'southeast_asian',ms:'southeast_asian',th:'southeast_asian',vi:'southeast_asian',tl:'southeast_asian'};
  /// Chosen voice, or for "Automatic" the first voice whose accent suits the language (Adrian otherwise).
  function voiceFor(lang){
    const voices=catalog?.voices||[];
    if(prefs.voice&&(!voices.length||voices.some(v=>v.id===prefs.voice)))return prefs.voice;
    const accent=ACCENT_FOR[lang];
    return (accent&&voices.find(v=>v.accent===accent)?.id)||(voices.some(v=>v.id==='Adrian')||!voices.length?'Adrian':voices[0].id);
  }
  const SAMPLES={en:'Hello, I am Okapi. I will read your intelligence briefings in this voice.',fr:'Bonjour, je suis Okapi. Je lirai vos synthèses de renseignement avec cette voix.',sw:'Habari, mimi ni Okapi. Nitakusomea taarifa zako za kiintelijensia kwa sauti hii.',es:'Hola, soy Okapi. Leeré tus informes de inteligencia con esta voz.',pt:'Olá, eu sou o Okapi. Vou ler os seus relatórios com esta voz.',de:'Hallo, ich bin Okapi. Ich lese Ihre Lageberichte mit dieser Stimme vor.',it:'Ciao, sono Okapi. Leggerò i tuoi rapporti con questa voce.',ar:'مرحبا، أنا أوكابي. سأقرأ تقاريرك بهذا الصوت.'};
  function renderPrefs(){
    if(!panel)return;
    const ls=$('[data-pref-lang]'),vs=$('[data-pref-voice]'),note=$('[data-pref-note]');
    const langs=(catalog?.languages||[{code:'en'},{code:'fr'},{code:'sw'}]).map(l=>l.code).filter(Boolean);
    const first=['fr','en','sw'].filter(c=>langs.includes(c)),rest=langs.filter(c=>!first.includes(c)).sort((a,b)=>langName(a).localeCompare(langName(b)));
    const opt=(value,label,sel)=>{const o=document.createElement('option');o.value=value;o.textContent=label;o.selected=sel;return o};
    ls.replaceChildren(opt('auto',L('Auto · detect French, English, Swahili','Auto · détecte français, anglais, swahili'),prefs.lang==='auto'),...first.map(c=>opt(c,langName(c),prefs.lang===c)));
    const more=document.createElement('optgroup');more.label=L('All languages','Toutes les langues');more.append(...rest.map(c=>opt(c,langName(c),prefs.lang===c)));ls.append(more);
    const lang=replyLang(),auto=voiceFor(lang),groups=new Map();
    for(const v of catalog?.voices||[]){const k=v.accent||'other';groups.set(k,[...(groups.get(k)||[]),v]);}
    const preferred=ACCENT_FOR[lang];
    vs.replaceChildren(opt('',L(`Automatic · ${auto}`,`Automatique · ${auto}`),!prefs.voice));
    for(const [accent,list] of [...groups].sort((a,b)=>(b[0]===preferred)-(a[0]===preferred)||a[0].localeCompare(b[0]))){
      const g=document.createElement('optgroup');g.label=accent.replace(/_/g,' ').replace(/^./,c=>c.toUpperCase())+(accent===preferred?' ★':'');
      g.append(...list.map(v=>opt(v.id,`${v.id} · ${v.gender||''}${v.style?.length?' · '+v.style.slice(0,2).join(', '):''}`,prefs.voice===v.id)));vs.append(g);
    }
    const chosen=(catalog?.voices||[]).find(v=>v.id===voiceFor(lang));
    note.textContent=[chosen?.description,catalog?.source==='built-in list'?L('Voice catalogue unavailable; showing the built-in list.','Catalogue indisponible ; liste intégrée affichée.'):'',['ln','kg','lua'].includes(pageLang())?'':L('Lingala, Kikongo and Tshiluba are not supported by Soniox yet.','Le lingala, le kikongo et le tshiluba ne sont pas encore pris en charge par Soniox.')].filter(Boolean).join(' ');
  }
  const $=s=>panel.querySelector(s);
  function speechStatus(t){if(panel)$('[data-speech-status]').textContent=t;}
  function unlockAudio(){
    try {
      const Context=window.AudioContext||window.webkitAudioContext;
      if(!Context)throw Error(L('Audio playback is not supported in this browser.','Lecture audio non prise en charge par ce navigateur.'));
      if(!audioContext||audioContext.state==='closed')audioContext=new Context();
      // Called synchronously from user clicks, before requests/transcription.
      if(audioContext.state==='suspended')audioContext.resume().catch(()=>speechStatus(L('Tap Replay reply to enable sound.','Appuyez sur Relire la réponse pour activer le son.')));
    } catch(e){speechStatus(e.message)}
  }
  function message(t){if(panel)$('[data-status]').textContent=t;}
  function modeState(label){if(!panel)return;const box=$('[data-voice-mode]');box.hidden=!voiceMode;if(voiceMode){box.dataset.state=label;$('[data-vm-label]').textContent={listening:L('Listening','À l’écoute'),thinking:L('Thinking','Réflexion'),speaking:L('Speaking','Réponse'),connecting:L('Connecting','Connexion')}[label]||label;}}
  function endVoiceMode(note){voiceMode=false;clearTimeout(resumeTimer);modeState('');if(note)message(note);}
  function failVoiceMode(text){release();message(text);if(voiceMode)endVoiceMode();}
  /// Next turn of voice mode: listen again once nothing is recording, thinking or speaking.
  function resume(){clearTimeout(resumeTimer);if(!voiceMode||!panel)return;resumeTimer=setTimeout(()=>{if(voiceMode&&panel&&!stream&&!connecting&&!running&&!ttsSocket&&!speechSources.size)record();},450);}
  function listen(){open();if(!panel)return;voiceMode=true;modeState('connecting');record();}
  function release(){clearTimeout(timer);clearTimeout(closeTimer);if(recorder?.state==='recording'){recorder.onstop=null;recorder.stop();}recorder=null;stream?.getTracks().forEach(t=>t.stop());stream=null;if(ws){ws.onclose=null;ws.close();ws=null;}connecting=false;if(panel){$('[data-mic]').disabled=false;$('[data-stop]').disabled=true;$('textarea').readOnly=false;}}
  function cancel(){epoch++;release();stopSpeaking();endVoiceMode();}
  function close(){cancel();panel?.remove();panel=null;previousFocus?.focus();}
  function stopping(){if(!recorder)return;message(L('Finishing transcription…','Finalisation de la transcription…'));if(recorder.state==='recording')recorder.stop();stream?.getTracks().forEach(t=>t.stop());clearTimeout(timer);closeTimer=setTimeout(()=>{release();message(L('Transcription timed out. Review the text before running it.','Transcription expirée. Vérifiez le texte avant exécution.'));},8000);}
  async function record(){
    if(connecting||stream||running)return;
    unlockAudio();connecting=true;const session=++epoch;
    stopSpeaking();$('[data-mic]').disabled=true;modeState('connecting');message(L('Requesting microphone…','Accès au microphone…'));
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
        socket.send(JSON.stringify({model:credential.model,audio_format:'auto',enable_endpoint_detection:true,...(prefs.lang==='auto'?{language_hints:['fr','en','sw'],enable_language_identification:true}:{language_hints:[prefs.lang],language_hints_strict:true})}));
        const mime=['audio/webm;codecs=opus','audio/webm','audio/mp4'].find(v=>MediaRecorder.isTypeSupported(v));
        if(!mime){release();message(L('No supported recording format. Use Chrome.','Aucun format compatible. Utilisez Chrome.'));return;}
        try{
          recorder=new MediaRecorder(stream,{mimeType:mime});
          recorder.ondataavailable=e=>{if(e.data.size&&socket.readyState===WebSocket.OPEN)socket.send(e.data);};
          recorder.onstop=()=>{if(socket.readyState===WebSocket.OPEN)socket.send('');};
          recorder.onerror=()=>{release();message(L('Microphone recording failed.','Échec de l’enregistrement.'));};
          recorder.start(250);connecting=false;$('[data-stop]').disabled=false;modeState('listening');message(voiceMode?L('Listening · just speak; Okapi replies when you pause.','Écoute · parlez, Okapi répond dès que vous marquez une pause.'):L('Listening · maximum 60 seconds. Press Stop when done.','Écoute · 60 secondes maximum. Appuyez sur Arrêter.'));timer=setTimeout(stopping,voiceMode?30000:55000);
        }catch(e){release();message(e.message);}
      };
      socket.onmessage=e=>{
        if(session!==epoch||!panel)return;
        try{
          const data=JSON.parse(e.data);
          if(data.error_code){failVoiceMode(L('Soniox ended the session. Review your transcript or retry.','Soniox a interrompu la session. Vérifiez le texte ou réessayez.'));return;}
          let partial='';for(const t of data.tokens||[]){const value=String(t.text||'').replace(/<[^>]*>/g,'');if(t.is_final){finalText+=value;if(prefs.lang==='auto'&&typeof t.language==='string'&&/^[a-z]{2}$/.test(t.language)&&value.trim())detectedLang=t.language;}else partial+=value;}
          $('textarea').value=(finalText+partial).slice(0,1000);
          if((data.tokens||[]).some(t=>t.text==='<end>')&&recorder?.state==='recording')stopping();
          if(data.finished){release();message(L('Transcript ready.','Transcription prête.'));if((voiceMode||$('[data-auto-send]').checked)&&$('textarea').value.trim())run();else if(voiceMode)endVoiceMode(L('No speech heard · voice mode ended to save Soniox credit.','Aucune parole entendue · mode vocal arrêté pour économiser Soniox.'));}
        }catch{release();message(L('Invalid transcription response.','Réponse de transcription invalide.'));}
      };
      socket.onerror=()=>{if(session===epoch)failVoiceMode(L('Voice connection failed. Check your Soniox settings.','Connexion vocale échouée. Vérifiez Soniox.'));};
      socket.onclose=()=>{if(session===epoch)failVoiceMode(L('Session ended. Review the transcript before running.','Session terminée. Vérifiez le texte avant exécution.'));};
    }catch(e){if(session===epoch)failVoiceMode(e.message);}
  }
  function showSources(citations){const list=$('[data-citations]');list.replaceChildren();for(const s of citations||[]){const li=document.createElement('li'),link=document.createElement('a');let u;if(typeof s.url!=='string'||!s.url)continue;try{u=new URL(s.url,location.origin);}catch{continue;}if(!['http:','https:'].includes(u.protocol))continue;link.href=u.href;link.target='_blank';link.rel='noopener noreferrer';link.textContent=typeof s.title==='string'?s.title:s.id;li.append(link);if(s.date)li.append(document.createTextNode(' · '+s.date));list.append(li);}}
  async function run(){
    if(running||connecting||stream)return;
    const question=$('textarea').value.trim();if(!question)return;
    unlockAudio();stopSpeaking();pending=null;$('[data-confirm]').hidden=true;running=true;$('[data-run]').disabled=true;
    const session=epoch;modeState('thinking');message(L('Okapi is reading your workspace…','Okapi consulte vos données…'));
    try{
      const response=await api('/v1/voice/chat',{message:question,history:history.slice(0,6).reverse().map(h=>({user:h.text,assistant:h.result})),view:currentView(),language:prefs.lang!=='auto'?prefs.lang:detectedLang});
      if(!panel||session!==epoch)return;
      if(!supportedAction(response.action)||typeof response.answer!=='string')throw Error(L('Unsupported assistant response. No action taken.','Réponse non prise en charge. Aucune action exécutée.'));
      reply=response.answer;showSources(response.citations);
      if(['navigate','search'].includes(response.action?.type)){await execute(response.action);}
      if(['collect','analyze'].includes(response.action.type)){pending=response.action;$('[data-confirm]').hidden=false;$('[data-confirm]').textContent=response.action.type==='collect'?L('Launch collection','Lancer la collecte'):L('Queue up to five AI analyses','Lancer jusqu’à cinq analyses IA');}
      message(reply);history.unshift({text:question,result:reply});history=history.slice(0,8);const list=$('[data-history]');list.replaceChildren();for(const item of history){const li=document.createElement('li');li.textContent=item.text+' — '+item.result;list.append(li);}
      if(voiceMode||$('[data-auto-speak]').checked)await speak();
    }catch(e){if(panel&&session===epoch)message(e.message);}
    finally{running=false;if(panel)$('[data-run]').disabled=false;resume();}
  }
  /// TTS language: the reply language when the catalogue supports it, else English.
  const ttsLang=()=>{const l=replyLang();return !catalog||catalog.languages?.some(x=>x.code===l)?l:'en'};
  function stopSpeaking(){
    speechEpoch++;clearTimeout(ttsTimer);
    if(ttsSocket){ttsSocket.onclose=null;ttsSocket.onerror=null;ttsSocket.onmessage=null;ttsSocket.close();ttsSocket=null;}
    for(const source of speechSources){source.onended=null;try{source.stop()}catch{}source.disconnect();}
    speechSources.clear();playhead=0;
    if(audio){audio.pause();audio=null;}if(audioUrl){URL.revokeObjectURL(audioUrl);audioUrl=null;}
    speechStatus(L('Speech stopped','Lecture arrêtée'));
  }
  async function speak(sample){
    const source=sample?.text||reply;
    if(!source||stream||connecting)return;
    unlockAudio();stopSpeaking();const session=epoch,utterance=speechEpoch;
    const valid=()=>panel&&session===epoch&&utterance===speechEpoch;
    const fail=text=>{if(!valid())return;stopSpeaking();speechStatus(text);resume();};
    speechStatus(L('Preparing spoken reply…','Préparation de la réponse vocale…'));
    try{
      if(!audioContext||audioContext.state!=='running')throw Error(L('Sound is blocked. Click Replay reply to enable playback.','Son bloqué. Cliquez sur Relire la réponse pour autoriser la lecture.'));
      const text=speechText(source);if(!text)throw Error(L('No text to speak.','Aucun texte à lire.'));
      const credential=await api('/v1/voice/token',{usage:'tts'});if(!valid())return;
      const socket=new WebSocket('wss://tts-rt.soniox.com/tts-websocket',['soniox-api-key',credential.api_key]);ttsSocket=socket;
      let bytes=0,ended=false,terminated=false,carry=null;
      const completed=()=>{if(valid()&&terminated&&!speechSources.size){speechStatus(L('Reply spoken · ready for your next question','Réponse lue · prêt pour votre prochaine question'));resume();}};
      ttsTimer=setTimeout(()=>fail(L('Speech timed out. Click Replay reply to retry.','Délai vocal dépassé. Cliquez sur Relire la réponse.')),60000);
      socket.onopen=()=>{if(!valid())return;socket.send(JSON.stringify({model:credential.model,voice:voiceFor(sample?.language||ttsLang()),language:sample?.language||ttsLang(),audio_format:'pcm_s16le',sample_rate:24000,stream_id:'okapi'}));socket.send(JSON.stringify({stream_id:'okapi',text,text_end:true}));};
      socket.onmessage=event=>{
        if(!valid())return;
        try{
          const data=JSON.parse(event.data);
          if(data.error_code)throw Error(L('Soniox speech error: ','Erreur vocale Soniox : ')+String(data.error_type||data.error_code));
          if(data.audio){
            const raw=atob(data.audio);bytes+=raw.length;
            if(bytes>6000000)throw Error('Audio response too large');
            const chunk=new Uint8Array(raw.length+(carry===null?0:1));let offset=0;
            if(carry!==null){chunk[0]=carry;offset=1;}for(let i=0;i<raw.length;i++)chunk[i+offset]=raw.charCodeAt(i);
            carry=chunk.length%2?chunk[chunk.length-1]:null;
            const samples=pcm16Samples(chunk.subarray(0,chunk.length-chunk.length%2));
            if(samples.length){
              if(audioContext.state!=='running')throw Error(L('Playback suspended. Click Replay reply.','Lecture suspendue. Cliquez sur Relire la réponse.'));
              const buffer=audioContext.createBuffer(1,samples.length,24000);buffer.copyToChannel(samples,0);
              const source=audioContext.createBufferSource();source.buffer=buffer;source.connect(audioContext.destination);
              speechSources.add(source);source.onended=()=>{speechSources.delete(source);source.disconnect();completed();};
              playhead=Math.max(playhead,audioContext.currentTime+.06);source.start(playhead);playhead+=buffer.duration;
              speechStatus(L('Okapi is speaking…','Okapi vous répond…'));modeState('speaking');
            }
          }
          if(data.audio_end)ended=true;
          if(data.terminated){
            if(!ended||!bytes||carry!==null)throw Error(L('Incomplete voice reply. Try Replay reply.','Réponse vocale incomplète. Essayez Relire la réponse.'));
            terminated=true;clearTimeout(ttsTimer);socket.onclose=null;socket.close();ttsSocket=null;completed();
          }
        }catch(e){fail(e.message)}
      };
      socket.onerror=()=>fail(L('Speech connection failed. Click Replay reply to retry.','Connexion vocale échouée. Cliquez sur Relire la réponse.'));
      socket.onclose=()=>{if(!terminated)fail(L('Speech interrupted. Click Replay reply to retry.','Lecture interrompue. Cliquez sur Relire la réponse.'));};
    }catch(e){fail(L('Voice reply unavailable: ','Réponse vocale indisponible : ')+e.message)}
  }
  function open(prompt){
    unlockAudio();
    if(panel){if(typeof prompt==='string')$('textarea').value=prompt;$('textarea').focus();return;}
    previousFocus=document.activeElement;panel=document.createElement('section');panel.className='voice-center';panel.setAttribute('role','dialog');panel.setAttribute('aria-label',L('Okapi assistant','Assistant Okapi'));
    panel.innerHTML=`<header><div><small>CONGO SIGNAL · OKAPI</small><h2>${L('Talk to Okapi','Parlez à Okapi')}</h2></div><button class="btn" data-close aria-label="${L('Close voice center','Fermer le centre vocal')}">×</button></header><div class="voice-mode" data-voice-mode hidden><span class="vm-orb" aria-hidden="true"><i></i><i></i><i></i></span><div><small>${L('VOICE MODE','MODE VOCAL')}</small><b data-vm-label role="status" aria-live="polite"></b></div><button class="btn sm" data-voice-end>${L('End voice mode','Quitter le mode vocal')}</button></div><div class="voice-prefs"><label><span>${L('Language','Langue')}</span><select class="select" data-pref-lang></select></label><label><span>${L('Voice','Voix')}</span><select class="select" data-pref-voice></select></label><button class="btn sm" data-preview title="${L('Plays a short sample (one Soniox speech session)','Lit un court extrait (une session vocale Soniox)')}">▶ ${L('Preview','Écouter')}</button></div><p class="voice-prefs-note" data-pref-note></p><p>${L('Ask about your news, social intelligence, mineral sites or agreements. Okapi answers with sources and can navigate for you.','Interrogez vos actualités, réseaux sociaux, sites miniers ou accords. Okapi répond avec ses sources et peut naviguer pour vous.')}</p><div class="actions"><button class="btn primary" data-mic>${L('Start microphone','Activer le micro')}</button><button class="btn" data-stop disabled>${L('Stop','Arrêter')}</button></div><p class="voice-status" data-status role="status" aria-live="polite">${L('Ready · microphone off','Prêt · microphone désactivé')}</p><textarea class="input" rows="3" maxlength="1000" aria-label="${L('Voice command','Commande vocale')}" placeholder="${L('Open minerals','Ouvre les minerais')}"></textarea><div class="actions"><button class="btn primary" data-run>${L('Ask Okapi','Demander à Okapi')}</button><button class="btn" data-speak>${L('Replay reply','Relire la réponse')}</button><button class="btn" data-silence>${L('Silence','Silence')}</button></div><div class="voice-options"><label><input type="checkbox" data-auto-send checked> ${L('Send when I finish speaking','Envoyer à la fin de ma phrase')}</label><label><input type="checkbox" data-auto-speak checked> ${L('Speak answers with Soniox','Lire les réponses avec Soniox')}</label></div><button class="btn primary" data-confirm hidden></button><p data-speech-status role="status" aria-live="polite"></p><ul data-citations></ul><details open><summary>${L('Try asking','Exemples de questions')}</summary><ul><li>${L('Can you show me the mineral sites?','Peux-tu me montrer les sites miniers ?')}</li><li>${L('What is happening across social intelligence?','Que se passe-t-il sur les réseaux sociaux ?')}</li><li>${L('What are the main news stories, and what sources support them?','Quelles sont les principales actualités et leurs sources ?')}</li></ul></details><p class="muted">${L('Audio goes to Soniox only while recording. Nothing is recorded in the background. Conversation stays in page memory and is sent with retrieved evidence to the configured AI provider. Collection and analysis use existing workers, permissions and budgets.','L’audio est transmis à Soniox pendant l’enregistrement uniquement. Aucune écoute en arrière-plan. La conversation reste en mémoire et est transmise avec les sources au fournisseur IA configuré. Collecte et analyse respectent les permissions et budgets existants.')}</p><ol data-history></ol>`;
    document.body.append(panel);$('[data-close]').onclick=close;$('[data-voice-end]').onclick=()=>{epoch++;release();stopSpeaking();endVoiceMode(L('Voice mode ended · microphone off','Mode vocal terminé · micro désactivé'));};$('[data-mic]').onclick=record;$('[data-stop]').onclick=stopping;$('[data-run]').onclick=run;$('[data-speak]').onclick=()=>speak();$('[data-pref-lang]').onchange=e=>{prefs.lang=e.target.value;detectedLang='';savePrefs();renderPrefs();};$('[data-pref-voice]').onchange=e=>{prefs.voice=e.target.value;savePrefs();renderPrefs();};$('[data-preview]').onclick=()=>{unlockAudio();const lang=ttsLang();speak({text:SAMPLES[lang]||SAMPLES.en,language:SAMPLES[lang]?lang:'en'});};renderPrefs();if(!catalog)api('/v1/voice/options').then(o=>{catalog=o;renderPrefs();}).catch(()=>{});$('[data-silence]').onclick=()=>{stopSpeaking();$('[data-auto-speak]').checked=false;speechStatus(L('Muted · enable Speak answers to hear replies again','Muet · activez Lire les réponses pour rétablir la voix'));};$('[data-auto-speak]').onchange=()=>{if(!$('[data-auto-speak]').checked)stopSpeaking();else{unlockAudio();speechStatus(L('Spoken replies enabled','Réponses vocales activées'));}};$('[data-confirm]').onclick=async()=>{if(!pending||running)return;const task=pending;pending=null;running=true;$('[data-confirm]').hidden=true;try{message(await execute(task));}catch(e){message(e.message);}finally{running=false;}};
    panel.addEventListener('keydown',e=>{e.stopPropagation();if(e.key==='Escape')close();if((e.ctrlKey||e.metaKey)&&e.key==='Enter'){e.preventDefault();run();}});if(typeof prompt==='string')$('textarea').value=prompt;$('textarea').focus();
    api('/v1/voice/status').then(s=>{if(panel&&s.paused)message(L('Voice spending paused · set Soniox limits in Costs & budgets. Text questions use your AI budget.','Budget vocal en pause · configurez Soniox dans Coûts et budgets. Questions écrites soumises au budget IA.'));else if(panel&&!s.configured)message(L('Soniox key missing. Typed commands still work.','Clé Soniox absente. Les commandes écrites restent disponibles.'));}).catch(e=>message(e.message));
  }
  document.addEventListener('visibilitychange',()=>{if(document.hidden){cancel();message(L('Microphone stopped because this tab was hidden.','Microphone arrêté : onglet masqué.'));}});
  window.addEventListener('pagehide',cancel);
  return {open,close,listen};
}
