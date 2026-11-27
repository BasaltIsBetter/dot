# Dot

[English](../../README.md) · [Français](../fr/README.md) · [Español](../es/README.md) · **Português**

Um agente de código para macOS e Linux, no espírito do Claude Code, pensado para ser mais rápido,
mais enxuto e mais ponderado: uma janela preta desenhada na GPU, no estilo do
[Spaceship](https://chuanqisun.github.io/spaceship/), onde você escreve para o Claude, que responde
em fluxo e trabalha na pasta do projeto com suas ferramentas (ler, escrever, executar comandos,
buscar, a web).

O Dot fala inglês, francês, espanhol e português do Brasil: ele segue os idiomas do sistema, e o
ajuste “Idioma” troca o idioma na hora.

## Instalar

Baixe a versão mais recente nas [releases](https://github.com/BasaltIsBetter/dot/releases):

- **Mac (Apple silicon)**: descompacte `Dot-<version>-macos-arm64.zip` e mova o `Dot.app` para
  Aplicativos. Estas pré-versões ainda não são assinadas pela Apple: na primeira vez, permita em
  Ajustes do Sistema › Privacidade e Segurança › “Abrir Mesmo Assim”, ou execute
  `xattr -dr com.apple.quarantine /Applications/Dot.app`.
- **Linux (x86_64 ou arm64, glibc 2.35 ou mais recente)**: descompacte o arquivo do seu processador
  e execute o `install.sh` dele. O Dot é instalado em `~/.local/share/dot-app`, o comando `dot` em
  `~/.local/bin`, com uma entrada no lançador de aplicativos.

A partir do código-fonte, `./crates/dot/macos/install.sh` compila o app e o instala em
`~/Applications`, com `dot` em `~/.local/bin`.

## Requisitos

- macOS em Apple silicon ou Intel, com uma GPU Metal; ou Linux com Wayland ou X11, com um driver
  Vulkan ou OpenGL (`mesa-vulkan-drivers` basta, sem placa de vídeo). Com X11, também é preciso o
  `libxkbcommon-x11`. Uma chave de API fica no chaveiro da área de trabalho (Secret Service: GNOME
  Keyring, KWallet…); sem chaveiro, como no WSL, é preciso se conectar pelo Claude Code ou pelo
  `ant`.
- Rust: a versão está fixada em `rust-toolchain.toml` (o rustup a instala sozinho).
- Para se conectar: um plano Claude pelo [Claude Code](https://claude.com/claude-code), uma conta
  Anthropic pelo `ant`, ou uma chave de API da Anthropic.

## Executar

```bash
cargo run --release -- ~/meu-projeto         # Dot aberto em uma pasta
./crates/dot/macos/bundle.sh                 # o app de Mac: target/release/bundle/macos/Dot.app
./crates/dot/linux/bundle.sh                 # o arquivo de Linux: target/release/bundle/linux/*.tar.gz
```

Atalhos: enter envia, shift+enter quebra a linha, esc interrompe a resposta, cmd+V cola texto ou
uma imagem, cmd+N começa uma nova conversa, ↑ e ↓ trazem de volta os prompts já enviados. No Linux,
ctrl substitui cmd, e ctrl+Q encerra.

O log é gravado em `~/Library/Logs/Dot/dot.log` no Mac e em `~/.local/state/dot/dot.log` no Linux
(1 MiB no máximo, o anterior guardado em `dot.log.1`); ele nunca contém uma chave nem um token.

## Agentes em contêineres

Agentes podem viver em contêineres Docker (ou Podman) e ser chamados sob demanda, de um terminal ou
pelo Claude no Dot (`@revisor olhe este arquivo`): cada agente mantém seu contêiner por projeto,
acordado quando necessário, e sua memória em um volume próprio.

```bash
cargo build --release -p dot-container            # o comando: target/release/dot-agents
dot-agents new revisor                            # descreve um agente, para editar depois
dot-agents build                                  # constrói a imagem, com o Claude Code dentro
dot-agents --folder ~/meu-projeto login revisor   # conecta o agente ao Claude, uma única vez
dot-agents --folder ~/meu-projeto call revisor "Revise o último commit"
dot-agents list                                   # os agentes e seus contêineres
dot-agents --folder ~/meu-projeto stop revisor    # põe o agente para dormir
```

Os agentes são descritos em JSON em `~/.config/dot/agents/` (`~/Library/Application
Support/Dot/agents/` no Mac), nunca dentro de um projeto: abrir um repositório desconhecido não
inicia nenhum.

| Campo | Função | Padrão |
|---|---|---|
| `description` | o que o Claude lê para escolher o agente | vazio |
| `workspace` | acesso à pasta do projeto: `none`, `read`, `write` | `read` |
| `network` | `bridge`, ou `none` para um agente sem rede (que, portanto, não pode ser o Claude) | `bridge` |
| `memory`, `cpus`, `processes` | limites do contêiner | `2g`, 2, 512 |
| `minutes` | duração máxima de uma chamada | 30 |
| `model`, `instructions` | modelo e instruções do Claude Code no contêiner | nenhum |
| `command` | programa que responde no lugar do Claude Code (pedido na entrada, resposta na saída) | Claude Code |
| `image` | imagem do contêiner; ela precisa ter `sh`, `tr`, `grep` e um `/home/agent` em que todos possam escrever | `dot-agent:latest` |
| `env` | nomes das variáveis repassadas ao agente | nenhuma |

O contêiner não tem nenhum privilégio (`--cap-drop ALL`, `no-new-privileges`), roda com a
identidade do usuário e só enxerga o projeto, no mesmo caminho que na máquina; a pasta pessoal
inteira e a raiz do disco são recusadas. Lá dentro, o Claude Code não pede permissão: é o contêiner
que limita o agente. Um agente em `write` pode, portanto, mudar qualquer coisa no projeto,
incluindo `.git/hooks`. O Claude pode chamar os agentes no Dot quando `dot-agents` está ao lado de
`dot` e a conexão passa pelo Claude Code; ainda não com uma chave de API.

## Verificar

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
cargo deny check
cargo bench -p dot-desktop --features bench    # benchmarks do criterion
```

A integração contínua (`.github/workflows/ci.yml`) executa as mesmas verificações a cada push e a
cada pull request. Para contribuir, veja [`CONTRIBUTING.md`](CONTRIBUTING.md); para relatar
uma vulnerabilidade, [`SECURITY.md`](SECURITY.md).

## Organização

Um crate por responsabilidade em `crates/`, com os testes à parte em `tests/<crate>/`:

| Crate | Função |
|---|---|
| `dot` | o binário: monta os componentes e mantém o log |
| `dot-api` | contratos e tipos compartilhados (conversa, ferramentas, permissões, sessão, textos) |
| `dot-auth` | a conexão com o Claude |
| `dot-compact` | encurta o que o Claude lê dos comandos |
| `dot-container` | os agentes que vivem em contêineres, e o comando `dot-agents` |
| `dot-desktop` | o app de Mac e Linux: janela, conversa, barra de status (winit + wgpu) |
| `dot-i18n` | as traduções (Fluent, `crates/dot-i18n/locales/`) |
| `dot-model` | as respostas do Claude: API Messages ou Claude Code |
| `dot-process` | os programas que o Dot inicia (ambiente, grupo, parada, espera) e a leitura limitada, linha a linha, do que ele lê deles |
| `dot-session` | a sessão de uma janela: pasta, branch do git, consumo; e o que é guardado entre duas execuções |
| `dot-shell` | substitui o shell dos comandos do Claude Code, para encurtar o que eles mostram |
| `dot-tool` | as ferramentas do Claude na máquina |
| `dot-transcript` | a conversa como é lida, sem janela nem GPU: histórico, quebra de linha, movimento |

## Licença

Licenciado sob [Apache 2.0](../../LICENSE); os créditos estão em [`NOTICE`](../../NOTICE).

- Parte das regras do `dot-compact` foi adaptada do [RTK](https://github.com/rtk-ai/rtk)
  (Apache 2.0): veja [`crates/dot-compact/third-party/rtk/`](../../crates/dot-compact/third-party/rtk/).
- O design da interface se inspira no [Spaceship](https://github.com/chuanqisun/spaceship), de Sun
  (MIT); nenhum código foi aproveitado dele.
- A fonte JetBrains Mono embutida é licenciada sob a SIL Open Font License 1.1.
