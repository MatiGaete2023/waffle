# Firma

- Agente: B · ChatGPT
- Herramienta/modelo: ChatGPT / GPT-6.2; sandbox, modo ZIP + Patch.
- Repositorio de referencia: MatiGaete2023/flow; rama estudio/paridad-python-rust-20261119.
- Base declarada: `5cdf7697d3d39623db42a0efc5442b68e19f0ef4`. Comprobada mediante comentario interno del ZIP (SHA completo) y los marcadores `$m.Display($false)` de `app/src/outlook.rs` y `contactos_python_migrados` de `app/src/config.rs`. Esto acredita coincidencia con los controles solicitados, no una consulta al remoto.
- Identidad Git temporal: Agente B <agente-b@example.test>.
- Copia original: `/tmp/flow_base`; copia de trabajo: `/tmp/flow_work`.
- Commit Git temporal de línea base: `fd2d8345767e5ae5134219b2c48a4b5121b83a63` — `base 5cdf769`. No es el SHA del remoto. No se crearon commits de implementación ni se publicó rama: entrega por parche unificado, según protocolo ZIP + Patch.
- Fecha: 2026-11-26. Encargo: E-P0-B1. Estado: código y pruebas incorporados al parche, pendiente compilación, ejecución y revisión del Orquestador. No equivale a aceptación.

## Cambios técnicos

**P0-02 / P1-01 — Entrega Windows.** El PS1 genera `.flow_rust_manifest.json` con esquema `FlowRustOwnership/1`, destino absoluto normalizado y nombres exactos de archivos propios. El desinstalador exige el manifiesto válido antes de cualquier borrado, comprueba una lista cerrada de nombres, rechaza enlaces/junctions en rutas y objetos de instalación, protege carpetas de usuario y datos Flow, y elimina exclusivamente archivos declarados, los dos residuos de actualización expresamente autorizados y el manifiesto. Borra el directorio con `Directory.Delete(..., false)` solo si está vacío; nunca usa borrado recursivo del destino. Los accesos directos y la clave de desinstalación se retiran únicamente si corresponden a ese destino. Se rechaza una instalación que chocaría con archivos de nombre reservado no acreditados por un manifiesto, en vez de atribuirles propiedad automáticamente.

El CMD usa `-SourceDir "%~dp0."`, conserva `-RequireSha256`, admite argumentos PS1 adicionales para probar la ruta real en un destino aislado y devuelve explícitamente código 0 en éxito. El PS1 rechaza `RequireSha256` con `SinHuella`, exige huella salvo excepción manual explícita, prepara todos los archivos en temporales exclusivos contiguos y comprueba nuevamente tanto el ejecutable como la huella copiada antes de publicar mediante `File.Replace`/`File.Move`. No hay alternativa recursiva en `UninstallString`; los literales de ruta escapan apóstrofos. Copia LEEME opcional y el ISS incluye LEEME con `skipifsourcedoesntexist`. El verificador Windows ejercita CMD con espacios y apóstrofo, ausencia de SHA, combinación contradictoria, desinstalación con archivo testigo, carpeta de extracción sin manifiesto, manifiesto adulterado y carpeta vacía.

**P1-02 — Actualización.** `buscar` y `buscar_con_adicional_pausa` exigen SHA-256 válida incluso para un binario igual al actual. Se reconoce la entrega CI normal/debug, BOM UTF-8 y digest sin nombre, se rechazan huellas contradictorias y las que no corresponden al ejecutable. Se limita ejecutable a 256 MiB y declaración SHA a 64 KiB; hash con `io::copy` a `Sha256` y copia con `io::copy` acotado. En Windows, apertura exclusiva `share_mode(0)` para el control de disponibilidad. La pareja se prepara en un directorio temporal exclusivo de la carpeta fija, se verifica la huella copiada, se respalda la pareja previa, se publica primero exe y luego SHA, y se revierte si falla una operación. Si la restauración no puede concluir, se conserva el respaldo y se informa su ruta. `aplicar` revalida además la huella declarada antes de tocar el programa. `volver` revierte el intercambio si falla el segundo o tercer renombrado y no sobreescribe un intercambio previo pendiente. Pruebas de ausencia/corrupción/otra versión, alteración posterior, fallo inyectado de segunda publicación y de tercer paso de intercambio; prueba de SHA copiada discordante preservando la pareja anterior. Se ajustó únicamente la entrada `actualizacion.rs` en el inventario (19 → 17 coincidencias de su patrón estático).

**P1-07 — Anonimizador.** Esquemas de `escritos` y `historico_proteccion` tomados de las columnas del Descargador (`app/src/descargador/paquete.rs`, solo lectura); siete estados de tablas para v1.0.0/v1.1.0 y nueve para v1.2.0. Se permiten CSV ausentes de tablas no solicitadas (como el productor original); se rechazan tablas fuera de la versión, IDs primarios vacíos/duplicados y referencias sin destino. La salida 1.2.0 conserva la versión y estados; la 1.1.0 conserva su versión si se reconstruye el consolidado (comportamiento anterior: se degrada a 1.0.0 si no se conserva el consolidado). `requester` seudonimizado como persona, `doc` como token, `linked_ingreso_id` comparte espacio de pseudónimos con `ingreso_id`, fechas desplazadas si válidas (ISO/DMY; literal `--` conservado), texto libre con las reglas del modo seleccionado. JSON literal de nuevas tablas conserva claves conocidas y anonimiza sus valores con el mismo tratamiento de columna; las claves desconocidas se renombrarán y sus valores textuales/numéricos se omiten preventivamente. La cuota ZIP usa `saturating_mul(300)` y el mínimo de cuota de miembro, ratio y saldo real; lectura limitada a ese máximo + 1, comprueba tamaño real contra el declarado y acumula `total_real` hasta 200 MiB. Pruebas sintéticas de ida y vuelta de las tres versiones, ambos modos, IDs, fechas, huellas, alteración de versión e ID duplicado, cuota y cabecera alterada; prueba de integración en la capa `app/src/anon.rs` con paquete sintético existente.

## Archivos tocados

- `installer/Instalar_Flow.cmd`, `installer/Instalar_Flow.ps1`, `installer/Desinstalar_Flow.ps1`, `installer/FlowRust.iss`
- `tools/verify_flow_rust_installer.ps1`, `app/tests/instalador.rs`
- `app/src/actualizacion.rs`, `tools/inventario_escrituras.py` (solo una línea)
- `app/src/anon.rs`, `rust/datos/captura_anonima.json`, `rust/crates/flow-anonimo/src/archivos.rs`, `rust/crates/flow-anonimo/src/formatos.rs`, `rust/crates/flow-anonimo/tests/paridad.rs`
- Este informe.

No se tocaron archivos reservados a otros agentes, documentos centrales, referencias Python ni workflows. No se accedió a RUS/SATURNO, no hubo comunicaciones ni datos reales incorporados. El diff incluye solo archivos autorizados.

## Verificación ejecutada (sandbox)

| Comando/control | Resultado observado |
|---|---|
| `python3 tools/verificar_solo_lectura.py` | 0 hallazgos, salida 0 |
| `python3 tools/inventario_escrituras.py` | 120 coincidencias en 25 archivos; 0 discrepancias, salida 0 |
| `python3 tools/check_real_ids.py app/src` | Sin RUT reales detectados, salida 0 |
| `python3 tools/check_contactos.py app/src` | Sin correos ni RIT nuevos, salida 0 |
| `python3 tools/verificar_textos_umbrales.py` | 58 textos Rust / 59 Python, 1 exclusión preexistente; 16 umbrales por lado, 0 discrepancias, salida 0 |
| `python3 tools/verificar_referencia_python.py` | Referencia afc4d12ae1ad; 0 diferencias, salida 0 |
| `python3 -m json.tool rust/datos/captura_anonima.json` | JSON válido, salida 0 |
| Comprobación Python dirigida de los encabezados/estados de tabla de 10 ZIP sintéticos de `flow-trabajo/tests/datos` | 0 discrepancias; comprobación adicional de controles/referencias del fixture de histórico: 0 discrepancias. No sustituye las pruebas del motor. |
| `git diff --check` | Sin errores de whitespace |

## Pendiente de compilación y pruebas reales

No estaban disponibles `cargo`, `rustc` ni `pwsh` en el PATH. No se ejecutaron Cargo, Clippy, Windows PowerShell, Inno Setup, ni CI; la inspección manual y los verificadores no certifican la sintaxis o semántica ejecutada. El Orquestador debe aplicar el parche, compilar y ejecutar `tools/probar_app.sh todo`, `cargo test --manifest-path rust/Cargo.toml --locked -p flow-anonimo --features serde_json/preserve_order`, `cargo clippy --manifest-path app/Cargo.toml --locked --all-targets`, el verificador PS1 sobre una entrega Windows real y la CI Linux/Windows correspondiente. Prestar atención a la compatibilidad PS 5.1, `File.Replace`, comillas en CMD, reparse points, regresiones de fixtures de captura y rollback.

## Decisiones y límites a revisar

- El PS1 no adopta automáticamente una instalación anterior que no tenga manifiesto. Si la carpeta ya contiene `flow_rust.exe` u otro archivo que se desea reemplazar, se rechaza para evitar apropiarse de archivos ajenos. El Orquestador debe decidir si ofrece una migración explícita, verificable y segura para instalaciones legadas (incluidas las de Inno). Esto es un cambio de compatibilidad deliberado y no una funcionalidad migratoria implementada.
- Los temporales llevan sufijo GUID aleatorio en vez de un nombre fijo `.tmp`, para evitar sobrescribir temporales preexistentes. La publicación del instalador es atómica por archivo, no una transacción de todo el instalador; un fallo entre reemplazos o antes del manifiesto puede requerir recuperación manual. Los accesos directos ajenos se conservan y una clave HKCU de otro destino se rechaza.
- La pareja del actualizador se revierte ante errores de operación notificados, pero no se implementó un diario de recuperación automática ante caída abrupta del proceso o transacciones simultáneas; los respaldos pueden necesitar recuperación manual. SHA-256 verifica integridad respecto de la declaración, no autenticidad criptográfica del proveedor.
- El tratamiento de los literales de v1.2 privilegia privacidad sobre reversibilidad: las claves libres desconocidas se anonimizan y sus valores se omiten; una fecha literal nueva no reconocible se deja vacía. Se preservan relaciones de IDs normalizados, no necesariamente toda semántica de texto libre en modo estricto. La revisión manual sigue siendo necesaria antes de compartir copias.
- El alcance no incluye otros lectores XML/ZIP de Flow, ni cambia reglas judiciales ni workflows.

## Texto sugerido para ESTADO/PLAN/REGISTRO (≤ 10 líneas)

E-P0-B1, Agente B: parche ZIP + Patch sobre 5cdf769, con manifiesto de propiedad, desinstalación acotada, instalación CMD/PS1/LEEME, SHA obligatorio y copia/rollback de actualizaciones, anonimizador CSMP_CAPTURE 1.0/1.1/1.2 y cuota real ZIP.
Verificadores Python y JSON ejecutados en sandbox: sin hallazgos ni discrepancias; se contrastaron encabezados/estados de 10 paquetes sintéticos.
Pruebas Rust/Windows añadidas pero no ejecutadas en sandbox; pendiente compilación, pruebas de CI e integración por Orquestador.
Revisar política de migración de instalaciones legadas sin manifiesto y recuperación ante caída abrupta. No marcar ACEPTADO hasta validación real y acta del usuario.
