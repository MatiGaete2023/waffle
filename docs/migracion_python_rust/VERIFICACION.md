# Verificación de `app/`

La versión anterior, con los comandos de PowerShell paso a paso y el crate `.aislado`, está en [historico/VERIFICACION_20261123.md](historico/VERIFICACION_20261123.md). El crate `.aislado` ya no hace falta.

## Niveles

| Nivel | Cuándo | Comando |
|---|---|---|
| Rápido | Mientras se trabaja un módulo | `tools/probar_app.sh rapido <filtro>` |
| Completo | Antes de publicar un encargo | `tools/probar_app.sh todo` |
| Crates reutilizados | Si cambia `rust/crates/` | `cargo test --manifest-path rust/Cargo.toml --locked -p flow-meta4 -p flow-anonimo --features serde_json/preserve_order` |
| Clippy | Antes de publicar cambios en `app/` | `cargo clippy --manifest-path app/Cargo.toml --locked --lib -- -D warnings` |
| CI | En cada push a `estudio/paridad-python-rust-20261119` o `estudio/agente-*` | Flujo [`.github/workflows/flow-rust-local.yml`](../../.github/workflows/flow-rust-local.yml) |

- `tools/probar_app.sh` fija `FLOW_CASOS`, `FLOW_FIXTURES` y `FLOW_REFERENCIA` con los datos del repositorio y filtra la salida a resultados y fallos.
- `todo` agrega `--include-ignored`: corre los oráculos (`oraculo_python`, `oraculo_historia`, paquete B, cronología) y las pruebas con datos versionados.
- En Linux, `eframe/x11` ya está declarado en `app/Cargo.toml` bajo `[target.'cfg(target_os = "linux")'.dependencies]` (E-V2), por lo que `cargo test --manifest-path app/Cargo.toml --locked --all-targets` compila y corre sin banderas adicionales.

## Pruebas que necesitan un ZIP generado por Flow Python

Quedan fuera de `todo` si falta `FLOW_E3_PAQUETE`. La CI de Windows las ejecuta siempre.

```bash
python app/qa/generar_contexto_e3.py <checkout de flow-python> <carpeta>
export FLOW_E3_PAQUETE=<carpeta>/contexto-e3.zip
export FLOW_E3_HISTORIA=<carpeta>/contexto-e3-historia.zip
python app/qa/esperado_contexto_e3.py "$FLOW_E3_HISTORIA" <carpeta>/esperado-historia.json
export FLOW_E3_ESPERADO=<carpeta>/esperado-historia.json
tools/probar_app.sh todo
```

Requiere Python 3.12 con `requirements/linux-py312.lock` o `windows-py312.lock`. `exporta_mismo_lote_de_python` (`FLOW_P2_PAQUETE`) y `velocidad_20000` se omiten por diseño.

## Oráculos grabados (23)

- Datos en `app/tests/datos/`: `a78`, `anulaciones`, `archivo_local`, `bitacoras`, `catastro`, `conciliacion`, `correos`, `estadisticas`, `excel`, `flujo_csmp`, `flujo_paquete`, `historial_local`, `lecturas`, `lote`, `meta4`, `ordenes`, `paquete`, `personalizacion`, `resultados`, `sync`, `sync_paquete`, `vinculos` y `word_tribunal` (`.json`).
- Generadores: `app/qa/generar_<nombre>.py`. Regenerar solo si cambian el generador o la referencia Python (`app/qa/REFERENCIA_PYTHON.json`), y revisar `git diff -- app/tests/datos`.
- Corpus del motor: `rust/casos/*.json`, generado por `rust/oraculo/generar_casos.py` y verificado con `cargo test --manifest-path rust/Cargo.toml --locked -p flow-motor`.

## Verificadores estáticos

Solo se corren en local cuando el cambio toca lo que revisan; la CI (`linux`) los corre en cada push:

```bash
python3 tools/check_real_ids.py app/src
python3 tools/check_contactos.py app/src
python3 tools/verificar_textos_umbrales.py
python3 tools/inventario_escrituras.py
python3 tools/verificar_solo_lectura.py --autoprueba
python3 tools/verificar_solo_lectura.py
python3 tools/verificar_referencia_python.py
python3 tools/check_links.py
```

## Windows y entrega

- El `.exe` y el instalador salen en el artefacto `Flow_Rust_Windows_exe` (`flow_rust.exe`, `SHA256.txt`, `Instalar_Flow.cmd`, `Instalar_Flow.ps1`, `Desinstalar_Flow.ps1`, `LEEME.txt` y, con `entrega=true`, el instalador Inno Setup `Instalar_Flow_Rust_*_Windows.exe`).
- Con `workflow_dispatch` y `entrega=true`, la CI además compila `--release`, corre `--self-test`, verifica el contraste V1 con `rust/` y sube las capturas sintéticas en `Flow_Rust_Local_Capturas_Windows` (contrastables con [DISENO.md](DISENO.md)).
- Verificación del instalador en Windows: `./tools/verify_flow_rust_installer.ps1 -BundleDir app/target/entrega`.
- Comprobación cruzada desde Linux (sin ejecutar): `rustup target add x86_64-pc-windows-gnu` y `cargo check --manifest-path app/Cargo.toml --locked --target x86_64-pc-windows-gnu --all-targets`.
