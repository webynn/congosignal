'use strict';
const $=id=>document.getElementById(id);
let lang=localStorage.getItem('congo-lang')==='fr'?'fr':'en';
let token=new URLSearchParams(location.hash.slice(1)).get('setup')||'';
// Keep the one-use secret out of history, referrers and server request logs.
if(token)history.replaceState(null,'',location.pathname);
let signupOpen=false,mode=token?'setup':(location.hash==='#signin'?'login':'signup'),busy=false;
const L=(en,fr)=>lang==='fr'?fr:en;
const T={
 edition:['Analyst edition','Édition analyste'],back:['← Back to the news','← Retour à l’actualité'],
 'pitch-title':['Go deeper than the headlines.','Allez au-delà des titres.'],
 'pitch-dek':['The analyst edition gives researchers, journalists and humanitarian teams the full picture behind the news from the DRC.','L’édition analyste donne aux chercheurs, journalistes et équipes humanitaires le tableau complet derrière l’actualité de la RDC.'],
 f1t:['War room','Salle de crise'],f1:['Who controls which towns, with a cited source and date for every change, and incidents on a live map.','Qui contrôle quelles localités, avec une source et une date pour chaque changement, et les incidents sur une carte en direct.'],
 f2t:['Event log','Journal des événements'],f2:['Dated incidents extracted from every report: what happened, where, who was named and the toll reported.','Incidents datés extraits de chaque article : ce qui s’est passé, où, qui est cité et le bilan rapporté.'],
 f3t:['Watchlists','Veilles'],f3:['Follow places, people, sanctioned names or military flights and get alerts as soon as they appear.','Suivez des lieux, des personnes, des noms sanctionnés ou des vols militaires et recevez une alerte dès qu’ils apparaissent.'],
 f4t:['Briefs','Synthèses'],f4:['Daily and weekly briefs in Word or PDF, every item cited to its source.','Synthèses quotidiennes et hebdomadaires en Word ou PDF, chaque élément cité.'],
 fine:['New accounts start with read access. Paid tools (AI analysis, X history, voice) are enabled by the editors for verified members.','Les nouveaux comptes ont un accès en lecture. Les outils payants (analyse IA, historique X, voix) sont activés par les éditeurs pour les membres vérifiés.'],
 'tab-signup':['Create account','Créer un compte'],'tab-login':['Sign in','Se connecter'],
 'name-label':['Full name','Nom complet'],'org-label':['Organisation','Organisation'],'org-opt':['(optional)','(facultatif)'],'purpose-label':['I work in','Je travaille dans'],
 'email-label':['Email','Adresse e-mail'],'password-label':['Password','Mot de passe'],'pw-hint':['At least 12 characters.','Au moins 12 caractères.'],'confirm-label':['Confirm password','Confirmez le mot de passe'],
 'accept-text':['I will use the analyst edition for legitimate research and reporting, and I understand that reports are unverified unless stated.','J’utiliserai l’édition analyste pour de la recherche et de l’information légitimes, et je comprends que les articles ne sont pas vérifiés sauf mention.'],
 forgot:['Forgot your password?','Mot de passe oublié ?'],foot:['Your email is used to sign you in and is never shared.','Votre e-mail sert uniquement à vous connecter et n’est jamais partagé.'],
};
const PURPOSES={research:['Research','Recherche'],journalism:['Journalism','Journalisme'],humanitarian:['Humanitarian / NGO','Humanitaire / ONG'],government:['Government / diplomacy','Gouvernement / diplomatie'],business:['Business','Entreprise'],education:['Education','Enseignement'],other:['Other','Autre']};
function render(){
 document.documentElement.lang=lang;
 for(const [id,[en,fr]] of Object.entries(T)){const el=$(id);if(el)el.textContent=lang==='fr'?fr:en}
 for(const o of $('purpose').options)if(PURPOSES[o.value])o.textContent=L(...PURPOSES[o.value]);
 $('lang-en').className=lang==='en'?'on':'';$('lang-fr').className=lang==='fr'?'on':'';
 const signup=mode==='signup',setup=mode==='setup';
 $('tab-signup').hidden=!signupOpen||setup;$('tab-login').hidden=setup;
 $('tab-signup').className=signup?'on':'';$('tab-login').className=mode==='login'?'on':'';
 $('tab-signup').setAttribute('aria-selected',signup);$('tab-login').setAttribute('aria-selected',mode==='login');
 $('form-title').textContent=setup?L('Set your password','Créez votre mot de passe'):signup?L('Create your account','Créez votre compte'):L('Welcome back','Bon retour');
 $('intro').textContent=setup?L('Choose at least 12 characters. This link can only be used once.','Choisissez au moins 12 caractères. Ce lien ne peut être utilisé qu’une fois.'):signup?L('It takes a minute. You can keep reading the news without an account.','Cela prend une minute. Vous pouvez lire l’actualité sans compte.'):L('Sign in to the analyst edition.','Connectez-vous à l’édition analyste.');
 $('signup-fields').hidden=!signup;$('accept-wrap').hidden=!signup;$('pw-hint').hidden=!(signup||setup);
 $('email').hidden=setup;$('email-label').hidden=setup;
 $('password').autocomplete=signup||setup?'new-password':'current-password';$('confirm-wrap').hidden=!setup;
 $('submit').textContent=setup?L('Save password','Enregistrer le mot de passe'):signup?L('Create account','Créer mon compte'):L('Sign in','Se connecter');$('submit').disabled=busy;
 $('forgot').hidden=mode!=='login';
}
function status(text,error=false){$('status').textContent=text;$('status').className=error?'error':''}
function setLang(l){lang=l;localStorage.setItem('congo-lang',lang);render()}
$('lang-en').onclick=()=>setLang('en');$('lang-fr').onclick=()=>setLang('fr');
$('tab-signup').onclick=()=>{mode='signup';status('');render();$('name').focus()};
$('tab-login').onclick=()=>{mode='login';status('');render();$('email').focus()};
$('forgot').onclick=()=>status(L('Ask an editor or administrator for a password reset link. No email has been sent.','Demandez un lien de réinitialisation à un éditeur ou à l’administrateur. Aucun e-mail n’a été envoyé.'));
$('login').onsubmit=async e=>{
 e.preventDefault();if(busy)return;
 const pw=$('password').value;
 if(mode==='signup'){
  if(!$('name').value.trim())return status(L('Enter your full name.','Saisissez votre nom complet.'),true);
  if(!$('purpose').value)return status(L('Tell us where you work.','Indiquez votre secteur.'),true);
  if(pw.length<12)return status(L('Choose a password of at least 12 characters.','Choisissez un mot de passe d’au moins 12 caractères.'),true);
  if(!$('accept').checked)return status(L('Please accept the terms of use.','Veuillez accepter les conditions d’utilisation.'),true);
 }
 if(mode==='setup'&&pw!==$('confirm').value)return status(L('Passwords do not match.','Les mots de passe ne correspondent pas.'),true);
 busy=true;render();status(L('Please wait…','Veuillez patienter…'));
 try{
  const url={setup:'/auth/password/setup',signup:'/auth/password/register',login:'/auth/password/login'}[mode];
  const body=mode==='setup'?{token,password:pw}:mode==='signup'?{name:$('name').value,email:$('email').value,password:pw,organisation:$('organisation').value,purpose:$('purpose').value,accept:$('accept').checked}:{email:$('email').value,password:pw};
  const r=await fetch(url,{method:'POST',headers:{'Content-Type':'application/json'},credentials:'same-origin',body:JSON.stringify(body)});
  const d=await r.json().catch(()=>({}));if(!r.ok)throw Error(d.error||L('Something went wrong. Please try again.','Une erreur est survenue. Veuillez réessayer.'));
  $('password').value='';$('confirm').value='';
  if(mode==='setup'){token='';mode='login';$('email').value=d.email||'';status(L('Password saved. Sign in with your new password.','Mot de passe enregistré. Connectez-vous avec votre nouveau mot de passe.'));render();$('password').focus()}
  else{localStorage.setItem('congo-mode','advanced');location.replace(d.redirect||'/')}
 }catch(err){status(err.message,true)}finally{busy=false;render()}
};
fetch('/auth/config').then(r=>r.json()).then(c=>{signupOpen=!!c.signup;if(!signupOpen&&mode==='signup')mode='login';render()}).catch(()=>{mode=mode==='signup'?'login':mode;render()});
render();
