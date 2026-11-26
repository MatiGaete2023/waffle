**Firma**

- Agente: C · ChatGPT
- Modelo: GPT-6.2; ejecución en sandbox Linux, modo ZIP + Patch.
- Identidad Git temporal: Agente C <agente-c@example.test>.
- Base: `5cdf7697d3d39623db42a0efc5442b68e19f0ef4`, identificada en el comentario del ZIP y por los dos marcadores requeridos. No es una comprobación contra el remoto.
- Copia original: `/tmp/flow_base`; copia de trabajo: `/tmp/flow_work`.
- Commit local de línea base artificial: `7158c954e5cadf1c0c8e7416f1a2857080214430` (`base 5cdf769`). No hay commit de implementación ni publicación remota: la entrega es el diff staged, conforme al modo solicitado.
- Rama de destino de integración: `estudio/paridad-python-rust-20261119`; no modificada en el remoto.

# E-P0-C1 — Rango de captura, persistencia y resultados

Fecha: 26 de noviembre de 2026. Estado: cambios entregados para revisión, compilación y pruebas del Orquestador; no aceptados ni validados mediante Cargo.

## Cambios

- `app/src/descargador_ui.rs`: construcción del plan vigente desde campos visibles, invalidación de fechas antiguas, validación de ambos extremos y orden, opción explícita Todas las fechas incompatible con campos escritos. Acciones de paquete y descarga avanzada reciben el plan vigente; un plan pendiente recuperado no reemplaza silenciosamente el alcance visible. Checkbox Un paquete por hoja con número previsto, reinicio en lanzamiento simple, reanudación y cambio de catálogo. Bitácoras reciben el mismo rango. La descarga conjunta CSMP se restringe a Todas las fechas porque su contrato separado no recibe fechas y `flujo_csmp.rs` está fuera del territorio autorizado.
- `app/src/descargador/flujo_paquete.rs`: control persistente con alcance solicitado, aplicado, aplicado en bitácoras, perfiles planificados, pausa en milisegundos, versión y SHA de compilación (nulo si GITHUB_SHA no existe); se registra también última compilación. Reanudación comprueba alcance y opciones y recompone perfiles. La reutilización de consultas exige comprobantes coherentes con el rango. Rechazo de controles antiguos sin alcance explícito; ausencia de pausa en control por lo demás válido adopta el valor predeterminado de 1 s; cero explícito se rechaza. Publicación del diagnóstico al lado del ZIP antes de limpiar temporal, respaldo exclusivo de ambos productos anteriores y recuperación si falla el renombrado del ZIP.
- `app/src/descargador/bitacoras.rs`: alcance de ingresos en `LoteBitacoras`, persistencia de fechas, propagación al prepare y comprobación de controles del formulario. No se trata una fecha omitida en el control como alcance sin filtro.
- `app/src/meta4_ui.rs`: diferencia período de observaciones de alcance de ingresos; lectura de resumen, comprobantes y, cuando corresponde, `proceso.json` asociado. Alcance no acreditado requiere confirmación explícita; se vuelve a comprobar al iniciar. Se instruye volver al Descargador si no se quiere continuar (no se añadió navegación en `ui.rs`, archivo prohibido).
- `app/src/descargador_resultados_ui.rs`: botones Quitar con identificadores egui separados y texto limpio; `leer()` devuelve error para archivo regular o ruta inexistente sin `unwrap_err()` sobre metadatos exitosos.
- `tools/inventario_escrituras.py`: única modificación en entrada `descargador/flujo_paquete.rs`, de 9 a 11 escrituras; describe publicación y respaldo.
- Este informe es el séptimo archivo. No se editó ningún oráculo JSON/Python, workflow, instalador, documento global ni módulo reservado a otro agente.

## Pruebas Rust incorporadas (no ejecutadas)

- `plan_vigente_sin_preparar_y_despues_de_editar`: verifica el helper que construye el plan usado por el lanzamiento directo, el cambio de campos tras generar un plan y rechazos de fechas incompletas, inválidas, invertidas y alcance ambiguo. No constituye una simulación de extremo a extremo del clic de descarga con SITFA.
- `rango_y_pausa_sobreviven_interrupcion_y_nueva_sesion`: transporte sintético, interrupción/reanudación con rango y pausa persistidos, controles de fecha, comprobantes, publicación de pareja y alteración de comprobante que debe invalidar el alcance.
- `publicar_reemplazo_conserva_pareja_anterior_y_revierte_si_falla_zip`: conservación, recuperación y publicación de pareja nueva.
- `meta4_distingue_alcance_acreditado_y_ausente`: rango, evidencia ausente, discrepancia y alcance explícito sin fechas.
- `ui_flujo_un_paquete_por_hoja`: extensión de prueba egui_kittest que acciona el checkbox y verifica estado y número previsto.
- `leer_rechaza_archivo_regular_y_ruta_inexistente`: errores sin pánico.
- La comparación con el oráculo congelado de flujo de paquete conserva su proyección histórica (omite pausa nueva de esa proyección) y acredita explícitamente las fixtures antiguas de reanudación cuyo alcance sintético sin fechas es conocido. Los campos nuevos y rechazo de ausencia de acreditación se comprueban separadamente. No se regeneró el oráculo ni se afirma paridad dinámica.

## Verificaciones efectivamente ejecutadas

Python 3.13.5, dentro de `/tmp/flow_work`, tras los cambios de código. Todos los comandos siguientes devolvieron exit 0.

| Comando | Resultado observado |
|---|---|
| `python3 tools/verificar_solo_lectura.py` | 0 hallazgos; PASA |
| `python3 tools/inventario_escrituras.py` | 124 escrituras en 25 archivos; 0 discrepancias; PASA |
| `python3 tools/check_real_ids.py app/src` | Sin RUT reales detectados |
| `python3 tools/check_contactos.py app/src` | Sin correos ni RIT nuevos |
| `python3 tools/verificar_textos_umbrales.py` | 58 textos Rust / 59 Python (1 excluido por regla existente); 16/16 umbrales; 0 discrepancias |
| `python3 tools/verificar_referencia_python.py` | `afc4d12ae1ad`; 0 diferencias |
| `git diff --check` | Sin salida, exit 0 |

## Riesgos, decisiones y pendientes

- Sin cargo ni rustc. Revisión estática manual de tipos de fechas, `Option`, `Value`, `Duration`, imports, referencias de catálogo, firmas y usos de `LoteBitacoras` y callbacks; no equivale a compilación. Ejecutar `tools/probar_app.sh todo`, `cargo clippy --manifest-path app/Cargo.toml --locked --all-targets` y CI Linux/Windows, incluido el oráculo congelado. Confirmar visualmente Meta 4 y probar el lanzamiento directo/cambio de campos en la interfaz completa. No se consultó SITFA real.
- El contrato de descarga conjunta CSMP no admite fechas dentro de los archivos permitidos. Se bloquea el rango en ese flujo, en vez de aparentar aplicarlo; queda pendiente ampliar su contrato bajo autorización del Orquestador. Los controles históricos sin alcance acreditado no se retoman automáticamente. Un lote avanzado sin fechas y sin declaración explícita persistida se trata como no acreditado en Meta 4, no como rango probado. El aviso permite aceptar explícitamente o cambiar de pantalla.
- El respaldo del ZIP anterior se lee en memoria; evaluar volumen representativo. Los dos renombrados no constituyen una transacción indivisible ante caída del proceso o energía entre operaciones. Existe respaldo anterior y recuperación de error ordinario, pero falta prueba Windows de reemplazo y recuperación ante caída abrupta.
- La precisión de pausa persistida es de milisegundos. El SHA depende de GITHUB_SHA en tiempo de compilación; una compilación local sin variable no acredita commit.
- No se cambiaron reglas judiciales, textos ni referencia Python. Ningún acceso de escritura a RUS/SATURNO.

## Texto sugerido para ESTADO / PLAN / REGISTRO (6 líneas)

E-P0-C1: parche de Agente C entregado para integración, no aceptado.
Rango visible validado y propagado a consultas y bitácoras; alcance explícito y pausa persistidos para reanudación.
Meta 4 distingue período de observaciones y alcance acreditado; exige decisión si falta evidencia.
Publicación ZIP + diagnóstico, checkbox por hoja y error de rutas corregidos en código.
Seis verificadores Python pasan; compilación, pruebas Rust y CI Linux/Windows pendientes.
Descarga conjunta CSMP con rango requiere ampliar contrato fuera del territorio; no usarla para descargas acotadas.
