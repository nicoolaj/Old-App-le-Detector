// SPDX-License-Identifier: MIT

mod en;
mod fr;

pub struct Strings {
    pub host_label: &'static str,
    pub arch_label: &'static str,
    pub macos_label: &'static str,
    pub generated_label: &'static str,
    pub scanned_dirs_label: &'static str,
    pub apps_intel_title: &'static str,
    pub execs_intel_title: &'static str,
    pub obsolete_title: &'static str,
    pub unknown_title: &'static str,
    pub none_label: &'static str,
    pub source_label: &'static str,
    pub archs_label: &'static str,
    pub summary_title: &'static str,
    pub summary_apps_intel: &'static str,
    pub summary_execs_intel: &'static str,
    pub summary_obsolete: &'static str,
    pub summary_unknown: &'static str,
    pub summary_native: &'static str,
    pub summary_scripts_skipped: &'static str,
    pub summary_unreadable: &'static str,
    pub unknown_format_error: &'static str,
    pub csv_header: &'static str,
}

/// Retient uniquement le sous-tag principal (ex: "fr" pour "fr-FR") et
/// retombe sur l'anglais pour toute locale non supportée.
pub fn for_locale(locale: &str) -> &'static Strings {
    match locale.split('-').next().unwrap_or("") {
        "fr" => &fr::STRINGS,
        _ => &en::STRINGS,
    }
}
