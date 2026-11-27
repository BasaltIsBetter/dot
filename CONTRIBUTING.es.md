# Contribuir a Dot

[English](CONTRIBUTING.md) · [Français](CONTRIBUTING.fr.md) · **Español** · [Português](CONTRIBUTING.pt.md)

Gracias por tu interés. Un error o una idea: abre una issue con su formulario. Una vulnerabilidad
de seguridad: nunca en público, consulta [`SECURITY.es.md`](SECURITY.es.md).

## Antes de escribir código

Para cualquier cambio mayor que una corrección, abre primero una issue para hablarlo. Una pull
request pequeña y centrada en una sola cosa se revisa y se fusiona mucho más rápido.

## Construir y comprobar

Los requisitos y la construcción están en el [`README`](README.es.md). Antes de abrir una pull
request:

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
cargo deny check
```

La integración continua ejecuta las mismas comprobaciones en macOS y Linux.

## Lo que necesita un cambio

- **Pruebas** para toda la lógica, en `tests/<crate>/`, en la misma ruta que el módulo probado, que
  demuestren los dos sentidos: la entrada válida pasa, la inválida se rechaza.
- **Ninguna acción de Claude sin permiso**, ninguna ruta fuera de la carpeta del proyecto, ningún
  secreto en un registro o un mensaje de error.
- **Medido, no supuesto**: un cambio en un camino crítico (el flujo del modelo, el dibujo de la
  interfaz, el arranque) da sus cifras antes y después (`cargo bench`).
- **Ningún texto visible escrito en el código**: cada texto pasa por `dot-i18n`, traducido a cada
  idioma (`crates/dot-i18n/locales/`: inglés, francés, español, portugués de Brasil). Las pruebas
  fallan si falta un texto en alguno de ellos.
- **Commits [Conventional Commits](https://www.conventionalcommits.org/)**: `feat(desktop): …`,
  `fix(model): …`, una descripción corta en imperativo, en inglés.

Al proponer una contribución, aceptas que se publique con la licencia del proyecto,
[Apache 2.0](LICENSE).
