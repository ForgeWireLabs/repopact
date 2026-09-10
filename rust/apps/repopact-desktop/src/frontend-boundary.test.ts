import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const sourceRoot = join(dirname(fileURLToPath(import.meta.url)));

describe("frontend authority boundary", () => {
  it("contains only typed invoke intent and no raw filesystem or mutation authority", () => {
    const source = readFileSync(join(sourceRoot, "App.tsx"), "utf8") + readFileSync(join(sourceRoot, "lib/api.ts"), "utf8");
    expect(source).not.toContain("file_operations");
    expect(source).not.toContain("read_file");
    expect(source).not.toContain("evidence_plan");
    expect(source).not.toContain("tauri-plugin-fs");
    expect(source).not.toContain("MutationPlan\"");
    expect(source).toContain("plan_handle");
    expect(source).toContain("apply_mutation_plan");
  });
});
