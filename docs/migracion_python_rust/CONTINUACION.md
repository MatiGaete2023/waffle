# Continuación · relevo del orquestador (26 de noviembre de 2026)

Para continuar la coordinación como **Orquestador (Antigravity)** con tres agentes ChatGPT en paralelo (**A**, **B**, **C**) en modo ZIP + `.patch`. La fuente del estado es [ESTADO.md](ESTADO.md); este archivo resume el proyecto, el reparto, el procedimiento y las lecciones.

## Resumen

**Hechos (26-nov, tras integración de Tandas 1 y 2 en Linux)**

- **Tanda 1 integrada en `estudio/paridad-python-rust-20261119`** (`E-P0-C1`, `E-P0-B1`, `E-P1-A1` + workflow CI actualizado con `rust/**`, `descargador_lite/**` e `installer/LEEME.txt`).
- **Tanda 2 integrada en `estudio/paridad-python-rust-20261119`** (pendiente CI Windows para pasar a `HECHO`; nada marcado `ACEPTADO`):
  - **E-P1-A2** (Agente A · ChatGPT): `BackgroundJob<T>`, `on_exit()`, `.id_source` único, error inline en `Corte`, 1× `copia_excel::base`, timeout + limpieza RCW en COM, paleta WCAG AA ≥ 4,5:1, rediseño Asistente (navegación 3 etapas, 4 pestañas Trabajo, sub-pestañas Ajustes, `ScrollArea` Resultados, banner seguridad/procedencia `Para`), `preparados.rs`, sanitización XML 1.0/Windows y estados en autodiagnóstico.
  - **E-P1-B2** (Agente B · ChatGPT): cuotas XML/ZIP reales (`Read::take`, `ss:Index <= 16_384`, profundidad ≤ 128, dimensiones OOXML compartidos desde `descargador::xls`), validación canónica anti-traversal y carga única de historias en `paquete.rs`, consolidado en memoria en `resultados.rs`, presentación en `flujo_csmp.rs` y fix `count="01"` + cuota ZIP en `flow-meta4` (32/32 pruebas).
  - **E-P1-C2** (Agente C · ChatGPT): puente HTTP `chunked`/acotado (256 KiB control / 112 MiB datos, timeouts, recuperación de `Mutex`), extensión propia `descargador/extension_integral/` (`2.10.0`, sin tocar `descargador/` congelado), «Modo Rápido Seguro» `V-01`..`V-04` (índice DOM, pipelining, pausa adaptativa, concurrencia acotada 1–3 con degradación, **sin caché entre lotes del día**) y display Descargador 3 pasos + Dos Calendarios + índice de Resultados cacheado.
- **Fuera del proyecto por decisión del usuario:** manuales, registrador de observaciones RUS, extractores de estructura, variables Word por fila, exclusión manual de una fila, paginación del Word manual, M6, M7, formatos antiguos de Meta 4, firma Authenticode del instalador y **caché entre lotes del día** en el Descargador.

**Siguiente paso (cierre Windows + prueba real)**

- Publicar `estudio/paridad-python-rust-20261119` en GitHub desde el bundle del usuario, verificar CI Linux + Windows verde (y lanzar `workflow_dispatch` con `entrega=true`), cerrar Tandas 1 y 2 como `HECHO` citando el run y ejecutar la prueba real en Windows (Chrome con extensión `2.10.0`, SITFA, Outlook COM, Excel COM, Meta 4 y criterio de salida mensual).

## Reparto vigente (decisión del usuario del 26-nov; revertible)

| Agente · ejecutor | Rama | Territorio |
|---|---|---|
| A · ChatGPT | `estudio/agente-a-<tema>-<fecha>` | Asistente: `sincronizar.rs`, `trabajo.rs`, `copia_excel.rs`, `producto_excel.rs`, `productos.rs`, `conciliar.rs`, `estadisticas.rs`, `ordenes.rs`, `config.rs`, `outlook.rs`, `preparados.rs`, `theme.rs`, `ui.rs` |
| B · ChatGPT | `estudio/agente-b-<tema>-<fecha>` | Entrega, seguridad y herramientas auxiliares: `installer/`, `tools/verify_flow_rust_installer.ps1`, `app/tests/instalador.rs`, `actualizacion.rs`, `anon.rs`, `rust/crates/flow-anonimo/`, `rust/crates/flow-meta4/`, `lectura.rs` |
| C · ChatGPT | `estudio/agente-c-<tema>-<fecha>` | Descargador y Meta 4 UI: `descargador/`, `descargador_ui.rs`, `descargador_resultados_ui.rs`, `meta4_ui.rs` |

- **El Orquestador (Antigravity)** compila, prueba (`tools/probar_app.sh todo`, `flow-anonimo`, `clippy --lib`, 6 verificadores), resuelve ajustes de compilación/oráculos, integra con `merge --no-ff` y es el único que edita `ESTADO.md`, `PLAN.md`, `REGISTRO_IMPLEMENTACION.md`, `AGENTS.md`, `CONTINUACION.md` y `.github/workflows/**`.

## Capacidades de cada ejecutor (observadas el 25 y 26-nov)

- **Antigravity:** clon Windows del usuario, con `cargo` y Python. Puede lanzar `workflow_dispatch`. Ha citado runs que no existían (45672133399): verificar siempre.
- **ChatGPT Terra (Codex):** el clon `/workspace/flow` **no tiene `cargo`**, y en Windows no tiene WSL, Docker ni VM. Se detuvo dos veces creyendo que necesitaba Linux local. Que publique en su rama y use la CI como verificación. Si la regeneración en Linux hace falta, la hace el orquestador.
- **Orquestador:** necesita Linux con `cargo` y el venv de los generadores, y acceso a GitHub Actions: conclusión de `linux` y `validar`, logs, `workflow_dispatch`. Sin eso, la integración queda `BLOQUEADO`.

## Procedimiento de revisión e integración

1. Leer el informe: firma completa y línea `Agente: <letra> · <herramienta>` en cada commit (`git log --format='%h %s | %(trailers:key=Agente,valueonly)'`). Confirmar con `git diff <principal>...<rama> --stat` que solo tocó sus archivos.
2. CI de la rama, leída por el orquestador y no citada por el agente: `linux` y `validar` en `success`, y «Regenerar los oráculos» **corrido** (no «skipped») con el oráculo del encargo. Si el commit solo tocó Rust, la regeneración se omite: lanzar `workflow_dispatch`, que regenera todos.
3. Regenerar en Linux el oráculo del encargo dos veces y exigir bytes idénticos al versionado.
4. `git merge --no-ff` a la rama principal. Correr `tools/probar_app.sh todo`, `cargo clippy --manifest-path app/Cargo.toml --locked --features eframe/x11 --lib -- -D warnings` y los verificadores: `inventario_escrituras`, `verificar_solo_lectura`, `verificar_referencia_python`, `check_real_ids app/src`, `check_contactos app/src`, `verificar_textos_umbrales`.
5. Actualizar `ESTADO`, `PLAN` y `REGISTRO` (entrada de 10 líneas como máximo) y publicar. Recién con la CI de Windows en verde en la rama principal, con la regeneración corrida, el encargo queda `HECHO`; el cierre se hace en un commit aparte que cita el run.
6. Lo ambiguo lo decide el orquestador y lo registra como revertible en PLAN. Solo se pregunta al usuario lo que cambie reglas o textos judiciales (tandas de opción múltiple, máximo 4).

## Lecciones operativas

- **Generadores deterministas e independientes del sistema:**
  - escribir bytes y normalizar a LF el JSON o texto antes de pasarlo a base64;
  - `Path(tmp).resolve()`, porque Windows entrega `RUNNER~1`;
  - `as_posix()` y `sorted(rglob(...))`;
  - en ZIP y docx, `ZipInfo.create_system = 3` y fecha fija; docx con `ZIP_STORED`;
  - importar `generar_lote`, que trae reloj fijo y `write_text` con LF;
  - sin horas locales;
  - si Python se comporta distinto por sistema (separadores `\`), emular Windows en el generador.
- Venv local: `apt-get install -y python3-tk`; `python3.12 -m venv --system-site-packages`, más `openpyxl pandas docxcompose python-docx xlrd lxml pillow numpy reportlab pytest icalendar`. El `python3` 3.13 no ve `tkinter`.
- Un agente sin checkout válido no arranca: cada prompt empieza con el paso 0 (clon con `origin` MatiGaete2023/flow, rama y commit verificados, o `BLOQUEADO`).
- Un ejecutor por letra y un solo orquestador activo; si hay dos, chocan `ESTADO`, `PLAN` y `REGISTRO`. Cuando una herramienta retoma un encargo, que lo haga desde el commit publicado, no desde cambios locales sin commit.
- Antes de reabrir un encargo, revisar ESTADO: Terra rehízo E-S3 ya cerrado (`AGENTE_C_20261125c.md`, continuación descartada).
- `check_contactos` rechaza direcciones con aspecto real en pruebas: usar `example.test`.
- No hacer trabajo pesado en cada cuadro de la interfaz.
- `tools/probar_app.sh todo` tarda ~20 s. Clippy va con `--lib`; el binario tiene código muerto anterior que no forma parte del gate (por ejemplo, un `mut` sobrante en `descargador_ui.rs`).

## Pendiente del usuario

- Prueba corta con el `.exe` de la entrega nueva: «Consultar y crear paquete», descarga conjunta CSMP y «Retomar pendientes», juntar hojas y bitácoras en Resultados, actualizar desde OneDrive y «Guardar un borrador de prueba en Outlook».
- Lista `dir /b` (solo nombres) de la carpeta del lote antiguo de Python que Resultados rechazó.
- Acta de un mes real en paralelo con Python (~25-dic).

## Prompt para el chat nuevo (orquestador)

```
Eres el orquestador y coordinador de la migración Python → Flow Rust en MatiGaete2023/flow, rama estudio/paridad-python-rust-20261119. Trabajas con dos agentes: A = Antigravity (Asistente) y C = ChatGPT GPT-5.6 Terra (Descargador, incluido lo que era de B). Tú decides, revisas, integras y validas; no implementas encargos de A ni de C, y vas en una sesión distinta de la de C.

Lee solo AGENTS.md y docs/migracion_python_rust/ESTADO.md, PLAN.md y CONTINUACION.md (resumen, reparto, capacidades de cada ejecutor, procedimiento y lecciones al 26-nov).

Paso 0 (o BLOQUEADO con el motivo):
- clon con origin MatiGaete2023/flow y rama al día (git fetch; si no comparte historia, respáldalo y alinéalo con origin);
- cargo disponible y venv de generadores (CONTINUACION, «Lecciones»);
- acceso a GitHub Actions: conclusión de linux y validar, logs, workflow_dispatch.

Primero:
1. Run 45740546126 (commit 6181828) de la rama principal: si linux y validar están en verde y «Regenerar los oráculos» muestra «flujo_csmp: 8 casos» y «Oráculos regenerados sin diferencias.», cierra E-Q11 como HECHO en ESTADO, PLAN, REGISTRO y CONTINUACION, en un commit que cite el run. Si falló, diagnostica.
2. Lanza una corrida de entrega (workflow_dispatch, entrega=true) sobre la rama principal, para que el .exe incluya E-A11, E-T6 y E-Q11. Cuando termine en verde, avisa al usuario: artefacto Flow_Rust_Windows_exe, con SHA256.txt, Instalar_Flow.cmd, el instalador Inno Setup y LEEME.txt.
3. A y C no tienen encargo. Propón el siguiente a partir de lo «Parcial» de ESTADO y de lo que el usuario informe de su prueba corta. Si no hay nada verificable sin datos reales, dilo y no inventes trabajo.

Revisión: sigue el «Procedimiento de revisión e integración» de CONTINUACION.md. No confíes en el run que cita un agente: léelo tú. Exige la firma en el informe y la línea «Agente: <letra> · <herramienta>» en cada commit. Regenera en Linux el oráculo del encargo y exige bytes idénticos. Terra no tiene cargo: que publique en su rama y use la CI; la regeneración en Linux la haces tú.

Prompts para agentes: completos en el chat y también en docs/migracion_python_rust/informes/PROMPTS_AGENTES_<fecha>.md. Cada uno con paso 0 de checkout, rama estudio/agente-<letra>-<tema>-<fecha>, archivos propios y prohibidos, oráculo con su prueba de alteración, criterio de parada (BLOQUEADO con motivo), firma e informe con texto propuesto para ESTADO, PLAN y registro.

Reglas: datos sintéticos; sin HAR, HTML, PDF, Excel ni capturas reales (si el usuario autoriza por escrito leer un archivo suyo: solo estructura y conteos, nada al repositorio, y borrar la copia al terminar); sin escritura en RUS ni SATURNO; Outlook solo borradores (Save()) con CC fijo ucc_concepcion@pjud.cl; sin credenciales en código, logs ni conversaciones; sin --force, sin merge a main y sin releases. HECHO = commit + CI de Windows verde con «Regenerar los oráculos» corrido; ACEPTADO solo lo da el usuario. Solo tú editas ESTADO, PLAN, REGISTRO, AGENTS, CONTINUACION y el workflow.

Proceso: decide y sigue; registra lo delegado como revertible en PLAN.md; pregunta en tandas de opción múltiple (máximo 4) solo lo que cambie reglas o textos judiciales. Estilo: español chileno, la primera línea es la respuesta, sin cortesías, hechos | inferencias | especulación separados.

Pendiente del usuario (no lo hagas tú): prueba corta con el .exe nuevo; lista dir /b de la carpeta del lote antiguo que Resultados rechazó; acta de un mes real en paralelo con Python (~25-dic).
```
