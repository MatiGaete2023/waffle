# Firma

- Agente: B · ChatGPT
- Herramienta/modelo: ChatGPT GPT-6.2, sandbox Linux, modalidad ZIP + parche.
- Identidad del Git temporal: Agente B <agente-b@example.test>.
- Repositorio lógico: MatiGaete2023/flow; rama de integración estudio/paridad-python-rust-20261119 (no se operó sobre remoto).
- Base declarada y verificada: `9831d8bd31799a2b89e6f3b14b9dfa044d53b8d3` (comentario interno de `flow_tanda1_integrada.zip`, extraído del contenedor de Google Drive `drive-download-20261126T161951Z-1-001.zip`). Los tres marcadores exigidos coinciden.
- Copias: `/tmp/flow_base` (original, sin edición de fuentes), `/tmp/flow_work` (trabajo).
- Único commit temporal: `e7e8373fcf0db08630fbd042de385479cbef3c87` — `base 9831d8b`. El commit temporal no es el SHA remoto. No hay commits de implementación ni push: se entrega el diff del índice, según encargo ZIP.
- Encargo: E-P1-B2, P1-08 + V-01 parcial + higiene de exportación. Estado: implementado en parche, pendiente de compilación, pruebas y revisión del Orquestador. No constituye aceptación.

## Resumen técnico

1. `lectura.rs`: límites explícitos de 80 MiB por entrada, 200 MiB acumulados para XLSX, 4.096 entradas ZIP, 200.000 filas, 16.384 columnas y 2.000.000 celdas; profundidad XML 128; índices SpreadsheetML positivos y hasta 16.384. Lectura efectiva `Read::take(max+1)` antes de Calamine. Prevalidación XML (DTD, referencias y estructura) y coordenadas/dimensiones dispersas OOXML antes de construir la matriz. Se mantienen la interpretación de formatos y los límites existentes del Descargador; se rechaza una hoja que Calamine no puede leer, en vez de omitirla silenciosamente. No se modifica el algoritmo de interpretación de las fechas de dominio.
2. `descargador/xls.rs`: utiliza la prevalidación XML/ZIP anterior, conserva el mensaje genérico de D-3 para DTD/entidad externa, conserva 200 columnas del Descargador y sus mensajes para índice 201, filas no crecientes y exceso de celdas; limita incluso filas/celdas vacías, acumulación de grillas y tablas anidadas. El HTML sigue pasando por el lector existente. Se usa aritmética saturada en el conteo de Calamine.
3. `descargador/paquete.rs`: la ruta referenciada debe ser un nombre local, resolverse dentro de la base canonicalizada y no ser un alias/enlace a otra ruta. Se preservan diagnósticos de ausencia de archivo. Los controles JSON y copias de respaldos se leen acotadamente (32 MiB JSON; 8 MiB HTML de bitácora e historia; 80 MiB original). Se evita la doble llamada a cargar historias y se usa la misma lectura del manifiesto para su huella y la construcción de trámites y Escritos. Se mantiene la interfaz de `cargar_historias` para Excel. En ZIP de captura generado se mide la expansión con `io::copy` y `take`, 80 MiB acumulados (el contrato existente ya tiene ese límite). Se conserva la validación del contrato CSMP_CAPTURE. Se prevalidan las referencias del histórico antes del lector delegado.
4. `descargador/resultados.rs`: se separa construcción de hojas/metadatos de publicación. `libro_consolidado` pasa directamente por `excel::libro` (`save_to_buffer` existente), eliminando la carpeta temporal de XLSX/JSON sensibles y su borrado recursivo. El XLSX final conserva el mecanismo `guardar_exclusivo` y el JSON hermano pasa al mismo mecanismo, sin sobrescribir. El ZIP de entrega lee archivos locales acotados, hasta 200 MiB acumulados, y verifica bytes reales de sus entradas, sin clonar todo el ZIP para leerlo.
5. `descargador/flujo_csmp.rs`: se conserva la ruta de conversión de valores existente, se recupera presentación útil de encabezados, ancho de columnas y formato de cuerpo al ensamblar; se retienen congelado y filtro de la hoja simple. El control y la carpeta de fase se validan dentro del conjunto, con temporal exclusivo y nombre distinto para guardar estado. La comprobación del oráculo de valores sigue pendiente de Cargo.
6. `flow-meta4/plantilla.rs`: lectura ZIP por parte y acumulada (20 MiB archivo de plantilla, 8 MiB por entrada de plantilla, 80 MiB por entrada del generado y 200 MiB acumulados), 512 partes; validación de profundidad, contadores, coordenadas y dimensiones antes de abrir con flow_tabular. Se usa una instantánea de bytes validada, sin releer del disco por la ruta. Se corrige el underflow de `cabecera.len()-n.len()` insertando primero el nuevo formato en el cierre original y cambiando luego la apertura. Se cuenta `numFmt` real, se rechazan contadores malformados, se acota identificador de formato y se valida fila/columna. Temporal con `create_new`, limpieza solo después de adquirirlo. No se cambia la lista de hojas de Meta 4.
7. `tools/inventario_escrituras.py`: solo la línea de Resultados, de 3 a 1 coincidencias estáticas (metadato exclusivo y eliminación de carpeta temporal). Las coincidencias de flujo CSMP se mantienen en 5. No se cambia ningún archivo de otro agente.

## Pruebas incorporadas (no ejecutadas en Rust)

- `lectura`: índices 0, negativos, 16.385 y fuera de rango numérico; XML malformado y profundidad 129; dimensión XLSX extrema; cuota real individual y acumulada con ZIP sintético de 8 bytes por entrada; conversión de serial nativo 25.569 a 1970-01-01.
- `xls`: índice 0, negativo y >16.384, XML malformado y anidado; las pruebas existentes de mensajes de D-3 y columna 201 se preservan.
- `paquete`: intento de escape `../`, rutas absolutas POSIX/Windows y relativas a unidad, enlace fuera del lote en Unix, ZIP de 8 bytes rechazado con cuota de 7. El test existente `paquete_igual_a_python` incorpora un contador de cargas por hilo en el escenario con historias, para exigir una sola llamada durante la construcción.
- `plantilla`: prueba unitaria de cuota real individual y acumulada con ZIP de entradas de 8 bytes; prueba de integración mutando estilos sintéticos con count 01, 0, abc y vacío, verificando salida válida o rechazo sin publicación. Las 30 pruebas originales y las nueve hojas auxiliares no se editaron.
- Contraste independiente Python (no ejecuta Rust): se inspeccionaron 14 XLSX sintéticos únicos incrustados en los oráculos disponibles de paquete, bitácoras, archivo local, flujo paquete, flujo CSMP, resultados, Excel y XLS, con lectura ZIP, XML, profundidad, tamaño y coordenadas. Resultado literal: `XLSX sintéticos únicos inspeccionados: 14 ; incidencias: 0 []`. No acredita equivalencia de valores ni ejecución de oráculos.

## Verificadores realmente ejecutados

Todos los comandos solicitados se ejecutaron sobre la versión final del código, antes del informe. Registro literal a continuación. La opción local de Git `core.whitespace=cr-at-eol` conserva los CRLF preexistentes de lectura.rs, sin cambiar archivos de configuración del repositorio; no deshabilita otras verificaciones de whitespace. El primer intento sin esta opción reportó CRLF como whitespace y fue corregido en la configuración local. El primer inventario antes de ajustar la línea de Resultados falló (3 vs 1); el último resultado es el que corresponde al parche.

```text

$ python3 tools/verificar_solo_lectura.py --autoprueba && python3 tools/verificar_solo_lectura.py
AUTOPRUEBA: PASA
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
descargador/resultados.rs: 1/1 [usuario] productos de Resultados (E-S1): carpeta del lote o la elegida; XLSX y metadato JSON con guardar_exclusivo; el libro «Para CSMP» del paquete se arma en memoria
estadisticas.rs: 2/2 [usuario/perfil] resumen de constancias en la carpeta de productos con nombre nuevo; metricas.json solo con la opción activada, vía config::atomic_write
manual.rs: 3/3 [usuario] Word manual en la carpeta de productos con un nombre nuevo (no reemplaza; archivo temporal y renombrado); los favoritos y lo escrito van por config::atomic_write
outlook.rs: 5/5 [temporal] borrador.json y la nómina en una carpeta temporal propia que se borra al terminar; Outlook solo recibe Save()
personalizar.rs: 4/4 [perfil] matrices Word importadas a data_dir()/plantillas (el original se respalda como .bak.docx; escritura por archivo temporal)
producto_excel.rs: 1/1 [usuario] carpeta de productos de Ajustes; el libro se guarda en una ruta nueva (nueva_ruta nunca reemplaza un archivo)
productos.rs: 6/6 [usuario/temporal] EML/XLSX/resoluciones en la carpeta de productos (nombres nuevos, carpeta Resoluciones_* nueva; Word por tribunal conserva matriz, índice y saltos); borrador Outlook vía temp_dir() que se limpia
revision_ui.rs: 5/5 [usuario] carpeta nueva de la revisión de bitácoras (create_dir falla si existe); ante error borra solo lo propio
trabajo.rs: 1/1 [perfil] limpieza de sesiones antiguas propias en data_dir()/sesiones (RETENCION_DIAS, SESIONES_ANTERIORES)
ui.rs: 5/5 [usuario/perfil] carpeta de productos de Ajustes; contactos.csv por diálogo Guardar; plantillas, datos y carpeta de actualizaciones (solo crear carpeta) en data_dir() y LOCALAPPDATA/FlowRust
ESCRITURAS: 120 en 25 archivos; RESULTADO: PASA (0 discrepancias)
EXIT=0

$ python3 tools/check_real_ids.py app/src && python3 tools/check_contactos.py app/src
Sin RUT reales detectados.
Sin correos ni RIT nuevos.
EXIT=0

$ python3 tools/verificar_textos_umbrales.py && python3 tools/verificar_referencia_python.py
EXCLUIDO HISTORIA.VARIOS: inalcanzable en Python: solo lo emite reglas_historia.constancia() con dos o más trámites, y la única función que se los pasa, constancias(), no tiene llamadores (rust/oraculo/generar_casos.py, fuera_del_motor)
TEXTOS: Rust 58, Python 59, excluidos 1
UMBRALES: Rust 16, Python 16
RESULTADO: PASA (0 discrepancias)
REFERENCIA_PYTHON: afc4d12ae1ad; RESULTADO: PASA (0 diferencias)
EXIT=0

$ git diff --check
EXIT=0
```

## Límites y decisiones para la revisión

- No hay cargo ni rustc locales. No se ejecutaron Cargo, clippy, los 23 oráculos Rust, las 30 pruebas previas de flow-meta4 ni las pruebas nuevas. No hay resultados CI ni garantía de compilación. Es imprescindible probar los cambios, en particular las API quick-xml 0.42 de app frente a 0.41 de Meta 4 y las condiciones de paridad. Si una prueba de estilo altera un valor esperado, debe prevalecer la paridad del oráculo, retirando la recuperación de presentación de flujo_csmp si resulta necesario.
- Fecha pre-1982: `lectura.rs` delega las fechas nativas de Calamine a `util::excel_serial`, que admite series positivas inferiores a 100.000, incluyendo 1970. Sin embargo, `util::get_date` y la ruta de texto numérico limitan números sin formato a `>30000 && <60000` (30.000 corresponde a 18-02-1982). No se cambia `util.rs` ni se convierte cualquier número a fecha, para evitar cambios de dominio no autorizados. Queda pendiente una decisión explícita sobre fechas sin formato en columnas semánticamente de fecha.
- Las cuotas son una decisión defensiva: pueden rechazar libros técnicamente válidos pero fuera de estos límites, y la prevalidación implica una pasada adicional de descompresión. El contraste de 14 fixtures no sustituye pruebas completas. BIFF queda sujeto al límite de entrada y control de rango tras Calamine: no se audita internamente su asignación previa (dependencia fuera del territorio). Los lectores de otros módulos/crates prohibidos no se modificaron.
- La validación de rutas es estricta: los alias de nombre y enlaces se rechazan incluso si su destino está dentro de la base; las rutas seleccionadas directamente por el usuario siguen siendo raíces independientes. No es una defensa atómica contra un atacante que intercambie rutas concurrentemente entre canonicalización y apertura. Histórico tiene un lector delegado fuera del territorio, prevalidado antes de invocarlo; no se afirma que sus lecturas propias sean atómicas ni libres de TOCTOU.
- La publicación de Resultados es exclusiva por archivo, no una transacción atómica de pareja XLSX/JSON; se conserva el mecanismo existente. El reemplazo de la planilla recién generada en Meta 4 conserva la operación final preexistente (eliminar/renombrar); si el renombrado falla, no se garantiza recuperación de esa planilla intermedia. El origen de plantilla nunca debe ser el generado. No se modifica el publicador externo de Meta 4.
- No se agregaron datos reales, contactos, acceso a red, operaciones de escritura en SITFA/RUS/SATURNO, cambios de reglas/umbrales judiciales, ni ediciones de oráculos o archivos prohibidos. No se ejecutaron operaciones Git remotas.

## Archivos tocados (lista exacta)

- app/src/lectura.rs
- app/src/descargador/xls.rs
- app/src/descargador/paquete.rs
- app/src/descargador/resultados.rs
- app/src/descargador/flujo_csmp.rs
- rust/crates/flow-meta4/src/plantilla.rs
- rust/crates/flow-meta4/tests/plantilla.rs
- tools/inventario_escrituras.py (solo línea de Resultados)
- docs/migracion_python_rust/informes/AGENTE_B_20261126_cuotas_lectores.md

## Validación pendiente en el repositorio real

Aplicar primero con `git apply --check`, después `git apply`. Ejecutar `tools/probar_app.sh todo`, `cargo clippy --manifest-path app/Cargo.toml --locked --all-targets`, `cargo test --manifest-path rust/Cargo.toml --locked -p flow-meta4` y CI Linux/Windows, incluyendo las pruebas de lectores, ZIP, paquetes, Resultados y flujo CSMP. Comparar los oráculos congelados, sin regenerarlos ni editar expectativas. Verificar que el aspecto de flujo CSMP no modifica valores. Si falla una API o prueba, corregir únicamente dentro del territorio asignado antes de integrar.

## Texto sugerido para ESTADO / PLAN / REGISTRO (7 líneas)

- E-P1-B2: parche Agente B sobre base 9831d8b, pendiente de compilación y revisión.
- P1-08: cuotas de lectura XML/ZIP y dimensiones, rutas canonicalizadas y Meta 4 count sin resta unsigned.
- V-01 parcial: reutilización de una única carga de historias y su huella; prueba añadida de conteo.
- Resultados: consolidado en memoria y metadato exclusivo; flujo CSMP conserva presentación útil, pendiente de paridad.
- Verificadores estáticos: solo lectura, inventario (120 escrituras, 0 discrepancias), identificadores, contactos, textos/umbrales y referencia Python pasan.
- No se ejecutó Cargo ni CI; validar 23 oráculos y pruebas flow-meta4 en integración.
- Pendientes explícitos: fechas numéricas sin formato pre-1982 en util.rs y garantía de recuperación ante fallos de publicación; no cambian reglas de dominio.

Agente: B · ChatGPT
