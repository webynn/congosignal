'use strict';
const $=id=>document.getElementById(id);
let lang=localStorage.getItem('congo-lang')==='fr'?'fr':'en';
let token=new URLSearchParams(location.hash.slice(1)).get('setup')||'';
// Keep the one-use secret out of history, referrers and server request logs.
if(token)history.replaceState(null,'',location.pathname);
let busy=false;
const L=(en,fr)=>lang==='fr'?fr:en;
function render(){
 document.documentElement.lang=lang;$('lang').textContent=lang==='fr'?'EN':'FR';
 $('title').textContent=token?L('Set your password','Créez votre mot de passe'):L('Welcome back','Bienvenue');
 $('intro').textContent=token?L('Choose at least 12 characters. This link can only be used once.','Choisissez au moins 12 caractères. Ce lien ne peut être utilisé qu’une fois.'):L('Sign in to your invited workspace.','Connectez-vous à votre espace sur invitation.');
 $('email-label').textContent=L('Email','Adresse email');$('password-label').textContent=L('Password','Mot de passe');$('confirm-label').textContent=L('Confirm password','Confirmez le mot de passe');
 $('email').hidden=!!token;$('email-label').hidden=!!token;$('email').required=!token;
 $('password').autocomplete=token?'new-password':'current-password';$('password').minLength=token?12:1;$('confirm-wrap').hidden=!token;$('confirm').required=!!token;
 $('submit').textContent=token?L('Save password','Enregistrer le mot de passe'):L('Sign in','Se connecter');$('submit').disabled=busy;
 $('forgot').hidden=!!token;$('forgot').textContent=L('Forgot password?','Mot de passe oublié ?');
 $('foot').textContent=L('Access is by invitation. Contact your workspace administrator for an invitation or password reset link.','Accès sur invitation. Contactez votre administrateur pour un lien d’invitation ou de réinitialisation.');
}
function status(text,error=false){$('status').textContent=text;$('status').className=error?'error':'';}
$('lang').onclick=()=>{lang=lang==='en'?'fr':'en';localStorage.setItem('congo-lang',lang);render()};
$('forgot').onclick=()=>status(L('Ask your workspace owner or administrator for a password reset link. No email has been sent.','Demandez un lien de réinitialisation au propriétaire ou à l’administrateur. Aucun email n’a été envoyé.'));
$('login').onsubmit=async e=>{
 e.preventDefault();if(busy)return;
 if(token&&$('password').value!==$('confirm').value){status(L('Passwords do not match.','Les mots de passe ne correspondent pas.'),true);return}
 busy=true;render();status(L('Please wait…','Veuillez patienter…'));
 try{
  const setup=!!token;
  const r=await fetch(setup?'/auth/password/setup':'/auth/password/login',{method:'POST',headers:{'Content-Type':'application/json'},credentials:'same-origin',body:JSON.stringify(setup?{token,password:$('password').value}:{email:$('email').value,password:$('password').value})});
  const d=await r.json();if(!r.ok)throw Error(d.error||L('Unable to sign in.','Connexion impossible.'));
  $('password').value='';$('confirm').value='';
  if(setup){token='';$('email').value=d.email;status(L('Password saved. Sign in with your new password.','Mot de passe enregistré. Connectez-vous avec votre nouveau mot de passe.'));render();$('password').focus()}
  else location.replace('/');
 }catch(e){status(e.message,true)}finally{busy=false;render()}
};
render();
