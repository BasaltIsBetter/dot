# Dot

[English](README.md) · [Français](README.fr.md) · **Español** · [Português](README.pt.md)

Un agente de código para macOS y Linux, en el espíritu de Claude Code, pensado para ser más rápido,
más sobrio y más reflexivo: una ventana negra dibujada en la GPU, al estilo de
[Spaceship](https://chuanqisun.github.io/spaceship/), donde le escribes a Claude, que responde en
flujo y trabaja en la carpeta del proyecto con sus herramientas (leer, escribir, ejecutar comandos,
buscar, la web).

Dot habla inglés, francés, español y portugués de Brasil: sigue los idiomas del sistema, y el
ajuste «Idioma» lo cambia al instante.

## Instalar

Descarga la última versión desde las [releases](https://github.com/BasaltIsBetter/dot/releases):

- **Mac (Apple silicon)**: descomprime `Dot-<version>-macos-arm64.zip` y mueve `Dot.app` a
  Aplicaciones. Estas versiones preliminares aún no están firmadas por Apple: la primera vez,
  permítela en Ajustes del Sistema › Privacidad y seguridad › «Abrir igualmente», o ejecuta
  `xattr -dr com.apple.quarantine /Applications/Dot.app`.
- **Linux (x86_64 o arm64, glibc 2.35 o más reciente)**: descomprime el archivo de tu procesador
  y ejecuta su `install.sh`. Dot se instala en `~/.local/share/dot-app`, el comando `dot` en
  `~/.local/bin`, con su entrada en el lanzador de aplicaciones.

Desde el código fuente, `./crates/dot/macos/install.sh` construye la app y la instala en
`~/Applications`, con `dot` en `~/.local/bin`.

## Requisitos

- macOS en Apple silicon o Intel, con una GPU Metal; o Linux con Wayland o X11, con un controlador
  Vulkan u OpenGL (basta con `mesa-vulkan-drivers`, sin tarjeta gráfica). Con X11 también hace
  falta `libxkbcommon-x11`. Una clave de API se guarda en el llavero del escritorio (Secret Service:
  GNOME Keyring, KWallet…); sin llavero, como en WSL, hay que conectarse con Claude Code o `ant`.
- Rust: la versión está fijada en `rust-toolchain.toml` (rustup la instala solo).
- Para conectarse: un plan de Claude a través de [Claude Code](https://claude.com/claude-code), una
  cuenta de Anthropic a través de `ant`, o una clave de API de Anthropic.

## Ejecutar

```bash
cargo run --release -- ~/mi-proyecto         # Dot abierto sobre una carpeta
./crates/dot/macos/bundle.sh                 # la app de Mac: target/release/bundle/macos/Dot.app
./crates/dot/linux/bundle.sh                 # el archivo de Linux: target/release/bundle/linux/*.tar.gz
```

Atajos: intro envía, mayús+intro salta de línea, esc detiene la respuesta, cmd+V pega texto o una
imagen, cmd+N empieza una conversación nueva, ↑ y ↓ recuperan los prompts ya enviados. En Linux,
ctrl sustituye a cmd, y ctrl+Q sale.

El registro se escribe en `~/Library/Logs/Dot/dot.log` en Mac y en `~/.local/state/dot/dot.log` en
Linux (1 MiB como máximo, el anterior se guarda en `dot.log.1`); nunca contiene una clave ni un
token.

## Agentes en contenedores

Los agentes pueden vivir en contenedores Docker (o Podman) y llamarse cuando hagan falta, desde un
terminal o por Claude en Dot (`@revisor mira este archivo`): cada agente conserva su contenedor por
proyecto, que se despierta cuando se necesita, y su memoria en un volumen propio.

```bash
cargo build --release -p dot-container           # el comando: target/release/dot-agents
dot-agents new revisor                           # describe un agente, para editarlo después
dot-agents build                                 # construye la imagen, con Claude Code dentro
dot-agents --folder ~/mi-proyecto login revisor  # conecta el agente a Claude, una sola vez
dot-agents --folder ~/mi-proyecto call revisor "Revisa el último commit"
dot-agents list                                  # los agentes y sus contenedores
dot-agents --folder ~/mi-proyecto stop revisor   # duerme al agente
```

Los agentes se describen en JSON en `~/.config/dot/agents/` (`~/Library/Application
Support/Dot/agents/` en Mac), nunca dentro de un proyecto: abrir un repositorio desconocido no
inicia ninguno.

| Campo | Función | Por defecto |
|---|---|---|
| `description` | lo que Claude lee para elegir el agente | vacío |
| `workspace` | acceso a la carpeta del proyecto: `none`, `read`, `write` | `read` |
| `network` | `bridge`, o `none` para un agente sin red (que por tanto no puede ser Claude) | `bridge` |
| `memory`, `cpus`, `processes` | límites del contenedor | `2g`, 2, 512 |
| `minutes` | duración máxima de una llamada | 30 |
| `model`, `instructions` | modelo e instrucciones de Claude Code en el contenedor | ninguno |
| `command` | programa que responde en lugar de Claude Code (petición por su entrada, respuesta por su salida) | Claude Code |
| `image` | imagen del contenedor; debe incluir `sh`, `tr`, `grep` y un `/home/agent` en el que todos puedan escribir | `dot-agent:latest` |
| `env` | nombres de las variables que se pasan al agente | ninguna |

El contenedor no tiene ningún privilegio (`--cap-drop ALL`, `no-new-privileges`), se ejecuta con la
identidad del usuario y solo ve el proyecto, en la misma ruta que en la máquina; la carpeta personal
entera y la raíz del disco se rechazan. Dentro, Claude Code no pide permiso: es el contenedor el
que limita al agente. Un agente en `write` puede por tanto cambiar cualquier cosa del proyecto,
`.git/hooks` incluido. Claude puede llamar a los agentes en Dot cuando `dot-agents` está junto a
`dot` y la conexión pasa por Claude Code; todavía no con una clave de API.

## Comprobar

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
cargo deny check
cargo bench -p dot-desktop --features bench    # benchmarks de criterion
```

La integración continua (`.github/workflows/ci.yml`) ejecuta las mismas comprobaciones en cada push
y en cada pull request. Para contribuir, consulta [`CONTRIBUTING.es.md`](CONTRIBUTING.es.md); para
informar de una vulnerabilidad, [`SECURITY.es.md`](SECURITY.es.md).

## Organización

Un crate por responsabilidad en `crates/`, con sus pruebas aparte en `tests/<crate>/`:

| Crate | Función |
|---|---|
| `dot` | el binario: ensambla los componentes y lleva el registro |
| `dot-api` | contratos y tipos compartidos (conversación, herramientas, permisos, sesión, textos) |
| `dot-auth` | la conexión con Claude |
| `dot-compact` | acorta lo que Claude lee de los comandos |
| `dot-container` | los agentes que viven en contenedores, y su comando `dot-agents` |
| `dot-desktop` | la app de Mac y Linux: ventana, conversación, barra de estado (winit + wgpu) |
| `dot-i18n` | las traducciones (Fluent, `crates/dot-i18n/locales/`) |
| `dot-model` | las respuestas de Claude: API Messages o Claude Code |
| `dot-process` | los programas que Dot lanza (entorno, grupo, parada, espera) y la lectura acotada, línea a línea, de lo que lee de ellos |
| `dot-session` | la sesión de una ventana: carpeta, rama de git, consumo; y lo que se guarda entre dos arranques |
| `dot-shell` | sustituye al shell de los comandos de Claude Code, para acortar lo que muestran |
| `dot-tool` | las herramientas de Claude en la máquina |
| `dot-transcript` | la conversación tal como se lee, sin ventana ni GPU: historial, saltos de línea, movimiento |

## Licencia

Con licencia [Apache 2.0](LICENSE); los créditos están en [`NOTICE`](NOTICE).

- Parte de las reglas de `dot-compact` está adaptada de [RTK](https://github.com/rtk-ai/rtk)
  (Apache 2.0): ver [`crates/dot-compact/third-party/rtk/`](crates/dot-compact/third-party/rtk/).
- El diseño de la interfaz se inspira en [Spaceship](https://github.com/chuanqisun/spaceship), de
  Sun (MIT); no se toma código de él.
- La fuente JetBrains Mono incluida tiene licencia SIL Open Font License 1.1.
