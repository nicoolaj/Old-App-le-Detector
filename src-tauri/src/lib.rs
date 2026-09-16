// SPDX-License-Identifier: MIT

mod locale;
mod macho;
mod scan;

use locale::Strings;
use scan::{Item, Report};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
fn host_info() -> scan::HostInfo {
    scan::host_info()
}

#[tauri::command]
async fn scan(opts: scan::ScanOptions) -> Report {
    // Le parcours du disque est bloquant (I/O + process spawn) : on le sort
    // du pool async pour ne jamais geler l'UI pendant un scan.
    tauri::async_runtime::spawn_blocking(move || scan::scan(&opts))
        .await
        .expect("le thread de scan a paniqué")
}

#[tauri::command]
async fn export_report(
    app: tauri::AppHandle,
    format: String,
    report: Report,
    locale: String,
) -> Result<Option<String>, String> {
    let strings = locale::for_locale(&locale);
    let (ext, content) = match format.as_str() {
        "csv" => ("csv", to_csv(&report, strings)),
        "txt" => ("txt", to_txt(&report, strings)),
        other => return Err(format!("{}: {other}", strings.unknown_format_error)),
    };
    let file_name = format!("audit-rosetta-{}.{}", report.host.hostname, ext);

    let picked = app
        .dialog()
        .file()
        .set_file_name(&file_name)
        .blocking_save_file();
    let Some(file_path) = picked else {
        return Ok(None); // annulé par l'utilisateur
    };
    let path = file_path.into_path().map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(Some(path.display().to_string()))
}

/// Aligne une liste de paires (libellé, valeur) sur la largeur du plus long
/// libellé : nécessaire car "Généré le" et "Generated on" n'ont pas la même
/// longueur, un padding en dur ne peut pas marcher pour les deux langues.
fn write_aligned_rows(s: &mut String, rows: &[(&str, String)]) {
    let width = rows
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);
    for (label, value) in rows {
        s.push_str(&format!("{label:<width$} : {value}\n"));
    }
}

fn to_txt(report: &Report, strings: &Strings) -> String {
    let mut s = String::new();
    s.push_str("=== Audit Rosetta 2 / OldAppDetector ===\n\n");
    write_aligned_rows(
        &mut s,
        &[
            (strings.host_label, report.host.hostname.clone()),
            (strings.arch_label, report.host.arch.clone()),
            (strings.macos_label, report.host.macos_version.clone()),
            (strings.generated_label, report.generated_at.clone()),
        ],
    );
    s.push_str(&format!("{}\n", strings.scanned_dirs_label));
    for d in &report.scanned_dirs {
        s.push_str(&format!("  - {d}\n"));
    }
    s.push('\n');

    let apps_intel: Vec<&Item> = report
        .items
        .iter()
        .filter(|i| i.kind == "app" && i.status == "intel_only")
        .collect();
    let execs_intel: Vec<&Item> = report
        .items
        .iter()
        .filter(|i| i.kind == "exec" && i.status == "intel_only")
        .collect();
    let obsolete: Vec<&Item> = report
        .items
        .iter()
        .filter(|i| i.status == "obsolete")
        .collect();
    let unknown: Vec<&Item> = report
        .items
        .iter()
        .filter(|i| i.status == "unknown")
        .collect();
    let native_count = report.items.iter().filter(|i| i.status == "native").count();

    write_txt_section(&mut s, strings.apps_intel_title, &apps_intel, strings);
    write_txt_section(&mut s, strings.execs_intel_title, &execs_intel, strings);
    write_txt_section(&mut s, strings.obsolete_title, &obsolete, strings);
    write_txt_section(&mut s, strings.unknown_title, &unknown, strings);

    s.push_str(&format!("{}\n", strings.summary_title));
    write_aligned_rows(
        &mut s,
        &[
            (strings.summary_apps_intel, apps_intel.len().to_string()),
            (strings.summary_execs_intel, execs_intel.len().to_string()),
            (strings.summary_obsolete, obsolete.len().to_string()),
            (strings.summary_unknown, unknown.len().to_string()),
            (strings.summary_native, native_count.to_string()),
            (
                strings.summary_scripts_skipped,
                report.scripts_skipped.to_string(),
            ),
            (strings.summary_unreadable, report.unreadable.to_string()),
        ],
    );
    s
}

fn write_txt_section(s: &mut String, title: &str, items: &[&Item], strings: &Strings) {
    s.push_str(&format!("--- {title} ({}) ---\n", items.len()));
    if items.is_empty() {
        s.push_str(&format!("  {}\n", strings.none_label));
    }
    for item in items {
        let where_ = if item.path == item.real_path {
            item.path.clone()
        } else {
            format!("{} → {}", item.path, item.real_path)
        };
        let version = item.version.as_deref().unwrap_or("—");
        let archs = if item.archs.is_empty() {
            "—".to_string()
        } else {
            item.archs.join(", ")
        };
        s.push_str(&format!(
            "  [{kind}] {name} ({version}) — {where_} — {source_lbl}: {source} — {archs_lbl}: {archs}\n",
            kind = item.kind,
            name = item.name,
            source_lbl = strings.source_label,
            source = item.source,
            archs_lbl = strings.archs_label,
        ));
    }
    s.push('\n');
}

fn csv_field(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

fn to_csv(report: &Report, strings: &Strings) -> String {
    let mut s = String::from("\u{FEFF}"); // BOM UTF-8, sinon Excel FR décode mal les accents
    s.push_str(strings.csv_header);
    s.push_str("\r\n");
    for item in &report.items {
        let version = item.version.as_deref().unwrap_or("");
        let archs = item.archs.join(", ");
        let fields = [
            csv_field(&item.kind),
            csv_field(&item.name),
            csv_field(version),
            csv_field(&item.path),
            csv_field(&item.real_path),
            csv_field(&item.source),
            csv_field(&archs),
            csv_field(&item.status),
        ];
        s.push_str(&fields.join(";"));
        s.push_str("\r\n");
    }
    s
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Menu macOS par défaut de Tauri (App/Edit/View/Window), avec l'item
/// "About" personnalisé : nom, version courte, icône et libellé en français
/// ("À propos de Old App Detector" plutôt que le "About <nom du crate>"
/// généré par défaut).
fn build_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{AboutMetadataBuilder, Menu, PredefinedMenuItem, Submenu};

    const APP_NAME: &str = "Old App(le) Detector";

    let about_metadata = AboutMetadataBuilder::new()
        .name(Some(APP_NAME))
        .version(Some(scan::short_version(env!("CARGO_PKG_VERSION"))))
        .copyright(Some("© 2026 Nicolas Jalibert"))
        .icon(app.default_window_icon().cloned())
        .build();

    let app_menu = Submenu::with_items(
        app,
        APP_NAME,
        true,
        &[
            &PredefinedMenuItem::about(
                app,
                Some(&format!("À propos de {APP_NAME}")),
                Some(about_metadata),
            )?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;

    let view_menu = Submenu::with_items(
        app,
        "View",
        true,
        &[&PredefinedMenuItem::fullscreen(app, None)?],
    )?;

    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;

    Menu::with_items(app, &[&app_menu, &edit_menu, &view_menu, &window_menu])
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let menu = build_menu(app.handle())?;
            app.set_menu(menu)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![host_info, scan, export_report])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use scan::HostInfo;

    fn sample_report() -> Report {
        Report {
            host: HostInfo {
                hostname: "mac-test".into(),
                arch: "arm64".into(),
                macos_version: "26.7".into(),
                app_version: "0.1".into(),
            },
            generated_at: "16/09/2026 10:00:00".into(),
            scanned_dirs: vec!["/Applications".into()],
            items: vec![
                Item {
                    kind: "app".into(),
                    name: "Vieille App \"pro\"".into(),
                    version: Some("1.0".into()),
                    path: "/Applications/Vieille App.app".into(),
                    real_path: "/Applications/Vieille App.app".into(),
                    source: "Applications".into(),
                    archs: vec!["x86_64".into()],
                    status: "intel_only".into(),
                },
                Item {
                    kind: "exec".into(),
                    name: "outil".into(),
                    version: None,
                    path: "/usr/local/bin/outil".into(),
                    real_path: "/usr/local/Cellar/outil/1.0/bin/outil".into(),
                    source: "Homebrew".into(),
                    archs: vec!["x86_64".into()],
                    status: "intel_only".into(),
                },
            ],
            scripts_skipped: 3,
            unreadable: 1,
        }
    }

    #[test]
    fn csv_escapes_quotes_and_has_bom_and_header() {
        let csv = to_csv(&sample_report(), locale::for_locale("fr"));
        assert!(csv.starts_with('\u{FEFF}'));
        assert!(csv.contains("type;nom;version;chemin;chemin_reel;source;architectures;statut\r\n"));
        // Le guillemet dans le nom doit être doublé, pas échappé en backslash.
        assert!(csv.contains("\"Vieille App \"\"pro\"\"\""));
        assert!(csv.contains("intel_only"));
    }

    #[test]
    fn csv_has_one_data_row_per_item() {
        let csv = to_csv(&sample_report(), locale::for_locale("fr"));
        assert_eq!(csv.lines().count(), 1 + 2); // en-tête + 2 items
    }

    #[test]
    fn csv_header_follows_locale() {
        let csv = to_csv(&sample_report(), locale::for_locale("en"));
        assert!(csv.contains("type;name;version;path;real_path;source;architectures;status\r\n"));
    }

    #[test]
    fn txt_shows_symlink_arrow_for_real_path_mismatch() {
        let txt = to_txt(&sample_report(), locale::for_locale("fr"));
        assert!(txt.contains("/usr/local/bin/outil → /usr/local/Cellar/outil/1.0/bin/outil"));
    }

    #[test]
    fn txt_summary_counts_match_items() {
        let txt = to_txt(&sample_report(), locale::for_locale("fr"));
        assert!(txt.contains("Apps nécessitant Rosetta 2 (1)"));
        assert!(txt.contains("Scripts ignorés"));
        assert!(txt.contains("Illisibles"));
        // Le résumé aligne les libellés sur la largeur du plus long : on
        // vérifie le contenu de la ligne plutôt qu'un padding en dur.
        let scripts_line = txt
            .lines()
            .find(|l| l.starts_with("Scripts ignorés"))
            .unwrap();
        assert!(scripts_line.trim_end().ends_with(": 3"));
        let unreadable_line = txt.lines().find(|l| l.starts_with("Illisibles")).unwrap();
        assert!(unreadable_line.trim_end().ends_with(": 1"));
    }

    #[test]
    fn txt_falls_back_to_english_for_unsupported_locale() {
        let txt = to_txt(&sample_report(), locale::for_locale("de"));
        assert!(txt.contains("Apps requiring Rosetta 2 (1)"));
        assert!(txt.contains("Host"));
    }
}
