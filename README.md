# OldAppDetector

Détecte les applications et exécutables **Intel (x86_64) sans version Apple Silicon (arm64)** sur un Mac, avant la fin du support de Rosetta 2.

Apple retire progressivement Rosetta 2 (la couche de traduction qui permet aux Mac Apple Silicon d'exécuter du code Intel). Une fois ce support disparu, tout logiciel n'ayant pas de version arm64 natif cessera de fonctionner. OldAppDetector fait l'inventaire de ce qui est concerné — **pas seulement les `.app`**, mais aussi tous les exécutables accessibles depuis le `PATH`, Homebrew et MacPorts.

## Fonctionnalités

- **Détection fiable** : lit directement l'en-tête Mach-O de chaque binaire (thin ou universel) pour savoir quelles architectures il contient. Ne dépend ni de `file`, ni de `lipo`, ni des Xcode Command Line Tools.
- **Sources multiples**, activables indépendamment :
  - `/Applications` et `~/Applications`
  - Spotlight (`mdfind`) — trouve les apps installées ailleurs sur le disque
  - `$PATH` résolu depuis le **shell de connexion** (une app lancée depuis le Finder n'hérite pas du PATH étendu par `.zshrc`)
  - Homebrew (`/opt/homebrew` et `/usr/local`, y compris les formules keg-only dans `Cellar/`)
  - MacPorts (`/opt/local`)
  - Dossiers supplémentaires au choix
- **Classement** par statut : nécessite Rosetta 2 (Intel only), obsolète (i386/PowerPC, déjà inexécutable), indéterminé (impossible de lire l'exécutable), natif (arm64).
- **Export** du rapport en TXT (lisible) ou CSV (`;`, compatible Excel FR) via une boîte de dialogue native.
- **Binaire universel** (arm64 + x86_64) : le même `.app` tourne sur les deux familles de Mac.

### Cas particuliers gérés

- **Apps iOS installées sur Apple Silicon** (catégorie « iPhone et iPad » du Mac App Store) : leur bundle est enveloppé différemment (`Wrapper/<Nom>.app`, structure iOS sans `Contents/`). Elles sont toujours arm64 — jamais concernées par Rosetta.
- **Raccourcis web (Safari « Ajouter au Dock », Progressive Web Apps Chrome)** : ces bundles n'ont parfois aucun exécutable propre. Ils sont exclus du rapport plutôt que de remonter comme « indéterminé ».
- Un lanceur shell-script qui exécute un binaire séparé (ex. `mon-outil` → `mon-outil-bin`) reste classé **indéterminé** : l'architecture réelle ne peut pas être déduite automatiquement, mieux vaut le signaler que deviner.

## Utilisation

```bash
cargo tauri dev          # mode développement, rechargement à chaud
```

Dans l'application : cocher les sources à analyser, cliquer **Scanner**, puis **Exporter en TXT/CSV** si besoin.

## Compilation

Le `Makefile` fournit les cibles courantes :

```bash
make help   # liste les cibles disponibles
make secu   # audit sécurité des dépendances Rust (cargo audit)
make app    # construit le .app universel (arm64 + x86_64)
make pkg    # construit le .dmg (inclut le .app)
```

Le `.app` est signé en ad hoc (`signingIdentity: "-"` dans `tauri.conf.json`), ce qui suffit pour un lancement local ou une distribution interne (MDM, partage réseau). Pour une distribution grand public hors Mac App Store, une signature Developer ID et la notarisation seraient nécessaires (non configurées ici).

### Prérequis

- macOS (Apple Silicon ou Intel)
- [Rust](https://rustup.rs) avec les deux cibles Apple : `rustup target add aarch64-apple-darwin x86_64-apple-darwin`
- [`tauri-cli`](https://v2.tauri.app) : `cargo install tauri-cli --locked` (les cibles `make app`/`make pkg` l'installent automatiquement si absent)

Aucune dépendance côté frontend (HTML/CSS/JS vanilla, pas de Node/npm requis pour builder l'app — Node n'est utile que si vous modifiez l'outillage frontend vous-même).

## Structure du projet

```
src/                    Frontend (HTML/CSS/JS vanilla)
src-tauri/src/
  macho.rs              Lecture des architectures depuis l'en-tête Mach-O
  scan.rs                Parcours des sources + classification de chaque binaire
  lib.rs                 Commandes Tauri (host_info, scan, export_report) + export TXT/CSV
Makefile                 Cibles help / secu / app / pkg
```

## Limitations connues

- Les exécutables imbriqués dans un bundle (helpers, plugins, Audio Units, extensions Safari) ne sont pas inspectés.
- Ne détecte pas les processus *actuellement* traduits par Rosetta (voir le Moniteur d'activité pour ça) — c'est un audit statique du disque, pas du runtime.
- Kexts et LaunchAgents/LaunchDaemons hors périmètre.
