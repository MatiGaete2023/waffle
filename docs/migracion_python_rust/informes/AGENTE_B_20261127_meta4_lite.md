# Firma

- **Agente:** B (`E-P3-B1`)
- **Herramienta y modelo:** Teamwork (Agente B) · Gemini
- **Identidad git:** `Agente B <agente-b@example.test>`
- **Ruta del clon:** `C:\Users\cmgaete\.gemini\antigravity-cli\scratch\teamwork_tmp\waffle_b`
- **Rama:** `estudio/agente-b-meta4-lite-20261127` (base `estudio/paridad-python-rust-20261119`, commit base `a34eddb`)
- **Commits de esta sesión:**
  - `df50b06e27e277ed09a40849a39a09bb167e86ff` — `flow-meta4: soporte de fuente JSON (V-JSON-01) y prueba de integracion`
  - `df526f9567235dcce83ac567ce78029e7e737426` — `descargador_lite: comprobante JSON, extension 1.2.0 search_and_download, pipelining y 2do plano`
  - *(commit siguiente de este informe firmado con trailer `Agente: B · Teamwork`)*
- **Estado:** **HECHO, pendiente de integración**

---

## Resumen técnico (`E-P3-B1` — Decisiones `D1-A`, `D3-A`, `D4-A`, `D5-A`)

1. **Crate `rust/crates/flow-meta4/` (`src/lib.rs`, `src/salida.rs`, `tests/meta4.rs`):**
   - Documentado `Fuente.tipo` en `src/lib.rs:180` para aceptar `"EXCEL"`, `"PDF"` y `"JSON"`.
   - En `src/salida.rs::celda_matriz`, añadido `else if fuentes.iter().any(|f| f.tipo == "JSON") { "JSON · 0 registros".into() }` antes de `"FALTANTE"`. `INVENTARIO_ARCHIVOS` y los conteos de cobertura de `MATRIZ_COBERTURA` contabilizan `"JSON"` igual que `"PDF"` (no `FALTANTE`).
   - Agregada prueba permanente `fuente_json_sin_registros_se_refleja_en_inventario_y_matriz` en `tests/meta4.rs` validando `INVENTARIO_ARCHIVOS` (`"JSON"`, `0`, `"SIN REGISTROS"`) y `MATRIZ_COBERTURA` (`"JSON · 0 registros"`, `Faltantes: 2`).

2. **SITFA Lite — Comprobante JSON `V-JSON-01` y Modo Rápido vs Modo Registro (`lotes.py`, `sitfa_lite.py`):**
   - `BatchRunner.run` conserva `html_bytes` crudos antes de `response_profile(...)`. Cuando `first is None` y `not self.evidencia`, calcula `sha256_html` y escribe `comprobante_sin_registros` en `verificacion NNN.json` conforme al **Contrato 1 (`V-JSON-01`)** (`tipo: "JSON_SIN_REGISTROS"`, `fecha_hora`, `sha256_html`, `contexto_consulta`), conservando `evidencia_omitida: True`.
   - `Preferences` en `sitfa_lite.py` cambia el valor por defecto de `pdf_sin_registros` a `False` e incorpora `modo_rapido: True`. En la UI se agrega el selector **«Modo Rápido» vs «Modo Registro»** sincronizado con el chip «PDF si no hay registros» (en Modo Rápido: sin PDF, pausa adaptativa mínima `0.2 s` y `fast_fsync` al final del lote).

3. **SITFA Lite — Extensión `1.2.0` (`search_and_download` + `autoDiscardable`) y aceleración multipágina (`motor.py`, `lotes.py`, `puente.py`, `version.py`, `extension/`):**
   - Versión subida a `1.2.0` con `protocolos: {'descarga_xls': 1, 'pdf_sin_registros': 1, 'search_and_download': 1}` en `extension/manifest.json`, `extension/pagina.js`, `app/version.py`, `tests/sitfa_ficticio.py` y `tests/test_lite.py`.
   - Agregado `'search_and_download'` a las listas blancas de `app/puente.py` y `extension/conexion.js`.
   - Implementado `search_and_download` en `extension/pagina.js` conforme al **Contrato 2 (`V-PAG-01`)**: `POST` con `pairs` validados, verificación HTML, extracción de `ShowExcel` validando `^https://familia\.pjud\.cl/sitfa/reportes/[A-Za-z0-9_-]+\.xls$` (y sufijo `template` si viene informado) y descarga `GET`, retornando `{html, excel}` (`excel: null` si no hay `ShowExcel`). `ChromeTransport` y `motor.py::run_sequence` hacen fallback automático a `search` + `download` si la extensión no anuncia `search_and_download`.
   - En `extension/conexion.js`, `lock` aplica `chrome.tabs.update(tabId, {autoDiscardable: false})` (en `try/catch`) y `unlock`/`disconnect()` restauran `autoDiscardable: true`.
   - En `motor.py::run_sequence`: eliminado el doble parseo XLS al releer de disco (se verifica únicamente `path.read_bytes() == content`), agregado pipelining local con `ThreadPoolExecutor(max_workers=1)` (valida/escribe página `N` en disco mientras el hilo principal solicita página `N+1`), pausa adaptativa (`max(0.0, pause - last_net_elapsed)`), soporte `initial_excel=None` y `fast_fsync` opcional al final del lote. Ante cualquier error del worker o red, se propaga la excepción y el manifiesto queda `"INCOMPLETA"`.

## Pruebas incorporadas

- `rust/crates/flow-meta4/tests/meta4.rs`: `fuente_json_sin_registros_se_refleja_en_inventario_y_matriz`
- `descargador_lite/tests/test_lite.py`:
  - `test_comprobante_json_sin_pdf_tiene_sha256_html`
  - `test_search_and_download_multipagina_y_fallback`
  - `test_pipelining_detecta_error_validacion_en_pagina_anterior`
  - `test_no_reparsea_xls_al_releer_disco`
- `descargador_lite/tests/test_ventana.py`:
  - `test_modo_rapido_por_defecto_y_cambio_a_modo_registro`

## Verificación ejecutada

| Comando | Resultado |
|---|---|
| `cargo test --manifest-path rust/crates/flow-meta4/Cargo.toml --locked` | **33/33 pasaron** (8 unit + 16 `meta4` + 4 `plantilla` + 4 `regenerar` + 1 `tribunales`) |
| `cargo test --manifest-path rust/Cargo.toml --locked -p flow-meta4 --features serde_json/preserve_order` | **33/33 pasaron** |
| `cargo clippy --manifest-path rust/crates/flow-meta4/Cargo.toml --locked --lib -- -D warnings` | **0 advertencias** (exit 0) |
| `node --check descargador_lite/extension/pagina.js` y `conexion.js` | **PASA** (exit 0) |
| `python tools/verificar_solo_lectura.py --autoprueba` y `verificar_solo_lectura.py` | **PASA** (`0 hallazgos`) |
| `python tools/check_real_ids.py rust/crates/flow-meta4` y `descargador_lite` | **PASA** (`Sin RUT reales detectados`) |
| `pytest descargador_lite/tests/` | **30/31 pasaron** (incluidas las 5 pruebas nuevas). El único fallo (`test_seguimiento_pestanas_modalidades_informes_medidas_y_fechas`) es preexistente por `MAX_PATH` en la ruta larga del worktree `waffle_b` (~148 chars en `lotes.py:94` `name_limit`), donde `Path(__file__).resolve()` en Windows resuelve incluso unidades `subst` a la ruta física. |

## Archivos tocados (exclusivamente territorio Agente B)

- `rust/crates/flow-meta4/src/lib.rs`, `rust/crates/flow-meta4/src/salida.rs`, `rust/crates/flow-meta4/tests/meta4.rs`
- `descargador_lite/app/lotes.py`, `descargador_lite/app/motor.py`, `descargador_lite/app/puente.py`, `descargador_lite/app/sitfa_lite.py`, `descargador_lite/app/version.py`
- `descargador_lite/extension/manifest.json`, `descargador_lite/extension/pagina.js`, `descargador_lite/extension/conexion.js`
- `descargador_lite/tests/sitfa_ficticio.py`, `descargador_lite/tests/test_lite.py`, `descargador_lite/tests/test_ventana.py`
- `docs/migracion_python_rust/informes/AGENTE_B_20261127_meta4_lite.md`

*Nota de integración para el Orquestador:* Al actualizar `descargador_lite/extension/manifest.json` a `"1.2.0"`, recordar que `app/src/descargador/extension.rs:196` (territorio Agente C) embebe dicho manifiesto vía `include_bytes!`; Agente C / Orquestador debe aceptar `"1.2.0"` en esa aserción al fusionar `B → C`.

## Texto propuesto para `ESTADO.md` / `PLAN.md` / `REGISTRO_IMPLEMENTACION.md`

- **E-P3-B1 (`D1-A`, `D3-A`, `D4-A`, `D5-A`) — Agente B:** `flow-meta4` admite `Fuente.tipo == "JSON"` (`"JSON · 0 registros"` en `MATRIZ_COBERTURA` y `"SIN REGISTROS"` en `INVENTARIO_ARCHIVOS`). SITFA Lite pasa a `pdf_sin_registros: False` por defecto con comprobante `JSON_SIN_REGISTROS` (`sha256_html` + `contexto_consulta`), selector «Modo Rápido» vs «Modo Registro», extensión `1.2.0` (`search_and_download` con fallback y `autoDiscardable: false` en 2do plano), eliminación del doble parseo XLS al releer disco y pipelining local con pausa adaptativa.
