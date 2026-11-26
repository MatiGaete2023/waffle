# Flow Rust · Asistente CSMP y Descargador

La aplicación vigente de esta rama (`estudio/paridad-python-rust-20261119`) es el escritorio [`app/`](app/), que compila `flow_rust.exe` (eframe/egui) junto con el instalador por usuario [`installer/`](installer/).

```powershell
cargo run --manifest-path app/Cargo.toml --locked
```

## Documentación vigente

- [Estado y siguiente paso](docs/migracion_python_rust/ESTADO.md)
- [Plan y decisiones](docs/migracion_python_rust/PLAN.md)
- [Verificación](docs/migracion_python_rust/VERIFICACION.md)
- [Diseño](docs/migracion_python_rust/DISENO.md)
- [Continuación y coordinación](docs/migracion_python_rust/CONTINUACION.md)
- [Índice de la migración](MIGRACION_RUST.md)

## Reglas invariantes

- No envía correos (Outlook solo borradores con `Save()` y CC fijo `ucc_concepcion@pjud.cl`).
- No escribe en RUS ni en SATURNO (solo lectura).
- Solo datos sintéticos en el repositorio (sin HAR, HTML, PDF, Excel ni identidades reales).
- Actualización en el lugar sin reinstalar (`%LOCALAPPDATA%\FlowRust\actualizaciones` o carpeta adicional elegida en Ajustes, con verificación SHA-256).

## Estructura del repositorio

- `app/` — aplicación Flow Rust (`flow_rust.exe`), extensión Integral `2.10.0`, pruebas y generadores de oráculos (`app/qa/`, `app/tests/datos/`).
- `installer/` — instalador y desinstalador por usuario (`Instalar_Flow.cmd`, `Instalar_Flow.ps1`, `Desinstalar_Flow.ps1`, `FlowRust.iss`, `LEEME.txt`).
- `rust/` — workspace histórico del que `app/` reutiliza `flow-meta4`, `flow-anonimo`, `flow-dominio`, el corpus `rust/casos/` y fixtures de prueba.
- `tools/` — script de pruebas (`probar_app.sh`), verificadores estáticos, anonimizador Excel y verificador del instalador Windows.
- `contracts/` — contratos y fixtures sintéticos `CSMP_CAPTURE`.
- `.github/workflows/flow-rust-local.yml` — CI de Linux y Windows (pruebas, oráculos, autodiagnóstico, instalador y entrega).

## Referencia funcional Python

Los árboles `descargador/`, `src/` y `tests/` están congelados en el commit fijado en [`app/qa/REFERENCIA_PYTHON.json`](app/qa/REFERENCIA_PYTHON.json) (verificado por `tools/verificar_referencia_python.py`) porque los oráculos de `app/qa/` y `rust/oraculo/` los ejecutan directamente, junto con `META4_Bitacoras/`, `descargador_lite/` y `anonimizador/`. Su documentación y pruebas completas viven en la rama [`flow-python`](https://github.com/MatiGaete2023/waffle/tree/flow-python).
