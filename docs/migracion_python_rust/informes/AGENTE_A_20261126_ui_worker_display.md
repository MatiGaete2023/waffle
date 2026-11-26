# Firma

- Agente: A · ChatGPT
- Herramienta/modelo: ChatGPT, GPT-6.2; sandbox ZIP + patch.
- Identidad Git temporal: Agente A <agente-a@example.test>.
- Base declarada: `9831d8bd31799a2b89e6f3b14b9dfa044d53b8d3` (comentario del ZIP interno comprobado; marcadores `FlowRustOwnership/1`, `flow-rust-v1`, `leer_alcance_ingresos` presentes).
- Copia base: `/tmp/flow_base`; copia de trabajo: `/tmp/flow_work`. Commit sintético local: `05bf4e9e52eed9ebfb0e3980652fcdf7fa6a95c3` (`base 9831d8b`). No es un commit de origin; no hubo push ni commit de implementación. Se entrega diff staged.
- Encargo: E-P1-A2 · P1-09 + P2 Asistente + higiene de productos.
- Estado: parche para revisión; compilación y aceptación funcional pendientes.

## Cambios técnicos

- `app/src/ui.rs`: BackgroundJob genérico con dos canales, progreso, repaint y captura de panic. Excel, guardado de borradores, Word por proyecto/tribunal y manual, Anonimizador, borrador de prueba y búsqueda de actualizaciones se ejecutan fuera del hilo UI. La operación de productos toma posesión de Trabajo y preparados hasta recuperar resultados; se bloquea edición y cierre normal durante la escritura, no el event loop. Los recibos/estado se recuperan y se intenta persistir antes de volver a la interfaz. La búsqueda de actualización es independiente. Se conserva la semántica de incertidumbre de Outlook. Navegación por tres etapas, progreso de revisión, scroll de Resultados, filtros en filas, foco Ctrl+F, scroll a selección por teclado/retorno de bitácora, inspector y Ajustes por pestañas, banner de correo y procedencia de Para. Se aíslan IDs de edición, se conserva corte/análisis ante entrada inválida, se persisten ediciones de preparados y se cachean favoritos, variables de matriz y huella de borrador; invalidación por edición y recarga explícita de archivos externos.
- `app/src/main.rs`: `on_exit` intenta persistir; guardado en transición de vista y en solicitudes normales de cierre, con cancelación si falla. Las modificaciones de contacto incompletas quedan en buffer y se validan al confirmar o salir; los formularios que requieren botón explícito de aplicar conservan ese contrato.
- `app/src/copia_excel.rs`: `exportar_con_base` reutiliza una sola base original; se lee el Excel efectivamente escrito para establecer línea base de sincronización, validando identificadores antes de modificar registros. Si falla, se informa que el producto existe pero no fue vinculado; no se reemplaza la vinculación anterior. Timeout de PowerShell de 180 s; referencias COM explícitas y finally.
- `app/src/outlook.rs`: plazo 90 s, limpieza de referencias COM en finally y cierre del mensaje sin segundo guardado; no Quit de Outlook del usuario. Solo Save; se mantiene `cc_obligatorio` y se informa incertidumbre si no hay confirmación suficiente.
- `app/src/theme.rs`: cinco colores solicitados, botones con blanco y prueba de luminancia WCAG. Archivo LF normalizado durante edición.
- `app/src/preparados.rs`: huella de dependencia incluye `gestiones_correo` y `resolucion`; clave de recibo con estructura JSON y búsqueda de ordinal libre incluso después de borrar recibos intermedios. Productos con sello anterior pueden requerir preparar nuevamente.
- `app/src/productos.rs`: nómina EML en memoria mediante `save_to_buffer`, sin el temporal fijo de PII; sanitización XML en escape existente. No se alteró `copiar_parte_recursiva`.
- `app/src/autodiagnostico.rs`: estados explícitos, versiones incluidas y comprobaciones opcionales de COM/extensiones sin afirmar que se ejecutaron. Se conserva el resultado general de pruebas requeridas.
- `app/src/util.rs`: caracteres XML 1.0 válidos, nombres reservados Windows, helper de plazo y drenaje concurrente de stdout/stderr. No se cambió cálculo de mayoría de edad.
- `tools/inventario_escrituras.py`: única modificación autorizada de conteo `productos.rs` de 6 a 5 por eliminación del temporal EML.
- `docs/migracion_python_rust/informes/AGENTE_A_20261126_ui_worker_display.md`: este informe.

## Pruebas y límites

Se añadieron pruebas Rust de los cinco contrastes, progreso/resultado sin bloqueo, corte inválido con trabajo sintético, dependencia y colisión de recibos, presencia de timeout/finally/ReleaseComObject y ausencia de envío, sanitización XML/nombres reservados y estados del autodiagnóstico. No se han ejecutado: `cargo`, `rustc`, `rustfmt` y PowerShell no están disponibles. No se verificó CI ni Office ni capturas de interfaz. El cálculo independiente de contraste sobre blanco para los cinco RGB dio, respectivamente, aproximadamente 4,92; 5,68; 5,88; 5,17; 5,38. No constituye auditoría WCAG de toda la interfaz.

Decisiones y límites para integrar: el plazo COM es por invocación, no por lote; si PowerShell queda colgado se termina solo el hijo propio, sin matar Office ajeno. Un kill puede impedir ejecutar finally y no acredita ausencia de procesos COM residuales; comprobar en Windows con cierre/error/timeout. Un cierre forzoso del proceso tampoco garantiza persistencia. El trabajador mantiene una pantalla de operación y excluye edición concurrente para proteger estado; no se implementó cancelación de operaciones COM ni edición paralela. La preparación de datos, reporte de proyectos Excel independiente, guardado aislado de nómina, diagnóstico y selección de matrices aún contienen operaciones síncronas fuera de las cinco exportaciones principales. La caché de archivos externos se refresca por acciones de edición, selección o recarga explícita, no por vigilancia del sistema de archivos. Verificar resolución 1100×680, navegación teclado, foco, pestañas y visualización en Windows. La nueva lectura de línea base debe contrastarse con XLS/XLSX/XLSM preservados, Excel 2010 y el exportador reducido. Los formularios de plantilla y corrección de origen sin aplicar conservan su contrato explícito; no se guardan como decisiones confirmadas. No se ha validado la compilación, ni el parser PowerShell, ni oráculos. Revisar especialmente Send de los tipos transferidos, API egui 0.31, cierre/persistencia y recuperación ante timeout. No se modificó la referencia Python ni reglas judiciales.

## Verificadores ejecutados (salida literal)


```text

$ python3 tools/verificar_solo_lectura.py --autoprueba
AUTOPRUEBA: PASA
EXIT=0

$ python3 tools/verificar_solo_lectura.py
SOLO_LECTURA: 0 hallazgos; RESULTADO: PASA
EXIT=0

$ python3 tools/inventario_escrituras.py
actualizacion.rs: 17/17 [perfil] E-I2/E-I3: copia atómica desde carpeta adicional (OneDrive) a LOCALAPPDATA/FlowRust/actualizaciones con verificación SHA-256; reemplazo del propio flow_rust.exe por el de la carpeta fija con su botón, guardando sesión; deja flow_rust.exe.anterior y restaura si falla; la actualización aplicada pasa a aplicadas/
anon.rs: 1/1 [usuario] carpeta de salida del anonimizador (solo crear carpeta); las copias las escribe flow-anonimo en una carpeta Anonimizados_* nueva, tras verificarlas
autodiagnostico.rs: 6/6 [temporal] carpeta temporal propia flow-autodiagnostico-<pid> (se borra al terminar) e informe --self-test con create_new: nunca reemplaza un archivo
config.rs: 3/3 [perfil] config::atomic_write: config.json y sesiones en data_dir() (APPDATA/FlowRust); temporal + rename
copia_excel.rs: 7/7 [usuario/temporal] copia del Excel en la carpeta de productos; ~flow-* y temp_dir() se limpian; rechaza destino existente
descargador/bitacoras.rs: 8/8 [usuario] lote de bitácoras (E-Q4): subcarpeta nueva «Bitacoras <fecha>» en el destino elegido (create_dir falla si existe); bitacoras.json y diagnostico_calendarios.json (anónimo, extensión de Rust, mismo patrón temporal+rename) y diagnostico_bitacoras.json (anónimo) por temporal + rename; instantáneas con guardar_exclusivo (nunca sobrescribe)
descargador/excel.rs: 1/1 [usuario] Excel del lote de bitácoras y de Resultados (E-Q5d/E-S1): Bitacoras_RUS.xlsx e Historia_SITFA.xlsx en la carpeta del lote (nombre con sufijo si existe) o en el destino elegido; crea la carpeta y guarda con guardar_exclusivo (nunca sobrescribe)
descargador/extension.rs: 3/3 [perfil] archivos de la extensión Lite/Integral en LOCALAPPDATA/FlowRust/extension-<edición> (carpeta fija, actualizada en el lugar); solo escribe lo que cambió y quita los archivos que ya no son de la extensión
descargador/flujo_csmp.rs: 5/5 [usuario] E-Q11: carpeta del conjunto y de sus fases; flujo.json por temporal y renombrado; copias comprobadas y libro con nombres nuevos
descargador/flujo_paquete.rs: 11/11 [usuario] E-Q9: control atómico en respaldo/proceso.json; ZIP y diagnóstico hermano preparados en temporal propio y renombrados antes de limpiarlo; respaldos exclusivos de ambos productos anteriores; si falla el ZIP se devuelve el diagnóstico nuevo al temporal y se restaura el anterior desde respaldo
descargador/historia.rs: 9/9 [usuario] historia del lote de bitácoras (E-Q4c): carpeta historia/ del lote; historias.json y diagnostico_historia.json por temporal + rename; estructura_escritos.json y estructura_pestanas.json (E-X2) anónimas, esta por temporal + rename; copia de ventanas íntegras de un lote anterior (solo si el destino no existe); ventanas con guardar_exclusivo
descargador/historico.rs: 4/4 [usuario] histórico del lote de bitácoras (E-Q4c): historico.json y estructura_historico.json anónima por temporal + rename dentro de la carpeta del lote; respaldo historico/historico-*.html con guardar_exclusivo
descargador/lote.rs: 7/7 [usuario] carpeta de destino del lote y subcarpeta nueva del lote (create_dir falla si existe); guardar_exclusivo nunca sobrescribe; resumen por temporal + rename; solo borra su propio archivo si la verificación falla
descargador/paquete.rs: 2/2 [usuario] ZIP CSMP_CAPTURE validado y diagnóstico de cobertura anónimo al lado, en el destino elegido, ambos con guardar_exclusivo; Resultados puede reunir lotes, fases de conjuntos y controles bitacoras.json; si falla el ZIP se retira solo el diagnóstico que se acaba de crear
descargador/preferencias.rs: 3/3 [perfil] descargador_preferencias.json en LOCALAPPDATA/FlowRust, incluye historial local; temporal + rename
descargador/resultados.rs: 3/3 [usuario/temporal] productos de Resultados (E-S1): carpeta del lote o la elegida; XLSX con guardar_exclusivo y nombre con marca de microsegundos (publish); metadato JSON al lado del XLSX recién creado; el libro «Para CSMP» del paquete se arma en una carpeta temporal propia que se borra
estadisticas.rs: 2/2 [usuario/perfil] resumen de constancias en la carpeta de productos con nombre nuevo; metricas.json solo con la opción activada, vía config::atomic_write
manual.rs: 3/3 [usuario] Word manual en la carpeta de productos con un nombre nuevo (no reemplaza; archivo temporal y renombrado); los favoritos y lo escrito van por config::atomic_write
outlook.rs: 5/5 [temporal] borrador.json y la nómina en una carpeta temporal propia que se borra al terminar; Outlook solo recibe Save()
personalizar.rs: 4/4 [perfil] matrices Word importadas a data_dir()/plantillas (el original se respalda como .bak.docx; escritura por archivo temporal)
producto_excel.rs: 1/1 [usuario] carpeta de productos de Ajustes; el libro se guarda en una ruta nueva (nueva_ruta nunca reemplaza un archivo)
productos.rs: 5/5 [usuario/temporal] EML/XLSX/resoluciones en la carpeta de productos (nombres nuevos, carpeta Resoluciones_* nueva; Word por tribunal conserva matriz, índice y saltos); borrador Outlook vía temp_dir() que se limpia
revision_ui.rs: 5/5 [usuario] carpeta nueva de la revisión de bitácoras (create_dir falla si existe); ante error borra solo lo propio
trabajo.rs: 1/1 [perfil] limpieza de sesiones antiguas propias en data_dir()/sesiones (RETENCION_DIAS, SESIONES_ANTERIORES)
ui.rs: 5/5 [usuario/perfil] carpeta de productos de Ajustes; contactos.csv por diálogo Guardar; plantillas, datos y carpeta de actualizaciones (solo crear carpeta) en data_dir() y LOCALAPPDATA/FlowRust
ESCRITURAS: 121 en 25 archivos; RESULTADO: PASA (0 discrepancias)
EXIT=0

$ python3 tools/check_real_ids.py app/src
Sin RUT reales detectados.
EXIT=0

$ python3 tools/check_contactos.py app/src
Sin correos ni RIT nuevos.
EXIT=0

$ python3 tools/verificar_textos_umbrales.py
EXCLUIDO HISTORIA.VARIOS: inalcanzable en Python: solo lo emite reglas_historia.constancia() con dos o más trámites, y la única función que se los pasa, constancias(), no tiene llamadores (rust/oraculo/generar_casos.py, fuera_del_motor)
TEXTOS: Rust 58, Python 59, excluidos 1
UMBRALES: Rust 16, Python 16
RESULTADO: PASA (0 discrepancias)
EXIT=0

$ python3 tools/verificar_referencia_python.py
REFERENCIA_PYTHON: afc4d12ae1ad; RESULTADO: PASA (0 diferencias)
EXIT=0

$ git diff --check
EXIT=0
```

## Texto sugerido para ESTADO / PLAN / REGISTRO (pendiente de integración)

- E-P1-A2: parche Agente A sobre base integrada 9831d8b con worker para operaciones principales, ajustes visuales y seguridad de productos.
- Verificadores Python sin hallazgos; inventario de escrituras actualizado por eliminación de temporal EML.
- Pendientes: compilación y pruebas Rust/oráculos, CI Linux/Windows, Office COM real y revisión de interfaz 1100×680; confirmar cierre y recuperación ante timeout.
- No declarar HECHO ni ACEPTADO hasta validación e integración por Orquestador.
