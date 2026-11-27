const byId=id=>document.getElementById(id);
let running=false,connecting=false,endpoint=null,key=null,frameId=null,busy=false;
const tabId=Number(new URLSearchParams(location.search).get("tab"));
const messages={
  DIARY_CONTEXT:"No se conserva el listado actual para abrir bitácoras. Vuelve a consultar el lote sin navegar en RUS durante la lectura.",
  DIARY_LINK:"El enlace de la bitácora está ausente o repetido en el listado actual. No se abrió una causa por suposición.",
  DIARY_IDENTITY:"La identidad solicitada no coincide con el tribunal, modalidad e ingreso del listado actual.",
  DIARY_NETWORK:"La consulta de bitácora falló al contactar con RUS o recibió una redirección. Comprueba la sesión y reintenta el lote.",
  DIARY_TIMEOUT:"RUS no respondió a tiempo al abrir la bitácora. Reintenta las lecturas fallidas.",
  DIARY_RESPONSE:"RUS no entregó una respuesta válida de bitácora. Revisa la sesión y reintenta las lecturas fallidas.",
  CALENDAR_CONTEXT:"No se conserva el listado actual para abrir el calendario de informes. Vuelve a consultar sin navegar en SITFA.",
  CALENDAR_LINK:"El enlace del calendario de informes no está en el listado actual. No se abrió un ingreso por suposición.",
  CALENDAR_IDENTITY:"El calendario de informes no coincide con el tribunal e ingreso del listado actual.",
  CALENDAR_NETWORK:"Falló el contacto con SITFA al abrir el calendario de informes. Comprueba la sesión.",
  CALENDAR_TIMEOUT:"SITFA no respondió a tiempo al abrir el calendario de informes.",
  CALENDAR_RESPONSE:"SITFA no entregó un calendario de informes válido. Revisa la sesión.",
  HISTORICO_CONTEXT:"No se conserva el listado actual para abrir el Ingreso Histórico.",
  HISTORICO_LINK:"El listado actual no acredita el RUT completo para el Ingreso Histórico.",
  HISTORICO_IDENTITY:"La identidad solicitada no coincide con la consulta vigente.",
  HISTORICO_RESPONSE:"SITFA no devolvió una ventana válida de Ingreso Histórico.",
  HISTORICO_NETWORK:"Falló la conexión al consultar el Ingreso Histórico.",
  HISTORICO_TIMEOUT:"SITFA agotó el tiempo de espera del Ingreso Histórico.",
  HISTORY_CONTEXT:"No se conserva el listado actual para abrir la historia. Vuelve a consultar sin navegar en SITFA.",
  HISTORY_LINK:"El enlace de historia no está en el listado actual. No se abrió una causa por suposición.",
  HISTORY_IDENTITY:"La causa pedida no coincide con el tribunal del listado actual.",
  HISTORY_NETWORK:"Falló el contacto con SITFA al abrir la historia. Comprueba la sesión.",
  HISTORY_TIMEOUT:"SITFA no respondió a tiempo al abrir la historia.",
  HISTORY_RESPONSE:"SITFA no entregó una historia válida. Revisa la sesión.",
  HISTORY_REFLECTED:"La ventana de historia repite valores internos de SITFA; se descartó para no exponerlos. Informa este código.",
  CONNECTION_CODE:"El descargador rechazó el código. Copia el código actual de la ventana que está abierta; cambia al reiniciar o desconectar.",
  EXTENSION_ORIGIN:"El descargador rechazó el origen de la extensión. Actualiza el descargador y recarga la extensión en chrome://extensions.",
  EXTENSION_CHANGED:"Este descargador está vinculado a otra copia de la extensión. Pulsa Desconectar en el descargador y usa el nuevo código.",
  LOCAL_UNREACHABLE:"No se pudo contactar con el descargador de este equipo. Comprueba que siga abierto y pega su código actual.",
  PAYLOAD_TOO_LARGE:"La respuesta excede el presupuesto de lectura del puente local. Acota la consulta.",
  LOCAL_TIMEOUT:"El descargador local no respondió a tiempo. Comprueba que siga abierto y vuelve a conectar.",
  LOCAL_PROTOCOL:"El descargador y la extensión no completaron el enlace. Actualiza ambos y vuelve a conectar.",
  SITE_ACCESS:"El enlace local funciona, pero Chrome no permite acceder a la pestaña SITFA. Comprueba que siga abierta y que la extensión tenga acceso a familia.pjud.cl. Abre de nuevo la conexión desde esa pestaña.",
  FOLLOWUP_REQUIRED:"El enlace local funciona, pero no se encuentra el formulario de Seguimiento. Entra a Seguimiento en la pestaña original de SITFA y, en el Descargador, elige «Más › Recargar opciones de SITFA».",
  MULTIPLE_FORMS:"Se encontró más de un formulario de Seguimiento. Abre la conexión desde una pestaña con un solo formulario.",
  SITFA_TASK:"SITFA no respondió como se esperaba. Revisa la sesión y la pantalla de consulta elegida.",
  SCREEN_OPEN:"No se pudo abrir la pantalla elegida en SITFA. Revisa la sesión, entra a una pantalla de consulta y, en el Descargador, elige «Más › Recargar opciones de SITFA».",
  EVIDENCE_CAPTURE:"No se pudo capturar la consulta vacía. Mantén SITFA abierto en una ventana visible, pulsa de nuevo el icono de la extensión desde SITFA y vuelve a conectar. El lote conserva los archivos guardados.",
  EVIDENCE_OPEN:"No se pudo cargar la vista SITFA para el PDF de la consulta vacía. Revisa que la sesión siga abierta y actualiza la extensión y el descargador juntos. Los archivos anteriores se conservan.",
};
function status(message){byId("status").textContent=message;}
function failure(code){return Object.assign(new Error(code),{code});}
// Motivos técnicos de la evidencia PDF: vocabulario fijo, sin datos de causas.
const evidenceReasons=["estructura","sesion","cambio","pantalla","tiempo","pagina","acceso","permiso","pestaña","ventana","captura"];
function safeCode(error,fallback){return Object.hasOwn(messages,error?.code)?error.code:fallback;}
let puenteRust=false;
async function bridge(path,body) {
  let response;
  try {
    response=await fetch(endpoint+path,{method:body===undefined?"GET":"POST",cache:"no-store",
      headers:{"X-Sitfa-Code":key,"X-Sitfa-Extension":chrome.runtime.id,
        ...(body===undefined?{}:{"Content-Type":"application/json"}),
        ...(puenteRust&&path==="/complete"&&body?.id?{"X-Sitfa-Task":body.id}:{})},
      ...(body===undefined?{}:{body:JSON.stringify(body)}),signal:AbortSignal.timeout(70000)});
  } catch (error) {throw failure(error?.name==="TimeoutError"?"LOCAL_TIMEOUT":"LOCAL_UNREACHABLE");}
  let data;
  try {data=await response.json();} catch (_) {throw failure("LOCAL_PROTOCOL");}
  if (!response.ok) throw failure(["CONNECTION_CODE","EXTENSION_ORIGIN","EXTENSION_CHANGED","TASK_EXPIRED","PAYLOAD_TOO_LARGE","LOCAL_TIMEOUT"].includes(data?.error)?data.error:"LOCAL_PROTOCOL");
  return data;
}
const allowedCommands=new Set([
  "catalog","capabilities","lock","unlock","prepare","search","download","search_and_download",
  "evidence_open","evidence_capture","evidence_close",
  "diary_open","history_open","calendar_open","historico_open",
]);
const allowedInjectCommands=new Set([...allowedCommands,"evidence_show","navigate"]);
async function inject(command,payload={}) {
  if(!allowedInjectCommands.has(command)) throw failure("SITFA_TASK");
  let results;
  try {
    results=await chrome.scripting.executeScript({target:frameId===null||command==="catalog"?{tabId,allFrames:true}:{tabId,frameIds:[frameId]},
      world:"MAIN",func:sitfaTask,args:[command,payload]});
  } catch (_) {throw failure("SITE_ACCESS");}
  const candidates=results.filter(r=>r.result!==null&&r.result!==undefined);
  if (!candidates.length) throw failure(command==="catalog"?"FOLLOWUP_REQUIRED":"SITFA_TASK");
  if (candidates.length!==1) throw failure("MULTIPLE_FORMS");
  if(candidates[0].result?.__sitfaError) throw Object.assign(failure(Object.hasOwn(messages,candidates[0].result.__sitfaError)?candidates[0].result.__sitfaError:"SITFA_TASK"),
    evidenceReasons.includes(candidates[0].result.motivo)?{motivo:candidates[0].result.motivo}:{});
  frameId=candidates[0].frameId;return candidates[0].result;
}
async function execute(command,payload={}) {
  if(!allowedCommands.has(command)) throw failure("SITFA_TASK");
  if(command==="evidence_capture") {
    let changed=false,stage="pestaña";
    const activated=info=>{if(info.windowId===windowId&&info.tabId!==tabId)changed=true;};
    let windowId;
    try {
      const tab=await chrome.tabs.get(tabId);windowId=tab.windowId;
      if(new URL(tab.url).origin!=="https://familia.pjud.cl" || tab.discarded)throw new Error("pestaña");
      stage="ventana";
      const win=await chrome.windows.get(windowId);
      // Chrome no pinta una ventana tapada por otra (p. ej., el descargador): se trae al frente para la captura.
      await chrome.windows.update(windowId,{...(win.state==="minimized"?{state:"normal"}:{}),focused:true});
      await chrome.tabs.update(tabId,{active:true});
      chrome.tabs.onActivated.addListener(activated);
      await inject("evidence_show",payload);
      // Dar tiempo al compositor, también cuando la pestaña estaba en segundo plano.
      await new Promise(resolve=>setTimeout(resolve,600));
      const active=await chrome.tabs.query({windowId,active:true});
      if(changed || active.length!==1 || active[0].id!==tabId)throw new Error("pestaña");
      stage="captura";
      let png;
      try {png=await chrome.tabs.captureVisibleTab(windowId,{format:"png"});}
      catch (error) {throw new Error(/activeTab|all_urls|permission/i.test(String(error?.message))?"permiso":"captura");}
      stage="pestaña";
      await inject("evidence_show",payload);
      if(changed || !(await chrome.tabs.get(tabId)).active || !png.startsWith("data:image/png;base64,"))throw new Error("pestaña");
      return {png:png.slice("data:image/png;base64,".length),digest:payload.digest};
    } catch (error) {
      const motivo=evidenceReasons.includes(error?.motivo)?error.motivo:evidenceReasons.includes(error?.message)?error.message:
        error?.code==="SITE_ACCESS"?"acceso":error?.code?"pagina":stage;
      throw Object.assign(failure("EVIDENCE_CAPTURE"),{motivo});
    }
    finally {chrome.tabs.onActivated.removeListener(activated);try {await inject("evidence_close");}catch(_) {}}
  }
  if(command==="lock") {
    const res=await inject(command,payload);
    try {await chrome.tabs.update(tabId,{autoDiscardable:false});} catch (_) {}
    return res;
  }
  if(command==="unlock") {
    try {return await inject(command,payload);}
    finally {try {await chrome.tabs.update(tabId,{autoDiscardable:true});} catch (_) {}}
  }
  if(command!=="catalog" || !payload.screen) return inject(command,payload);
  if(!["seguimiento","litigantes","calendario_informes","calendario_medidas","carga"].includes(payload.screen)) throw failure("SCREEN_OPEN");
  const current=await inject("catalog");
  if(current.pantalla===payload.screen) return current;
  status("Abriendo la pantalla de consulta elegida en SITFA…");
  await inject("navigate",{screen:payload.screen});frameId=null;
  const until=Date.now()+15000;
  while(Date.now()<until) {
    await new Promise(resolve=>setTimeout(resolve,250));
    try {
      const catalog=await inject("catalog");
      if(catalog.pantalla===payload.screen) {
        status("Conectado. Opciones de la pantalla elegida cargadas.");return catalog;
      }
    } catch (_) {}
  }
  throw failure("SCREEN_OPEN");
}
async function disconnect() {
  running=false;
  if (frameId!==null) {try {await execute("unlock");} catch (_) {}}
  try {await chrome.tabs.update(tabId,{autoDiscardable:true});} catch (_) {}
  key=null;endpoint=null;frameId=null;connecting=false;
  byId("connect").disabled=false;byId("disconnect").disabled=true;byId("code").disabled=false;
}
byId("connect").addEventListener("click",async()=>{
  if (running || busy || connecting) return;
  if (!Number.isInteger(tabId) || tabId<=0) {status("Abre esta conexión pulsando el icono de la extensión desde la pestaña SITFA.");return;}
  const match=/^(\d{1,5})-([0-9a-f]{32})$/.exec(byId("code").value.trim());
  if (!match || Number(match[1])<1 || Number(match[1])>65535) {status("Copia el código completo desde el descargador.");return;}
  endpoint="http://127.0.0.1:"+match[1];key=match[2];frameId=null;
  connecting=true;byId("connect").disabled=true;byId("code").disabled=true;
  status("Comprobando el enlace con el descargador local…");
  try {
    // Validar el enlace local no depende de que Seguimiento ya esté abierto.
    const enlace=await bridge("/connect",{});puenteRust=Number.isInteger(enlace.parallel);
    running=true;connecting=false;byId("code").value="";
    byId("connect").disabled=true;byId("disconnect").disabled=false;
    status("Conectado. Vuelve al Descargador: las opciones se cargan solas. Mantén abierta esta pestaña.");
    // El puente es la autoridad del despacho. Con Flow Python (sin `parallel`) permanece secuencial.
    const activos=new Set();let tope=1,fallas=0,degradado=false,fallo=null;
    const completar=async task=>{
      let result;
      try {result={id:task.id,ok:true,result:await execute(task.command,task.payload)};fallas=0;}
      catch (error) {
        if(++fallas>=2)degradado=true;
        const code=safeCode(error,"SITFA_TASK");
        const motivo=evidenceReasons.includes(error?.motivo)?error.motivo:undefined;
        result={id:task.id,ok:false,error:code,...(motivo?{motivo}:{})};
        status(messages[code]+(motivo?" Motivo técnico: "+motivo+".":""));
      }
      try {await bridge("/complete",result);}
      catch(error){
        if(error?.code==="TASK_EXPIRED")return;
        if(["PAYLOAD_TOO_LARGE","LOCAL_TIMEOUT"].includes(error?.code)){
          if(++fallas>=2)degradado=true;
          status(messages[error.code]);
          // Entrega el diagnóstico pequeño al mismo solicitante; no repite la lectura SITFA.
          try {await bridge("/complete",{id:task.id,ok:false,error:error.code});}
          catch(e){if(e?.code!=="TASK_EXPIRED")throw e;}
          return;
        }
        throw error;
      }
    };
    try {
      while(running&&!fallo){
        if(activos.size>=(degradado?1:tope)){await Promise.race(activos);continue;}
        const task=await bridge("/next");
        if(!running||fallo)break;
        tope=Number.isInteger(task.parallel)?Math.max(1,Math.min(3,task.parallel)):1;
        if(task.idle)continue;
        while(activos.size>=(degradado?1:tope))await Promise.race(activos);
        // El puente no entrega tareas de formulario con una lectura en vuelo. Defensa adicional local.
        if(!["diary_open","history_open","calendar_open","historico_open"].includes(task.command))await Promise.all(activos);
        busy=true;byId("disconnect").disabled=true;
        let trabajo;
        trabajo=completar(task).catch(error=>{fallo=error;}).finally(()=>{
          activos.delete(trabajo);busy=activos.size>0;byId("disconnect").disabled=busy;
        });
        activos.add(trabajo);
      }
    } finally {await Promise.all(activos);}
    if(fallo)throw fallo;
  } catch (error) {
    status(messages[safeCode(error,"LOCAL_PROTOCOL")]);
  } finally {busy=false;await disconnect();}
});
byId("disconnect").addEventListener("click",()=>{
  if (busy || connecting) return;
  // Cambia la bandera; /next finaliza su espera acotada antes de limpiar el contexto.
  running=false;status("Desconectado. Para volver a conectar, copia el código actual del descargador.");
});
