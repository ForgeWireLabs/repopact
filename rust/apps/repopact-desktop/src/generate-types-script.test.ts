import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = join(dirname(fileURLToPath(import.meta.url)), "..", "scripts", "generate-types.mjs");

describe("generate-types.mjs target-directory isolation", () => {
  it("never defaults CARGO_TARGET_DIR to a shared temp directory", () => {
    const source = readFileSync(scriptPath, "utf8");
    expect(source).not.toContain("tmpdir");
    expect(source).not.toContain("repopact-rust-target");
    expect(source).not.toContain("node:os");
  });

  it("passes the caller's environment through unchanged, preserving any explicit override", () => {
    const source = readFileSync(scriptPath, "utf8");
    expect(source).toContain("env: process.env");
  });
});
