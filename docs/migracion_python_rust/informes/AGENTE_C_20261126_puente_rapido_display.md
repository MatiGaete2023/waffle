# Firma

- Agente: C · ChatGPT
- Herramienta/modelo: ChatGPT GPT-6.2, sandbox ZIP + patch (Python 3.13.5, Node.js v22.16.0).
- Identidad Git temporal: Agente C <agente-c@example.test>.
- Repositorio de origen indicado: MatiGaete2023/flow; rama de referencia `estudio/paridad-python-rust-20261119`.
- Base acreditada mediante comentario ZIP y tres marcadores: `9831d8bd31799a2b89e6f3b14b9dfa044d53b8d3`.
- Directorios: `/tmp/flow_base` (árbol original), `/tmp/flow_work` (copia de trabajo).
- Commit exclusivamente local de línea base: `ed80dfda0ac787cad9c1f9c0a091bf1ffb7d4429` — `base 9831d8b`. No hay commits de cambios, rama remota ni push: se entrega diff binario indexado, conforme al modo ZIP.
- Estado: implementación propuesta, verificadores estáticos ejecutados, compilación Rust y aceptación pendientes.

## Resumen técnico — E-P1-C2

**P1-08.** Se reemplaza el adaptador HTTP del puente por un lector TCP/HTTP local acotado. Solo escucha en loopback. Ocho trabajadores de conexión, una petición por socket; cabeceras hasta 16 KiB y 5 s; cuerpo hasta 30 s; escritura hasta 10 s. Control/estado: 256 KiB; respuestas de datos: 112 MiB para acomodar hasta 80 MiB binarios codificados en base64. Chunked con tamaño real acumulado, cuota de framing y trailers; rechaza framing ambiguo. Se conserva autenticación local, identidad de extensión, filtro de comandos y errores en español. Para clientes anteriores sin cabecera de tarea, el presupuesto se deduce de tareas activas cuando es posible y se comprueba nuevamente tras decodificar; respuestas tardías pueden requerir la cuota general. Se elimina el unwrap de locks de producción y se recupera el guard envenenado en operaciones de colección. El despacho mantiene exclusión de cambios de formulario/evidencia; solo las lecturas de contexto verificadas se solapan, tope 3, reducción a 1 tras dos fallas. Una tarea vencida no libera anticipadamente una lectura todavía en vuelo.

**V-01.** La extensión propia Integral vive en `app/src/descargador/extension_integral/`; la distribución Chrome pasa a 2.10.0 y conserva el protocolo de capacidades 2.9.0 exigido por la validación congelada fuera de territorio, con marcadores separados de distribución/capacidad rápida. `pagina.js` indexa enlaces literales por página para observaciones, historia e histórico; mantiene comprobaciones de identidad y unicidad. El flujo de paquete rápido usa un ámbito aleatorio de memoria, por la misma ejecución, entre listados y bitácoras; solo reutiliza igualdad exacta de pares de búsqueda y no modifica datos de SITFA. Límites: 128 entradas, 2 MiB por entrada y 24 MiB totales (base64); cualquier diferencia exige lectura nueva. Se borra el ámbito al terminar/desbloquear o iniciar otra corrida. No hay persistencia de HTML del ámbito ni reutilización entre lotes. Se elimina la ruta de reutilización automática de bitácoras de otros lotes y se limita historia a su propio lote retomado. La preferencia antigua queda deshabilitada; el historial local sigue disponible para selección explícita de fuentes de productos, sin agregarlas automáticamente a un nuevo paquete. Resultados mantiene un índice en memoria de referencias, disponibilidad, tipo y flujo.json (lectura acotada); invalida al cambiar carpeta o selección, finalizar acciones o pulsar Actualizar.

**V-02.** Trabajador CPU con alcance limitado, procesa la respuesta N mientras el hilo dueño anticipa las siguientes ventanas verificadas de la misma página. Los resultados vuelven en orden de solicitud y el manifiesto se modifica en un único hilo. Se conservan comprobaciones de huellas, parámetros, identidad, escrituras exclusivas y límites de lecturas. En bitácoras la anticipación se desactiva al retomar, al combinar calendario/histórico o ante vínculos/identidades dudosas; las otras lecturas siguen siendo conservadoras respecto del orden. Historia resuelve conflictos de identidad de la página antes de anticipar ventanas. La anticipación no cruza páginas.

**V-03/V-04.** Preferencia optativa, fija en modo tradicional (valor UI 0,5–10 s; predeterminado 1 s). Modo rápido: pausa inicial 1 s, baja 250 ms por cada dos respuestas sanas consecutivas (latencia menor de 400 ms) hasta la pausa configurable 200–350 ms; escala a 1–2 s ante lentitud/error. Concurrencia configurable 1–3, predeterminada 2 cuando se activa el modo; en modo tradicional es 1. Dos fallas consecutivas fuerzan el tope 1 en el control adaptativo/puente y la extensión también se protege. La latencia de grupo incluye transporte local y la medición es conservadora; no se afirma un tiempo de mejora. El puente acepta la orden interna `context_batch` solo para lecturas autorizadas y devuelve en orden original aunque Chrome responda fuera de orden. Una extensión sin marcador propio permanece en modo conservador. La configuración rápida del paquete se guarda en sus opciones cuando está activa.

**P2.** Descargador con pasos numerados, filtros visibles inicialmente, selector rápido y protección de sesión explicada; acción conjunta diferenciada como todas las fechas, eventos traducidos sin formato Debug, Avanzado separado y Resultados con Actualizar. Meta 4 identifica período de observaciones y alcance acreditado de ingresos como dos calendarios independientes, con aviso y acción de confirmación cuando falta acreditación.

## Archivos exactos

- `app/src/descargador/bitacoras.rs`
- `app/src/descargador/extension.rs`
- `app/src/descargador/extension_integral/background.js`
- `app/src/descargador/extension_integral/conexion.css`
- `app/src/descargador/extension_integral/conexion.html`
- `app/src/descargador/extension_integral/conexion.js`
- `app/src/descargador/extension_integral/manifest.json`
- `app/src/descargador/extension_integral/pagina.js`
- `app/src/descargador/flujo_paquete.rs`
- `app/src/descargador/historia.rs`
- `app/src/descargador/preferencias.rs`
- `app/src/descargador/puente.rs`
- `app/src/descargador_resultados_ui.rs`
- `app/src/descargador_ui.rs`
- `app/src/meta4_ui.rs`
- `docs/migracion_python_rust/informes/AGENTE_C_20261126_puente_rapido_display.md`

No se modificaron `descargador/**`, `descargador_lite/**`, oráculos, `app/qa/**`, módulos prohibidos, workflows ni documentación de coordinación. No fue necesario modificar el inventario: las coincidencias de escritura de producción permanecen clasificadas.

## Pruebas Rust incorporadas (NO ejecutadas)

- Puente: POST chunked real al loopback, cuerpo chunked y trailers, exceso por cuota real y Content-Length, códigos/mensajes 413, timeout absoluto de lectura, recuperación de mutex envenenado, tope/degradación y grupo real de tres peticiones con respuestas desordenadas.
- Preferencias: pausa sana de 250 ms, incremento por latencia/error, reducción gradual y degradación; compatibilidad con pausa fija.
- Extensión: archivos Integral embebidos propios y manifest 2.10.0, marcador rápido e índice, límite de concurrencia; pruebas preexistentes de instalación ajustadas a la nueva versión.
- Bitácoras: comparación del procesamiento secuencial con anticipación sobre respuestas sintéticas congeladas (dos ventanas, mismo manifiesto y hashes); prueba explícita de un nuevo lote que no usa las bitácoras de otro. La prueba de pipeline fuerza una anticipación de una ventana; no mide velocidad ni reproduce por sí sola un listado real de múltiples filas. Historia requiere contraste adicional del modo rápido con varias causas en una página.
- Interfaz: traducción de eventos e índice que mantiene su resultado hasta invalidar/actualizar. Las pruebas UI completas preexistentes deben ejecutarse con Cargo.

No se dispone de cargo/rustc; tampoco del analizador tree-sitter en Python. Se revisaron imports, firmas y llamadas, separación entre módulos de biblioteca/binario, ownership de los datos de trabajadores y alcance de closures. Esto no sustituye compilación ni prueba de ejecución. No hay resultados de Cargo, Clippy, CI, Chrome real ni mediciones SITFA.

## Diferencias, riesgos y decisiones que requieren revisión del Orquestador

1. **[CONFLICTO_ABIERTO] Versión de extensión vs oráculo de paquete.** La distribución solicitada 2.10.0 es incompatible con la comparación literal congelada en `app/src/descargador/paquete.rs`: genera `downloader.version` a través de `extension::version(Integral)` y el oráculo `app/tests/datos/paquete.json` espera 2.9.0 en sus cinco escenarios; el comparador lee el `manifest.json` completo. Por inspección se anticipa una diferencia de ese campo, no un resultado de Cargo observado. No se disfraza la versión 2.10.0 ni se altera el oráculo, `paquete.rs` o la verificación central de capacidades. Se requiere autorización al Orquestador para distinguir versión de protocolo y distribución o ajustar el comparador de metadatos de versión, manteniendo el contraste de contenido judicial. Las capacidades continúan en protocolo 2.9.0, con la distribución 2.10.0 declarada separadamente.
2. **Diferencia deliberada:** el caso histórico `reutilizar_de_otro_lote` representa la conducta ahora expresamente prohibida. Se conserva el fixture byte a byte, se excluye solo ese caso de la comparación de paridad y se añade una prueba independiente que usa su carpeta antigua y exige dos lecturas nuevas. Los demás casos siguen sujetos a comparación. No se afirma que pasan.
3. V-01 solo reutiliza búsquedas exactamente iguales en memoria dentro de un paquete rápido, no al retomar ni entre corridas; no se ha medido la tasa de aciertos. V-02 no anticipa bitácoras con calendario/histórico simultáneos, ni recuperaciones; historia evita lecturas con conflicto de identidad antes del despacho, lo que puede reducir el número de solicitudes comparado con la enumeración secuencial de una página contradictoria. Debe comprobarse cobertura, no solo velocidad.
4. La capacidad del puente se configura desde la UI; la pausa y tope solicitados del propio lote se recuperan de `proceso.json`, pero el tope del puente puede ser más restrictivo si la preferencia de interfaz cambió desde la captura. La degradación del puente permanece hasta reconectar (conservador). El límite interno de 3 es absoluto. La extensión antigua puede no interpretar el error de cuota de forma tan detallada; la propia reenvía únicamente el diagnóstico local pequeño, sin repetir la consulta SITFA.
5. El lector HTTP se implementó sin dependencias nuevas; requiere CI Linux/Windows y pruebas reales de Chrome con respuestas grandes, fragmentación, plazos, cierre de sockets, CORS y compatibilidad con `sitfa_ficticio`. La recepción JSON/base64 no es streaming hasta el disco: la cuota acota el cuerpo pero puede haber varias representaciones en memoria. No habilitar modo rápido por defecto antes de pruebas operativas.
6. El índice de Resultados no detecta cambios externos en tiempo real: se actualiza explícitamente o al cambiar de carpeta/completar acciones; se siguen verificando originales al producir. No se cambian reglas ni textos judiciales.

## Verificadores locales — salida literal

```text
AUTOPRUEBA: PASA
SOLO_LECTURA: 0 hallazgos; RESULTADO: PASA
solo_lectura rc=0
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
productos.rs: 6/6 [usuario/temporal] EML/XLSX/resoluciones en la carpeta de productos (nombres nuevos, carpeta Resoluciones_* nueva; Word por tribunal conserva matriz, índice y saltos); borrador Outlook vía temp_dir() que se limpia
revision_ui.rs: 5/5 [usuario] carpeta nueva de la revisión de bitácoras (create_dir falla si existe); ante error borra solo lo propio
trabajo.rs: 1/1 [perfil] limpieza de sesiones antiguas propias en data_dir()/sesiones (RETENCION_DIAS, SESIONES_ANTERIORES)
ui.rs: 5/5 [usuario/perfil] carpeta de productos de Ajustes; contactos.csv por diálogo Guardar; plantillas, datos y carpeta de actualizaciones (solo crear carpeta) en data_dir() y LOCALAPPDATA/FlowRust
ESCRITURAS: 122 en 25 archivos; RESULTADO: PASA (0 discrepancias)
inventario rc=0
Sin RUT reales detectados.
Sin correos ni RIT nuevos.
ids_contactos rc=0
EXCLUIDO HISTORIA.VARIOS: inalcanzable en Python: solo lo emite reglas_historia.constancia() con dos o más trámites, y la única función que se los pasa, constancias(), no tiene llamadores (rust/oraculo/generar_casos.py, fuera_del_motor)
TEXTOS: Rust 58, Python 59, excluidos 1
UMBRALES: Rust 16, Python 16
RESULTADO: PASA (0 discrepancias)
REFERENCIA_PYTHON: afc4d12ae1ad; RESULTADO: PASA (0 diferencias)
textos_referencia rc=0
git_diff_check rc=0
node_check rc=0
```

## Integración sugerida (≤ 10 líneas)

- E-P1-C2: parche C sobre 9831d8b; aplicar en rama de integración y compilar Linux/Windows.
- Puente con presupuestos diferenciados, chunked, plazos, recuperación de locks y despacho limitado a 3.
- Integral propia 2.10.0, modo rápido optativo, índice de página y anticipación local sin reutilización entre lotes.
- Resultados indexado y Descargador/Meta 4 con dos calendarios y flujo guiado.
- Verificadores Python: 0 hallazgos/discrepancias/diferencias; JS sintáctico; pruebas Rust pendientes.
- Resolver diferencia de metadato de versión del oráculo de paquete sin alterar la referencia Python.
- Validar equivalencia de evidencia/manifiestos, degradación, cancelación/reanudación y rendimiento en sesión controlada antes de habilitar rapid por defecto.


## Comprobación de entrega

Territorio autorizado: 16 archivos, 0 fuera de alcance. `git diff --cached --check`, `git diff --check` y `git -C /tmp/flow_base apply --check`: código 0. No se aplicó el parche a la base.
