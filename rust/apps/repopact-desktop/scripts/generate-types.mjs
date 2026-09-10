import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

const packageRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const result = spawnSync(
  "cargo",
  ["run", "--manifest-path", join(packageRoot, "..", "..", "Cargo.toml"), "-p", "repopact-desktop-api", "--bin", "generate-types", "--", join(packageRoot, "src", "generated", "types.ts")],
  {
    cwd: packageRoot,
    env: { ...process.env, CARGO_TARGET_DIR: join(tmpdir(), "repopact-rust-target-055") },
    stdio: "inherit",
  },
);
process.exit(result.status ?? 1);
