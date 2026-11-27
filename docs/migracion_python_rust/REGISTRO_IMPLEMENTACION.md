# Registro de implementación · Flow Rust

Entradas desde el 24 de noviembre de 2026. Lo anterior (turnos 1 a 17, auditorías y entregas E-B a E-Q6) está en [historico/REGISTRO_hasta_20261124.md](historico/REGISTRO_hasta_20261124.md). El estado vigente y el siguiente paso están en [ESTADO.md](ESTADO.md); este archivo no se lee al iniciar.

## Formato

Una entrada por encargo, de diez líneas como máximo, la más reciente arriba:

```
## <fecha> · <encargo> · <commit>
- Cambió: <archivos o módulos>
- Acredita: <prueba u oráculo y su conteo>
- Diferencias declaradas: <ninguna | detalle>
- Pendiente: <ninguno | detalle>
```

No se copian salidas completas ni listas de mutaciones: la prueba afirma sus conteos y la CI del commit es la evidencia.

## 2026-11-27 · Tres pestañas en el Descargador Integral
- Cambió: Descarga con tres pestañas en el flujo habitual, incluidos listados, empaquetado, descarga conjunta y recuperación.
- Reúne los listados en un lote estándar, valida alcance y huellas y conserva fases de respaldo, comprobantes y diagnósticos.
- Contexto, PDFs y filtros privados mantienen la pestaña principal. Si falla la comprobación de conexiones auxiliares, el lote se ejecuta desde la principal.
- Evidencia: prueba sintética de cinco pantallas, recuperación de un lote parcial y cotejo de tablas y cobertura del paquete.
- Pendiente: CI del conjunto y prueba en SITFA real de las fases añadidas.

## 2026-11-27 · Piloto de tres pestañas
- Cambió: comparación local de una y tres conexiones de Seguimiento, reparto de consultas completas y guardado de cada fase con su cobertura.
- Evidencia: SITFA ficticio compara cuatro consultas en ambas corridas y detecta tres filas ausentes al retirar una página. La prevalidación rechaza dos códigos de la misma pestaña.
- Diferencia: el piloto usa JSON para consultas vacías, descarta filtros adicionales de formulario y requiere la extensión Integral 2.11.1 recargada.
- Pendiente: CI y medición con SITFA real antes de adoptar la concurrencia en el flujo habitual.

## 2026-11-27 · Revisión DeepSeek de la Tanda 3
- Correos: guardar borradores incorpora los contactos en el hilo de la interfaz y conserva las ediciones en curso.
- SITFA Lite: se valida el contrato de la extensión 1.1.0 para activar el flujo compatible. Modo Registro conserva la pausa fija.
- Integral: la verificación de lote habilita las lecturas de contexto. Historia registra la versión 2.10.0 o 2.11.0 que anunció la extensión.
- Evidencia: escenarios de borradores hermanos y edición concurrente, recorrido Lite multipágina con la extensión anterior, y suite Rust de Flow.
- Pendiente: CI de este ajuste y validación real del cambio de modo en descarga conjunta y de una cancelación en plena descarga.

## 2026-11-27 · Integración de la Tanda 3
- Cambió: se integraron las ramas B (`cefcd77`), C (`b8ed66e`) y A (`358fed1`). El puente registra la búsqueda y descarga conjunta al validar la versión y los protocolos de la extensión.
- Evidencia: [informe A](informes/AGENTE_A_20261127_correos.md), [informe B](informes/AGENTE_B_20261127_meta4_lite.md), [informe C](informes/AGENTE_C_20261127_descargador.md) y [plan técnico](PLAN_TANDA_3_20261127.md).
- Diferencias declaradas: comprobante JSON para consultas vacías, extensiones Integral 2.11.0 y Lite 1.2.0, destinatarios aprendidos al guardar borradores.
- Pendiente: CI del conjunto y prueba operativa.

## 2026-11-26 · Manuales PDF y video guía exclusivos de SITFA Lite (sin imágenes ni referencias al Asistente)
- Cambió: retiradas `descargador_lite/docs/img/flow_lite_*.png`; regenerados `descargador_lite/docs/{SITFA_Lite_Manual_Instalacion.pdf,SITFA_Lite_Manual_Uso.pdf,SITFA_Lite_Video_Instalacion_y_Uso.mp4}` usando únicamente capturas reales de SITFA Lite (`a0..a6*.png`) y de la extensión Chrome (`r01..r07*.png`), sin comparaciones ni menciones a otras herramientas.
- Acredita: revisión visual de las 8 páginas PDF (4 de Instalación + 4 de Uso) y video H.264 720p 30fps (49,8 s · 2,03 MB).
- Diferencias declaradas: ninguna (material destinado exclusivamente a usuarios de SITFA Lite).
- Pendiente: ninguno.

## 2026-11-26 · Manuales PDF y video guía de instalación y uso del Descargador Lite con capturas reales
- Cambió: `app/src/{descargador_ui.rs,ui.rs}` (vistas `--vista descargador-lite-{alcance,ejecucion,resultados}` para capturas), `descargador_lite/build/manual_visual/generar_manuales_y_video_flow_rust.py`, 4 capturas reales (`descargador_lite/docs/img/flow_lite_*.png`), `descargador_lite/docs/{SITFA_Lite_Manual_Instalacion.pdf,SITFA_Lite_Manual_Uso.pdf,SITFA_Lite_Video_Instalacion_y_Uso.mp4}` y `docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md`.
- Acredita: verificación visual de las 8 páginas PDF (4 de Instalación + 4 de Uso, sin rutas personales), video H.264 720p 30fps (48,6 s · 1,93 MB), compilación/pruebas `app/` y verificadores estáticos en PASA.
- Diferencias declaradas: ninguna (solo material visual, capturas y documentación de Descargador Lite).
- Pendiente: ninguno.

## 2026-11-26 · Auditoría integral (seguridad/PII, núcleo, descargador, rendimiento), UI Opción A y consolidado Lite solo originales
- Cambió: `app/src/{theme.rs,ui.rs,descargador_ui.rs,descargador_resultados_ui.rs,meta4_ui.rs,revision_ui.rs,actualizacion.rs,util.rs,config.rs,trabajo.rs,sincronizar.rs,productos.rs,manual.rs,personalizar.rs,producto_excel.rs,copia_excel.rs,outlook.rs,lectura.rs,captura_trabajo.rs,motor/historia.rs}`, `app/src/descargador/{resultados.rs,puente.rs,flujo_paquete.rs,flujo_csmp.rs,paquete.rs,lote.rs,xls.rs,historia.rs,bitacoras.rs,vinculos.rs,lecturas.rs,archivo_local.rs,historico.rs,meta4.rs,extension_integral/{conexion.js,pagina.js}}`, `app/Cargo.toml` (`codegen-units = 1`), `app/qa/generar_ordenes.py`, `app/tests/datos/ordenes.json`, `anonimizador/anonimizador_csmp/safezip.py`, `installer/{Instalar_Flow.cmd,Instalar_Flow.ps1}`, `tools/{verificar_solo_lectura.py,check_real_ids.py,check_contactos.py,inventario_escrituras.py}`, `.github/workflows/flow-rust-local.yml` y `docs/migracion_python_rust/{ESTADO.md,REGISTRO_IMPLEMENTACION.md,informes/*}`.
- Acredita: `exportar_libro_solo_originales_omite_columnas_del_asistente` (modo Lite sin columnas `SITFA_*`), `aplicar_preserva_anterior_si_falla_renombrado_y_rechaza_no_regular`, `cargo test --manifest-path app/Cargo.toml --all-targets --locked` (270 passed: 115 lib + 151 bin + 4 integración, 0 failed), `node --check` sobre la extensión y todos los verificadores estáticos (`verificar_solo_lectura` con `--autoprueba`, `check_real_ids`, `check_contactos`, `inventario_escrituras` 120/120 y `verificar_textos_umbrales`) en PASA.
- Diferencias declaradas: en modo Lite, «Consolidar XLSX» omite las 7 columnas `ORIGEN` (`SITFA_*`) del asistente por instrucción expresa del usuario; el modo Integral / Para CSMP conserva paridad exacta con Python.
- Pendiente: CI Linux/Windows del commit publicado y validación operativa real del usuario (no `ACEPTADO`).

## 2026-11-26 · Limpieza del repositorio y actualización completa de documentación
- Cambió: retirados 66 archivos obsoletos de Flow Python en esta rama (10 workflows de otras ramas, lanzadores `.bat`/`.spec`/`pyproject.toml`, `installer/CSMP.iss`, `portable/`, `prototipos/`, `ui/`, `docs/aceptacion/`, `docs/implementacion/` y 35 scripts/manuales heredados de `tools/`); actualizados `README.md`, `MIGRACION_RUST.md`, `tools/check_links.py` y `docs/migracion_python_rust/{ESTADO.md,PLAN.md,VERIFICACION.md,DISENO.md,CONTINUACION.md,REGISTRO_IMPLEMENTACION.md}` cerrando Tandas 1–2, `E-Q11`, `E-P2-OPT` y Plan Waffle como `HECHO` (CI run `45894872026`).
- Acredita: `tools/probar_app.sh todo`, `check_links.py` (0 enlaces rotos) y los 6 verificadores estáticos (`check_real_ids`, `check_contactos`, `verificar_textos_umbrales`, `inventario_escrituras`, `verificar_solo_lectura` con `--autoprueba` y `verificar_referencia_python`) en PASA.
- Conservado intacto: referencia congelada (`descargador/`, `src/`, `tests/`), `META4_Bitacoras/`, `descargador_lite/`, `anonimizador/`, `contracts/`, `rust/` y `docs/evidencia_p0/golden/`.
- Pendiente: CI de entrega (`workflow_dispatch`, `entrega=true`) y validación operativa real del usuario (nada marcado `ACEPTADO`).

## 2026-11-26 · Plan Waffle (Paso 4): validación diferenciada de fechas en `lote.rs` y `bitacoras.rs` (commit separado)
- Cambió: `app/src/descargador/{lote.rs,bitacoras.rs}`, `app/src/descargador_ui.rs` (limpieza `unused_mut`) y `docs/migracion_python_rust/{ESTADO.md,PLAN.md,REGISTRO_IMPLEMENTACION.md}`.
- Acredita: comprobador común `validar_fechas_formulario` detiene la búsqueda antes de `search` distinguiendo `FLG_Consulta`, `CHK_Consulta`, `FEC_Inicio`, `FEC_Fin`, «todas las fechas» y rangos invertidos, respetando Carga/calendario, alcance persistido al retomar y lecturas frescas en lote nuevo; suite Linux completa, `clippy --lib -D warnings` y los 6 verificadores en PASA.
- Diferencias declaradas: ninguna (mantiene contratos de Carga, calendario, descarga conjunta y no reutilización entre lotes).
- Pendiente: CI Windows en `waffle` y validación con sesión autorizada de SITFA (no `ACEPTADO`).

## 2026-11-26 · Plan Waffle (Pasos 1–3 y 5): política multi-tribunal con perfil Tomé, productos y oráculos · `be9d9ae` / `450f72e`
- Cambió: `flow-python` (`be9d9ae`: `utilidades.py`, `rus/util.py`, `work.py`, `record_edits.py`, `reglas_espera.py`, `outputs.py`, `resolutions.py`, `test_reglas_informes.py`), sincronizado en `src/`/`tests/`/`descargador/`; `app/src/{util.rs,motor.rs,productos.rs,ui.rs}`; `rust/crates/{flow-dominio,flow-motor}`; `rust/oraculo/generar_casos.py` + `rust/casos/*.json`; `app/qa/REFERENCIA_PYTHON.json`; `.github/workflows/flow-rust-local.yml`; `app/{MECHANICS.md,mechanics.json}`.
- Acredita: perfil Tomé en otros tribunales válidos (DCE solo correo) y revisión manual sin observación/acciones ante vacíos, marcadores o rótulos ambiguos; identidad original conservada y aviso de matriz faltante; Python `test_reglas_informes` + `test_retomar_paquete` 27 passed (5 xfailed), `flow-motor` 10/10 (924/924/911/943 casos), `app/` verde y 6 verificadores en PASA (`REFERENCIA_PYTHON: be9d9aee717d`, 0 diferencias).
- Diferencias declaradas: ninguna (criterio compartido en los tres motores).
- Pendiente: push de `flow-python` (`be9d9ae`) y paridad a `waffle`, CI Windows (`entrega=true`) y validación operativa real (no `ACEPTADO`).

## 2026-11-26 · E-P2-OPT: calibración real de Modo Rápido Seguro, aviso extensión 2.10.0 y limpieza de rendimiento/clippy · `b9402bf`
- Cambió: `descargador/{preferencias.rs,bitacoras.rs,flujo_paquete.rs,puente.rs}`, `descargador.rs`, `descargador_ui.rs`, `descargador_resultados_ui.rs`, `lectura.rs`, `sincronizar.rs`, `captura_trabajo.rs`, `producto_excel.rs`, `productos.rs`, `trabajo.rs`, `ui.rs`, `util.rs`, `rust/crates/flow-meta4/src/plantilla.rs`.
- Acredita: habilita concurrencia 2–3 de bitácoras también con `calendarios`/`historico` activos, calibra `PausaAdaptativa` para latencia real de SITFA (arranca en 150–350 ms con tope 2–3; solo frena en ≥ 4 s / ≥ 8 s o fallas), aplica pausa rápida en listados/informes, avisa en UI y evento `warning` si Chrome sigue en extensión `2.9.0`, elimina recompilación de `Regex` por celda HTML en `lectura.rs` y limpia advertencias de clippy; suite Linux completa y 6 verificadores PASA.
- Diferencias declaradas: ninguna adicional sobre el contrato Python (Modo Rápido sigue siendo opt-in, solo lectura y sin caché entre lotes del día).
- Pendiente: CI Windows en `waffle` y nueva medición real con extensión `2.10.0` recargada (no `ACEPTADO`).

## 2026-11-26 · E-P1-C2: puente HTTP chunked/acotado, extensión 2.10.0, Modo Rápido Seguro V-01..V-04 y display (Agente C · ChatGPT)
- Cambió: `descargador/{puente.rs,extension.rs,preferencias.rs,bitacoras.rs,historia.rs,flujo_paquete.rs}`, nueva extensión propia `descargador/extension_integral/*` (2.10.0), `descargador_ui.rs`, `descargador_resultados_ui.rs`, `meta4_ui.rs` e informe `AGENTE_C_20261126_puente_rapido_display.md`.
- Acredita: lector HTTP loopback con `chunked`, presupuestos diferenciados (256 KiB control / 112 MiB datos), timeouts y recuperación de `Mutex`; índice DOM por página (`V-01`), pipelining local (`V-02`), pausa adaptativa (`V-03`) y concurrencia acotada 1–3 con degradación (`V-04`), sin reutilización entre lotes del día; Descargador en 3 pasos + Dos Calendarios + índice de Resultados en memoria; suite Linux y 6 verificadores PASA.
- Diferencias declaradas (orquestador): `downloader.version` emite `"2.10.0"` (extensión propia de Flow Rust; comparadores de `paquete.rs` y `descargador_ui.rs` validan `"2.10.0"` y normalizan contra el oráculo congelado Python 2.9.0); `reutilizar_de_otro_lote` se reemplaza por prueba que exige lecturas frescas.
- Pendiente: CI Windows y perfilado/validación con Chrome y SITFA reales (no `ACEPTADO`).

## 2026-11-26 · E-P1-B2: cuotas XML/ZIP, rutas canónicas, lectura única de historias y flow-meta4 (Agente B · ChatGPT)
- Cambió: `descargador/{xls.rs,paquete.rs,resultados.rs,flujo_csmp.rs}`, `descargador.rs`, `lectura.rs`, `rust/crates/flow-meta4/{src,tests}/plantilla.rs`, `tools/inventario_escrituras.py` e informe `AGENTE_B_20261126_cuotas_lectores.md`.
- Acredita: cuotas XML/ZIP con `Read::take(max+1)` real, `ss:Index <= 16_384`, profundidad XML ≤ 128 y dimensiones OOXML; validación canónica anti-traversal y carga única de historias en `paquete.rs`; consolidado en memoria en `resultados.rs`; presentación en `flujo_csmp.rs`; fix de underflow `count="01"` y cuota ZIP en `flow-meta4` (32/32 pruebas); suite Linux y 6 verificadores PASA.
- Ajuste del orquestador: comparte los validadores puros XML/ZIP desde `descargador::xls` (`pub mod xls`) para compilar tanto `--lib` como `--bin` sin duplicación y pasar `clippy --lib -D warnings`.
- Pendiente: CI Windows (no `ACEPTADO`).

## 2026-11-26 · E-P1-A2: estabilidad UI BackgroundJob, on_exit, WCAG AA, rediseños y COM seguro (Agente A · ChatGPT)
- Cambió: `main.rs`, `ui.rs`, `theme.rs`, `outlook.rs`, `copia_excel.rs`, `preparados.rs`, `productos.rs`, `autodiagnostico.rs`, `util.rs`, `tools/inventario_escrituras.py` e informe `AGENTE_A_20261126_ui_worker_display.md`.
- Acredita: `BackgroundJob<T>` para COM/Word/Anonimizador/Actualizaciones, `on_exit()` + guardado al cambiar vista, `.id_source` único por registro, error inline en `Corte`, 1× `copia_excel::base`, timeout + limpieza RCW en scripts COM (`Save()` y CC fijo intactos), paleta WCAG AA ≥ 4,5:1 verificada en prueba, Trabajo con 4 pestañas + teclado + `Ctrl+F`, Ajustes en sub-pestañas, `ScrollArea` en Resultados, `preparados.rs` con `gestiones_correo`/`resolucion`, sanitización XML 1.0/Windows y estados por componente en autodiagnóstico; suite Linux y 6 verificadores PASA.
- Pendiente: CI Windows, pruebas Office COM reales y revisión visual 1100×680 / 1440×900 (no `ACEPTADO`).

## 2026-11-26 · E-P1-A1: integridad de decisiones, Word por fila, conciliación y correos (Agente A · ChatGPT)
- Cambió: `sincronizar.rs`, `copia_excel.rs`, `producto_excel.rs`, `productos.rs`, `conciliar.rs`, `estadisticas.rs`, `ordenes.rs`, `config.rs`, `outlook.rs`, `preparados.rs`, `ui.rs` e informe `AGENTE_A_20261126_integridad_decisiones.md`.
- Acredita: unificación canónica `NURUS_DECISIONES` (límite 32.000 UTF-16, proyección preservada), precedencia Word por fila + protección de ciclos `.rels`, conciliación de `resolucion`/`excluido`, estadísticas con propuestas aprobadas, `null` en OB, procedencia de `Para`, migración Python por fuente, CC exacto y sanitización MIME; `SYNC: 38/38`, suite Linux completa verde y 6 verificadores PASA.
- Ajustes del orquestador: dereferencia `**k != "res"` (`sincronizar.rs:518`), `tiene_res_origen` por fila de paquete (`productos.rs:610`) y paridad `sync.py:177` en `overrides` sin cambio de origen.
- Pendiente: CI de Windows y validación real con Excel/COM y Outlook del usuario (no `ACEPTADO`).

## 2026-11-26 · E-P0-B1: entrega Windows segura, actualizador SHA-256 y anonimizador v1.2.0 (Agente B · ChatGPT)
- Cambió: `installer/{Instalar_Flow.cmd,Instalar_Flow.ps1,Desinstalar_Flow.ps1,FlowRust.iss}`, `tools/verify_flow_rust_installer.ps1`, `app/tests/instalador.rs`, `app/src/{actualizacion.rs,anon.rs}`, `rust/crates/flow-anonimo/src/{archivos.rs,formatos.rs,texto.rs}`, `rust/crates/flow-anonimo/tests/paridad.rs`, `rust/datos/captura_anonima.json`, `tools/inventario_escrituras.py` e informe `AGENTE_B_20261126_entrega_anonimo.md`.
- Acredita: manifiesto de propiedad `FlowRustOwnership/1` y desinstalación sin borrado recursivo, `Instalar_Flow.cmd` con `"%~dp0."` y `-RequireSha256`, SHA-256 obligatorio + publicación atómica y rollback en `actualizacion.rs`, soporte `CSMP_CAPTURE` 1.0/1.1/1.2 y cuota real ZIP en `flow-anonimo` (40/40 pruebas del crate), suite Linux completa verde y 6 verificadores PASA.
- Ajuste del orquestador: `Sesion::persona` omite `STATE_WORDS` (p. ej. `Centro` en `bitacoras.author`) para no corromper `origin=CENTRO` en v1.2.0 (`author` sigue seudonimizándose). Workflow actualizado con `rust/**`, `descargador_lite/**` y `installer/LEEME.txt`.
- Pendiente: CI de Windows (incluyendo verificador PS1 en runner real) y validación del usuario (no `ACEPTADO`).

## 2026-11-26 · E-P0-C1: rango único de captura, pausa en reanudación y Meta 4 (Agente C · ChatGPT)
- Cambió: `app/src/descargador/{bitacoras.rs,flujo_paquete.rs}`, `app/src/{descargador_ui.rs,descargador_resultados_ui.rs,meta4_ui.rs}`, `tools/inventario_escrituras.py` e informe `AGENTE_C_20261126_rango_captura.md`.
- Acredita: lectura de fechas visibles en todas las acciones de descarga, opción explícita «Todas las fechas», persistencia de alcance/pausa/versión en `proceso.json` y `bitacoras.rs`, publicación segura de pareja ZIP + diagnóstico hermano, separación de período de observaciones vs alcance acreditado en Meta 4, corrección de botones `Quitar` y `leer()` en Resultados; suite Linux completa verde y 6 verificadores PASA.
- Diferencias declaradas: descarga conjunta CSMP se bloquea con rango explícito hasta ampliar su contrato (`flujo_csmp.rs`, Tanda 2).
- Pendiente: CI de Windows y repetición real del rango del usuario (no `ACEPTADO`).

## 2026-11-26 · Destinatarios y firma de Outlook, período de Meta 4

- Cambió: migración de destinatarios y alias de Flow Python, resolución al volver a preparar y edición de borradores en Outlook clásico sin reemplazar la firma.
- Cambió: Desde y Hasta inclusivos en Meta 4, guardados en el reporte y conservados al regenerar.
- Acredita: pruebas sintéticas de migración sin sobrescribir, destinatarios al volver a preparar, orden de operaciones de Outlook, extremos del período y regeneración.
- Pendiente: comprobación de la firma y el catastro con el perfil de Outlook del usuario.

## 2026-11-24 · E-A5: cotejos de correos, órdenes de búsqueda y edición masiva

- Cambió: `ordenes.rs` (nuevo; lista en Trabajo), `trabajo.rs` (`aplicar_masivo`, `deshacer`), `ui.rs` (barra de edición masiva) y los oráculos `app/qa/generar_correos.py` y `app/qa/generar_ordenes.py`, agregados a la regeneración de la CI.
- Acredita: `CORREOS: 18/18` contra `prepare_drafts` (con 14 correos de Cumplimiento que difieren solo en «modulo»/«módulo», declarado); `ORDENES: 21/21` contra `avisos_orden`; `la_edicion_masiva_cambia_solo_lo_pedido_y_se_deshace`; los dos `oraculo_de_*_detecta_alteraciones`; `tools/probar_app.sh todo`: 64 y 76 pruebas, 0 fallos.
- Diferencias declaradas: las de [ESTADO.md](ESTADO.md) (F02, F08 y F12).
- CI de Windows en verde (run 45449454854). Pendiente: decisión del usuario sobre la tilde de «módulo».

## 2026-11-24 · E-T3: estadísticas y métricas

- Cambió: `estadisticas.rs` (nuevo), `config.rs` (`metricas_locales`) y `ui.rs` (tarjeta en Resultados, exportar resumen, opción en Ajustes y registro de importación, exportación y borrador); oráculo `app/qa/generar_estadisticas.py` → `app/tests/datos/estadisticas.json`, agregado a la regeneración de la CI.
- Acredita: `ESTADISTICAS: 14/14` contra `statistics.summarize`, `export_summary` y `review_metrics` (conteos, hojas, límite de 60 ciclos, archivo dañado, opción apagada); `oraculo_de_estadisticas_detecta_alteraciones` y `el_resumen_de_un_trabajo_cuenta_lo_que_la_persona_completo`; `tools/probar_app.sh todo`: 64 y 71 pruebas, 0 fallos; `inventario_escrituras` PASA (78 en 21).
- Diferencias declaradas: las de [PLAN.md](PLAN.md).
- CI de Windows en verde (run 45447605617).

## 2026-11-24 · E-T2: formularios manuales

- Cambió: `manual.rs` (nuevo), `productos.rs` y `outlook.rs` (adjuntos libres verificados por huella), `preparados.rs` (huella con adjuntos), `ui.rs` («Correo manual» y «Word manual»); el oráculo `generar_personalizacion.py` suma 11 Word y 8 bloques.
- Acredita: `MANUAL: 19/19` contra `generate_word`, `block_variables` y `render_block`; `oraculo_del_word_manual_detecta_alteraciones`; `un_correo_manual_exige_destinatario_y_conserva_la_huella_del_adjunto`, `lo_que_se_escribe_y_los_favoritos_se_recuperan` y el flujo `un_correo_manual_se_guarda_sin_excel_con_su_adjunto_y_el_cc_fijo`; `tools/probar_app.sh todo`: 64 y 68 pruebas, 0 fallos; `inventario_escrituras` PASA (76 en 20).
- Diferencias declaradas: las de [PLAN.md](PLAN.md).
- CI de Windows en verde (run 45445728806).

## 2026-11-24 · E-T1: cuenta de Outlook y fallos al guardar

- Cambió: `outlook.rs` (nuevo; sale de `productos.rs`), `config.rs` (`cuenta_outlook`), `preparados.rs` (recibo «incierto») y `ui.rs` (cuenta en Ajustes, bloqueo de reintento y «Ya lo revisé en Outlook»).
- Acredita: `el_script_solo_guarda_y_nunca_envia`, `el_cc_fijo_y_la_cuenta_viajan_al_script`, `un_fallo_antes_de_guardar_se_distingue_de_uno_despues` (cuenta inexistente, fallo previo, fallo tras `PREPARADO`, identidad incompleta) y `un_guardado_incierto_de_outlook_bloquea_el_reintento_hasta_conciliarlo`; `tools/probar_app.sh todo`: 64 y 63 pruebas, 0 fallos; `inventario_escrituras` PASA.
- Diferencias declaradas: sin prueba con Outlook real; la firma y el cuerpo HTML de Python no se portan.
- CI de Windows en verde (run 45443830208). Pendiente: prueba del usuario con su Outlook.

## 2026-11-24 · E-A4: ajustes

- Cambió: `personalizar.rs` (nuevo), `config.rs` (alias, plantillas anteriores, archivada y categoría), `ui.rs` (alias en Contactos, editor de plantillas, importar matrices y plantilla de «Gestión particular») y `productos.rs` (una plantilla archivada no prepara correos); oráculo `app/qa/generar_personalizacion.py` → `app/tests/datos/personalizacion.json`, agregado a la regeneración de la CI.
- Acredita: `AJUSTES: 44/44` contra `save_contact`, `duplicate_template`, `archive_template`, `restore_template`, `template_validation.variables` e `import_templates`; `oraculo_de_ajustes_detecta_alteraciones`; `una_copia_de_plantilla_sirve_para_gestion_particular_y_una_archivada_no`; `tools/probar_app.sh todo`: 64 y 59 pruebas, 0 fallos; `inventario_escrituras` PASA (71 en 18).
- Diferencias declaradas: las de [PLAN.md](PLAN.md).
- CI de Windows en verde (run 45441868390), con el oráculo regenerado en Windows.

## 2026-11-24 · E-A3: recibos, vigencia y recuperación

- Cambió: `preparados.rs` (nuevo), `ui.rs` (sellado al preparar, aviso «desactualizado», bloqueo al guardar, borradores repetidos, recibos, guardado y recuperación de la sesión), `conciliar.rs` (los productos pasan a la captura nueva marcados `REVISAR_CAPTURA_ANTERIOR`), `productos.rs` y `trabajo.rs` (`Preparados` en la sesión).
- Acredita: `un_producto_desactualizado_no_se_guarda_y_uno_repetido_tampoco` (editar un registro desactualiza el producto, no se guarda, volver a prepararlo sí, el mismo contenido no se duplica, la sesión recupera productos y recibos) y la extensión del flujo de conciliación; `tools/probar_app.sh todo`: 64 y 55 pruebas, 0 fallos.
- Diferencias declaradas: huellas propias de Rust; sin estado «incierto» ante un fallo de Outlook.
- CI de Windows en verde (run 45439994977).

## 2026-11-24 · E-A2: conciliación entre capturas

- Cambió: `conciliar.rs` (nuevo), `ui.rs` (abrir un paquete con otro abierto, diálogo «Conciliar capturas», resumen «Frente a la captura anterior» y etiqueta por fila), `Trabajo.cambios` y el oráculo `app/qa/generar_conciliacion.py` → `app/tests/datos/conciliacion.json`, agregado a la regeneración de la CI.
- Acredita: `CONCILIACION: 12/12` contra `capture_reconciliation.reconcile` (mismo paquete, identidad perdida, fuente cambiada con y sin edición, conservar, reiniciar, respuesta ajena, faltantes); `oraculo_de_conciliacion_detecta_alteraciones`; flujo `un_paquete_nuevo_conserva_decisiones_y_pide_respuesta_ante_un_conflicto`; `tools/probar_app.sh todo`: 64 y 54 pruebas, 0 fallos.
- Diferencias declaradas: las de [PLAN.md](PLAN.md).
- CI de Windows en verde (run 45437385372), con el oráculo regenerado en Windows.

## 2026-11-24 · E-U1: Abrir en Trabajo (D-9)

- Cambió: `descargador_ui.rs` (botón «Abrir en Trabajo» bajo el paquete creado y `paquete_para_trabajo`) y `ui.rs` (abre el paquete y pasa a Trabajo).
- Acredita: la extensión de `ui_descargador_bitacoras_y_paquete` (el clic deja la ruta del paquete, que se recoge una sola vez); `tools/probar_app.sh todo`: 64 y 51 pruebas, 0 fallos.
- Diferencias declaradas: sin solicitudes, respuestas ni recuperación como Python (decisión del usuario, opción B).
- CI de Windows en verde (run 45432865102).

## 2026-11-24 · E-A1: Actualizar desde Excel

- Cambió: `sincronizar.rs` (nuevo), `Registro` (`campos`, `id_excel`, `sync_base`, `sync_local`, guardados en la sesión), `producto_excel.rs` (escribe `FECHA_OBS`, `TT` y `CC`), `ui.rs` (botón, diálogo de conflictos y memoria de la última copia) y el oráculo `app/qa/generar_sync.py` → `app/tests/datos/sync.json`.
- Acredita: `SYNC: 12/12` contra `nurus.personal.sync.refresh` (remota, local, conflicto y sus resoluciones, misma edición, RES, sin columna técnica, identidad cambiada, faltan registros); `oraculo_de_sync_detecta_alteraciones`; `la_copia_guardada_se_importa_sin_cambios`; flujo `guardar_la_copia_y_actualizar_desde_excel_trae_la_revision`; `tools/probar_app.sh todo`: 64 y 51 pruebas, 0 fallos.
- Diferencias declaradas: las de [PLAN.md](PLAN.md) (observación vaciada, origen editado, `NURUS_DECISIONES`).
- CI de Windows en verde (run 45429755148), con el oráculo regenerado en Windows. La CI regenera solo los oráculos que cambian (a78 no coincide byte a byte en Windows desde antes). Pendiente: copia preservada de Excel de escritorio sin prueba con Excel real.

## 2026-11-24 · E-W1: producto de Trabajo sin consolidado

- Cambió: `producto_excel.rs` (nuevo), `captura_trabajo.rs` (columnas completas y avisos agrupados), `copia_excel.rs` (motivo sin consolidado), `descargador/paquete.rs` (`CONSOLIDADO_OMITIDO`), `revision_ui.rs` (filtros de atención) y `ui.rs` (avisos del paquete).
- Acredita: `paquete_sin_consolidado_exporta_la_estructura_de_flow_python` (columnas SITFA, orden de revisión, clave de ingreso, hojas y trazabilidad), la extensión de `paquete_con_consolidado` y `la_revision_destaca_las_filas_sin_bitacora_y_con_advertencia`; `tools/probar_app.sh todo`: 64 y 47 pruebas, 0 fallos; `inventario_escrituras` PASA.
- Diferencias declaradas: libro nuevo en lugar de copia preservada; `FECHA_OBS`, `TT` y `CC` salen vacías hasta que Trabajo las modele; avisos agrupados solo en pantalla.
- CI de Windows en verde (run 45420239558). Pendiente: revisión del usuario con su paquete.

## 2026-11-24 · E-S1: vista de Resultados del Descargador

- Cambió: `app/src/descargador_resultados_ui.rs` (nuevo) y su sección en `descargador_ui.rs`, equivalente a `ventana_resultados.py`: carpeta del lote, «Permitir resultados INCOMPLETOS», tabla de consultas con su compatibilidad CSMP, y los botones Consolidar XLSX, Libro CSMP, Añadir informes al cruce, Comparar con otro lote, CSV, Calendario ICS, ZIP y Abrir carpeta. Cada producto recarga y verifica el lote en un hilo.
- Acredita: `ui_resultados_abre_un_lote_y_crea_sus_productos` (CSV de la vista igual al de Python, rechazos de Libro CSMP e ICS, lote alterado rechazado) y `resultados_cruza_compara_y_rechaza_como_python`; `tools/probar_app.sh todo` en Linux: 64 y 45 pruebas, 0 fallos; `clippy --lib -D warnings` limpio.
- Diferencias declaradas: sin doble clic para abrir el primer archivo ni «Preparar pendientes» (lo cubre «Recuperar pendientes de un lote…»); «Libro CSMP» no lanza CSMP aparte.
- Pendiente: CI de Windows; revisión visual del usuario.

## 2026-11-24 · E-R0

- Cambió: `app/Cargo.toml` y `app/Cargo.lock` (dependencias por ruta `flow-meta4` y `flow-anonimo`); la CI de Windows corre sus pruebas.
- Acredita: `tools/probar_app.sh todo` sin cambios (64 y 43); `cargo test -p flow-meta4` y `-p flow-anonimo` con `serde_json/preserve_order`: 29 y 38 pruebas, 0 fallos.
- Diferencias declaradas: dos versiones de `rust_xlsxwriter` y de `quick-xml` en el ejecutable; `calamine` con `dates` unificado (sin efecto en la suite).
- Pendiente: compilación y pruebas en Windows (CI).

## 2026-11-24 · E-Q7

- Cambió: `descargador/bitacoras.rs` (`ejecutar_con_avance`, eventos `query`, `diary_progress` y `warning`) y `descargador_ui.rs` (filas revisadas, estado y avisos).
- Acredita: `lote_de_bitacoras_igual_a_python` afirma una fila de avance por registro en los 16 escenarios (`AVANCE`); `ui_descargador_bitacoras_y_paquete` comprueba las etiquetas.
- Pendiente: ver la lectura real con la extensión (usuario).

## 2026-11-24 · E-V3

- Cambió: `.github/workflows/flow-rust-local.yml` (trabajo Linux, detección de cambios, entrega a pedido) y `motor/historia.rs` (el paquete B usa `prueba_entorno::requerida`).
- Acredita: localmente, lo que corre el trabajo Linux: `cargo test --locked --all-targets` (64 y 43, 0 fallos) y los cinco verificadores en PASA.
- Pendiente: primer run en GitHub (no verificable desde esta sesión).

## 2026-11-24 · E-V1 y E-V2

- Cambió: `descargador/`, `src/` y `tests/` igualados a `flow-python` `afc4d12` (9 archivos); `app/qa/REFERENCIA_PYTHON.json`, `tools/verificar_referencia_python.py` y la CI (checkout del SHA y verificación); `bitacoras.json` y `paquete.json` regenerados; `prueba_entorno.rs` con rutas por omisión; `x11` para Linux en `app/Cargo.toml`; `contrastar_bitacoras.py` pide `--include-ignored`.
- Acredita: `tools/probar_app.sh todo`; contraste V2 local contra `afc4d12` (10 casos V1 y 5 P2); `verificar_referencia_python.py` en PASA.
- Diferencias declaradas: ninguna nueva. Rust ya seguía la revisión de bitácoras de `afc4d12`.
- Pendiente: CI de Windows; E-V3.

## 2026-11-24 · E-S1 · `d56c1ed` y el commit de cierre

- Cambió: `descargador/resultados.rs` (CSV, ICS, ZIP de entrega y comparador), `excel.rs` (hoja con formato de cuerpo), oráculo `app/qa/generar_resultados.py` → `app/tests/datos/resultados.json`, agregado a la CI.
- Acredita: `PRODUCTOS: 13/13`; pruebas permanentes `oraculo_de_productos_detecta_alteraciones` y `comparador_de_excel_detecta_alteraciones`; `tools/probar_app.sh todo`.
- Diferencias declaradas: sin `include_originals` (ninguna ruta de Python lo activa).
- Pendiente: CI de Windows; ofrecer los productos en la vista de Resultados.

## 2026-11-24 · D-8: retiro de la documentación Python de esta rama

- Cambió: se retiraron 656 archivos de `docs/` y, de la raíz, `CONTINUAR.md`, `PLAN_VIGENTE.md` y `RESUMEN_PROYECTO_NURUS_CSMP_20261009.md`. `README.md` reescrito. `docs/` pasó de 36 MB a 1,2 MB.
- Se conservan: `docs/migracion_python_rust/`, `docs/aceptacion/`, `docs/evidencia_p0/golden/`, `docs/implementacion/linea_base.json` y `docs/implementacion/verificacion/FLOW_D.json` (los usa `rust/`).
- Acredita: `tools/probar_app.sh todo` sin fallos; `check_real_ids`, `check_contactos`, `verificar_textos_umbrales` e `inventario_escrituras` en PASA.
- Diferencias declaradas: 18 archivos de pruebas y herramientas Python citan documentos retirados; esas pruebas se ejecutan en `flow-python`. No se ejecutó la suite Python en esta rama.
- Pendiente: flujos de CI de otras ramas, `rust/*.md`, `prototipos/` y el árbol Python (tras E-V1) siguen sin tocar.

## 2026-11-24 · E-Q5d y parte de E-S1 · `c734f84` (otra sesión)

- Cambió: `descargador/excel.rs` y `descargador/resultados.rs` (nuevos), `paquete.rs`, `xls.rs`, `descargador_ui.rs` y el oráculo `app/qa/generar_excel.py`.
- Acredita: `EXCEL_BITACORAS: 3/3` y `LIBROS: 14/14`. Repetido tras la fusión con la consolidación: `tools/probar_app.sh todo`, `61 passed` y `43 passed`, 0 fallos.
- Pendiente: CI de Windows de ese commit; de E-S1 faltan CSV, ICS, ZIP y el comparador. Esa sesión no dejó entrada de registro; esta la escribió la sesión de consolidación a partir del mensaje del commit.

## 2026-11-24 · Consolidación de la documentación y decisiones D-5 a D-7

- Cambió: `ESTADO.md` (nuevo), `PLAN.md` y `VERIFICACION.md` (reescritos), este registro (rotado), `AGENTS.md`, `MIGRACION_RUST.md`, `CONTINUAR.md` y `tools/probar_app.sh` (nuevo). Catorce documentos pasaron a `historico/`. En `app/`, solo el mensaje de `prueba_entorno.rs`, que ahora remite a `VERIFICACION.md`.
- Acredita: `tools/probar_app.sh todo` en Linux, `58 passed` y `43 passed; 10 filtered out`, 0 fallos (antes de fusionar `c734f84`).
- Decisiones del usuario: D-5 (el Descargador recolecta y Flow procesa), D-6 (F27 sigue en el alcance) y D-7 (orden Descargador, exportaciones, Asistente). Detalle en [PLAN.md](PLAN.md).
- Pendiente: D-8; E-V1 a E-V3. No se eliminó ningún archivo ni se cambió la CI.

## 2026-11-24 · Revisión del plan y de la documentación · `751634b`

- Cambió: [REVISION_20261124.md](REVISION_20261124.md). Sin cambios de código.
- Hallazgos: los oráculos de `app/qa` usan el Python de esta rama, que difiere de `flow-python` (`afc4d12`) en `descargador/flujo_paquete.py` y `src/nurus/personal/revision_bitacoras.py`; la matriz tenía capacidades pendientes sin encargo; la suite corre en Linux con `--features eframe/x11`, sin `.aislado`.

## 2026-11-24 · E-Q5 y E-Q6 · `61b307c` a `09448d4`

- Cambió: `descargador/paquete.rs`, `descargador_ui.rs` y `descargador/preferencias.rs`.
- Acredita: `PAQUETE: 5/5` contra `export_capture` y `package_current_lot`; `ui_descargador_bitacoras_y_paquete`.
- Diferencias declaradas: las de [ESTADO.md](ESTADO.md).
- Pendiente: E-Q5d y E-Q7.

## 2026-11-24 · E-T4 · Anonimizador con `flow-anonimo`

- Cambió: `app/src/anon.rs` (motor propio reemplazado por `flow_anonimo::lote::ejecutar`), filtro de archivos en `ui.rs`, `tools/inventario_escrituras.py`.
- Acredita: `anon::tests` (texto, ZIP y paquete de captura sin identidades y con el original intacto; captura alterada sin copias) y las 38 pruebas del crate.
- Diferencia declarada: el informe no cuenta reemplazos por tipo.
- CI Windows verde (run 45457412572). Pendiente: E-R1 y E-R2 (adaptador de lotes).

## 2026-11-24 · E-T5 · Autodiagnóstico y ayuda

- Cambió: `app/src/autodiagnostico.rs`, `ayuda.rs`, `main.rs` (`--self-test`), Ajustes en `ui.rs`, CI y `tools/inventario_escrituras.py`.
- Acredita: `autodiagnostico::tests` (7 comprobaciones pasan, el informe no se sobrescribe, código 2 si existe) y `ayuda::tests` (una sección por vista y reglas fijas).
- Diferencia declarada: sin `portable/` ni instalador; el equivalente de `package_check.py` cubre lo que existe en Rust.
- CI Windows verde (run 45458302666).

## 2026-11-24 · Diagnóstico anónimo de calendarios e históricos

- Cambió: `app/src/descargador/bitacoras.rs` escribe además `diagnostico_calendarios.json` (estados de calendario e histórico y códigos `CAL_*`, sin texto libre). Extensión de Rust: `diagnostico_bitacoras.json` queda igual al de Flow Python, porque `descargador/` es la referencia congelada (`verificar_referencia_python.py`).
- Motivo: el diagnóstico del lote del 24-nov (114 bitácoras, 92 historias, 114 históricos, todo LEIDA) no mostraba calendarios, y de ahí o del calendario de informes por mes salen las lecturas parciales que reporta el usuario.
- Acredita: `diagnostico_cuenta_calendarios_e_historicos_sin_texto_libre`.
- Pendiente: un diagnóstico nuevo del usuario con este archivo; luego E-R1.
- CI: los runs 45461517036 (verificador de la referencia congelada) y 45462587676 fallaron; este último por el detector de oráculos, que comparó contra el push anterior (donde `descargador/` había cambiado y se restauró) y regeneró «todos», incluido `a78`, que no es idéntico byte a byte desde antes. Sin cambios de código en la corrección.
- CI: con «todos» ya no se regenera `a78` (ZIP en base64 no reproducible entre ejecutores); se contrasta cuando cambian su generador o sus datos.
- CI Windows verde (run 45463811153, commit `8cd013b`).

## 2026-11-24 · Análisis del informe técnico RUS/SITFA del usuario

- Documento: `ANALISIS_INFORME_RUS_SITFA_20261124.md`, sobre `a47b1e3`. Sin cambios de código.
- Conflictos: el informe parte de otra rama (`9707e72`, ausente en el clon), da a Rust por congelado (lo contradicen D-4 y D-5) y propone un registrador que `AGENTS.md` prohíbe.
- Riesgos heredados de Python, con paridad exacta:
  - `conciliar.rs:344` rotula «egresadas» a las filas ausentes sin mirar la cobertura (`comparar_lotes` sí la mira);
  - R11 cierra la orden por cercanía y no aplica R2 (`motor/historia.rs:442`);
  - C10 se suprime con cualquier nomenclatura posterior (`escritos.rs:120`).
- Propuestas: decisiones D-10 a D-12, más E-X1 (guardia de solo lectura) y E-X2 (diagnóstico de Notificación y Diligencias).

## 2026-11-24 · E-R1 y E-R2 · Meta 4 desde los lotes del Descargador

- Cambió: `app/src/descargador/meta4.rs` (lector del formato del Descargador: `calcular`, `fecha_lectura`, `tribunales_disponibles`), `app/src/meta4_ui.rs` y la vista «Meta 4» en `ui.rs`, `ayuda.rs` y `main.rs`; `paquete.rs` expone `leer_registro`, `Clave` y `Fuente::original`; `flow-dominio` por ruta en `app/Cargo.toml`.
- Acredita: `meta4::tests` (lote y bitácoras de Flow Python pasados a Residencia: registros, identidad visible, lecturas, inventario, publicación de los cinco productos, rechazo de lote duplicado, tribunal ajeno, lote incompleto y bitácoras ausentes) y `meta4_ui::pruebas` (genera y regenera). `tools/probar_app.sh todo`: 68 y 84 pruebas, 0 fallos; clippy de la biblioteca limpio.
- Diferencias declaradas: no lee `bitacoras_meta4.json` ni carpetas de reportes Excel sueltos; sin `PERIODO_INCOMPLETO` (Python no lo tiene).
- Pendiente: oráculo R1b (`meta4_bitacoras.generar` sobre un lote del Descargador, fila a fila) y CI de Windows.

## 2026-11-24 · E-X1 y E-X2 · Guardia de solo lectura y diagnóstico de pestañas de la causa

- Cambió: `tools/verificar_solo_lectura.py` (con `--autoprueba`) y su paso en la CI; `descargador/historia.rs` (`pestanas_causa`, `resumen_pestanas`, `estructura_pestanas.json` al cerrar); `tools/inventario_escrituras.py` (+2 escrituras).
- Acredita: `--autoprueba` (cada control prohibido y `.Send(` se detectan; los módulos de prueba se ignoran), `pruebas_pestanas` (tablas, filas y encabezados sin valores; vacía frente a ausente) y los oráculos de bitácoras e historia sin cambios. `probar_app.sh todo`: 69 y 84 pruebas, 0 fallos; clippy limpio.
- Origen: análisis del informe técnico RUS/SITFA del usuario.
- Pendiente: el usuario envía `estructura_pestanas.json` de un lote real; de ahí sale la decisión sobre un lector de Notificación y Diligencias.

## 2026-11-24 · E-R1b · Oráculo de Meta 4 y corrección de Windows

- Causa del rojo de los runs 45470928947 y 45471386495: `meta4::tests` devolvía «BITÁCORA NO VERIFICABLE» en Windows; `copia_local` compara la carpeta con la ruta canónica y `leer_bitacoras` no la canonicalizaba (nombre corto de 8.3). Corrección: `paquete::rutas_bitacoras` y su prueba con un enlace simbólico (falla sin la corrección).
- Cambió: `app/qa/generar_meta4.py` (22 casos, `meta4.json`, entrada nueva en el workflow), `meta4.rs` (mensajes de lote y de tribunales como `_lote`, `pruebas_oraculo`).
- Acredita: `meta4_del_descargador_igual_a_python` (22 casos, 7 errores iguales, 30 filas, 5 casos con libros y reporte) y `oraculo_de_meta4_detecta_alteraciones`. `probar_app.sh todo`: 72 y 84 pruebas, 0 fallos; clippy limpio.
- Diferencias: 9 hojas auxiliares de los productos del crate (fijadas en `HOJAS_DECLARADAS`); `requieren_revision` suma alertas.
- CI: run 45487699648 verde (`linux` y `validar`, `meta4.json` regenerado en Windows). Pendiente: decisión D-13.

## 2026-11-24 · E-A6 · Importación de contactos

- Cambió: `personalizar.rs` (`importar_catastro`, `parse_contactos`, `importar_contactos_archivo`), `ui.rs` (botón «Importar» de Contactos), `app/qa/generar_catastro.py` y `catastro.json` (en el workflow).
- Acredita: `importacion_de_contactos_igual_a_python` (36 casos, 5 errores, 4 conflictos), `oraculo_de_catastro_detecta_alteraciones` y dos pruebas de comportamiento (JSON con tribunales y conflictos; CSV releído). `probar_app.sh todo`: 72 y 88 pruebas, 0 fallos; clippy de la biblioteca limpio.
- Corrige: la importación anterior pisaba contactos, aceptaba direcciones inválidas y no informaba conflictos.
- Diferencias: filas «Tribunal: X» de CSV/HTML actualizan el tribunal (extensión de Rust).
- CI: el run 45502222110 pasó las pruebas y falló al regenerar `catastro.json`: `ZipInfo.create_system` vale 0 en Windows y 3 en Linux, y cambiaba los bytes del `.xlsx`. Fijado a 3 en el generador.
- CI: run 45503587810 verde (`linux` y `validar`, `catastro.json` regenerado igual en Windows).

## 2026-11-25 · D-10 y D-11 (decisiones delegadas) y oráculo de correos

- Decisiones del asistente por delegación del usuario («Decide tú»): D-10 y D-11 aprobadas, D-12 diferida, D-13 conservar (ver `PLAN.md`).
- Cambió: `conciliar.rs` y `ui.rs` (ausentes con/sin cobertura), `motor/historia.rs` (R11 no se redacta con varias órdenes en causa compartida), `util.rs`/`productos.rs` (`hoy()`), `generar_correos.py` y `correos.json` (fecha fija).
- Acredita: `ausente_no_implica_egreso_sin_la_misma_consulta_completa`, `d11_varias_ordenes_en_causa_compartida_no_se_atribuyen` y los oráculos de historia con `d11 == 3` declarados. `probar_app.sh todo`: 72 y 90 pruebas, 0 fallos; clippy limpio.
- Corrige: `correos_iguales_a_python` fallaba al cambiar el día (el oráculo guardaba la fecha de generación).
- CI: run 45609310864 verde (`linux` y `validar`, con el contraste E3 con el ZIP de Python y `correos.json` regenerado en Windows).

## 2026-11-25 · E-A7 · Recálculo al editar columnas de origen en la copia

- Cambió: `sincronizar.rs` (`actualizar` con `Config`, `Resumen.recalculadas`), `ui.rs` (aviso de filas recalculadas), `generar_sync.py` y `sync.json` (17 casos, con propuesta y reglas de Python).
- Acredita: `actualizar_desde_excel_igual_a_python` (17 casos: reglas iguales y propuesta igual salvo R6) y `oraculo_de_sync_detecta_alteraciones`. `probar_app.sh todo`: 72 y 90 pruebas, 0 fallos; clippy limpio.
- Diferencias: solo archivos de RUS (un paquete avisa); se reanalizan todas las filas; `NURUS_DECISIONES` no se combina (sin F08 no hay estado que combinar).
- CI: run 45625544370 verde (`linux` y `validar`, `sync.json` regenerado igual en Windows).

## 2026-11-25 · E-A8 · Gestión de correo por fila y NURUS_DECISIONES

- Cambió: `trabajo.rs` (`gestiones_correo`, `sync_correo`, sesión), `productos.rs` (borradores), `conciliar.rs`, `copia_excel.rs`, `sincronizar.rs` (`leer_decisiones`, combinación) y el selector de la fila en `ui.rs`; `generar_correos.py` y `generar_sync.py` con sus JSON.
- Acredita: `correos_iguales_a_python` (24 casos), `actualizar_desde_excel_igual_a_python` (25 casos), las dos pruebas permanentes que alteran lo esperado y dos de comportamiento. `probar_app.sh todo`: 72 y 92 pruebas, 0 fallos; clippy limpio.
- Diferencias: solo la gestión de correo de `NURUS_DECISIONES` se combina (Python combina todo el estado de la fila); sin exclusión manual, anulaciones de origen ni variables de Word por fila.
- CI: run 45634055296 verde (`linux` y `validar`, `correos.json` y `sync.json` regenerados igual en Windows).

## 2026-11-25 · Decisiones del usuario: reversión de D-10 y D-11

- El usuario respondió las decisiones pendientes: «módulo» con tilde, revertir D-10 y D-11, D-12 diferida y D-13 conservar.
- Cambió: `conciliar.rs`, `estadisticas.rs`, `ui.rs` y `motor/historia.rs` vuelven a su estado anterior a `a8a0338` (se quitan la clasificación de ausentes, el aviso `HISTORIA.R11_NO_ATRIBUIBLE`, los conteos `d11` de los dos oráculos de historia y las dos pruebas propias). Se conserva la corrección del oráculo de correos (fecha fija).
- Acredita: los oráculos de conciliación e historia vuelven a pasar sin casos declarados. `probar_app.sh todo`: 72 y 90 pruebas, 0 fallos; clippy limpio.
- CI: run 45637998137 verde (`linux` y `validar`).

## 2026-11-25 · E-A9 · Recálculo al editar columnas de origen en un trabajo de paquete

- Cambió: `sincronizar.rs` (se quita la restricción a archivos de RUS), `app/qa/generar_sync_paquete.py` y `sync_paquete.json` (en el workflow).
- Acredita: `actualizar_desde_excel_en_un_paquete_igual_a_python` (6 casos con el paquete armado por Python) y `oraculo_de_sync_en_paquete_detecta_alteraciones`. `probar_app.sh todo`: 72 y 92 pruebas, 0 fallos; el contraste E3 con el ZIP de Python (6 pruebas) también pasa en local.
- Decisiones del usuario registradas el mismo día: D-5 confirmada, formato de proyectos Word a elección (E-P1 abierto), M7 no por ahora.
- Entorno: el generador necesita `tkinter` (como los demás de paquetes); la CI de Windows lo trae.
- CI: run 45639115067 verde (`linux` y `validar`, `sync_paquete.json` regenerado igual en Windows).

## 2026-11-25 · E-I2 · Actualización en el lugar y extensión de carpeta fija

- Pedido: corregir sin reinstalar todo (ni la extensión de Chrome). Decididos por el usuario: carpeta local de actualizaciones y aviso con botón.
- Cambió: `actualizacion.rs` nuevo, `ui.rs` (aviso al abrir y sección en Ajustes), `descargador/extension.rs` y `descargador_ui.rs` (carpeta fija, actualización en el lugar, mensajes por caso), `tools/inventario_escrituras.py` y la regla en `AGENTS.md`.
- Acredita: tres pruebas nuevas (aplicar y volver atrás, rechazos y restauración, extensión en el lugar). `probar_app.sh todo`: 73 y 94 pruebas, 0 fallos; clippy de la biblioteca limpio; inventario de escrituras al día.
- Corrige: la huella del ejecutable se calculaba en cada cuadro de Ajustes (la corrida de pruebas pasó de más de 10 minutos a 20 segundos al corregirlo).
- CI: run 45644045890 verde (`linux` y `validar`; sin cambios de oráculos, la regeneración se omitió). Pendiente: prueba del usuario con un `.exe` real (la primera actualización es manual).

## 2026-11-25 · E-U2 · Correcciones de la primera prueba con el .exe

- Hallazgos del usuario: el Descargador no se podía usar con la ventana pequeña, «Validar perfil» no dejaba elegir varios y Resultados rechazó un lote antiguo («falta resumen.json»).
- Cambió: `ui.rs` (desplazamiento general en Descargador y Meta 4), `descargador_ui.rs` (texto de la consulta única) y `descargador/paquete.rs` (mensajes de `cargar_lote`).
- Acredita: `una_carpeta_sin_resumen_dice_que_es_y_que_elegir`. `probar_app.sh todo`: 74 y 94 pruebas, 0 fallos; clippy limpio. El desplazamiento solo lo valida el usuario.
- Sin resolver: abrir conjuntos `flujo.json` en Resultados; hace falta la lista de nombres de archivos del lote antiguo para confirmar su tipo.
- Coordinación: toca `descargador_ui.rs` y `paquete.rs`, que usa el Agente C; integrar con cuidado.
- CI: run 45648680625 verde (`linux` y `validar`). Pendiente: confirmación del usuario con el `.exe` nuevo.

## 2026-11-25 · E-A10 · Anulaciones de origen por fila con recálculo de pares y sincronización

- Cambió: `trabajo.rs` (`anulaciones`, `sync_anulaciones`, `_prepass_peers`, `deshacer`), `sincronizar.rs` (`overrides` en `NURUS_DECISIONES`), `conciliar.rs`, `copia_excel.rs`, `preparados.rs`, `productos.rs` y `ui.rs` (panel "Corregir datos de origen"); oráculos `generar_anulaciones.py` y `generar_sync.py` con sus JSON y workflow.
- Acredita: `anulaciones_iguales_a_python` (13 casos), `actualizar_desde_excel_igual_a_python` (31 casos), `oraculo_de_anulaciones_detecta_alteraciones` y `oraculo_de_sync_detecta_alteraciones`; `las_anulaciones_sobreviven_a_la_sesion_y_conciliacion`.
- Verificadores: 98 pruebas pasan en local; `verificar_textos_umbrales`, `check_real_ids` e `inventario_escrituras` en PASA (0 discrepancias).
- Diferencias declaradas: F08 completado para anulaciones; variables Word y exclusión manual siguen fuera.
- CI: run 45652241372 verde (commit `886d222`, linux y validar), con «Regenerar los oráculos» omitido; la regeneración de `anulaciones` y `sync` sigue pendiente.

## 2026-11-25 · Orquestador · CI de E-A10 e integración del Agente C (E-Q8)

- CI de E-A10 (`faee8ea`, run 45651312385): `validar` falló en `la_edicion_masiva_cambia_solo_lo_pedido_y_se_deshace` («Cannot detect file format»); `linux` pasó. Causa: `demo::libro` escribía siempre `Ejemplo_ficticio_<modo>.xlsx` en la misma ruta temporal y la prueba nueva `las_anulaciones_sobreviven_a_la_sesion_y_conciliacion` la reescribía en paralelo; en Windows una leía el libro a medio escribir. El orquestador (`8f49ce1`, carpeta propia por llamada) y el agente A (`886d222`, nombre único `Ejemplo_ficticio_<modo>_<pid>_<n>.xlsx`) lo corrigieron en paralelo; al integrar se conservó la de A, sin escritura nueva en el inventario.
- E-A10: el agente A editó el workflow (lista de regeneración con `anulaciones`) y marcó `HECHO` sin CI. Se acepta el cambio del workflow; el estado pasa a «implementado» hasta la CI verde (confirmada luego en run 45652241372).
- Agente C (E-Q8): revisado `050574c..190ca49` (solo sus archivos, `paquete.rs` con las firmas protegidas intactas y su línea del inventario); CI de su rama verde (run 45650217541, «Regenerar los oráculos» corrido). Integrado con `git merge --no-ff` (`3689c92`) sin conflictos con E-U2 ni E-A10.
- Acredita en local tras integrar: `tools/probar_app.sh todo` 77 y 97 pruebas, 0 fallos; clippy de la biblioteca limpio; `check_real_ids`, `check_contactos`, `verificar_textos_umbrales`, `inventario_escrituras`, `verificar_solo_lectura` (con autoprueba) y `verificar_referencia_python` en PASA.
- Siguiente: CI de Windows de la rama principal para cerrar E-A10 y E-Q8 como `HECHO`; encargo E-S2 al Agente C.
- Cierre: E-Q8 `HECHO` con el run 45652560712 (`bf27446`; regeneró `bitacoras` y `lecturas`). E-A10 sigue sin `HECHO`: ningún run regeneró `anulaciones` ni `sync` en Windows; se lanzó la CI a mano (`workflow_dispatch`, regenera todos los oráculos).
- CI manual 45655320215 (`1065344`, «todos»): falló `lote.json`, no E-A10. Python escribe los JSON del lote en modo texto y en Windows salen con CRLF. `generar_lote.py` los fija en LF antes del base64 (regenera idéntico en Linux), y el paso de regeneración ahora recorre todos los oráculos y lista al final los que difieren.

## 2026-11-25 · Orquestador · Prueba del Descargador del usuario

- Revisión autorizada por escrito de 8 controles JSON del usuario (estructura y conteos; nada copiado al repositorio): listados `VALIDADA` (2 consultas), 84/84 bitácoras y calendarios `LEIDA`, 66/66 causas de historia; el paquete de la misma sesión trae todo (302 consultas).
- Causa de la impresión de falla: aviso «cobertura incompleta» disparado solo por `BITACORA_ADVERTENCIA (3)`. `paquete::diagnostico_cobertura` ahora distingue advertencias de lectura de motivos que degradan; prueba `advertencias_de_lectura_no_se_anuncian_como_cobertura_incompleta` con la forma del lote (81 + 3) y el caso con una bitácora fallida.
- Pendiente: E-S2 ampliado (juntar carpetas y bitácoras separadas en Resultados) y E-Q9 («Consultar y crear paquete» en una acción, `flujo_paquete.py`).
- CI manual 45657877437 (`622a56b`, «todos»): `sync`, `anulaciones` y los demás coinciden en Windows; fallan `paquete` (JSON con CRLF), `excel` y `resultados` (la carpeta temporal llega con nombre corto `RUNNER~1` y Python la resuelve al largo, así que `<BASE>` no se reemplazaba). Corregido: `Path(tmp).resolve()` como base y JSON en LF con rutas `as_posix()`; los tres regeneran idénticos en Linux.

## 2026-11-25 · Orquestador · Integra al Agente B (E-R3)

E-R3 · Meta 4: contraste ampliado (`92ab9c1`, CI Windows verde, run 45656558813). `generar_meta4.py` pasa a 33 casos (55 filas, 7 errores): Espera, FAE y Residencia en un lote, Laja, Mulchén, dos lotes, filtro de varios tribunales, bordes de la ventana y etapas distintas el mismo día; agrega regeneración (3 escenarios, 11 libros) y plantilla propia (5 plantillas) con 2 hojas auxiliares declaradas más. `plantilla.rs` acepta plantillas sin anchos de columna. Rust conserva en Registros las filas fuera de FAE/Residencia y, al regenerar, la fecha de corte en vez del día 1 del mes.
- Integración: merge `3a2da44`; `probar_app.sh todo` (82 y 97 pruebas, 0 fallos), clippy `--lib` y verificadores en verde. Falta la CI de Windows sobre la rama principal.
- Revisión del informe de B (E-R3, ya integrado por otra sesión): alcance correcto, CI de su rama verde con regeneración corrida; sus diferencias (fecha de emisión al regenerar, filas fuera de FAE/Residencia, plantillas con celdas combinadas) se conservan como decisiones revertibles. E-Q9 pasa al Agente B. Aclaración del usuario: el ZIP de la prueba eran controles de dos procesos en carpetas separadas.
- CI manual 45660388335 (`f7b0481`): `resultados` ya coincide; `paquete` y `excel` no. Causas: `ventana_historia.html` leído con CRLF del checkout de Windows (cambia la huella del HTML, `historias.json` y `capture_id`) y `xlsx_fijo` sin `create_system = 3` (en Windows vale 0 y cambia los bytes del XLS). Corregidos en `generar_paquete.py`; los tres regeneran idénticos en Linux.
- CI manual 45663137240 (`af48086`): `excel` ya coincide y `word_tribunal` queda resuelto por `a986d7c` (ZIP sin compresión, de A). `paquete` difería en `capture_id` y huellas de tablas: Python suma las huellas de `bitacoras.json` e `historias.json`, que en Windows `Path.write_text` escribe con CRLF. `generar_lote.py` (que importan paquete, excel y resultados) hace que `write_text` escriba LF durante la generación; la referencia no cambia.

## 2026-11-25 · E-P1 · Proyectos Word por proyecto o por tribunal (agente A)

- Cambió: `productos.rs` (`generar_word_por_tribunal`, `generar_word_formato`, `lineas_indice`; preserva estilos y quita cursivas; `generar_word` sin cambios), `ui.rs` (selector en Resoluciones, conteos y recibos por archivo de tribunal), `generar_word_tribunal.py` → `word_tribunal.json` y la línea `word_tribunal` del workflow.
- Acredita: `word_por_tribunal_igual_a_python` (6 casos, 7 archivos), `oraculo_de_word_tribunal_detecta_alteraciones` y `por_tribunal_soporta_tribunal_generico_y_excluidos`.
- CI: run 45663684583 (A) y 45665516823 (todos los oráculos) verdes. Dos ejecutores de A en paralelo (Matias Gaete y OpenAI Codex `4966b6d`); el merge `05dfd19` dejó una implementación.

## 2026-11-25 · Orquestador · Cierres con todos los oráculos regenerados en Windows

- Run 45665516823 (`db33e78`, `workflow_dispatch`, «todos»): `linux` y `validar` verdes; regeneración sin diferencias. Cierra como `HECHO`: E-A10, E-P1, la corrección del aviso de cobertura (`980c6ed`), E-R3 sobre la rama principal y el determinismo de los generadores.

## 2026-11-25 · E-P2 · Word por tribunal con imágenes, numeración y relaciones (agente A · Antigravity)

- Cambió: `productos.rs` (`DocumentoCombinado`: `rId` renumerados, medios deduplicados por SHA-256, encabezados y pies con sus relaciones, estilos usados, `numbering.xml` con el reinicio de `docxcompose`, `[Content_Types].xml`); `generar_word_tribunal.py` y `word_tribunal.json` de 6 a 12 casos.
- Acredita: `word_por_tribunal_igual_a_python` (12 casos, 13 archivos), `cada_docx_generado_resuelve_relaciones_y_content_types` y alteraciones de relación y numeración en `oraculo_de_word_tribunal_detecta_alteraciones`. Orquestador en Linux: 82 + 101 + 1 pruebas, `word_tribunal` idéntico.
- CI: run 45676018885 verde (`c52f626`). Observaciones de proceso: citó primero un run inexistente (45672133399) y `3bd07c5` no lleva la línea `Agente:`; el informe `6957323` sí trae firma.
- Duplicado detectado: ChatGPT Luna también implementó E-P2 (`f7ff7c9`, local en su clon, sin publicar; 11 casos). Se detuvo al ver la entrega remota y no hizo push: conducta correcta. Se descarta; Luna pasa a C (E-S2).

## 2026-11-25 · E-Q9 · «Consultar y crear paquete» en una acción (agente B · DeepSeek)

- Cambió: `descargador/flujo_paquete.rs` (nuevo: `run`, `retomar`, `proceso.json`, opciones y textos de Python), una línea en `descargador.rs`, la acción y su avance en `descargador_ui.rs`, `generar_flujo_paquete.py` y `flujo_paquete.json`, su línea del workflow y del inventario.
- Acredita: `flujo_paquete_igual_a_python` (12 casos, 4 errores, 26 fases), `oraculo_de_flujo_paquete_detecta_alteraciones`, `deja_una_sola_carpeta_con_las_fuentes_y_el_zip`, `retomar_resuelve_separadores_y_reutiliza_consultas` y la prueba de vista. Orquestador en Linux tras integrar: 86 + 102 + 1 pruebas; `flujo_paquete` idéntico.
- CI de la rama: 45671145795 y 45672730810 rojos (retomar no reutilizaba; oráculo con `\` de Windows), 45675971822 rojo (hora del ejecutor en el historial), 45682691364 verde con `flujo_paquete` regenerado. Devuelto una vez; el informe corregido trae firma.
- Cierre: `HECHO` con el run 45685219486 de la rama principal (`310d783`; `linux` y `validar` verdes, regeneración corrida).

## 2026-11-25 · E-S2 · Conjuntos y bitácoras de otras carpetas en Resultados (agente C · ChatGPT Luna)

- Cambió: `descargador/archivo_local.rs` (nuevo), una línea en `descargador.rs`, `descargador_resultados_ui.rs` (colección de carpetas y bitácoras), el texto de `cargar_lote` para `flujo.json` en `paquete.rs`, `generar_archivo_local.py` y `archivo_local.json`, su línea del workflow y la del inventario.
- Acredita: `conjuntos_iguales_a_python` (22 casos) y su prueba de alteración, y la prueba de vista. Orquestador tras integrar: 88 + 103 + 1 pruebas; inventario 108 en 24.
- CI de la rama: 45685311235 rojo (bytes del XLSX), 45687407129 rojo solo en `linux` (destino de diagnóstico repetido; regeneración de `archivo_local` sin diferencias), 45688144080 verde. Firma completa (modelo declarado: «GPT-6, ejecución Codex»).
- Al integrar: el generador dependía de cómo Python resuelve «\\» en Linux (caso `separadores_ambos`, con ruta temporal en el error); se reproduce la conducta de Windows y regenera idéntico en Linux.
- Cierre: `HECHO` con el run 45690521141 de la rama principal (`0610040`; `linux` y `validar` verdes, regeneración corrida).

## 2026-11-25 · E-I1 · Instalador por usuario (agente A · Antigravity)

- Cambió: `installer/` (`Instalar_Flow.cmd`/`.ps1`, `Desinstalar_Flow.ps1`, `FlowRust.iss`), `tools/verify_flow_rust_installer.ps1`, `app/tests/instalador.rs` y el paso «Preparar instalador por usuario (E-I1)» del workflow (agregado por el agente sin autorización; aceptado tras revisión).
- Revisión del orquestador: por usuario y sin administrador; el `.cmd` exige la huella; rechaza ejecutables manipulados o en uso; el desinstalador conserva datos y actualizaciones. Observaciones pasadas a E-I3: mensaje «verificado» sin `SHA256.txt` al llamar el `.ps1` directo y desinstalación aceptada por el nombre de carpeta.
- CI: 45668678717 (instalador PowerShell) y 45691070212 (`entrega=true`: `--release`, Inno Setup, todos los oráculos regenerados, contraste V1 y capturas), ambos verdes.

## 2026-11-25 · Integración de E-Q10 (B), E-S3 (C) y revisión de E-I3 (A)

- E-Q10 (B · DeepSeek): `ejecutar_por_hoja` y opción en la vista; oráculo `flujo_paquete` ampliado, determinista e idéntico en Linux. CI de rama 45703174530 y 45704817368 verdes. Firma completa.
- E-S3 (C): `archivo_local` (recordar, descubrir, referencias) e historial en preferencias; Resultados con referencias y «Ver descarga conjunta» con libro verificado por huella; oráculo `historial_local` (14 casos). Ejecutado por Luna y continuado por Terra (los commits dicen Luna). Al integrar: el generador recorría `rglob` sin ordenar y el orden de archivos cambiaba entre Windows y Linux; se ordenó.
- E-I3 (A · Antigravity): carpeta adicional de actualizaciones (OneDrive) con control de sincronización y ajustes del instalador. CI 45703688750 verde (la primera, 45702997351, falló). Sin cambios al workflow; commits firmados.
- Orquestador tras integrar B y C: 92 + 109 + 1 pruebas; `flujo_paquete` e `historial_local` idénticos en Linux; clippy y verificadores en verde.

## 2026-11-25 · Decisión del usuario: fuera del proyecto

- Manuales aparte (F28 pasa a «Sí»), registrador de observaciones RUS y extractores de estructura (`extraer_estructura_*.py`). `AGENTS.md` deja de citar `extraer_estructura_har.py` en la regla de datos; la regla (solo estructura anónima) no cambia. La referencia Python no se toca.
- Cierre: E-Q10, E-S3 y E-I3 `HECHO` (CI de la rama principal verde con todos los oráculos regenerados, runs 45708615263 y 45708628960, sobre `60fe600`).

## 2026-11-25 · E-A11 y E-T6 (agente A · Antigravity)

- Cambió: `sincronizar.rs` y `trabajo.rs` (estado completo de `NURUS_DECISIONES`, como `record_edits` de Python), `generar_sync.py` y `sync.json` (31 → 38 casos), `outlook.rs` (borrador de prueba) y `ui.rs` (botón en Ajustes; búsqueda inicial de actualizaciones en un hilo).
- CI: 45718105751 rojo (`check_contactos` por una dirección real de ejemplo en una prueba; corregido a `example.test` en `b6e0b70`), 45719672640 verde con todos los oráculos (`workflow_dispatch` lanzado por el agente).
- Orquestador: `sync` idéntico en Linux; 92 + 110 + 1 pruebas; clippy, solo lectura, contactos, RUT, textos e inventario en verde; CC fijo por constante y sin `.Send(`.
- Corrida de entrega 45708908715 (`60fe600`) verde: `.exe` final e instalador listos (no incluye E-A11 ni E-T6).

## 2026-11-26 · E-Q11 (agente B · ChatGPT Terra)

- Cambió: `descargador/flujo_csmp.rs` (nuevo: plan, fases, `flujo.json`, libro con huella, recuperación por fase), acción conjunta en `descargador_ui.rs`, «Retomar pendientes» en Resultados, `generar_flujo_csmp.py` y `flujo_csmp.json` (8 casos), una línea del workflow.
- CI de la rama: 45720394795 verde, `flujo_csmp` regenerado. El primer informe se declaró `BLOQUEADO` por falta de Linux; el orquestador regeneró el oráculo en Linux (idéntico) y Terra cerró el informe (`7d4bf17`).
- Orquestador: fusión sin conflictos; 99 + 112 + 1 pruebas; clippy, solo lectura, contactos, RUT, textos, referencia e inventario (122 escrituras) en verde. Aprobado el cambio de B al workflow.
- Descartado: la continuación de E-S3 por Terra (`AGENTE_C_20261125c.md`, no publicada); E-S3 ya estaba `HECHO`.

## 2026-11-26 · Relevo del orquestador y reparto con dos agentes

- Decisión del usuario: solo A · Antigravity y C · ChatGPT Terra; B sin ejecutor y sus archivos pasan a C. `AGENTS.md` y PLAN lo registran.
- `CONTINUACION.md` reescrito: resumen, reparto, capacidades observadas de cada ejecutor (Terra sin `cargo`), procedimiento, lecciones y prompt del orquestador nuevo.
- Pendiente al relevo: cerrar E-Q11 con el run 45740546126 y lanzar una entrega nueva.

