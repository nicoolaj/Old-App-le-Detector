const { invoke } = window.__TAURI__.core;

let lastReport = null;

const el = (id) => document.getElementById(id);

const STATUS_LABELS = {
  intel_only: "Nécessite Rosetta 2",
  obsolete: "Obsolète",
  unknown: "Indéterminé",
  native: "Natif (arm64)",
};

function statusLabel(status) {
  return STATUS_LABELS[status] ?? status;
}

function escapeHtml(value) {
  const div = document.createElement("div");
  div.textContent = value ?? "";
  return div.innerHTML;
}

async function loadHostInfo() {
  try {
    const host = await invoke("host_info");
    el("host-name").textContent = host.hostname;
    el("host-macos").textContent = `macOS ${host.macos_version}`;
    const isAppleSilicon = host.arch === "arm64";
    const badge = el("host-arch-badge");
    badge.textContent = isAppleSilicon ? "Apple Silicon" : "Intel";
    badge.classList.add(isAppleSilicon ? "badge-arm" : "badge-intel");
  } catch (err) {
    el("host-name").textContent = "Machine inconnue";
    console.error(err);
  }
}

function currentOptions() {
  return {
    apps: el("src-apps").checked,
    spotlight: el("src-spotlight").checked,
    path: el("src-path").checked,
    homebrew: el("src-homebrew").checked,
    macports: el("src-macports").checked,
    extra_dirs: el("extra-dirs").value.split("\n"),
  };
}

function renderTable(report) {
  const showNative = el("show-native").checked;
  const items = showNative ? report.items : report.items.filter((item) => item.status !== "native");

  const tbody = el("results-body");
  tbody.innerHTML = items
    .map(
      (item) => `
      <tr>
        <td>${item.kind === "app" ? "App" : "Exécutable"}</td>
        <td class="cell-name" title="${escapeHtml(item.name)}">${escapeHtml(item.name)}</td>
        <td>${escapeHtml(item.version ?? "—")}</td>
        <td class="cell-path" title="${escapeHtml(item.real_path)}">${escapeHtml(item.path)}</td>
        <td>${escapeHtml(item.source)}</td>
        <td class="cell-archs">${escapeHtml(item.archs.join(", ")) || "—"}</td>
        <td><span class="status-pill status-pill-${item.status}">${statusLabel(item.status)}</span></td>
      </tr>`,
    )
    .join("");

  el("table-panel").hidden = items.length === 0;
}

function renderSummary(report) {
  const apps = report.items.filter((item) => item.kind === "app").length;
  const execs = report.items.filter((item) => item.kind === "exec").length;
  const intelOnly = report.items.filter((item) => item.status === "intel_only").length;
  const obsolete = report.items.filter((item) => item.status === "obsolete").length;
  const unknown = report.items.filter((item) => item.status === "unknown").length;

  el("summary-text").textContent =
    `${apps} app(s), ${execs} exécutable(s) analysés — ` +
    `${intelOnly} nécessitent Rosetta 2, ${obsolete} obsolète(s), ${unknown} indéterminé(s). ` +
    `${report.scripts_skipped} script(s) ignoré(s), ${report.unreadable} fichier(s) illisible(s).`;

  el("summary-panel").hidden = false;
  el("export-txt").disabled = false;
  el("export-csv").disabled = false;
}

async function runScan() {
  const btn = el("scan-btn");
  btn.disabled = true;
  btn.textContent = "Scan en cours…";
  el("scan-status").textContent = "";
  el("export-msg").textContent = "";

  try {
    const report = await invoke("scan", { opts: currentOptions() });
    lastReport = report;
    renderSummary(report);
    renderTable(report);
  } catch (err) {
    el("scan-status").textContent = `Erreur pendant le scan : ${err}`;
    console.error(err);
  } finally {
    btn.disabled = false;
    btn.textContent = "Scanner";
  }
}

async function exportReport(format) {
  if (!lastReport) return;
  const msg = el("export-msg");
  msg.textContent = "";
  try {
    const path = await invoke("export_report", { format, report: lastReport });
    msg.textContent = path ? `Fichier enregistré : ${path}` : "Export annulé.";
  } catch (err) {
    msg.textContent = `Erreur pendant l'export : ${err}`;
    console.error(err);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  loadHostInfo();
  el("scan-btn").addEventListener("click", runScan);
  el("show-native").addEventListener("change", () => {
    if (lastReport) renderTable(lastReport);
  });
  el("export-txt").addEventListener("click", () => exportReport("txt"));
  el("export-csv").addEventListener("click", () => exportReport("csv"));
});
