export function npmPackagesFromLock(lock, manifestForPath = () => ({})) {
  return Object.entries(lock.packages ?? {})
    .filter(([, entry]) => !entry.link)
    .map(([packagePath, entry]) => {
      const manifest = manifestForPath(packagePath);
      return {
        name: manifest.name ?? entry.name ?? packageNameFromPath(packagePath),
        version: manifest.version ?? entry.version ?? "0.0.0",
        license: manifest.license ?? entry.license ?? null,
      };
    })
    .filter((entry) => entry.name)
    .sort((left, right) => `${left.name}@${left.version}`.localeCompare(`${right.name}@${right.version}`));
}

function packageNameFromPath(packagePath) {
  const marker = "node_modules/";
  const index = packagePath.lastIndexOf(marker);
  if (index < 0) return packagePath || "desktoplab";
  return packagePath.slice(index + marker.length);
}
