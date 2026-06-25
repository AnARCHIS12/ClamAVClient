import { execFileSync } from "node:child_process";

const platformBundles = {
  linux: ["deb", "rpm"],
  darwin: ["app", "dmg"],
  win32: ["msi", "nsis"]
};

const bundles = platformBundles[process.platform];

if (!bundles) {
  console.error(`Plateforme non prise en charge pour le build desktop: ${process.platform}`);
  process.exit(1);
}

const bundleArg = bundles.join(",");

console.log(`Build desktop pour ${process.platform} avec les bundles: ${bundleArg}`);

execFileSync("npm", ["run", "prepare:clamav"], {
  stdio: "inherit",
  shell: process.platform === "win32"
});

execFileSync("npm", ["run", "tauri", "--", "build", "--bundles", bundleArg], {
  stdio: "inherit",
  shell: process.platform === "win32"
});
