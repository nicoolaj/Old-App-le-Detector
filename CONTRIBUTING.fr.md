[English](CONTRIBUTING.md) | **Français**

# Contribuer à Old App(le) Detector

Vous cherchez plutôt le guide utilisateur ? Voir [README.fr.md](README.fr.md).

## Fonctionnement

Old App(le) Detector lit directement l'en-tête **Mach-O** de chaque binaire (thin ou universel) pour savoir quelles architectures il contient. Il ne dépend ni de `file`, ni de `lipo`, ni de la présence des Xcode Command Line Tools — voir [`src-tauri/src/macho.rs`](src-tauri/src/macho.rs).

Il analyse plusieurs sources, chacune activable indépendamment :

- `/Applications` et `~/Applications`
- Spotlight (`mdfind`) — trouve les apps installées ailleurs sur le disque
- `$PATH`, résolu depuis le **shell de connexion** — une app lancée depuis le Finder n'hérite pas du `PATH` étendu par `.zshrc`, le scan lance donc un shell de connexion pour obtenir le vrai `PATH`
- Homebrew (`/opt/homebrew` et `/usr/local`, y compris les formules keg-only dans `Cellar/`)
- MacPorts (`/opt/local`)
- Tout dossier supplémentaire ajouté par l'utilisateur

Cas particuliers gérés explicitement (voir [`src-tauri/src/scan.rs`](src-tauri/src/scan.rs)) :

- **Apps iOS installées sur Apple Silicon** (catégorie « iPhone et iPad » du Mac App Store) : leur bundle est structuré différemment (`Wrapper/<Nom>.app`, pas de `Contents/`). Elles sont toujours arm64, donc jamais signalées.
- **Raccourcis web** (Safari « Ajouter au Dock », Progressive Web Apps Chrome) : ces bundles n'ont parfois aucun exécutable propre — ils sont exclus du rapport plutôt que remontés comme « Indéterminé ».
- Un lanceur shell-script qui exécute un binaire séparé (ex. `mon-outil` → `mon-outil-bin`) reste classé **Indéterminé** : l'architecture réelle ne peut pas être déduite automatiquement, mieux vaut le signaler que deviner.

## Structure du projet

```
src/                      Frontend (HTML/CSS/JS vanilla, pas de bundler)
  index.html
  main.js
  styles.css
  i18n.js                 Détection de la locale + aide à la traduction
  locales/
    en.js, fr.js           Un fichier par langue de l'interface

src-tauri/src/
  macho.rs                 Lecture des architectures depuis l'en-tête Mach-O
  scan.rs                  Parcours des sources + classification de chaque binaire
  lib.rs                   Commandes Tauri (host_info, scan, export_report) + export TXT/CSV
  locale/
    mod.rs                 Struct Strings + dispatch de locale (fallback anglais)
    en.rs, fr.rs            Un fichier par langue d'export

Makefile                   Cibles help / secu / app / pkg / clean / mrproper
.github/workflows/
  release.yml               Build + publie une release GitHub sur un tag `v*`
  security.yml               Lance `make secu` sur push/PR + chaque semaine
```

## Prérequis

- macOS (Apple Silicon ou Intel)
- [Rust](https://rustup.rs) avec les deux cibles Apple : `rustup target add aarch64-apple-darwin x86_64-apple-darwin`
- [`tauri-cli`](https://v2.tauri.app) : `cargo install tauri-cli --locked` (installé automatiquement par `make app`/`make pkg` si absent)

Aucune dépendance côté frontend — HTML/CSS/JS vanilla, pas de Node/npm requis pour builder l'app. Node n'est utile que si vous modifiez l'outillage frontend lui-même (il n'y en a aucun aujourd'hui).

## Boucle de développement

```bash
cargo tauri dev
```

Rechargement à chaud du frontend ; voir [README.fr.md](README.fr.md) pour la prise en main de l'interface.

## Compilation

```bash
make help   # liste les cibles disponibles
make secu   # sécurité + lint (fmt --check, clippy -D warnings, cargo audit)
make app    # construit le .app universel (arm64 + x86_64)
make pkg    # construit le .dmg (inclut le .app)
```

Le `.app` est signé en ad hoc (`signingIdentity: "-"` dans `tauri.conf.json`), ce qui suffit pour un usage local ou une distribution interne (MDM, partage réseau). Une distribution publique hors Mac App Store nécessiterait une signature Developer ID et la notarisation — non configurées ici.

## Tests

```bash
cd src-tauri && cargo test
```

23 tests répartis entre `macho.rs`, `scan.rs` et `lib.rs`. **Non exécutés en CI actuellement** — à lancer localement avant d'ouvrir une PR.

## Lint & sécurité

`make secu` lance `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` et `cargo audit`. C'est exactement ce que la CI exécute : [`security.yml`](.github/workflows/security.yml) se déclenche sur push/PR vers `main` touchant `src-tauri/**` ou le `Makefile`, plus un audit hebdomadaire par cron pour les nouvelles failles connues.

## Ajouter une langue

L'app détecte automatiquement la langue du navigateur et retombe sur l'anglais ; ajouter une langue revient à ajouter un fichier de chaque côté :

**Frontend (interface) :**
1. Copiez `src/locales/en.js` vers `src/locales/xx.js` et traduisez chaque valeur (gardez les clés).
2. Déclarez-le dans la map `LOCALES` de [`src/i18n.js`](src/i18n.js) — `detectLocale()` le détecte automatiquement via `navigator.languages`.

**Backend (export TXT/CSV) :**
1. Copiez `src-tauri/src/locale/en.rs` vers `xx.rs` et remplissez chaque champ de `Strings`.
2. Déclarez `mod xx;` et ajoutez une branche dans `for_locale()` dans [`src-tauri/src/locale/mod.rs`](src-tauri/src/locale/mod.rs).

`write_aligned_rows` dans `lib.rs` aligne dynamiquement les colonnes de l'export TXT : pas besoin de compter les espaces à la main pour une traduction plus longue ou plus courte. Le frontend détecte sa propre locale et la transmet à la commande `export_report` via le paramètre `locale` — gardez les deux jeux de locales synchronisés pour un même code de langue.

## Processus de release

Réservé au mainteneur : pousser un tag `v*`. [`release.yml`](.github/workflows/release.yml) construit le `.dmg` universel (`make pkg`) et publie la release GitHub avec des notes générées automatiquement. Les contributeurs n'ont pas à le faire eux-mêmes.

## Contribuer

- Des PR petites et ciblées.
- Respectez le style de code existant.
- Lancez `make secu` et `cargo test` localement avant d'ouvrir une PR (seul `secu` tourne en CI).
- Tout nouveau fichier source porte un en-tête `// SPDX-License-Identifier: MIT`, comme le reste du code.
- Bugs et propositions : [GitHub Issues](https://github.com/nicoolaj/Old-App-le-Detector/issues).

## Licence

MIT — voir [LICENSE](LICENSE). En contribuant, vous acceptez que vos changements soient sous cette licence.
