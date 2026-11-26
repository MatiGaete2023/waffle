# Flow Rust · migración funcional

La aplicación Rust vigente de esta rama es [`app/`](app/), el proyecto de escritorio eframe/egui. Su entrada es `app/src/main.rs` y el ejecutable es `flow_rust.exe`. La referencia funcional congelada es la rama [`flow-python`](https://github.com/MatiGaete2023/waffle/tree/flow-python) (SHA fijado en [`app/qa/REFERENCIA_PYTHON.json`](app/qa/REFERENCIA_PYTHON.json)).

| Documento | Contenido |
|---|---|
| [Estado](docs/migracion_python_rust/ESTADO.md) | Qué hay en `app/`, qué falta, siguiente paso y pendientes del usuario |
| [Plan](docs/migracion_python_rust/PLAN.md) | Encargos por tramo y decisiones del usuario |
| [Verificación](docs/migracion_python_rust/VERIFICACION.md) | Comandos de prueba, oráculos y verificadores estáticos |
| [Diseño](docs/migracion_python_rust/DISENO.md) | Pantallas, navegación y referencia visual |
| [Continuación](docs/migracion_python_rust/CONTINUACION.md) | Coordinación, reparto de agentes y procedimiento de integración |
| [Registro](docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md) | Entradas cortas por encargo desde el 24-nov-2026 |
| [Informes](docs/migracion_python_rust/informes/README.md) | Informes firmados de agentes en paralelo |
| [Revisión del 24-nov](docs/migracion_python_rust/REVISION_20261124.md) | Fundamento de la consolidación |
| [Histórico](docs/migracion_python_rust/historico/) | Registro, encargos, matriz e informes anteriores al 24-nov |

El workspace [`rust/`](rust/) es la implementación anterior; `app/` reutiliza sus crates `flow-meta4`, `flow-anonimo` y `flow-dominio`, además del corpus `rust/casos` y los fixtures de prueba.
