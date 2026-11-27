# Firma

- Agente: A · Teamwork
- Herramienta y modelo: Teamwork Agent (`implementer`, `qa`, `specialist`)
- Identidad git: `Carlos Matias Gaete Aguero <cmgaete@poderjudicial.cl>`
- Ruta del clon: `C:\Users\cmgaete\.gemini\antigravity-cli\scratch\teamwork_tmp\waffle_a`
- Rama: `estudio/agente-a-correos-20261127`
- Base: `a34eddb9cdc4bf25e00098d97a2e5cd2118520f2` (`estudio/paridad-python-rust-20261119`)
- Commits de esta sesión:
  - `2e3bd2ecbb9710792c0a1e828fbc3f619678dbb9`: E-P3-A1 (D2-A): destinatarios inteligentes, tribunales dinámicos, sugerencias y ResolveAll
  - `2997734a7281364da36d286e3819913b5f22841d`: docs: informe firmado Agente A E-P3-A1 (correos)
  - `9b9f3b9418d5f5878832084e75feabd4f4a56e9e`: E-P3-A1 (D2-A): corrige hallazgos de revisión en UTF-8, borradores hermanos, alias y tribunales
  - *(el commit siguiente en esta rama incorpora únicamente la actualización de este informe firmado)*
- Estado: **HECHO, pendiente de integración**

---

# Informe de Encargo: E-P3-A1 (Decisión D2-A) — Destinatarios Inteligentes en Correos

## 1. Lo implementado

### 1.1 Aprendizaje automático de contactos (`app/src/productos.rs` + `app/src/ui.rs`)
- **`app/src/productos.rs`**:
  - Se implementó `pub fn contactos_aprendidos(cfg: &Config, borradores: &[Borrador]) -> BTreeMap<String, String>` con paridad respecto a `src/nurus/personal/outputs.py:196-214`, ampliada para tribunales sin contacto previo (`Tribunal: <clave/nombre>`). Valida direcciones con `personalizar::correos`, omite entradas ya existentes (en `contactos`, `aliases`, `contacto_programa` o `tribunales` con dirección no vacía) y descarta claves ambiguas con direcciones distintas en el lote.
  - Se agregó `pub fn aplicar_contactos_aprendidos(cfg: &mut Config, aprendidos: &BTreeMap<String, String>)` para incorporar programas en `cfg.contactos` y tribunales en `cfg.tribunales` sin sobrescribir entradas no vacías.
- **`app/src/ui.rs`**:
  - En `OperacionProductos::guardar_borradores`, tras guardar borradores exitosamente (Outlook o `.eml`), se aprenden los destinatarios válidos escritos manualmente, se re-sellan los borradores vigentes no afectados para evitar falsos avisos de obsolescencia, se persiste mediante `config::save(&self.cfg)` (sin nuevas escrituras crudas en disco) y se emite el aviso `Contacto guardado para el próximo lote: ...`.
  - **Corrección crítica en `App::recuperar_operacion` (`app/src/ui.rs`):** se sincroniza `self.cfg = op.cfg;` y se limpian `self.contacto_buf` y `self.contactos_pendientes` para conservar los contactos aprendidos al volver del hilo trabajador.

### 1.2 Tribunales dinámicos (`app/src/config.rs` + `app/src/personalizar.rs` + `app/src/productos.rs`)
- **`app/src/personalizar.rs`**:
  - En `guardar_contacto`: normaliza claves `Tribunal: <NOMBRE>` a mayúsculas, valida con `crate::util::perfil_tribunal` e inserta o actualiza dinámicamente en `r.tribunales` (conservando rechazo `"Tribunal no reconocido."` solo para marcadores inválidos o vacíos).
  - En `importar_contactos_archivo`: procesa filas `Tribunal: <X>` válidas también en `.xlsx` y `.xls` (además de `.json`, `.csv` y `.html`), creando dinámicamente el tribunal en `r.tribunales`.
  - Oráculos congelados intactos (`app/tests/datos/personalizacion.json` y `app/tests/datos/catastro.json`): se reconciliaron las dos divergencias intencionales `D2-A` (`"Tribunal: INEXISTENTE"` y `"Tribunal: LAJA"` en `.xlsx`) mediante el patrón acotado `aceptadas` / `DIFERENCIA_ACEPTADA` (`xls.rs:810-820`), y se actualizó la prueba unitaria `importar_json_respeta_tribunales_conflictos_y_no_toca_la_configuracion_original` para verificar la creación dinámica de `"CORONEL"` en `r.cfg.tribunales`.
- **`app/src/config.rs`**:
  - `Config::tribunal_nombre` busca por clave exacta y por `clave()` normalizada; `completar_contactos_python` inserta dinámicamente entradas `Tribunal:` válidas según `perfil_tribunal`.
- **`app/src/productos.rs`**:
  - Se agregó `tribunal_para` para resolver destinatarios de tribunal por clave exacta y por `clave()` normalizada tanto sobre la clave como sobre `t.nombre` en `preparar_borradores` y `borrador_plantilla`.
- **`app/src/ui.rs`**:
  - En `v_correos`, los filtros de tribunales incluyen dinámicamente los tribunales adicionales presentes en el trabajo abierto junto a `LAJA`, `MULCHEN` y `TOME`.

### 1.3 Resolución en cascada, sugerencias en 1 clic y reimportación (`app/src/config.rs` + `app/src/ui.rs`)
- **`app/src/config.rs`**:
  - `Config::contacto_programa(&self, programa: &str)`: (1) coincidencia exacta por `clave()` en `contactos` o `aliases`; (2) coincidencia tras `sin_parentesis(programa)` únicamente si resuelve a una única dirección inequívoca.
  - `pub fn sugerir_contactos(&self, programa: &str, max: usize) -> Vec<(String, String)>`: puntuación determinista por tokens compartidos (filtrando stopwords) y coincidencia/prefijo de sigla.
  - `pub fn reimportar_contactos_python(&mut self) -> usize`: reinicia las banderas de migración y ejecuta `migrar_contactos_python` sin sobrescribir contactos locales existentes.
- **`app/src/ui.rs`**:
  - En `v_correos`, cuando `b.para` está vacío y `b.programa` no está vacío, muestra hasta 3 botones compactos `Usar: <nombre> (guardar alias)`; al pulsar, completa `b.para`, guarda el alias en `self.cfg.aliases`, re-sella el borrador vigente y persiste `config.json`.
  - En `v_contactos`, agrega el botón `Reimportar contactos desde Flow Python` llamando a `self.cfg.reimportar_contactos_python()`, limpiando `contacto_buf` y guardando `config.json`.

### 1.4 Outlook COM `ResolveAll()` (`app/src/outlook.rs`)
- En `SCRIPT`, después de `$m.To` y `$m.CC` y antes de `Write-Output 'PREPARADO'` / `$m.Save()`, se incorporó `try { [void]$m.Recipients.ResolveAll() } catch {}`. Se mantiene `$m.Save()`, el CC obligatorio `ucc_concepcion@pjud.cl` y cero `.Send(` en producción.

### 1.5 Endurecimiento posterior a revisión adversa (`9b9f3b9`)
- **UTF-8 seguro en prefijo `Tribunal:` (`app/src/personalizar.rs` + `app/src/config.rs`):** `extraer_tribunal` y `completar_contactos_python` reemplazan el corte directo `[..9]` por `.get(..9).is_some_and(|p| p.eq_ignore_ascii_case("tribunal:"))` para evitar *panic* en nombres con caracteres multibyte en el byte 9 (p. ej. `"PRM Biobío"`, `"Fae Biobío"`, `"Tríbunal: ..."`).
- **Re-sellado limpio de borradores hermanos (`app/src/ui.rs` + `app/src/productos.rs`):** `OperacionProductos::guardar_borradores` y `aplicar_sugerencia_alias` usan `productos::destinatario_borrador` y `resellar_borradores_vigentes` para autocompletar hermanos sin `para` que coincidan con un contacto/alias recién aprendido y re-sellar borradores vigentes no obsoletos cuyo destinatario no fue invalidado, sin marcar `OBSOLETO` a hermanos no relacionados ni ocultar obsolescencias genuinas.
- **Protección de alias previos (`app/src/ui.rs`):** `aplicar_sugerencia_alias` protege `!self.cfg.aliases.keys().any(|a| clave(a) == k_prog)` además de `contactos`; si ya existe un alias previo para `k_prog`, aplica el correo únicamente al borrador actual sin sobrescribir `self.cfg.aliases`.
- **Importación de tribunales sin borrar correos ni pisar conflictos (`app/src/personalizar.rs`):** en `importar_contactos_archivo` (`.xlsx`/`.xls`/`.csv`/`.html`/`.json`), una celda `Mail` vacía preserva el `t.para` existente y una dirección distinta a la ya configurada registra conflicto en `conflictos` preservando el correo previo.
- **Claves de tribunal en minúscula sin duplicación (`app/src/productos.rs` + `app/src/personalizar.rs` + `app/src/config.rs`):** `aplicar_contactos_aprendidos`, `clave_tribunal_existente` y `completar_contactos_python` resuelven la clave existente por `clave(k)` / `clave(&t.nombre)` antes de actualizar, evitando crear duplicados en mayúsculas.

---

## 2. Pruebas permanentes y verificación

### Pruebas permanentes agregadas (solo `@example.test` y RIT año `2099`)
- `productos.rs`: `aprendizaje_guarda_nuevo_y_no_pisa_existente_ni_ambiguo`, `aprendizaje_actualiza_clave_tribunal_en_minuscula_sin_duplicar`
- `personalizar.rs`: `guardar_e_importar_tribunal_adicional_desde_excel_y_json`, `nombres_utf8_en_borde_nueve_no_paniquean_e_importacion_vacia_preserva_tribunal`
- `config.rs`: `contacto_programa_resuelve_sin_parentesis_y_sugiere_candidatos` (incluye clave en minúscula y prefijo UTF-8 con tilde en `migrar_contactos_python`)
- `outlook.rs`: `outlook_script_resuelve_destinatarios_y_nunca_envia`
- `ui.rs`: `guardar_borrador_aprende_contacto_sincroniza_cfg_y_sugerencia_guarda_alias`, `guardar_borrador_no_obsoleta_hermano_vacio_y_respeta_alias_existente`

### Resultados ejecutados en `waffle_a`
- `tools/probar_app.sh rapido config`: `6 passed; 0 failed` (bin)
- `tools/probar_app.sh todo personalizar`: `9 passed; 0 failed` (`AJUSTES: 43/44, fallos 0, aceptadas 1`; `CATASTRO: 35/36, fallos 0, aceptadas 1`)
- `tools/probar_app.sh rapido productos`: `2 passed` (lib) + `15 passed` (bin), `0 failed`
- `tools/probar_app.sh rapido outlook`: `7 passed; 0 failed` (bin)
- `tools/probar_app.sh rapido pruebas_correos_d2a`: `2 passed; 0 failed` (bin)
- `tools/probar_app.sh todo`: `115 passed` (lib) + `159 passed; 10 filtered out` (bin, sin `FLOW_E3_PAQUETE`) + `4 passed` (`instalador`), `0 failed`
- `cargo clippy --manifest-path app/Cargo.toml --locked --lib -- -D warnings`: falla únicamente por un hallazgo preexistente fuera de territorio en `app/src/descargador/puente.rs:240` (`clippy::needless_option_as_deref` bajo `rustc 1.100.0`); con `-A clippy::needless_option_as_deref` pasa con **0 advertencias** en todo `--lib`.
- Verificadores estáticos:
  - `python tools/inventario_escrituras.py`: `ESCRITURAS: 120 en 24 archivos; RESULTADO: PASA (0 discrepancias)`
  - `python tools/verificar_solo_lectura.py`: `SOLO_LECTURA: 0 hallazgos; RESULTADO: PASA`
  - `python tools/check_contactos.py app/src installer tools`: `Sin correos ni RIT nuevos.`
  - `python tools/check_real_ids.py app/src app/qa app/tests/datos installer tools`: `Sin RUT reales detectados.`
  - `python tools/verificar_textos_umbrales.py`: `RESULTADO: PASA (0 discrepancias)`
  - `python tools/verificar_referencia_python.py`: `REFERENCIA_PYTHON: be9d9aee717d; RESULTADO: PASA (0 diferencias)`

---

## 3. Texto propuesto para el Orquestador

### Para `docs/migracion_python_rust/ESTADO.md`
- **F11 / F12 / F13 (`E-P3-A1`, `D2-A`):** aprendizaje automático de destinatarios válidos al guardar borradores (`productos::contactos_aprendidos` + sincronía `op.cfg` en `recuperar_operacion`), creación/importación dinámica de tribunales válidos (`guardar_contacto`, `.xlsx`/`.xls`/`.json`), cascada `sin_parentesis`, sugerencias en 1 clic con guardado de alias, botón de reimportación desde Flow Python y `ResolveAll()` seguro antes de `Save()` en `outlook.rs`.

### Para `docs/migracion_python_rust/PLAN.md`
- **E-P3-A1 (`D2-A`) · Destinatarios inteligentes en Correos — `HECHO` (`2e3bd2e`, `9b9f3b9`).** Aprendizaje al guardar borradores, tribunales dinámicos en Contactos/Excel/JSON, cascada `sin_parentesis`, sugerencias en 1 clic, reimportación Python y `ResolveAll()` en Outlook COM.

### Para `docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md`
```markdown
### 2026-11-27 · E-P3-A1 (D2-A): Destinatarios inteligentes en Correos · Agente A
- productos.rs/ui.rs: `contactos_aprendidos` al guardar borradores, re-sellado limpio de borradores hermanos y sincronía `self.cfg = op.cfg`.
- personalizar.rs/config.rs/productos.rs: tribunales dinámicos en guardar/importar (.xlsx/.xls/.json), protección UTF-8/Mail vacío/conflictos y `tribunal_para` normalizado (2 diferencias D2-A aceptadas en oráculo).
- config.rs/ui.rs: cascada `sin_parentesis`, `sugerir_contactos` en 1 clic sin pisar alias previos y botón de reimportación desde Flow Python.
- outlook.rs: `try { [void]$m.Recipients.ResolveAll() } catch {}` antes de `$m.Save()`.
- Commits: 2e3bd2e, 9b9f3b9 (y cierre de informe en rama `estudio/agente-a-correos-20261127`).
```