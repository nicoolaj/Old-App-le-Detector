[English](README.md) | **Français**

# Old App(le) Detector

[![Dernière release](https://img.shields.io/github/v/release/nicoolaj/Old-App-le-Detector?label=derni%C3%A8re%20release)](https://github.com/nicoolaj/Old-App-le-Detector/releases/latest)
[![Licence : MIT](https://img.shields.io/badge/licence-MIT-blue)](LICENSE)

Découvrez quelles apps de votre Mac cesseront de fonctionner à la fin du support de Rosetta 2.

Rosetta 2 est la couche de compatibilité qui permet aux Mac Apple Silicon (M1, M2, M3...) de faire tourner d'anciens logiciels Intel. Apple la retire progressivement — une fois disparue, toute app ou outil en ligne de commande sans version native Apple Silicon cessera tout simplement de se lancer. Old App(le) Detector scanne votre Mac et vous dit précisément ce qui est concerné — **pas seulement les apps du dossier Applications**, mais aussi les outils en ligne de commande installés via Homebrew, MacPorts, ou accessibles depuis votre `PATH`.

## Téléchargement

👉 [**Dernière version (.dmg)**](https://github.com/nicoolaj/Old-App-le-Detector/releases/latest)

1. Téléchargez le `.dmg`
2. Ouvrez-le
3. Glissez l'app dans votre dossier `Applications`

Toutes les versions : [page des releases](https://github.com/nicoolaj/Old-App-le-Detector/releases).

## Utilisation

1. Ouvrez l'app et cochez les sources à analyser — Applications, Homebrew, MacPorts, votre `PATH`, ou un dossier personnalisé.
2. Cliquez sur **Scanner**.
3. Consultez les résultats.
4. Cliquez éventuellement sur **Exporter en TXT** ou **Exporter en CSV** si vous souhaitez en garder une copie.

## Comprendre les résultats

| Statut | Signification |
|---|---|
| **Nécessite Rosetta 2** | Intel uniquement aujourd'hui — cessera de fonctionner à la fin du support de Rosetta 2 |
| **Obsolète** | Architecture encore plus ancienne (32 bits/PowerPC) — déjà inexécutable sur ce Mac |
| **Indéterminé** | Impossible à lire — mérite une vérification manuelle |
| **Natif (arm64)** | Déjà Apple Silicon — rien à faire |

## Bon à savoir

- Les exécutables auxiliaires nichés dans une app (plugins, extensions) ne sont pas inspectés — seule l'app ou l'outil lui-même l'est.
- C'est une photographie de ce qui est installé sur le disque, pas de ce qui tourne actuellement — l'app ne dira pas quelles applications sont *en cours d'exécution* sous Rosetta en ce moment (c'est le rôle du Moniteur d'activité).

## Pour les développeurs

Envie de compiler le projet, de comprendre son fonctionnement ou d'y contribuer ? Voir [CONTRIBUTING.md](CONTRIBUTING.md) (English) ou [CONTRIBUTING.fr.md](CONTRIBUTING.fr.md) (Français).

## Licence

MIT — voir [LICENSE](LICENSE).
