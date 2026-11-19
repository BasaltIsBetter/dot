# Dot

Agent de code pour macOS, dans l'esprit de Claude Code, pensé pour être plus rapide, plus sobre et
plus réfléchi : une fenêtre noire dessinée au GPU, dans le style de
[Spaceship](https://chuanqisun.github.io/spaceship/), où l'on écrit à Claude, qui répond en flux et
travaille dans le dossier du projet avec ses outils (lire, écrire, lancer des commandes, chercher,
le web).

## Prérequis

- macOS sur Apple Silicon ou Intel, avec un GPU Metal.
- Rust : la version est épinglée dans `rust-toolchain.toml` (rustup l'installe tout seul).
- Pour se connecter : un abonnement Claude via [Claude Code](https://claude.com/claude-code), un
  compte Anthropic via `ant`, ou une clé API Anthropic.

## Lancer

```bash
cargo run --release -- ~/mon-projet          # Dot ouvert sur un dossier
./crates/dot/macos/bundle.sh                  # l'app : target/release/bundle/macos/Dot.app
```

Raccourcis : entrée envoie, maj+entrée va à la ligne, échap arrête la réponse, cmd+V colle du
texte ou une image, cmd+N commence une nouvelle conversation.

Le journal est écrit dans `~/Library/Logs/Dot/dot.log` (1 Mio au plus, le précédent gardé dans
`dot.log.1`) ; il ne contient jamais de clé ni de jeton.

## Vérifier

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo deny check
cargo bench -p dot-desktop --features bench    # bancs criterion
```

L'intégration continue (`.github/workflows/ci.yml`) lance les mêmes vérifications à chaque push.

## Organisation

Un crate par responsabilité dans `crates/`, leurs tests à part dans `tests/<crate>/` :

| Crate | Rôle |
|---|---|
| `dot` | le binaire : assemble les composants, tient le journal |
| `dot-api` | contrats et types partagés (conversation, outils, permissions, session, textes) |
| `dot-desktop` | l'app Mac : fenêtre, conversation, barre d'état (winit + wgpu) |
| `dot-i18n` | les traductions (Fluent, `crates/dot-i18n/locales/`) |
| `dot-auth` | la connexion à Claude |
| `dot-model` | les réponses de Claude : API Messages ou Claude Code |
| `dot-process` | les programmes que Dot lance (environnement, groupe, arrêt, attente) |
| `dot-session` | la session d'une fenêtre : dossier, branche git, consommation |
| `dot-tool` | les outils de Claude sur le Mac |

## Licence

Propriétaire, tous droits réservés : voir [`LICENSE`](LICENSE). La police JetBrains Mono embarquée
est sous licence SIL Open Font License 1.1.
