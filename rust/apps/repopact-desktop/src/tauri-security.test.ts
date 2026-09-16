import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const appRoot = join(process.cwd(), "src-tauri");

describe("Tauri capability boundary", () => {
  it("does not grant generic filesystem, shell, remote, or default capability authority", () => {
    const capability = readFileSync(join(appRoot, "capabilities/main-window.json"), "utf8");
    const permission = readFileSync(join(appRoot, "permissions/workbench.toml"), "utf8");
    const config = readFileSync(join(appRoot, "tauri.conf.json"), "utf8");
    // `\bremote\b` (not a bare `remote` substring): WI067 introduces typed
    // `remote_*` commands (Decision 0061 -- "remote repository provider"),
    // which must not trip this check merely for containing the word
    // "remote" in an underscore-joined command name. The boundary
    // pattern still catches a genuine standalone/colon-suffixed dangerous
    // token such as a `"remote"` or `remote:` capability/permission scope.
    expect(capability).not.toMatch(/core:default|fs:|shell:|http:|\bremote\b/i);
    expect(permission).not.toMatch(/read_file|write_file|fs:|shell:|http:|\bremote\b/i);
    expect(config).toContain('"withGlobalTauri": false');
    expect(config).toContain('"devUrl": "http://127.0.0.1:1420"');
    const nonLocalConfig = config
      .replace("https://schema.tauri.app", "")
      .replace("http://127.0.0.1:1420", "")
      .replace("http://ipc.localhost", "");
    expect(nonLocalConfig).not.toMatch(/https?:\/\//i);
  });

  it("keeps the custom command surface explicit", () => {
    const permission = readFileSync(join(appRoot, "permissions/workbench.toml"), "utf8");
    expect(permission).toContain('"plan_mutation"');
    expect(permission).toContain('"apply_mutation_plan"');
    expect(permission).toContain('"poll_repository_events"');
    expect(permission).not.toContain("read_file");
  });
});
