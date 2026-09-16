// SPDX-License-Identifier: MIT

use super::Strings;

pub const STRINGS: Strings = Strings {
    host_label: "Machine",
    arch_label: "Architecture",
    macos_label: "macOS",
    generated_label: "Généré le",
    scanned_dirs_label: "Dossiers scannés :",
    apps_intel_title: "Apps nécessitant Rosetta 2",
    execs_intel_title: "Exécutables nécessitant Rosetta 2",
    obsolete_title: "Obsolètes (architecture trop ancienne, déjà inexécutable)",
    unknown_title: "Indéterminés (lecture de l'exécutable impossible)",
    none_label: "(aucun)",
    source_label: "source",
    archs_label: "archs",
    summary_title: "--- Résumé ---",
    summary_apps_intel: "Apps nécessitant Rosetta 2",
    summary_execs_intel: "Exécutables nécessitant Rosetta 2",
    summary_obsolete: "Obsolètes",
    summary_unknown: "Indéterminés",
    summary_native: "Natifs (arm64)",
    summary_scripts_skipped: "Scripts ignorés",
    summary_unreadable: "Illisibles",
    unknown_format_error: "Format d'export inconnu",
    csv_header: "type;nom;version;chemin;chemin_reel;source;architectures;statut",
};
