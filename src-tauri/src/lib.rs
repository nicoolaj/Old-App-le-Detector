mod macho;
mod scan;

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
async fn export_report(app: tauri::AppHandle, format: String, report: Report) -> Result<Option<String>, String> {
    let (ext, content) = match format.as_str() {
        "csv" => ("csv", to_csv(&report)),
        "txt" => ("txt", to_txt(&report)),
        other => return Err(format!("Format d'export inconnu : {other}")),
    };
    let file_name = format!("audit-rosetta-{}.{}", report.host.hostname, ext);

    let picked = app.dialog().file().set_file_name(&file_name).blocking_save_file();
    let Some(file_path) = picked else {
        return Ok(None); // annulé par l'utilisateur
    };
    let path = file_path.into_path().map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(Some(path.display().to_string()))
}

fn to_txt(report: &Report) -> String {
    let mut s = String::new();
    s.push_str("=== Audit Rosetta 2 / OldAppDetector ===\n\n");
    s.push_str(&format!("Machine        : {}\n", report.host.hostname));
    s.push_str(&format!("Architecture   : {}\n", report.host.arch));
    s.push_str(&format!("macOS          : {}\n", report.host.macos_version));
    s.push_str(&format!("Généré le      : {}\n", report.generated_at));
    s.push_str("Dossiers scannés :\n");
    for d in &report.scanned_dirs {
        s.push_str(&format!("  - {d}\n"));
    }
    s.push('\n');

    let apps_intel: Vec<&Item> = report.items.iter().filter(|i| i.kind == "app" && i.status == "intel_only").collect();
    let execs_intel: Vec<&Item> = report.items.iter().filter(|i| i.kind == "exec" && i.status == "intel_only").collect();
    let obsolete: Vec<&Item> = report.items.iter().filter(|i| i.status == "obsolete").collect();
    let unknown: Vec<&Item> = report.items.iter().filter(|i| i.status == "unknown").collect();
    let native_count = report.items.iter().filter(|i| i.status == "native").count();

    write_txt_section(&mut s, "Apps nécessitant Rosetta 2", &apps_intel);
    write_txt_section(&mut s, "Exécutables nécessitant Rosetta 2", &execs_intel);
    write_txt_section(&mut s, "Obsolètes (architecture trop ancienne, déjà inexécutable)", &obsolete);
    write_txt_section(&mut s, "Indéterminés (lecture de l'exécutable impossible)", &unknown);

    s.push_str("--- Résumé ---\n");
    s.push_str(&format!("Apps nécessitant Rosetta 2      : {}\n", apps_intel.len()));
    s.push_str(&format!("Exécutables nécessitant Rosetta 2 : {}\n", execs_intel.len()));
    s.push_str(&format!("Obsolètes                       : {}\n", obsolete.len()));
    s.push_str(&format!("Indéterminés                    : {}\n", unknown.len()));
    s.push_str(&format!("Natifs (arm64)                   : {}\n", native_count));
    s.push_str(&format!("Scripts ignorés                  : {}\n", report.scripts_skipped));
    s.push_str(&format!("Illisibles                       : {}\n", report.unreadable));
    s
}

fn write_txt_section(s: &mut String, title: &str, items: &[&Item]) {
    s.push_str(&format!("--- {title} ({}) ---\n", items.len()));
    if items.is_empty() {
        s.push_str("  (aucun)\n");
    }
    for item in items {
        let where_ = if item.path == item.real_path {
            item.path.clone()
        } else {
            format!("{} → {}", item.path, item.real_path)
        };
        let version = item.version.as_deref().unwrap_or("—");
        let archs = if item.archs.is_empty() { "—".to_string() } else { item.archs.join(", ") };
        s.push_str(&format!(
            "  [{}] {} ({version}) — {where_} — source: {} — archs: {archs}\n",
            item.kind, item.name, item.source
        ));
    }
    s.push('\n');
}

fn csv_field(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

fn to_csv(report: &Report) -> String {
    let mut s = String::from("\u{FEFF}"); // BOM UTF-8, sinon Excel FR décode mal les accents
    s.push_str("type;nom;version;chemin;chemin_reel;source;architectures;statut\r\n");
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
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
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
            host: HostInfo { hostname: "mac-test".into(), arch: "arm64".into(), macos_version: "26.7".into() },
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
        let csv = to_csv(&sample_report());
        assert!(csv.starts_with('\u{FEFF}'));
        assert!(csv.contains("type;nom;version;chemin;chemin_reel;source;architectures;statut\r\n"));
        // Le guillemet dans le nom doit être doublé, pas échappé en backslash.
        assert!(csv.contains("\"Vieille App \"\"pro\"\"\""));
        assert!(csv.contains("intel_only"));
    }

    #[test]
    fn csv_has_one_data_row_per_item() {
        let csv = to_csv(&sample_report());
        assert_eq!(csv.lines().count(), 1 + 2); // en-tête + 2 items
    }

    #[test]
    fn txt_shows_symlink_arrow_for_real_path_mismatch() {
        let txt = to_txt(&sample_report());
        assert!(txt.contains("/usr/local/bin/outil → /usr/local/Cellar/outil/1.0/bin/outil"));
    }

    #[test]
    fn txt_summary_counts_match_items() {
        let txt = to_txt(&sample_report());
        assert!(txt.contains("Apps nécessitant Rosetta 2 (1)"));
        assert!(txt.contains("Scripts ignorés                  : 3"));
        assert!(txt.contains("Illisibles                       : 1"));
    }
}
