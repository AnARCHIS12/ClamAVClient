import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { execFileSync } from "node:child_process";

const projectRoot = process.cwd();
const vendorRoot = path.join(projectRoot, "src-tauri", "vendor", "clamav");

const platformMap = {
  linux: "linux",
  darwin: "macos",
  win32: "windows"
};

const currentPlatform = platformMap[process.platform];

if (!currentPlatform) {
  console.error(`Plateforme non prise en charge pour la preparation ClamAV: ${process.platform}`);
  process.exit(1);
}

const spec = buildPlatformSpec(currentPlatform);
const destination = {
  root: path.join(vendorRoot, currentPlatform),
  bin: path.join(vendorRoot, currentPlatform, "bin"),
  db: path.join(vendorRoot, currentPlatform, "db")
};

fs.mkdirSync(destination.bin, { recursive: true });
fs.mkdirSync(destination.db, { recursive: true });

const resolvedBinDir = firstMatchingBinDir(spec.binCandidates, spec.requiredBins);
const resolvedDbDir = firstMatchingDbDir(spec.dbCandidates);

if (!resolvedBinDir) {
  console.error("Impossible de trouver les binaires ClamAV requis pour cette plateforme.");
  printHelp(spec);
  process.exit(1);
}

const copiedBins = copyRequiredFiles(resolvedBinDir, destination.bin, spec.requiredBins, spec.optionalBins);
let copiedDb = [];

if (resolvedDbDir) {
  copiedDb = copySignatureDb(resolvedDbDir, destination.db);
}

if (!copiedDb.length) {
  copiedDb = bootstrapSignatureDb(destination.db, path.join(destination.bin, spec.requiredBins[1]));
}

if (!copiedBins.length) {
  console.error("Aucun binaire ClamAV requis n'a pu etre copie.");
  process.exit(1);
}

if (!copiedDb.length) {
  console.error("Impossible d'obtenir une base de signatures initiale.");
  printHelp(spec);
  process.exit(1);
}

console.log(`ClamAV prepare pour ${currentPlatform}`);
console.log(`Binaires source: ${resolvedBinDir}`);
console.log(`Base source: ${resolvedDbDir ?? "telechargee via freshclam"}`);
console.log(`Binaires copies: ${copiedBins.join(", ")}`);
console.log(`Signatures copiees: ${copiedDb.join(", ")}`);

function buildPlatformSpec(platform) {
  if (platform === "linux") {
    return {
      binCandidates: compact([
        process.env.CLAMAV_BIN_DIR,
        withJoin(process.env.CLAMAV_SOURCE_ROOT, "bin"),
        "/usr/local/bin",
        "/usr/bin"
      ]),
      dbCandidates: compact([
        process.env.CLAMAV_DB_DIR,
        withJoin(process.env.CLAMAV_SOURCE_ROOT, "share", "clamav"),
        "/usr/local/share/clamav",
        "/var/lib/clamav"
      ]),
      requiredBins: ["clamscan", "freshclam"],
      optionalBins: ["clamdscan", "clamd"]
    };
  }

  if (platform === "macos") {
    return {
      binCandidates: compact([
        process.env.CLAMAV_BIN_DIR,
        withJoin(process.env.CLAMAV_SOURCE_ROOT, "bin"),
        "/usr/local/clamav/bin",
        "/usr/local/bin",
        "/opt/homebrew/bin"
      ]),
      dbCandidates: compact([
        process.env.CLAMAV_DB_DIR,
        withJoin(process.env.CLAMAV_SOURCE_ROOT, "share", "clamav"),
        "/usr/local/clamav/share/clamav",
        "/usr/local/var/lib/clamav",
        "/opt/homebrew/var/lib/clamav"
      ]),
      requiredBins: ["clamscan", "freshclam"],
      optionalBins: ["clamdscan", "clamd"]
    };
  }

  return {
    binCandidates: compact([
      process.env.CLAMAV_BIN_DIR,
      process.env.CLAMAV_SOURCE_ROOT,
      "C:\\Program Files\\ClamAV",
      "C:\\Program Files (x86)\\ClamAV"
    ]),
    dbCandidates: compact([
      process.env.CLAMAV_DB_DIR,
      withJoin(process.env.CLAMAV_SOURCE_ROOT, "database"),
      withJoin(process.env.CLAMAV_SOURCE_ROOT, "db"),
      "C:\\ProgramData\\.clamwin\\db",
      "C:\\Program Files\\ClamAV\\Database"
    ]),
    requiredBins: ["clamscan.exe", "freshclam.exe"],
    optionalBins: ["clamdscan.exe", "clamd.exe", "libclamav.dll", "libcrypto-3-x64.dll", "libssl-3-x64.dll", "zlib1.dll", "libxml2.dll", "pcre2-8.dll", "iconv.dll"]
  };
}

function compact(values) {
  return values.filter(Boolean);
}

function withJoin(root, ...parts) {
  return root ? path.join(root, ...parts) : undefined;
}

function firstMatchingBinDir(candidates, requiredBins) {
  return candidates.find((candidate) => {
    try {
      if (!fs.statSync(candidate).isDirectory()) {
        return false;
      }

      return requiredBins.every((fileName) => fs.existsSync(path.join(candidate, fileName)));
    } catch {
      return false;
    }
  });
}

function firstMatchingDbDir(candidates) {
  return candidates.find((candidate) => {
    try {
      if (!fs.statSync(candidate).isDirectory()) {
        return false;
      }

      return hasSignatureFiles(candidate);
    } catch {
      return false;
    }
  });
}

function copyRequiredFiles(sourceDir, targetDir, required, optional) {
  const copied = [];

  for (const fileName of required) {
    const source = path.join(sourceDir, fileName);
    if (!fs.existsSync(source)) {
      console.error(`Fichier requis manquant: ${source}`);
      process.exit(1);
    }

    const target = path.join(targetDir, fileName);
    fs.copyFileSync(source, target);
    copied.push(fileName);
  }

  for (const fileName of optional) {
    const source = path.join(sourceDir, fileName);
    if (!fs.existsSync(source)) {
      continue;
    }

    const target = path.join(targetDir, fileName);
    fs.copyFileSync(source, target);
    copied.push(fileName);
  }

  return copied;
}

function copySignatureDb(sourceDir, targetDir) {
  const copied = [];
  const allowedExtensions = new Set([".cvd", ".cld", ".cdiff"]);
  const entries = fs.readdirSync(sourceDir, { withFileTypes: true });

  for (const entry of entries) {
    if (!entry.isFile()) {
      continue;
    }

    const extension = path.extname(entry.name).toLowerCase();
    if (!allowedExtensions.has(extension)) {
      continue;
    }

    const source = path.join(sourceDir, entry.name);
    const target = path.join(targetDir, entry.name);
    fs.copyFileSync(source, target);
    copied.push(entry.name);
  }

  return copied;
}

function bootstrapSignatureDb(targetDir, freshclamPath) {
  if (!fs.existsSync(freshclamPath)) {
    return [];
  }

  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "clamavclient-"));
  const tempDbDir = path.join(tempRoot, "db");
  const tempConfig = path.join(tempRoot, "freshclam.conf");
  fs.mkdirSync(tempDbDir, { recursive: true });

  fs.writeFileSync(
    tempConfig,
    `DatabaseDirectory ${tempDbDir}\nDatabaseMirror database.clamav.net\nChecks 1\nForeground yes\n`
  );

  try {
    execFileSync(
      freshclamPath,
      [`--config-file=${tempConfig}`, `--datadir=${tempDbDir}`, "--stdout"],
      { stdio: "inherit" }
    );
  } catch {
    return [];
  }

  return copySignatureDb(tempDbDir, targetDir);
}

function hasSignatureFiles(directory) {
  const allowedExtensions = new Set([".cvd", ".cld", ".cdiff"]);
  return fs.readdirSync(directory, { withFileTypes: true }).some((entry) => {
    if (!entry.isFile()) {
      return false;
    }

    return allowedExtensions.has(path.extname(entry.name).toLowerCase());
  });
}

function printHelp(spec) {
  console.error("Dossiers de recherche essayes pour les binaires:");
  for (const dir of spec.binCandidates) {
    console.error(`- ${dir}`);
  }

  console.error("Dossiers de recherche essayes pour la base:");
  for (const dir of spec.dbCandidates) {
    console.error(`- ${dir}`);
  }

  console.error("Vous pouvez surcharger avec:");
  console.error("- CLAMAV_SOURCE_ROOT");
  console.error("- CLAMAV_BIN_DIR");
  console.error("- CLAMAV_DB_DIR");
}
