# Continuación · relevo del orquestador (26 de noviembre de 2026)

Para continuar la coordinación como **Orquestador (Antigravity)** con tres agentes ChatGPT en paralelo (**A**, **B**, **C**) en modo ZIP + `.patch`. La fuente del estado es [ESTADO.md](ESTADO.md); este archivo resume el proyecto, el reparto, el procedimiento y las lecciones.

## Relevo del 27 de noviembre de 2026

La [Tanda 3](PLAN_TANDA_3_20261127.md) sucede al resumen del 26. Los encargos A, B y C se integraron en el orden B, C y A. Las tareas son confirmar la CI, preparar el ejecutable de Windows y probar los flujos reales con la extensión Integral 2.11.0 y la Lite 1.2.0. El resultado de la comparación de encabezados HTML y XLS determina el alcance de la Tanda 4.

## Resumen

**Hechos (26-nov, CI Linux + Windows + regeneración de oráculos en verde — run `45894872026`, commit `60cdfce` en `MatiGaete2023/waffle`)**

- **Tanda 1 (`HECHO`):** `E-P0-C1` (rango único, persistencia y Meta 4), `E-P0-B1` (entrega segura `FlowRustOwnership/1`, actualizador SHA-256 y `flow-anonimo` v1.2.0 40/40) y `E-P1-A1` (integridad `NURUS_DECISIONES`, Word por fila, conciliación, correos y CC exacto).
- **Tanda 2 (`HECHO`):** `E-P1-A2` (`BackgroundJob<T>`, `on_exit()`, WCAG AA ≥ 4,5:1, rediseños Asistente y COM seguro), `E-P1-B2` (cuotas XML/ZIP reales, rutas canónicas anti-traversal, lectura única de historias y `flow-meta4` 32/32) y `E-P1-C2` (puente HTTP `chunked`/acotado, extensión Integral `2.10.0`, «Modo Rápido Seguro» `V-01`..`V-04` sin caché entre lotes del día y display Descargador).
- **E-Q11 (`HECHO`):** descarga conjunta CSMP (`flujo_csmp.rs`) y «Retomar pendientes».
- **E-P2-OPT (`HECHO`):** calibración real de «Modo Rápido Seguro», aviso si Chrome sigue en extensión `2.9.0` y limpieza de rendimiento/clippy.
- **Plan Waffle (`HECHO`):** política multi-tribunal con perfil Tomé para otros tribunales válidos y revisión manual para vacíos/marcadores/ambiguos (`flow-python` `be9d9ae`, `app/` y `flow-motor`), regeneración de `rust/casos` y validación diferenciada de fechas del formulario SITFA en `lote.rs` y `bitacoras.rs`.
- **Fuera del proyecto por decisión del usuario:** manuales aparte, registrador de observaciones RUS, extractores de estructura, variables Word por fila, exclusión manual de una fila, paginación del Word manual, M6, M7, formatos antiguos de Meta 4, conteo del Anonimizador, firma Authenticode del instalador y **caché entre lotes del día** en el Descargador.

**Siguiente paso (entrega `--release` + prueba operativa real)**

1. Lanzar `workflow_dispatch` con `entrega=true` en `MatiGaete2023/waffle` (`estudio/paridad-python-rust-20261119`) para producir el artefacto `Flow_Rust_Windows_exe` (`--release`, `SHA256.txt`, `Instalar_Flow.cmd`, instalador Inno Setup y `LEEME.txt`) y `Flow_Rust_Local_Capturas_Windows`.
2. Prueba operativa real del usuario en Windows (extensión Integral `2.10.0`, SITFA real, Outlook COM, Excel preservado COM, Meta 4 y actualización desde OneDrive).
3. Comparación mensual (`01/11/2026`–`26/11/2026` y mes completo en paralelo con Python) para el acta de aceptación (solo el usuario declara `ACEPTADO`).

## Reparto vigente (decisión del usuario del 26-nov; revertible)

| Agente · ejecutor | Rama | Territorio |
|---|---|---|
| A · ChatGPT | `estudio/agente-a-<tema>-<fecha>` | Asistente: `sincronizar.rs`, `trabajo.rs`, `copia_excel.rs`, `producto_excel.rs`, `productos.rs`, `conciliar.rs`, `estadisticas.rs`, `ordenes.rs`, `config.rs`, `outlook.rs`, `preparados.rs`, `theme.rs`, `ui.rs` |
| B · ChatGPT | `estudio/agente-b-<tema>-<fecha>` | Entrega, seguridad y herramientas auxiliares: `installer/`, `tools/verify_flow_rust_installer.ps1`, `app/tests/instalador.rs`, `actualizacion.rs`, `anon.rs`, `rust/crates/flow-anonimo/`, `rust/crates/flow-meta4/`, `lectura.rs` |
| C · ChatGPT | `estudio/agente-c-<tema>-<fecha>` | Descargador y Meta 4 UI: `descargador/`, `descargador_ui.rs`, `descargador_resultados_ui.rs`, `meta4_ui.rs` |

- **El Orquestador (Antigravity)** compila, prueba (`tools/probar_app.sh todo`, `flow-anonimo`, `flow-meta4`, `clippy --lib`, 6 verificadores), resuelve ajustes de compilación/oráculos, integra con `merge --no-ff` (o aplica `.patch` en modo ZIP + `.patch`) y es el único que edita `ESTADO.md`, `PLAN.md`, `REGISTRO_IMPLEMENTACION.md`, `AGENTS.md`, `CONTINUACION.md` y `.github/workflows/**`.

## Procedimiento de revisión e integración

1. Leer el informe: firma completa y línea `Agente: <letra> · <herramienta>` en cada commit (`git log --format='%h %s | %(trailers:key=Agente,valueonly)'`). Confirmar con `git diff --stat` que solo tocó sus archivos.
2. Verificar la CI directamente (`gh run list` / `gh run view`), nunca solo la cita del agente: `linux` y `validar` en `success`, y «Regenerar los oráculos» **corrido** (no «skipped») cuando el cambio toca oráculos o referencia.
3. Si hubo cambios de oráculos, regenerar en Linux el oráculo del encargo dos veces y exigir bytes idénticos al versionado.
4. Integrar en `estudio/paridad-python-rust-20261119`. Correr `tools/probar_app.sh todo`, `cargo clippy --manifest-path app/Cargo.toml --locked --lib -- -D warnings` y los 6 verificadores estáticos (`inventario_escrituras`, `verificar_solo_lectura`, `verificar_referencia_python`, `check_real_ids app/src`, `check_contactos app/src`, `verificar_textos_umbrales`) más `check_links.py`.
5. Actualizar `ESTADO.md`, `PLAN.md` y `REGISTRO_IMPLEMENTACION.md` (entrada de 10 líneas como máximo) y publicar. Recién con la CI de Windows en verde en la rama principal el encargo queda `HECHO`.
6. Lo ambiguo lo decide el orquestador y lo registra como revertible en `PLAN.md`. Solo se pregunta al usuario lo que cambie reglas o textos judiciales (tandas de opción múltiple, máximo 4).

## Lecciones operativas

- **Generadores deterministas e independientes del sistema:**
  - escribir bytes y normalizar a LF el JSON o texto antes de pasarlo a base64;
  - `Path(tmp).resolve()`, porque Windows entrega `RUNNER~1`;
  - `as_posix()` y `sorted(rglob(...))`;
  - en ZIP y docx, `ZipInfo.create_system = 3` y fecha fija; docx con `ZIP_STORED`;
  - importar `generar_lote`, que trae reloj fijo y `write_text` con LF;
  - sin horas locales;
  - si Python se comporta distinto por sistema (separadores `\`), emular Windows en el generador.
- Venv local para regenerar oráculos: `python3.12 -m venv --system-site-packages` (con `python3-tk` instalado), más `openpyxl pandas docxcompose python-docx xlrd lxml pillow numpy reportlab pytest icalendar`.
- Un agente sin checkout válido no arranca: cada prompt empieza con el paso 0 (clon con `origin` `MatiGaete2023/waffle`, rama y commit verificados, o `BLOQUEADO`).
- Un ejecutor por letra y un solo orquestador activo; si hay dos, chocan `ESTADO`, `PLAN` y `REGISTRO`.
- `check_contactos` rechaza direcciones con aspecto real en pruebas: usar `example.test`.
- No hacer trabajo pesado en cada cuadro de la interfaz (`BackgroundJob<T>`).
- Clippy se corre con `--lib -- -D warnings`.

## Pendiente del usuario

- Prueba operativa real con el `.exe` de la entrega (`entrega=true`): extensión Integral `2.10.0`, «Consultar y crear paquete», descarga conjunta CSMP y «Retomar pendientes», Modo Rápido Seguro, Meta 4 con alcance acreditado, actualizar desde OneDrive, Excel preservado COM y «Guardar un borrador de prueba en Outlook».
- Lista `dir /b` (solo nombres) si persiste algún lote antiguo de Python rechazado por Resultados.
- Acta de un mes real en paralelo con Python (~25-dic).
