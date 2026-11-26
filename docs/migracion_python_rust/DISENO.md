# Diseño de Flow Rust

La interfaz de escritorio vive en [`app/`](../../app/) (`main.rs`, `ui.rs`, `theme.rs`, `descargador_ui.rs`, `descargador_resultados_ui.rs`, `meta4_ui.rs`, `revision_ui.rs`). Las referencias locales de abajo se generaron en Windows con el ejemplo ficticio y un perfil aislado; para reproducirlas en CI (`entrega=true`), ver [Verificación](VERIFICACION.md).

| Vista | Referencia local | Qué comprobar |
|---|---|---|
| Trabajo | [Captura](../../app/docs/referencia_visual/trabajo.png) | Navegación lateral en 3 etapas (Captura, Revisión y Productos/Herramientas). Barra superior con apertura, guardado, corte (con validación inline) y etapa. Contadores, filtros, `Ctrl+F`, tabla con navegación por teclado y panel derecho en 4 pestañas. |
| Correos | [Captura](../../app/docs/referencia_visual/correos.png) | Selector de modalidades y tribunales arriba, tarjetas de borradores a la izquierda y destinatarios (con procedencia automática/manual de `Para`), CC fijo, texto, acciones y nómina a la derecha; pestaña de Correo manual. |
| Resoluciones | [Captura](../../app/docs/referencia_visual/resoluciones.png) | Acciones de preparación y Word (un `.docx` por proyecto o por tribunal), aviso si falta matriz para un tribunal, proyectos a la izquierda, campos editables y vista previa a la derecha; pestaña de Word manual. |
| Anonimizador y Ajustes | [Anonimizador](../../app/docs/referencia_visual/anonimizador.png) · [Ajustes en oscuro](../../app/docs/referencia_visual/ajustes-oscuro.png) | Misma navegación lateral, tarjetas coherentes, controles de entradas y salida. Ajustes organizado en sub-pestañas (General, Plantillas, Actualizaciones, Autodiagnóstico y Ayuda). El cambio claro/oscuro permanece en la barra lateral. |

## Vistas adicionales y criterios vigentes

- **Descargador** (`descargador_ui.rs` y `descargador_resultados_ui.rs`): flujo principal en 3 pasos (Conexión/extensión `2.10.0`, Rango único con «Todas las fechas» y Dos Calendarios, y Acción principal «Consultar y crear paquete» / «Un paquete por hoja» / «Descarga conjunta CSMP»), «Modo Rápido Seguro» opt-in, avance por fila con eventos traducidos, pasos sueltos en «Avanzado» y sección «Resultados del lote» con historial local e índice cacheado.
- **Meta 4** (`meta4_ui.rs`): selección de lotes y bitácoras, período de observaciones separado del alcance acreditado del lote, tribunales, plantilla opcional, generar y regenerar.
- **Ventana y accesibilidad:** `main.rs` configura la ventana de `1440 × 900` puntos con mínimo `1100 × 680`. `theme.rs` define las paletas clara y oscura con contraste WCAG AA ≥ 4,5:1 (verificado en prueba), tarjetas, chips y acciones principales.
- **Operaciones en segundo plano:** las acciones lentas o COM (Outlook, Excel preservado, Word, Anonimizador, búsqueda de actualizaciones) corren en `BackgroundJob<T>` para no bloquear la interfaz; `on_exit()` y el cambio de vista guardan la sesión pendiente.
- **Reproducción de capturas:** `app/qa/capturar_windows.py` genera las capturas sintéticas en CI (`workflow_dispatch` con `entrega=true`, artefacto `Flow_Rust_Local_Capturas_Windows`). Fecha, tiempo de análisis y renderizado de fuentes pueden variar por entorno.
