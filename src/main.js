import "./styles.css";
import clamavLogo from "./assets/clamav-official-logo.png";

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification
} from "@tauri-apps/plugin-notification";

const state = {
  mode: "quick",
  selectedTargets: [],
  logs: [],
  quarantine: [],
  dashboard: null,
  realtime: null,
  system: null,
  config: null,
  currentScanId: null
};

const viewMeta = {
  dashboard: {
    tag: "Dashboard",
    title: "Vue d'ensemble de la protection",
    description:
      "Surveillez le moteur ClamAV, l'activite recente, l'etat du poste et la configuration de la protection temps reel."
  },
  scan: {
    tag: "Scan",
    title: "Lancer et suivre une analyse",
    description:
      "Choisissez un mode de scan, suivez la progression en direct et consultez immediatement les menaces detectees."
  },
  quarantine: {
    tag: "Quarantaine",
    title: "Gerer les fichiers isoles",
    description:
      "Restaurez ou supprimez les elements suspects tout en conservant une trace claire de leur detection."
  },
  logs: {
    tag: "Logs",
    title: "Historique des analyses",
    description:
      "Retrouvez le detail des scans executes, les chemins inspectes, les erreurs et les signatures detectees."
  }
};

const elements = {
  navItems: [...document.querySelectorAll(".nav__item")],
  views: [...document.querySelectorAll(".view")],
  modeCards: [...document.querySelectorAll(".mode-card")],
  brandLogo: document.querySelector("#brand-logo"),
  dashboardCards: document.querySelector("#dashboard-cards"),
  lastScanCard: document.querySelector("#last-scan-card"),
  realtimeCard: document.querySelector("#realtime-card"),
  configWatchPaths: document.querySelector("#config-watch-paths"),
  addWatchFolderButton: document.querySelector("#add-watch-folder-button"),
  downloadsToggle: document.querySelector("#downloads-toggle"),
  systemAccessCard: document.querySelector("#system-access-card"),
  quarantineList: document.querySelector("#quarantine-list"),
  logsList: document.querySelector("#logs-list"),
  selectedTargets: document.querySelector("#selected-targets"),
  scanResults: document.querySelector("#scan-results"),
  quickScanButton: document.querySelector("#quick-scan-button"),
  startScanButton: document.querySelector("#start-scan-button"),
  pickFilesButton: document.querySelector("#pick-files-button"),
  pickFolderButton: document.querySelector("#pick-folder-button"),
  updateButton: document.querySelector("#update-button"),
  toggleRealtimeButton: document.querySelector("#toggle-realtime-button"),
  clamdCheckbox: document.querySelector("#clamd-checkbox"),
  quarantineCheckbox: document.querySelector("#quarantine-checkbox"),
  scanStatusLabel: document.querySelector("#scan-status-label"),
  scanProgressLabel: document.querySelector("#scan-progress-label"),
  scanProgressBar: document.querySelector("#scan-progress-bar"),
  scanCurrentTarget: document.querySelector("#scan-current-target"),
  engineStatusLabel: document.querySelector("#engine-status-label"),
  engineDot: document.querySelector("#engine-dot"),
  currentViewTag: document.querySelector("#current-view-tag"),
  currentViewTitle: document.querySelector("#current-view-title"),
  currentViewDescription: document.querySelector("#current-view-description"),
  topbarEngineLabel: document.querySelector("#topbar-engine-label"),
  platformLabel: document.querySelector("#platform-label"),
  permissionLabel: document.querySelector("#permission-label"),
  watcherLabel: document.querySelector("#watcher-label"),
  overviewEngineSource: document.querySelector("#overview-engine-source"),
  overviewPermissionLevel: document.querySelector("#overview-permission-level"),
  overviewPlatform: document.querySelector("#overview-platform"),
  overviewRealtimeState: document.querySelector("#overview-realtime-state")
};

function formatDate(value) {
  if (!value) {
    return "Jamais";
  }

  return new Intl.DateTimeFormat("fr-FR", {
    dateStyle: "medium",
    timeStyle: "short"
  }).format(new Date(value));
}

function formatMode(mode) {
  const labels = {
    Quick: "Rapide",
    Full: "Complet",
    Custom: "Personnalise",
    Realtime: "Temps reel"
  };

  return labels[mode] ?? mode;
}

function setActiveView(viewName) {
  elements.navItems.forEach((item) => {
    item.classList.toggle("nav__item--active", item.dataset.view === viewName);
  });

  elements.views.forEach((view) => {
    view.classList.toggle("view--active", view.id === `view-${viewName}`);
  });

  const meta = viewMeta[viewName];
  if (meta) {
    elements.currentViewTag.textContent = meta.tag;
    elements.currentViewTitle.textContent = meta.title;
    elements.currentViewDescription.textContent = meta.description;
  }
}

function renderDashboard() {
  if (!state.dashboard) {
    return;
  }

  const cards = [
    {
      label: "Menaces détectées",
      value: state.dashboard.totalThreats,
      tone: "danger"
    },
    {
      label: "Scans exécutés",
      value: state.dashboard.totalScans,
      tone: "default"
    },
    {
      label: "Fichiers en quarantaine",
      value: state.dashboard.quarantinedItems,
      tone: "warning"
    },
    {
      label: "Dernier scan",
      value: state.dashboard.lastScan ? formatDate(state.dashboard.lastScan.finishedAt) : "Jamais",
      tone: "default"
    }
  ];

  elements.dashboardCards.innerHTML = cards
    .map(
      (card) => `
        <article class="stat-card stat-card--${card.tone}">
          <span class="stat-card__label">${card.label}</span>
          <strong class="stat-card__value">${card.value}</strong>
        </article>
      `
    )
    .join("");

  if (state.dashboard.lastScan) {
    const lastScan = state.dashboard.lastScan;
    elements.lastScanCard.innerHTML = `
      <div class="summary-card">
        <div>
          <span class="summary-card__label">Mode</span>
          <strong>${formatMode(lastScan.mode)}</strong>
        </div>
        <div>
          <span class="summary-card__label">Moteur</span>
          <strong>${lastScan.engine}</strong>
        </div>
        <div>
          <span class="summary-card__label">Menaces</span>
          <strong>${lastScan.infectedFiles}</strong>
        </div>
        <div>
          <span class="summary-card__label">Termin&eacute;</span>
          <strong>${formatDate(lastScan.finishedAt)}</strong>
        </div>
      </div>
    `;
  } else {
    elements.lastScanCard.innerHTML = "Les informations du dernier scan apparaîtront ici.";
  }

  elements.engineStatusLabel.textContent = state.dashboard.engineReady
    ? `Moteur ClamAV détecté (${state.dashboard.engineSource})`
    : "ClamAV introuvable";
  elements.engineDot.classList.toggle("status-dot--online", Boolean(state.dashboard.engineReady));
  elements.topbarEngineLabel.textContent = state.dashboard.engineReady
    ? `${state.dashboard.engineSource} pret`
    : "Moteur indisponible";
  elements.overviewEngineSource.textContent = state.dashboard.engineReady
    ? state.dashboard.engineSource
    : "Indisponible";
}

function renderRealtimeCard() {
  const realtime = state.realtime;
  if (!realtime) {
    elements.realtimeCard.innerHTML = `<div class="placeholder">Protection temps réel indisponible.</div>`;
    return;
  }

  elements.toggleRealtimeButton.textContent = realtime.enabled ? "Désactiver" : "Activer";
  elements.watcherLabel.textContent = realtime.enabled
    ? `Protection active sur ${realtime.watchedPaths.length} zone(s)`
    : "Protection temps réel inactive";
  elements.overviewRealtimeState.textContent = realtime.enabled ? "Active" : "Inactif";

  elements.realtimeCard.innerHTML = `
    <div class="stacked-list__item">
      <span>État</span>
      <strong>${realtime.enabled ? "Actif" : "Inactif"}</strong>
    </div>
    <div class="stacked-list__item">
      <span>Auto-quarantaine</span>
      <strong>${realtime.autoQuarantine ? "Oui" : "Non"}</strong>
    </div>
    <div class="stacked-list__item">
      <span>Moteur</span>
      <strong>${realtime.usingClamd ? "clamdscan" : "clamscan"}</strong>
    </div>
    <div class="stacked-list__item">
      <span>Téléchargements</span>
      <strong>${realtime.downloadsProtected ? "Protégés" : "Non surveillés"}</strong>
    </div>
    <div class="stacked-list__item stacked-list__item--column">
      <span>Zones surveillées</span>
      <div class="badge-list">
        ${
          realtime.watchedPaths.length
            ? realtime.watchedPaths.map((path) => `<span class="badge">${path}</span>`).join("")
            : `<span class="badge">Aucune</span>`
        }
      </div>
    </div>
  `;
}

function renderConfig() {
  if (!state.config) {
    return;
  }

  elements.downloadsToggle.checked = state.config.autoScanDownloads;
  elements.clamdCheckbox.checked = state.config.useClamdByDefault;
  elements.quarantineCheckbox.checked = state.config.autoQuarantine;

  elements.configWatchPaths.innerHTML = state.config.watchedPaths.length
    ? state.config.watchedPaths
        .map(
          (path) => `
            <button class="badge badge--action" data-remove-watch="${path}" title="Retirer ce dossier">
              ${path}
            </button>
          `
        )
        .join("")
    : `<span class="placeholder">Aucun dossier surveillé configuré.</span>`;

  if (state.system) {
    elements.overviewPermissionLevel.textContent = state.system.permissionLevel;
    elements.overviewPlatform.textContent = state.system.platform;

    elements.systemAccessCard.innerHTML = `
      <div class="stacked-list__item">
        <span>Niveau de privilèges</span>
        <strong>${state.system.permissionLevel}</strong>
      </div>
      <div class="stacked-list__item">
        <span>Scan système large</span>
        <strong>${state.system.canScanSystemPaths ? "Possible" : "Limité"}</strong>
      </div>
      <div class="stacked-list__item">
        <span>Source moteur</span>
        <strong>${state.system.engineSource}</strong>
      </div>
      <div class="stacked-list__item">
        <span>Base signatures</span>
        <strong>${state.system.databasePath ?? "Emplacement système"}</strong>
      </div>
      <div class="stacked-list__item stacked-list__item--column">
        <span>Conseil</span>
        <strong>${state.system.permissionHint}</strong>
      </div>
    `;
  }
}

function renderSelectedTargets() {
  if (!state.selectedTargets.length) {
    elements.selectedTargets.innerHTML = "Aucune cible personnalisée.";
    return;
  }

  elements.selectedTargets.innerHTML = `
    <div class="badge-list">
      ${state.selectedTargets.map((path) => `<span class="badge">${path}</span>`).join("")}
    </div>
  `;
}

function renderLogs() {
  if (!state.logs.length) {
    elements.logsList.innerHTML = `<div class="placeholder">Aucun log disponible.</div>`;
    return;
  }

  elements.logsList.innerHTML = state.logs
    .map(
      (log) => `
        <article class="result-card">
          <div class="result-card__topline">
            <span class="badge">${formatMode(log.mode)}</span>
            <span>${formatDate(log.finishedAt)}</span>
          </div>
          <h4 class="path-value">${log.targets.join(", ")}</h4>
          <div class="meta-grid">
            <div class="meta-grid__item">
              <span>Menaces</span>
              <strong>${log.infectedFiles}</strong>
            </div>
            <div class="meta-grid__item">
              <span>Propres</span>
              <strong>${log.cleanFiles}</strong>
            </div>
            <div class="meta-grid__item">
              <span>Inspectes</span>
              <strong>${log.scannedFiles}</strong>
            </div>
            <div class="meta-grid__item">
              <span>Moteur</span>
              <strong>${log.engine}</strong>
            </div>
          </div>
          ${
            log.accessDeniedCount
              ? `<div class="muted-line">${log.accessDeniedCount} accès refusé(s) détecté(s).</div>`
              : ""
          }
          ${
            log.threats.length
              ? `<div class="threat-list">${log.threats
                  .map((threat) => `<span class="threat-pill">${threat.signature} · ${threat.path}</span>`)
                  .join("")}</div>`
              : `<div class="muted-line">Aucune menace détectée.</div>`
          }
        </article>
      `
    )
    .join("");
}

function renderQuarantine() {
  if (!state.quarantine.length) {
    elements.quarantineList.innerHTML = `<div class="placeholder">Aucun fichier isolé.</div>`;
    return;
  }

  elements.quarantineList.innerHTML = state.quarantine
    .map(
      (item) => `
        <article class="result-card">
          <div class="result-card__topline">
            <span class="badge badge--danger">${item.signature}</span>
            <span>${formatDate(item.detectedAt)}</span>
          </div>
          <h4>${item.fileName}</h4>
          <div class="path-value">${item.originalPath}</div>
          <div class="actions-inline">
            <button class="button button--ghost button--small" data-restore-id="${item.id}">Restaurer</button>
            <button class="button button--danger button--small" data-delete-id="${item.id}">Supprimer</button>
          </div>
        </article>
      `
    )
    .join("");
}

function renderScanResults(report) {
  if (!report) {
    elements.scanResults.innerHTML =
      '<div class="placeholder">Les fichiers infectés ou propres apparaîtront ici après un scan.</div>';
    return;
  }

  elements.scanResults.innerHTML = `
    <article class="result-card">
      <div class="result-card__topline">
        <span class="badge">${formatMode(report.mode)}</span>
        <span>${formatDate(report.finishedAt)}</span>
      </div>
      <h4>${report.infectedFiles ? "Menaces détectées" : "Aucune menace détectée"}</h4>
      <div class="meta-grid">
        <div class="meta-grid__item">
          <span>Inspectes</span>
          <strong>${report.scannedFiles}</strong>
        </div>
        <div class="meta-grid__item">
          <span>Propres</span>
          <strong>${report.cleanFiles}</strong>
        </div>
        <div class="meta-grid__item">
          <span>Erreurs</span>
          <strong>${report.errorCount}</strong>
        </div>
        <div class="meta-grid__item">
          <span>Moteur</span>
          <strong>${report.engine}</strong>
        </div>
      </div>
      ${
        report.accessDeniedCount
          ? `<div class="muted-line">${report.accessDeniedCount} chemin(s) avec accès refusé.</div>`
          : ""
      }
      ${
        report.threats.length
          ? `<div class="threat-list">${report.threats
              .map((threat) => `<span class="threat-pill">${threat.signature} · ${threat.path}</span>`)
              .join("")}</div>`
          : `<div class="muted-line">Résultat propre.</div>`
      }
    </article>
  `;
}

function updateProgress(payload) {
  if (!payload) {
    return;
  }

  state.currentScanId = payload.scanId;
  const percentage = Number(payload.percent ?? 0).toFixed(0);
  elements.scanStatusLabel.textContent = payload.status;
  elements.scanProgressLabel.textContent = `${percentage}%`;
  elements.scanProgressBar.style.width = `${percentage}%`;
  elements.scanCurrentTarget.textContent = payload.path
    ? `Analyse en cours: ${payload.path}`
    : "Préparation du scan...";
}

async function notifyThreat(payload) {
  let granted = await isPermissionGranted();
  if (!granted) {
    const permission = await requestPermission();
    granted = permission === "granted";
  }

  if (!granted) {
    return;
  }

  const threat = payload?.threat;
  const title = payload?.realtime ? "Menace détectée en temps réel" : "Menace détectée";
  const body = threat
    ? `${threat.signature} dans ${threat.path}`
    : "Une activité suspecte a été détectée.";

  await sendNotification({ title, body });
}

async function refreshState() {
  const [dashboard, logs, quarantine, realtime, system, config] = await Promise.all([
    invoke("get_dashboard_data"),
    invoke("get_scan_history"),
    invoke("get_quarantine_items"),
    invoke("get_realtime_status"),
    invoke("get_system_status"),
    invoke("get_app_config")
  ]);

  state.dashboard = dashboard;
  state.logs = logs;
  state.quarantine = quarantine;
  state.realtime = realtime;
  state.system = system;
  state.config = config;

  elements.platformLabel.textContent = system.platform;
  elements.permissionLabel.textContent = `Privilèges: ${system.permissionLevel}`;

  renderDashboard();
  renderLogs();
  renderQuarantine();
  renderRealtimeCard();
  renderConfig();
}

async function runScan(modeOverride) {
  const mode = modeOverride ?? state.mode;

  if (mode === "custom" && !state.selectedTargets.length) {
    elements.scanStatusLabel.textContent = "Choisissez au moins un fichier ou dossier.";
    return;
  }

  setActiveView("scan");
  elements.scanStatusLabel.textContent = "Initialisation...";
  elements.scanProgressLabel.textContent = "0%";
  elements.scanProgressBar.style.width = "0%";
  elements.scanCurrentTarget.textContent = "Connexion au moteur ClamAV...";

  const request = {
    mode,
    targets: mode === "custom" ? state.selectedTargets : [],
    useClamd: elements.clamdCheckbox.checked,
    quarantineDetected: elements.quarantineCheckbox.checked
  };

  try {
    const report = await invoke("scan_target", { request });
    renderScanResults(report);
    await refreshState();
  } catch (error) {
    elements.scanStatusLabel.textContent = "Le scan a échoué.";
    elements.scanCurrentTarget.textContent = String(error);
  }
}

async function toggleRealtime() {
  if (state.realtime?.enabled) {
    state.realtime = await invoke("stop_realtime_protection");
  } else {
    state.realtime = await invoke("start_realtime_protection", {
      autoQuarantine: state.config?.autoQuarantine,
      useClamd: state.config?.useClamdByDefault
    });
  }

  renderRealtimeCard();
}

async function pickFiles() {
  const selection = await open({
    directory: false,
    multiple: true
  });

  if (!selection) {
    return;
  }

  state.mode = "custom";
  state.selectedTargets = Array.isArray(selection) ? selection : [selection];
  syncModeCards();
  renderSelectedTargets();
}

async function pickFolder() {
  const selection = await open({
    directory: true,
    multiple: false
  });

  if (!selection) {
    return;
  }

  state.mode = "custom";
  state.selectedTargets = Array.isArray(selection) ? selection : [selection];
  syncModeCards();
  renderSelectedTargets();
}

function uniquePaths(paths) {
  return [...new Set(paths)];
}

async function persistConfig() {
  if (!state.config) {
    return;
  }

  state.config = await invoke("save_app_config", { config: state.config });
  await refreshState();
}

async function addWatchedFolder() {
  const selection = await open({
    directory: true,
    multiple: false
  });

  if (!selection || !state.config) {
    return;
  }

  const nextPath = Array.isArray(selection) ? selection[0] : selection;
  state.config.watchedPaths = uniquePaths([...state.config.watchedPaths, nextPath]);
  await persistConfig();
}

function syncModeCards() {
  elements.modeCards.forEach((card) => {
    card.classList.toggle("mode-card--active", card.dataset.mode === state.mode);
  });
}

async function updateSignatures() {
  elements.updateButton.disabled = true;
  elements.updateButton.textContent = "Mise à jour...";

  try {
    const report = await invoke("update_signatures");
    elements.scanStatusLabel.textContent = report.success
      ? "Signatures mises à jour."
      : "Mise à jour terminée avec erreur.";
    elements.scanCurrentTarget.textContent = report.output.split("\n").slice(-1)[0] || report.output;
  } catch (error) {
    elements.scanStatusLabel.textContent = "La mise à jour a échoué.";
    elements.scanCurrentTarget.textContent = String(error);
  } finally {
    elements.updateButton.disabled = false;
    elements.updateButton.textContent = "Mettre à jour les signatures";
    await refreshState();
  }
}

async function restoreItem(id) {
  await invoke("restore_quarantine_item", { id });
  await refreshState();
}

async function deleteItem(id) {
  await invoke("delete_quarantine_item", { id });
  await refreshState();
}

function attachEvents() {
  elements.navItems.forEach((item) => {
    item.addEventListener("click", () => setActiveView(item.dataset.view));
  });

  elements.modeCards.forEach((card) => {
    card.addEventListener("click", () => {
      state.mode = card.dataset.mode;
      syncModeCards();
    });
  });

  elements.quickScanButton.addEventListener("click", () => runScan("quick"));
  elements.startScanButton.addEventListener("click", () => runScan());
  elements.pickFilesButton.addEventListener("click", pickFiles);
  elements.pickFolderButton.addEventListener("click", pickFolder);
  elements.addWatchFolderButton.addEventListener("click", addWatchedFolder);
  elements.updateButton.addEventListener("click", updateSignatures);
  elements.toggleRealtimeButton.addEventListener("click", toggleRealtime);
  elements.downloadsToggle.addEventListener("change", async (event) => {
    if (!state.config) {
      return;
    }

    state.config.autoScanDownloads = event.target.checked;
    await persistConfig();
  });
  elements.clamdCheckbox.addEventListener("change", async (event) => {
    if (!state.config) {
      return;
    }

    state.config.useClamdByDefault = event.target.checked;
    await persistConfig();
  });
  elements.quarantineCheckbox.addEventListener("change", async (event) => {
    if (!state.config) {
      return;
    }

    state.config.autoQuarantine = event.target.checked;
    await persistConfig();
  });

  elements.quarantineList.addEventListener("click", async (event) => {
    const restoreId = event.target.dataset.restoreId;
    const deleteId = event.target.dataset.deleteId;

    if (restoreId) {
      await restoreItem(restoreId);
    }

    if (deleteId) {
      await deleteItem(deleteId);
    }
  });

  elements.configWatchPaths.addEventListener("click", async (event) => {
    const path = event.target.dataset.removeWatch;
    if (!path || !state.config) {
      return;
    }

    state.config.watchedPaths = state.config.watchedPaths.filter((entry) => entry !== path);
    await persistConfig();
  });
}

async function attachTauriListeners() {
  await listen("scan-progress", (event) => updateProgress(event.payload));

  await listen("signature-update-complete", async () => {
    await refreshState();
  });

  await listen("threat-detected", async (event) => {
    await notifyThreat({ ...event.payload, realtime: false });
  });

  await listen("realtime-threat", async (event) => {
    await notifyThreat({ ...event.payload, realtime: true });
    await refreshState();
  });

  await listen("realtime-status", (event) => {
    state.realtime = event.payload;
    renderRealtimeCard();
  });
}

async function bootstrap() {
  elements.brandLogo.src = clamavLogo;
  attachEvents();
  syncModeCards();
  renderSelectedTargets();
  renderScanResults(null);
  await attachTauriListeners();
  await refreshState();
}

bootstrap().catch((error) => {
  elements.scanStatusLabel.textContent = "Erreur d'initialisation";
  elements.scanCurrentTarget.textContent = String(error);
});
