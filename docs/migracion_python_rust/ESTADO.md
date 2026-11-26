# Estado de Flow Rust

Actualizado: 26 de noviembre de 2026. Las **Tandas 1 y 2** (`E-P0-C1`, `E-P0-B1`, `E-P1-A1`, `E-P1-A2`, `E-P1-B2`, `E-P1-C2`), `E-Q11`, **E-P2-OPT** y el **Plan Waffle** (política de perfil Tomé para otros tribunales válidos con revisión manual para vacíos/marcadores/ambiguos en `flow-python` `be9d9ae`, `app/` y `flow-motor`, preservación de identidad y aviso de matriz faltante, regeneración de `rust/casos`, y validación diferenciada de fechas en `lote.rs` y `bitacoras.rs`) están **`HECHO`** con CI Linux y Windows en verde y regeneración de oráculos corrida (run `45894872026`, commit `60cdfce`). Es la única fuente del estado: se edita en el mismo commit que cierra cada encargo.

- Aplicación: `app/` (`flow_rust.exe`). Referencia funcional: rama `flow-python` en `MatiGaete2023/waffle` (fijada en `be9d9aee717d`).
- Rama única: `estudio/paridad-python-rust-20261119`.
- Estados: **Sí** (existe y tiene contraste con Python), **Parcial** (falta lo que se indica), **No** (ausente). Ninguno significa «aceptado»: eso solo lo declara el usuario en un acta.

## Siguiente paso

1. **Generar entrega `--release` en CI (`workflow_dispatch`, `entrega=true`):** lanzar el flujo `Flow Rust local · Windows` con `entrega=true` sobre `estudio/paridad-python-rust-20261119` para obtener el ejecutable `--release`, el instalador Inno Setup, las capturas de Windows y el contraste V1 con `rust/`.
2. **Prueba operativa real del usuario (cierre de brechas Windows/SITFA/Office):** pulsar **«Instalar extensión…»** y recargar la extensión Integral **`2.10.0`** en `chrome://extensions` (carpeta fija `%LOCALAPPDATA%\FlowRust\extension-integral`), medir «Modo Rápido Seguro» (`V-01`..`V-04`, concurrencia 2–3 sin caché entre lotes del día), verificar análisis → revisión → exportación con un tribunal adicional y uno sin identificar, probar rango de Seguimiento / reanudación / lote nuevo, Outlook COM (2 cuentas, firma, CC fijo), Excel preservado COM, Meta 4 con alcance acreditado y actualización desde OneDrive.
3. **Criterio de salida mensual (Acta):** comparar la extracción `01/11/2026`–`26/11/2026` y completar un mes real en paralelo con Flow Python antes de firmar el acta de aceptación.
4. **Decisiones vigentes del usuario:** ver «Decisiones del usuario vigentes» de [PLAN.md](PLAN.md). Descartados: M6, M7, formatos antiguos de Meta 4, conteo del Anonimizador, paginación del Word manual, manuales aparte, firma Authenticode del instalador y caché entre lotes del día.

## Pendiente del usuario

- **Prueba operativa real con el `.exe` de la entrega (`Flow_Rust_Windows_exe`):** comprobar en Windows la extensión Integral `2.10.0`, «Consultar y crear paquete», descarga conjunta CSMP y «Retomar pendientes», Modo Rápido Seguro, contactos migrados y borrador de prueba con firma en Outlook clásico, copia preservada Excel COM, Meta 4 con período/alcance acreditado y actualización desde OneDrive (`%LOCALAPPDATA%\FlowRust\actualizaciones`). Anotar lo que falle; no hace falta acta todavía.
- **Diagnósticos de paquetes parciales (si ocurren):** si Flow muestra «Paquete con cobertura incompleta», enviar solo el `.diagnostico_cobertura.json` hermano del ZIP (botón «Abrir carpeta del paquete») y el texto del aviso con sus códigos y números; si detiene con `CAL_MENSUAL_INCOMPLETO`, el código, los meses faltantes y cuántas consultas quedaron verificadas. Guía en [informes/AGENTE_C_20261125.md](informes/AGENTE_C_20261125.md#qué-enviar).
- **Lote antiguo de Flow Python rechazado por Resultados (si persiste tras E-S2):** enviar solo la lista de nombres de archivos (`dir /b`), sin contenido.
- **Acta de uso real:** un mes real completo, en paralelo con Python (prevista alrededor del 25 de diciembre de 2026), más revisión visual de las capturas del run de entrega.

## Capacidades

### Asistente

- **F01 Lectura de hojas — Sí.** `lectura.rs` (con cuotas XML/ZIP compartidas desde `descargador::xls` y compilación única de regex).
- **F02 Trabajo y cruces — Parcial.** Tres etapas, Hoja 2, calendario 91, CAL-03, egresados (E-A2) y órdenes de búsqueda OB-01 a OB-05 (E-A5, 21 casos contra Python; soporte `null` en E-P1-A1). Política multi-tribunal con perfil Tomé para otros tribunales válidos y revisión manual para vacíos/marcadores/ambiguos (Plan Waffle). Falta la identidad multifila: no hay en Python un comportamiento distinto del ya cotejado en el corpus multifila de E3.
- **F03 Reglas de Excel — Sí.** `oraculo_python` y `flow-motor` (924/924/911/943 casos regenerados con Plan Waffle).
- **F04 Contexto de causa — Sí.** `oraculo_historia`, 2.585 casos; paquete C con ZIP de Flow Python.
- **F05 Otro programa, reiteración y Escritos — Sí.** Cronología con 181 casos, R5 parcial y consignación con deshacer.
- **F06 Revisión de bitácoras — Sí.** V1 y V2 contrastadas en la CI.
- **F07 Importación de paquetes — Sí.** Conciliación entre capturas (E-A2, E-P1-A1): traslada decisiones, `resolucion` y `excluido` por identidad remota, resuelve conflictos en pantalla y muestra filas nuevas, modificadas y egresadas. Contrastado con Python en 12 casos. Trabajo concilia la etapa abierta.
- **F08 Edición y revisión — Parcial.** Edición masiva con «Deshacer» (E-A5), gestión de correo por fila `include`/`omit` (E-A8, 24 casos en `correos.json`) y anulaciones de columnas de origen por fila con recálculo de pares `_prepass_peers` (E-A10, 13 casos en `anulaciones.json`). Trabajo en 4 pestañas, navegación por teclado y `Ctrl+F` (E-P1-A2). Variables de Word por fila y exclusión manual quedan fuera por decisión del usuario (25-nov).
- **F09 Sesión y vigencia — Sí, sin oráculo de huellas.** Los correos y proyectos preparados se sellan con la huella de lo que leen (registros, plantillas, contactos, firma, captura, `gestiones_correo`, `resolucion`); si cambia algo quedan «desactualizados» y no se guardan. Guardado automático al cambiar de vista y en `on_exit()` (E-P1-A2). Recibos persistidos en sesión.
- **F10 Excel preservado — Parcial.** «Actualizar desde Excel» (E-A1, E-A7, E-A8, E-A9, E-A11, E-P1-A1): sincroniza revisión, columnas de origen recalculadas (en archivos RUS y paquetes) y estado completo de `NURUS_DECISIONES` (codificador canónico único con límite 32.000 UTF-16 y proyección preservada); 38 casos en `sync.json` y 6 en `sync_paquete.json`. Excel COM corre en `BackgroundJob<T>` con timeout y limpieza de RCWs (E-P1-A2). Sin libro base, genera libro nuevo con la estructura `FAE_RES`. La copia preservada exige Windows y Excel de escritorio (pendiente prueba real).
- **F11 Configuración y contactos — Sí.** 16 umbrales y textos con verificador; alias de contactos, editor de plantillas de correo, importación de matrices Word ZIP (E-A4, 44 casos), importación de catastros Excel/JSON (E-A6, 36 casos) y migración Python independiente por fuente conservando contactos locales (E-P1-A1). Ajustes en sub-pestañas (E-P1-A2).
- **F12 Correos y nómina — Sí.** Agrupación, destinatarios con procedencia automática/manual de `Para`, CC fijo por coincidencia exacta, sanitización MIME, firma y alcance contrastados en 24 casos (E-A5, E-A8, E-P1-A1). Diferencia declarada y decidida por el usuario (25-nov): «módulo» con tilde en Cumplimiento.
- **F13 Borradores Outlook — Parcial.** EML y Outlook COM en `BackgroundJob<T>` con cuenta elegible, solo `Save()`, CC fijo, firma de cuenta preservada, timeout + limpieza RCW y estado «incierto» ante fallo posterior a guardar (E-T1, E-P1-A2). Botón «Guardar un borrador de prueba en Outlook» en Ajustes (E-T6). Falta prueba con Outlook real del usuario.
- **F14 Proyectos Word — Sí (E-P1, E-P2, E-P1-A1, Plan Waffle).** Matrices de Laja y Mulchén; aviso en Resoluciones si falta matriz para otro tribunal válido. Elección «Un .docx por proyecto» o «Un .docx por tribunal» con índice, saltos de página, medios/relaciones/numeración al estilo `docxcompose`, protección contra ciclos `.rels`, selección Word por fila y `RESUELTAS_POR_EVIDENCIA`. Oráculo `word_tribunal` (12 casos, 13 archivos).
- **F15 Formularios manuales — Sí.** Correo manual con adjuntos verificados por huella y CC fijo; Word libre o con matriz `{{CAMPO}}`, bloques favoritos y recuperación de lo escrito (E-T2, 19 casos). Sin paginación manual (descartada por el usuario).
- **F16 Resultados y estadísticas — Sí.** Vista con `ScrollArea`, estadísticas de constancias con propuestas aprobadas y exportación Excel, y métricas locales opcionales sin datos de casos (E-T3, E-P1-A1, E-P1-A2; 14 casos).

### Descargador

- **F17 Integral y Lite — Sí** sobre SITFA ficticio. Extensión Integral propia **`2.10.0`** en `app/src/descargador/extension_integral/` con índice DOM por página (`V-01`) y concurrencia acotada 1–3 con degradación (`V-04`), puente HTTP `chunked`/acotado (E-P1-C2), «Modo Rápido Seguro» calibrado (`V-01`..`V-04`, sin caché entre lotes del día, E-P2-OPT) y validación diferenciada de fechas del formulario antes de `search` (Plan Waffle Paso 4). Sin prueba final con Chrome/SITFA real tras `2.10.0`.
- **F18 Gestión de lotes — Sí.** Pausa, cancelación, pendientes, favoritos, preferencias, rango único con «Todas las fechas» y Dos Calendarios (E-P0-C1, E-P1-C2).
- **F19 Bitácora, historia y calendario — Sí** sobre SITFA ficticio (35 ventanas de lectura, 21 recorridos de bitácoras; E-Q1..E-Q8, E-P1-C2, E-P2-OPT). Pipelining local (`V-02`) y pausa adaptativa (`V-03`). Cobertura parcial controlada (`CALENDARIO_FALLIDO`, `CAL_MENSUAL_INCOMPLETO`) y aviso en UI si Chrome sigue en extensión `2.9.0`.
- **F20 Escritos — Sí** dentro de la lectura de historia.
- **F21 Histórico de Protección — Sí**, con suspensión por fallas.
- **F22 Exportaciones de lote y flujos — Sí (`HECHO`, run `45894872026`).**
  - Paquete `CSMP_CAPTURE` publicado en pareja con `.diagnostico_cobertura.json` anónimo, validación canónica anti-traversal y carga única de historias (E-P0-C1, E-P1-B2).
  - Excel del lote (`EXCEL_BITACORAS` 3/3), libro consolidado en memoria y «Para CSMP» (`LIBROS` 14/14), CSV, ICS, ZIP y comparador (`PRODUCTOS` 13/13).
  - «Consultar y crear paquete» en una acción con reanudación (E-Q9, 12 casos), «Un paquete por hoja» (E-Q10, E-P0-C1) y descarga conjunta CSMP con «Retomar pendientes» (E-Q11, 8 casos).
  - «Resultados del lote» con conjuntos `flujo.json`, reunión de bitácoras de otras carpetas (E-S2, 22 casos), historial local, vista conjunta (E-S3) e índice cacheado en memoria (E-P1-C2).
- **F27 Intercambio entre herramientas — Sí, opción B (D-9).** «Abrir en Trabajo» tras crear el paquete.

### Otras herramientas

- **F23 y F24 Meta 4 — Sí (alcance A, E-R1..E-R3, E-P0-C1, E-P1-B2; `HECHO`).** `descargador/meta4.rs` + `flow-meta4` (32/32 pruebas, fix `count="01"` y cuota ZIP). Vista «Meta 4» con período Desde/Hasta inclusivo separado del alcance acreditado del lote, tribunales, plantilla opcional, generar y regenerar. Oráculo `meta4` (33 casos, 55 filas, 3 regeneraciones, 5 plantillas). Formatos antiguos (`bitacoras_meta4.json` y Excel sueltos) descartados por el usuario.
- **F25 Anonimizador — Sí (E-T4, E-P0-B1; `HECHO`).** `app/src/anon.rs` delega en `flow-anonimo` (40/40 pruebas): compatible con `CSMP_CAPTURE` 1.0/1.1/1.2, cuota real de descompresión ZIP, verificación previa a publicación y carpeta `Anonimizados_*` nueva. Conteo de reemplazos por tipo descartado por el usuario.
- **F26 Distribución y actualización — Sí (E-T5, E-I1..E-I3, E-P0-B1; `HECHO`).** `--self-test` en CI; actualizador en el lugar con SHA-256 obligatorio en ambas rutas (`%LOCALAPPDATA%\FlowRust\actualizaciones` y carpeta adicional OneDrive), `share_mode(0)`, publicación atómica y rollback (`flow_rust.exe.anterior`); extensión de Chrome en carpeta fija; instalador por usuario sin administrador (`Instalar_Flow.cmd`, `Instalar_Flow.ps1`, `FlowRust.iss`) con manifiesto de propiedad `FlowRustOwnership/1` y desinstalación acotada sin borrado recursivo (`Desinstalar_Flow.ps1`). Sin firma Authenticode (decisión del usuario).
- **F28 Utilidades y ayuda — Sí (E-T5, E-P1-A2; `HECHO`).** Ayuda por vista, reglas fijas y autodiagnóstico con estados explícitos por componente en Ajustes. Manuales aparte descartados por el usuario.

## Diferencias declaradas frente a Python

- **Extensión Integral `2.10.0` (E-P1-C2):** `downloader.version` emite `"2.10.0"` (extensión propia de Flow Rust); los comparadores normalizan ese metadato al cotejar el oráculo congelado Python `2.9.0`.
- **Correos:** «módulo» con tilde en la plantilla de Cumplimiento (Python: «modulo»). Decisión del usuario del 25-nov: se conserva la tilde.
- **Meta 4:** mensajes de error de lote con las palabras de `_lote`; 9 hojas auxiliares declaradas del crate (D-13); al regenerar conserva la fecha de corte y filas `fuera_de_meta4`; rechaza plantillas con celdas combinadas o dibujos.
- **R1 a R6 y orden de columnas NURUS:** decisiones del usuario (ver [PLAN.md](PLAN.md)).
- **D-3:** mensaje propio de Rust ante un XML 2003 con entidad externa.
- **Carga en XLS BIFF:** Python escribe `5.0` y Rust `5`. En HTML y XLSX coinciden.
- **Historia:** Rust la lee siempre que se elige «Historia de la causa»; Python solo al retomar.
- **Lecturas:** el puente de Rust no trae el `code` de la extensión, y las fallas quedan con el código genérico.
- **Paquete:** sin `include_originals` (ninguna ruta de Python lo activa).

## Documentos

- Lectura al iniciar: `AGENTS.md`, este archivo, [PLAN.md](PLAN.md) y, para continuar una sesión anterior, [CONTINUACION.md](CONTINUACION.md).
- Según necesidad: [VERIFICACION.md](VERIFICACION.md), [DISENO.md](DISENO.md), [REGISTRO_IMPLEMENTACION.md](REGISTRO_IMPLEMENTACION.md), [REVISION_20261124.md](REVISION_20261124.md), [ANALISIS_INFORME_RUS_SITFA_20261124.md](ANALISIS_INFORME_RUS_SITFA_20261124.md) e [informes/](informes/README.md).
- Fuera de esta carpeta solo se conserva `docs/evidencia_p0/golden/` (usado por las pruebas del workspace `rust/`). La documentación del proyecto Python sigue en `flow-python`.
- [historico/](historico/) guarda el registro, los encargos, la matriz y los informes anteriores al 24 de noviembre. No se lee al iniciar.
