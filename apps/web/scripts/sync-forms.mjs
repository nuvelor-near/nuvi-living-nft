import { copyFile, mkdir, stat } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const sourceDir = path.resolve(scriptDir, "../../../assets/forms");
const targetDir = path.resolve(scriptDir, "../public/forms");
const files = ["basic.png", "form_a.png", "form_b.png"];

await mkdir(targetDir, { recursive: true });

let copied = 0;
for (const file of files) {
  const source = path.join(sourceDir, file);
  const target = path.join(targetDir, file);
  try {
    const info = await stat(source);
    if (!info.isFile()) continue;
    await copyFile(source, target);
    copied += 1;
  } catch {
    // Images are supplied separately; a missing form must not block code build.
  }
}

console.log(`NUVI forms synced: ${copied}/${files.length}`);
