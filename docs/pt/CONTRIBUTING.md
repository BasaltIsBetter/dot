# Contribuir com o Dot

[English](../../.github/CONTRIBUTING.md) · [Français](../fr/CONTRIBUTING.md) · [Español](../es/CONTRIBUTING.md) · **Português**

Obrigado pelo interesse. Um bug ou uma ideia: abra uma issue com o formulário dela. Uma
vulnerabilidade de segurança: nunca em público, veja [`SECURITY.md`](SECURITY.md).

## Antes de escrever código

Para qualquer mudança maior que uma correção, abra primeiro uma issue para conversar. Uma pull
request pequena e focada em uma só coisa é revisada e integrada muito mais rápido.

## Compilar e verificar

Os requisitos e a compilação estão no [`README`](README.md). Antes de abrir uma pull request:

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
cargo deny check
```

A integração contínua executa as mesmas verificações no macOS e no Linux.

## O que uma mudança precisa ter

- **Testes** para toda lógica, em `tests/<crate>/`, no mesmo caminho do módulo testado, provando os
  dois lados: a entrada válida passa, a inválida é recusada.
- **Nenhuma ação do Claude sem permissão**, nenhum caminho fora da pasta do projeto, nenhum segredo
  em um log ou em uma mensagem de erro.
- **Medido, não suposto**: uma mudança em um caminho crítico (o fluxo do modelo, o desenho da
  interface, a inicialização) traz seus números antes e depois (`cargo bench`).
- **Nenhum texto exibido escrito no código**: cada texto passa pelo `dot-i18n`, traduzido em cada
  idioma (`crates/dot-i18n/locales/`: inglês, francês, espanhol, português do Brasil). Os testes
  falham se faltar um texto em algum deles.
- **Commits [Conventional Commits](https://www.conventionalcommits.org/)**: `feat(desktop): …`,
  `fix(model): …`, uma descrição curta no imperativo, em inglês.

Ao propor uma contribuição, você concorda que ela seja publicada sob a licença do projeto,
[Apache 2.0](../../LICENSE).
