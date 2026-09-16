// WI067 Checkpoint C, Phase 20: proves the frontend's GitHub surface has
// no generic URL-fetch shape anywhere -- no command call in this source
// tree ever sends an arbitrary `url`, `headers` map, or filesystem
// `destination`/`path` alongside a repository/ref identifier. The real
// authority boundary is enforced server-side (the Rust command signatures
// in `remote_provider.rs` literally have no such parameters, which is
// what makes injecting one impossible, not this test) -- this is the
// frontend-side companion proof that nothing here even attempts to.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const sourceRoot = join(dirname(fileURLToPath(import.meta.url)));

describe("remote provider frontend authority boundary", () => {
  it("never sends an arbitrary url/headers/destination alongside a remote command", () => {
    const source =
      readFileSync(join(sourceRoot, "lib/remote-api.ts"), "utf8") +
      readFileSync(join(sourceRoot, "RemoteRepositoryPanel.tsx"), "utf8");
    expect(source).not.toMatch(/\burl\s*:/i);
    expect(source).not.toMatch(/\bheaders\s*:/i);
    expect(source).not.toMatch(/\bdestination(Path)?\s*:/i);
    expect(source).not.toMatch(/\bfile:\/\//i);
    expect(source).not.toMatch(/localhost/i);
    expect(source).not.toContain("github_request");
    expect(source).not.toContain("provider_request");
    expect(source).not.toContain("authenticated_fetch");
    expect(source).not.toContain("fetch(");
  });

  it("only invokes the narrow typed remote_* commands, never a generic one", () => {
    const source = readFileSync(join(sourceRoot, "lib/remote-api.ts"), "utf8");
    const invokedCommands = [...source.matchAll(/invoke<[^>]*>\(\s*"([^"]+)"/g)].map((match) => match[1]);
    expect(invokedCommands.length).toBeGreaterThan(0);
    for (const command of invokedCommands) {
      expect(command.startsWith("remote_")).toBe(true);
      expect(command).not.toMatch(/request|fetch|proxy/i);
    }
  });
});
