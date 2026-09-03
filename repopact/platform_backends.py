"""OS-neutral backend SPI with honest capability declarations."""
from __future__ import annotations

import os
import platform
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class PlatformBackend:
    name: str
    os_name: str
    protected_state_location: Path
    process_boundary: bool = False
    path_boundary: bool = False

    @property
    def security_level(self) -> str:
        if self.process_boundary and self.path_boundary: return "sandbox/process-enforced"
        return "pre-action"

    def capabilities(self) -> dict[str, object]:
        return {"os": self.os_name, "backend": self.name, "protected_state": str(self.protected_state_location),
                "path_confinement": self.path_boundary, "process_confinement": self.process_boundary,
                "protected_from_gated_principal": False,
                "integrity_checked": self.protected_state_location.exists(),
                "security_level": self.security_level if self.process_boundary and self.path_boundary else "not-covered"}

    def health(self) -> dict[str, object]:
        return {**self.capabilities(), "healthy": self.protected_state_location.parent.exists() or self.protected_state_location.parent == Path.home()}

    def normalize(self, path: str | Path) -> str:
        return os.path.normcase(str(Path(path).expanduser().resolve(strict=False))).replace("\\", "/").rstrip("/")


class WindowsBackend(PlatformBackend):
    def __init__(self, location: Path | None = None): super().__init__("windows-reference", "windows", location or (Path.home() / ".repopact" / "registrations"))


class LinuxBackend(PlatformBackend):
    def __init__(self, location: Path | None = None): super().__init__("linux-reference", "linux", location or (Path.home() / ".local" / "state" / "repopact" / "registrations"))


class MacOSBackend(PlatformBackend):
    def __init__(self, location: Path | None = None): super().__init__("macos-reference", "macos", location or (Path.home() / "Library" / "Application Support" / "RepoPact" / "registrations"))


def current_backend() -> PlatformBackend:
    system = platform.system().lower()
    return WindowsBackend() if system == "windows" else LinuxBackend() if system == "linux" else MacOSBackend() if system == "darwin" else PlatformBackend("generic", system, Path.home() / ".repopact" / "registrations")
