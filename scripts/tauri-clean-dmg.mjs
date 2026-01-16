import { readdir, rm } from "node:fs/promises";
import { join } from "node:path";

if (process.platform !== "darwin") {
  process.exit(0);
}

const cwd = process.cwd();
const baseDir = cwd.endsWith("src-tauri") ? cwd : join(cwd, "src-tauri");
const dmgDir = join(baseDir, "target", "release", "bundle", "macos");

try {
  const entries = await readdir(dmgDir);
  const dmgFiles = entries.filter((entry) => entry.endsWith(".dmg"));
  await Promise.all(dmgFiles.map((entry) => rm(join(dmgDir, entry), { force: true })));
} catch (error) {
  if (error && typeof error === "object" && "code" in error && error.code === "ENOENT") {
    process.exit(0);
  }
  throw error;
}
