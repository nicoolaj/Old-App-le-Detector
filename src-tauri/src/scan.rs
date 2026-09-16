// SPDX-License-Identifier: MIT

//! Parcours des sources (Applications, Spotlight, PATH, Homebrew, MacPorts,
//! dossiers supplémentaires) et classification de chaque binaire trouvé.
//!
//! Note IPC : tous les champs des structs ci-dessous sont sérialisés tels
//! quels (snake_case), aucun rename côté JS n'est nécessaire — le front-end
//! utilise les mêmes noms de champs.

use crate::macho;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInfo {
    pub hostname: String,
    pub arch: String,
    pub macos_version: String,
    pub app_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanOptions {
    pub apps: bool,
    pub spotlight: bool,
    pub path: bool,
    pub homebrew: bool,
    pub macports: bool,
    pub extra_dirs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub kind: String, // "app" | "exec"
    pub name: String,
    pub version: Option<String>,
    pub path: String,
    pub real_path: String,
    pub source: String,
    pub archs: Vec<String>,
    pub status: String, // "native" | "intel_only" | "obsolete" | "unknown"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub host: HostInfo,
    pub generated_at: String,
    pub scanned_dirs: Vec<String>,
    pub items: Vec<Item>,
    pub scripts_skipped: usize,
    pub unreadable: usize,
}

pub fn host_info() -> HostInfo {
    HostInfo {
        hostname: run_trimmed("hostname", &[]).unwrap_or_else(|| "inconnu".to_string()),
        // std::env::consts::ARCH vaut "aarch64" sur Apple Silicon ; on affiche
        // le terme Apple ("arm64") utilisé partout ailleurs dans l'app.
        arch: match std::env::consts::ARCH {
            "aarch64" => "arm64".to_string(),
            other => other.to_string(),
        },
        macos_version: run_trimmed("sw_vers", &["-productVersion"])
            .unwrap_or_else(|| "inconnue".to_string()),
        app_version: short_version(env!("CARGO_PKG_VERSION")),
    }
}

/// "0.1.0" -> "0.1" (patch nul superflu à l'affichage) ; "1.2.3" inchangé.
pub(crate) fn short_version(v: &str) -> String {
    v.strip_suffix(".0").unwrap_or(v).to_string()
}

pub fn scan(opts: &ScanOptions) -> Report {
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    let mut scanned_dirs = Vec::new();
    let mut scripts_skipped = 0usize;
    let mut unreadable = 0usize;
    let home = std::env::var("HOME").unwrap_or_default();

    if opts.apps {
        for dir in [
            PathBuf::from("/Applications"),
            PathBuf::from(format!("{home}/Applications")),
        ] {
            if dir.is_dir() {
                scanned_dirs.push(dir.display().to_string());
                scan_dir(
                    &dir,
                    4,
                    "Applications",
                    &mut items,
                    &mut seen,
                    &mut scripts_skipped,
                    &mut unreadable,
                );
            }
        }
    }

    if opts.spotlight {
        scanned_dirs.push("Spotlight (mdfind)".to_string());
        for app_path in spotlight_apps() {
            add_app(&app_path, "Spotlight", &mut items, &mut seen);
        }
    }

    if opts.path {
        scanned_dirs.push("$PATH (shell de connexion)".to_string());
        for dir in login_shell_path() {
            scan_dir(
                &dir,
                1,
                "PATH",
                &mut items,
                &mut seen,
                &mut scripts_skipped,
                &mut unreadable,
            );
        }
    }

    if opts.homebrew {
        for dir in homebrew_dirs() {
            scanned_dirs.push(dir.display().to_string());
            scan_dir(
                &dir,
                1,
                "Homebrew",
                &mut items,
                &mut seen,
                &mut scripts_skipped,
                &mut unreadable,
            );
        }
    }

    if opts.macports {
        for sub in ["bin", "sbin"] {
            let dir = Path::new("/opt/local").join(sub);
            if dir.is_dir() {
                scanned_dirs.push(dir.display().to_string());
                scan_dir(
                    &dir,
                    1,
                    "MacPorts",
                    &mut items,
                    &mut seen,
                    &mut scripts_skipped,
                    &mut unreadable,
                );
            }
        }
    }

    for raw in &opts.extra_dirs {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let expanded = match trimmed.strip_prefix("~/") {
            Some(rest) => format!("{home}/{rest}"),
            None => trimmed.to_string(),
        };
        let dir = PathBuf::from(&expanded);
        if dir.is_dir() {
            scanned_dirs.push(expanded);
            scan_dir(
                &dir,
                4,
                "Dossier supplémentaire",
                &mut items,
                &mut seen,
                &mut scripts_skipped,
                &mut unreadable,
            );
        }
    }

    items.sort_by(|a, b| {
        status_rank(&a.status)
            .cmp(&status_rank(&b.status))
            .then_with(|| a.name.cmp(&b.name))
    });

    Report {
        host: host_info(),
        generated_at: run_trimmed("date", &["+%d/%m/%Y %H:%M:%S"]).unwrap_or_default(),
        scanned_dirs,
        items,
        scripts_skipped,
        unreadable,
    }
}

fn status_rank(status: &str) -> u8 {
    match status {
        "intel_only" => 0,
        "obsolete" => 1,
        "unknown" => 2,
        _ => 3, // native
    }
}

fn run_trimmed(cmd: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(cmd).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// PATH tel que résolu par le shell de connexion de l'utilisateur : une app
/// lancée depuis le Finder n'hérite que de /usr/bin:/bin:/usr/sbin:/sbin,
/// pas du PATH étendu par .zshrc (Homebrew, cargo, mise, etc.).
fn login_shell_path() -> Vec<PathBuf> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let marker = "__PATH__=";
    let line = std::process::Command::new(&shell)
        .args(["-ilc", &format!("printf '{marker}%s\\n' \"$PATH\"")])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            s.lines()
                .rev()
                .find_map(|l| l.strip_prefix(marker).map(str::to_string))
        });

    match line {
        Some(p) => std::env::split_paths(&p).collect(),
        None => std::env::var_os("PATH")
            .map(|p| std::env::split_paths(&p).collect())
            .unwrap_or_default(),
    }
}

fn spotlight_apps() -> Vec<PathBuf> {
    let out = std::process::Command::new("mdfind")
        .arg("kMDItemContentType == 'com.apple.application-bundle'")
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
            .collect(),
        _ => Vec::new(),
    }
}

/// bin/sbin des préfixes Homebrew + Cellar/*/*/{bin,sbin} pour les formules
/// keg-only non liées dans bin/ (ex: openssl@3, curl).
fn homebrew_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for prefix in ["/opt/homebrew", "/usr/local"] {
        let prefix = Path::new(prefix);
        if !prefix.is_dir() {
            continue;
        }
        for sub in ["bin", "sbin"] {
            let d = prefix.join(sub);
            if d.is_dir() {
                dirs.push(d);
            }
        }
        let Ok(formulas) = std::fs::read_dir(prefix.join("Cellar")) else {
            continue;
        };
        for formula in formulas.flatten() {
            let Ok(versions) = std::fs::read_dir(formula.path()) else {
                continue;
            };
            for version in versions.flatten() {
                for sub in ["bin", "sbin"] {
                    let d = version.path().join(sub);
                    if d.is_dir() {
                        dirs.push(d);
                    }
                }
            }
        }
    }
    dirs
}

enum Found {
    App(PathBuf),
    Exec(PathBuf),
}

/// Parcourt `dir` jusqu'à `max_depth` niveaux. Un `.app` rencontré est
/// rapporté sans qu'on descende dedans (les helpers imbriqués sont hors
/// périmètre). Suit les symlinks (courant pour les shims Homebrew/mise).
fn walk(dir: &Path, max_depth: u32, out: &mut Vec<Found>) {
    if max_depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        }; // suit les symlinks
        if meta.is_dir() {
            if path.extension().is_some_and(|e| e == "app") {
                out.push(Found::App(path));
            } else {
                walk(&path, max_depth - 1, out);
            }
        } else if meta.is_file() && is_executable(&meta) {
            out.push(Found::Exec(path));
        }
    }
}

#[cfg(unix)]
fn is_executable(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

fn scan_dir(
    dir: &Path,
    max_depth: u32,
    source: &str,
    items: &mut Vec<Item>,
    seen: &mut HashSet<PathBuf>,
    scripts_skipped: &mut usize,
    unreadable: &mut usize,
) {
    let mut found = Vec::new();
    walk(dir, max_depth, &mut found);
    for f in found {
        match f {
            Found::App(path) => add_app(&path, source, items, seen),
            Found::Exec(path) => add_exec(&path, source, items, seen, scripts_skipped, unreadable),
        }
    }
}

#[derive(Deserialize, Default)]
struct InfoPlist {
    #[serde(rename = "CFBundleExecutable")]
    executable: Option<String>,
    #[serde(rename = "CFBundleShortVersionString")]
    version: Option<String>,
    #[serde(rename = "CFBundleName")]
    name: Option<String>,
    #[serde(rename = "CFBundleDisplayName")]
    display_name: Option<String>,
}

/// (dossier contenant Info.plist, dossier contenant l'exécutable) pour un
/// bundle .app. Gère la structure classique (`Contents/{Info.plist,MacOS/}`)
/// et celle des apps iOS installées sur Apple Silicon (Mac App Store,
/// catégorie iPhone/iPad) : le vrai bundle est enveloppé dans
/// `Wrapper/<Nom>.app` (visé par le symlink `WrappedBundle`) et suit la
/// structure iOS — Info.plist et l'exécutable directement à la racine, pas
/// dans Contents/. Ces apps sont toujours arm64 (jamais de version Intel).
fn bundle_layout(path: &Path) -> (PathBuf, PathBuf) {
    if path.join("Contents/Info.plist").is_file() {
        return (path.join("Contents"), path.join("Contents/MacOS"));
    }
    if let Ok(target) = std::fs::read_link(path.join("WrappedBundle")) {
        let wrapped = path.join(target);
        if wrapped.join("Info.plist").is_file() {
            return (wrapped.clone(), wrapped);
        }
    }
    (path.join("Contents"), path.join("Contents/MacOS")) // repli : comportement classique même si absent
}

fn add_app(path: &Path, source: &str, items: &mut Vec<Item>, seen: &mut HashSet<PathBuf>) {
    let real = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if !seen.insert(real.clone()) {
        return;
    }

    let bundle_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("?")
        .to_string();
    let (info_dir, exe_dir) = bundle_layout(path);
    let info: InfoPlist = plist::from_file(info_dir.join("Info.plist")).unwrap_or_default();
    let exe_name = info.executable.unwrap_or_else(|| bundle_stem.clone());
    let display_name = info.display_name.or(info.name).unwrap_or(bundle_stem);
    let exe_path = exe_dir.join(&exe_name);

    if !exe_path.is_file() {
        // Bundle "vitrine" sans exécutable propre (web-app Safari/Chrome,
        // LSTemplateApplication...) : rien à auditer, on l'ignore plutôt que
        // d'afficher un "indéterminé" qui ne peut de toute façon jamais être
        // Intel-only.
        return;
    }

    let (archs, status) = match macho::read_archs(&exe_path) {
        Ok(Some(a)) => {
            let s = macho::classify(&a).as_str().to_string();
            (a.iter().map(|x| x.label().to_string()).collect(), s)
        }
        _ => (Vec::new(), "unknown".to_string()),
    };

    items.push(Item {
        kind: "app".to_string(),
        name: display_name,
        version: info.version,
        path: path.display().to_string(),
        real_path: real.display().to_string(),
        source: source.to_string(),
        archs,
        status,
    });
}

fn add_exec(
    path: &Path,
    source: &str,
    items: &mut Vec<Item>,
    seen: &mut HashSet<PathBuf>,
    scripts_skipped: &mut usize,
    unreadable: &mut usize,
) {
    let real = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if !seen.insert(real.clone()) {
        return;
    }

    match macho::read_archs(path) {
        Ok(Some(archs)) => {
            let status = macho::classify(&archs).as_str().to_string();
            items.push(Item {
                kind: "exec".to_string(),
                name: path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string(),
                version: None,
                path: path.display().to_string(),
                real_path: real.display().to_string(),
                source: source.to_string(),
                archs: archs.iter().map(|a| a.label().to_string()).collect(),
                status,
            });
        }
        Ok(None) => *scripts_skipped += 1,
        Err(_) => *unreadable += 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn short_version_strips_trailing_zero_patch() {
        assert_eq!(short_version("0.1.0"), "0.1");
        assert_eq!(short_version("1.0.0"), "1.0");
        assert_eq!(short_version("1.2.3"), "1.2.3"); // pas de .0 final -> inchangé
    }

    fn write_thin_x86_64(path: &Path) {
        // MH_MAGIC_64 (little-endian) + cputype x86_64 (little-endian) + padding.
        let mut bytes = vec![0xCFu8, 0xFAu8, 0xEDu8, 0xFEu8];
        bytes.extend_from_slice(&0x0100_0007u32.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 8]);
        std::fs::write(path, bytes).unwrap();
        let mut perms = std::fs::metadata(path).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
        std::fs::set_permissions(path, perms).unwrap();
    }

    #[test]
    fn fake_app_bundle_is_intel_only() {
        let tmp = std::env::temp_dir().join(format!("oad-test-app-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let app = tmp.join("Foo.app");
        std::fs::create_dir_all(app.join("Contents/MacOS")).unwrap();

        let mut plist_file = std::fs::File::create(app.join("Contents/Info.plist")).unwrap();
        write!(
            plist_file,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleExecutable</key><string>Foo</string>
  <key>CFBundleShortVersionString</key><string>1.2.3</string>
  <key>CFBundleDisplayName</key><string>Foo App</string>
</dict></plist>"#
        )
        .unwrap();

        write_thin_x86_64(&app.join("Contents/MacOS/Foo"));

        let mut items = Vec::new();
        let mut seen = HashSet::new();
        add_app(&app, "Test", &mut items, &mut seen);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status, "intel_only");
        assert_eq!(items[0].name, "Foo App");
        assert_eq!(items[0].version.as_deref(), Some("1.2.3"));

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn fake_exec_is_intel_only_and_script_is_skipped() {
        let tmp = std::env::temp_dir().join(format!("oad-test-bin-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let bin = tmp.join("mytool");
        write_thin_x86_64(&bin);

        let script = tmp.join("myscript.sh");
        std::fs::write(&script, "#!/bin/sh\necho hi\n").unwrap();
        let mut perms = std::fs::metadata(&script).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
        std::fs::set_permissions(&script, perms).unwrap();

        let mut items = Vec::new();
        let mut seen = HashSet::new();
        let mut scripts_skipped = 0;
        let mut unreadable = 0;
        scan_dir(
            &tmp,
            1,
            "Test",
            &mut items,
            &mut seen,
            &mut scripts_skipped,
            &mut unreadable,
        );

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status, "intel_only");
        assert_eq!(items[0].kind, "exec");
        assert_eq!(scripts_skipped, 1);

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn dedup_by_canonical_path_skips_second_hit() {
        let tmp = std::env::temp_dir().join(format!("oad-test-dedup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let bin = tmp.join("tool");
        write_thin_x86_64(&bin);

        let mut items = Vec::new();
        let mut seen = HashSet::new();
        let (mut a, mut b) = (0, 0);
        add_exec(&bin, "First", &mut items, &mut seen, &mut a, &mut b);
        add_exec(&bin, "Second", &mut items, &mut seen, &mut a, &mut b);

        assert_eq!(
            items.len(),
            1,
            "le second passage sur le même chemin réel ne doit pas dupliquer"
        );

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn app_bundle_not_descended_into() {
        let tmp = std::env::temp_dir().join(format!("oad-test-nodive-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let app = tmp.join("Nested.app");
        std::fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        write_thin_x86_64(&app.join("Contents/MacOS/Nested"));
        // Un exécutable planqué à l'intérieur du bundle ne doit jamais être
        // rapporté séparément : `walk` doit s'arrêter au `.app`.
        std::fs::create_dir_all(app.join("Contents/Helpers")).unwrap();
        write_thin_x86_64(&app.join("Contents/Helpers/sneaky"));

        let mut found = Vec::new();
        walk(&tmp, 4, &mut found);

        assert_eq!(found.len(), 1);
        assert!(matches!(&found[0], Found::App(p) if p == &app));

        std::fs::remove_dir_all(&tmp).ok();
    }

    fn write_thin_arm64(path: &Path) {
        let mut bytes = vec![0xCFu8, 0xFAu8, 0xEDu8, 0xFEu8];
        bytes.extend_from_slice(&0x0100_000Cu32.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 8]);
        std::fs::write(path, bytes).unwrap();
        let mut perms = std::fs::metadata(path).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
        std::fs::set_permissions(path, perms).unwrap();
    }

    /// Reproduit la structure d'une app iOS installée sur Apple Silicon
    /// (ex: AccuWeather via l'App Store) : `WrappedBundle` pointe vers
    /// `Wrapper/<Nom>.app`, qui a Info.plist et l'exécutable à sa racine
    /// (pas de Contents/). Ces apps sont toujours arm64.
    #[test]
    fn ios_wrapped_app_resolves_to_wrapped_binary() {
        let tmp = std::env::temp_dir().join(format!("oad-test-ioswrap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let app = tmp.join("Foo.app");
        let wrapped = app.join("Wrapper/Foo.app");
        std::fs::create_dir_all(&wrapped).unwrap();
        std::os::unix::fs::symlink("Wrapper/Foo.app", app.join("WrappedBundle")).unwrap();

        let mut plist_file = std::fs::File::create(wrapped.join("Info.plist")).unwrap();
        write!(
            plist_file,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleExecutable</key><string>Foo</string>
</dict></plist>"#
        )
        .unwrap();
        write_thin_arm64(&wrapped.join("Foo"));

        let mut items = Vec::new();
        let mut seen = HashSet::new();
        add_app(&app, "Test", &mut items, &mut seen);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status, "native");
        // Le chemin affiché reste le bundle extérieur, pas Wrapper/Foo.app.
        assert_eq!(items[0].path, app.display().to_string());

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// Reproduit une web-app Safari/Chrome (LSTemplateApplication) : un
    /// Info.plist existe mais il n'y a aucun exécutable dans le bundle.
    /// Ne peut structurellement jamais être Intel-only -> ignoré, pas
    /// compté comme "indéterminé".
    #[test]
    fn template_app_without_executable_is_skipped() {
        let tmp = std::env::temp_dir().join(format!("oad-test-template-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let app = tmp.join("WebApp.app/Contents");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(
            app.join("Info.plist"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>LSTemplateApplication</key><true/>
</dict></plist>"#,
        )
        .unwrap();

        let mut items = Vec::new();
        let mut seen = HashSet::new();
        add_app(&tmp.join("WebApp.app"), "Test", &mut items, &mut seen);

        assert!(
            items.is_empty(),
            "une web-app sans exécutable ne doit produire aucun item"
        );

        std::fs::remove_dir_all(&tmp).ok();
    }
}
