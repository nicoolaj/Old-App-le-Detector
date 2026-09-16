// SPDX-License-Identifier: MIT

use super::Strings;

pub const STRINGS: Strings = Strings {
    host_label: "Host",
    arch_label: "Architecture",
    macos_label: "macOS",
    generated_label: "Generated on",
    scanned_dirs_label: "Scanned folders:",
    apps_intel_title: "Apps requiring Rosetta 2",
    execs_intel_title: "Executables requiring Rosetta 2",
    obsolete_title: "Obsolete (architecture too old, already unrunnable)",
    unknown_title: "Unknown (unable to read the executable)",
    none_label: "(none)",
    source_label: "source",
    archs_label: "archs",
    summary_title: "--- Summary ---",
    summary_apps_intel: "Apps requiring Rosetta 2",
    summary_execs_intel: "Executables requiring Rosetta 2",
    summary_obsolete: "Obsolete",
    summary_unknown: "Unknown",
    summary_native: "Native (arm64)",
    summary_scripts_skipped: "Scripts skipped",
    summary_unreadable: "Unreadable",
    unknown_format_error: "Unknown export format",
    csv_header: "type;name;version;path;real_path;source;architectures;status",
};
