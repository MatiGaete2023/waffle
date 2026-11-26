# Descargador SITFA Lite

Aplicación de escritorio independiente (`descargador_lite.exe`) en modo **solo lectura** para descargar listados desde SITFA usando la sesión activa del usuario en Chrome.

## Alcance

- **Pantallas admitidas (4):**
  - **Seguimiento:** pestañas Espera, Cumplimiento, Informes y Egresados; tribunales; modalidades (Residencia, Ambulatorio, FAE, DCE); filtros de tipos de informe y de medidas; rango de fechas o «Todas las fechas»; opción de conservar filtros adicionales del formulario.
  - **Búsqueda de litigantes:** tribunales, estados de búsqueda, rango de fechas o «Todas las fechas», y opción de conservar filtros adicionales.
  - **Informes por vencer:** tribunales, meses y año.
  - **Medidas por vencer:** tribunales, meses y año.
- **Pantallazo en PDF sin registros:** cuando una consulta no devuelve registros, captura el pantallazo de SITFA y guarda un PDF verificado.
- **Etiquetado oficial de archivos:** conserva las reglas vigentes de nomenclatura y verificación (`.xls`, `.pdf`, `verificacion NNN.json`, `resumen.json`) sin sobrescribir archivos existentes, y permite retomar pendientes de un lote interrumpido.
- **Consolidación en Excel (.xlsx):** reúne una o más carpetas descargadas en un único libro Excel con hojas `Resumen`, `SITFA_ARCHIVOS`, hojas por familia/esquema y `Vencimientos`.
- **Extensión de Chrome compartida:** utiliza exactamente la misma extensión de Chrome que la versión completa (`%LOCALAPPDATA%\FlowRust\extension-integral`, versión `2.10.0`).

## Compilación y pruebas

```bash
cargo run --manifest-path app/Cargo.toml --locked --bin descargador_lite
cargo test --manifest-path app/Cargo.toml --locked --bin descargador_lite
cargo build --manifest-path app/Cargo.toml --locked --release --bin descargador_lite
```

En GitHub Actions, el flujo [`.github/workflows/descargador-lite.yml`](.github/workflows/descargador-lite.yml) prueba en Linux y Windows y publica el artefacto `Descargador_SITFA_Lite_Windows_exe` (`descargador_lite.exe` y `SHA256.txt`).
