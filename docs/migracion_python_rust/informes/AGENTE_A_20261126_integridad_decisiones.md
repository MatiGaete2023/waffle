# Firma

- Agente: A · ChatGPT
- Herramienta/modelo: ChatGPT, GPT-6.2; sandbox Linux; Git temporal y Python 3.13.5.
- Encargo: E-P1-A1, tanda 1 (P1-03 a P1-06), 26-11-2026.
- Repositorio de destino: MatiGaete2023/flow, rama de partida estudio/paridad-python-rust-20261119.
- Base declarada y verificada: `5cdf7697d3d39623db42a0efc5442b68e19f0ef4` (comentario interno del ZIP y presencia de `$m.Display($false)` y `contactos_python_migrados`). No se consultó el remoto.
- Referencia funcional Python: `afc4d12ae1ad`; verificador de referencia sin diferencias.
- Directorios: `/tmp/flow_base`, `/tmp/flow_work`. Identidad Git temporal: Agente A <agente-a@example.test>.
- Commit local de línea base: `42f559c876078d6610987ade5e9136094305ffdb` (no es un commit del remoto). No hay commits de implementación ni push: se entrega diff binario unificado del índice conforme al modo ZIP + Patch.
- Estado: cambios propuestos, verificadores estáticos ejecutados; compilación, pruebas Rust, CI y aceptación pendientes del Orquestador.

## 1. Cambios técnicos

**P1-03.** El codificador de sincronización pasa a ser la única implementación de NURUS_DECISIONES, compartida por ambos exportadores, con versión 1 y marcador `encoder` de Flow Rust. Se normalizan representaciones antiguas para comparar semánticamente (ausencia de `structured`/`mail`, RES tipado frente a `review` heredado), sin equiparar modos explícitos diferentes de dos libros canónicos. Se limita el JSON a 32.000 unidades UTF-16, conservador respecto del límite de Excel, y se rechaza el exceso antes de exportar o al importar. Se proyecta la revisión inicial de la copia preservada usando la base y sus instrucciones de escritura; el producto nuevo usa sus propias filas. Se incorporan FECHA_OBS, TT y CC de Trabajo al plan de copia. La actualización valida las filas antes de mutar, comprueba las claves de overrides, amplía las filas cortas, recalcula ante cambio de overrides incluso sin cambio de origen y conserva RES/exclusiones manuales en el recálculo. No se considera RES una columna cruda de origen.

**P1-04.** Se elimina la selección global de RES: se decide por fila y luego se agrupa por caso/tipo, considerando la exclusión, los modos estructurados, RES heredado y la evidencia de historia. En las relaciones Word se utiliza un mapa de partes visitadas por documento fuente; se reutilizan destinos al detectar un ciclo y se devuelve error al llegar al límite de 32 niveles de partes nuevas. No se cambian las matrices ni textos judiciales.

**P1-05.** La conciliación considera decisiones explícitas Word y diferencias de exclusión respecto de la regla COMUN.NO_SEGUIMIENTO como edición humana; conserva `resolucion` y `sync_decisiones`, y solicita revisión ante cambio de reglas/acciones relevantes. Estadísticas toma observación efectiva y aprobación sin edición textual. Órdenes de búsqueda convierte JSON null a cadena vacía.

**P1-06.** Se agrega procedencia de Para mediante un valor automático recordado, compatible con sesiones anteriores; una edición manual (incluso vaciar el campo) rompe esa procedencia. Al preparar de nuevo se conserva la edición manual y se actualiza el automático. Se registran por separado migraciones de catastro y configuración Python, preservando los contactos locales existentes con valor. Se centraliza CC obligatorio por coincidencia exacta de dirección (separadores coma/punto y coma, comparación sin distinción de mayúsculas), se utiliza en Outlook, EML, configuración y sello de preparado, y se eliminan CR/LF/comillas de cabeceras MIME. Se conserva Outlook Save(), sin envío.

## 2. Archivos

Solo los 11 archivos Rust autorizados y este informe:

- `app/src/copia_excel.rs`, `app/src/producto_excel.rs`, `app/src/sincronizar.rs`
- `app/src/productos.rs`, `app/src/conciliar.rs`, `app/src/estadisticas.rs`, `app/src/ordenes.rs`
- `app/src/config.rs`, `app/src/outlook.rs`, `app/src/preparados.rs`, `app/src/ui.rs` (solo correo y su prueba).
- `docs/migracion_python_rust/informes/AGENTE_A_20261126_integridad_decisiones.md`

El índice del parche es la relación exacta de archivos y líneas. No se modificaron workflows, oráculos, referencia Python, archivos de otros agentes ni documentación del Orquestador.

## 3. Verificación ejecutada en el sandbox

| Comando | Código | Resultado observado |
|---|---:|---|
| `python3 tools/verificar_solo_lectura.py` | 0 | PASA, 0 hallazgos |
| `python3 tools/inventario_escrituras.py` | 0 | PASA, 122 escrituras en 25 archivos, 0 discrepancias |
| `python3 tools/check_real_ids.py app/src` | 0 | Sin RUT reales detectados |
| `python3 tools/check_contactos.py app/src` | 0 | Sin correos ni RIT nuevos |
| `python3 tools/verificar_textos_umbrales.py` | 0 | PASA, 0 discrepancias; 58 textos Rust, 59 Python, 1 excluido preexistente; 16 umbrales en cada implementación |
| `python3 tools/verificar_referencia_python.py` | 0 | PASA, 0 diferencias |
| `git diff --check` | 0 | Sin errores de whitespace |

Contraste Python adicional: se ejecutaron mediante AST las funciones originales de `src/nurus/personal/resolutions.py` y sus auxiliares necesarios sobre ocho filas sintéticas del nuevo test Rust. Resultado real: `[(0, 'PC_INFO'), (2, 'NOMENCL'), (4, 'PC_IE'), (6, 'PC_INFO'), (7, 'PC_IE')]`, igual a lo esperado. La importación integral del módulo no estaba disponible por falta de `docxcompose`; este contraste dirigido no sustituye la suite ni prueba el código Rust.

## 4. Pruebas incorporadas, pendientes de ejecutar

- NURUS_DECISIONES v1 heredado/canónico, diferenciación de modos, plan preservado intacto y conflicto real, overrides remotos válidos e inválidos, fila corta/fila cero y límite de longitud. Se conserva la prueba de ida y vuelta de producto XLSX existente.
- Selección Word mixta con evidencia y exclusión; relaciones cíclicas sintéticas y profundidad excesiva; cabeceras MIME y CC engañoso.
- Conciliación de decisiones/exclusión, constancia aprobada sin editar, null en OB; migración independiente, CC exacto y actualización de destinatarios automáticos/manuales.

No hay Cargo disponible; **no se ejecutó compilación Rust, Clippy, tests Rust, COM, Excel, Outlook ni CI**. La ruta preservada se prueba por simulación del plan de instrucciones, no por Excel real. Los oráculos completos existentes y los nuevos tests deben ejecutarse con `tools/probar_app.sh todo` y `cargo clippy --manifest-path app/Cargo.toml --locked --all-targets`; verificar además el roundtrip Excel/COM y CI Linux/Windows, sin presumir resultados.

## 5. Decisiones y límites declarados

- `encoder` adicional dentro de JSON v1: preserva el formato de lectura Python y permite distinguir modos de dos representaciones canónicas. La comparación de libros heredados hace equivalentes `review` y tipo con la misma RES tipada. No se modificó el esquema general de Trabajo ni firmas públicas de otros territorios.
- Las sesiones antiguas de borradores no conservan procedencia de Para: se tratan conservadoramente como manuales. En sesiones nuevas se distingue incluso una edición a vacío. No se puede reconstruir con certeza la intención histórica.
- `contactos_python_migrados=true` antiguo no permite saber qué fuente fue procesada. Se reexaminan fuentes sin marca individual, completando valores faltantes y preservando los existentes no vacíos. Un contacto anteriormente eliminado o vaciado intencionalmente antes de disponer de marcas por fuente puede reaparecer; esta ambigüedad requiere revisión si es relevante. No se añade una política de tombstones.
- La detección de exclusión manual compara contra COMUN.NO_SEGUIMIENTO. Se solicita revisión por cambios en reglas/acciones; no es una comparación exhaustiva de evidencias que no modifiquen esas señales.
- El plan de copia preservada se proyecta desde el libro original; la equivalencia real con los tipos/conversiones de Excel y todas las variantes de libros heredados queda pendiente de ejecución. La paridad total no se acredita mediante verificadores estáticos.
- No se declara HECHO ni ACEPTADO. El parche no publica nada en el repositorio real.

## 6. Texto sugerido para ESTADO/PLAN/REGISTRO (no aplicado)

- E-P1-A1: Agente A entrega parche ZIP sobre 5cdf769 para P1-03 a P1-06, 11 archivos Rust y un informe.
- Unificación NURUS_DECISIONES, sincronización y decisión Word por fila, protección de relaciones, conciliación/estadísticas/OB y seguridad de correo.
- Seis verificadores Python pasan; contraste dirigido de selección Word con Python coincide.
- Aplicar en rama propia, compilar y ejecutar tests/oráculos, Clippy y CI Linux/Windows; validar roundtrip COM/Excel y borradores Outlook.
- Revisar ambigüedades heredadas de procedencia Para y migración de contactos descritas en el informe.
- Estado: propuesto, pendiente de validación e integración; no aceptado.
