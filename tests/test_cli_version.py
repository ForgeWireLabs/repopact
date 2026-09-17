"""Tests for the top-level `repopact --version` flag (work item 069)."""

from __future__ import annotations

import io
import subprocess
import sys
import unittest
from contextlib import redirect_stdout

import repopact
from repopact import cli


class VersionFlagTests(unittest.TestCase):
    def test_version_exits_zero_without_a_subcommand(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit) as ctx:
                cli.main(["--version"])
        self.assertEqual(0, ctx.exception.code)

    def test_version_output_uses_the_governed_package_identity(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit):
                cli.main(["--version"])
        self.assertEqual(f"repopact {repopact.__version__}", stdout.getvalue().strip())

    def test_console_invocation_reports_version_and_exits_zero(self):
        result = subprocess.run(
            [sys.executable, "-m", "repopact.cli", "--version"],
            capture_output=True,
            text=True,
        )
        self.assertEqual(0, result.returncode)
        self.assertTrue(result.stdout.strip().startswith("repopact "))

    def test_no_command_and_no_version_still_requires_a_subcommand(self):
        with self.assertRaises(SystemExit) as ctx:
            cli.main([])
        self.assertNotEqual(0, ctx.exception.code)


if __name__ == "__main__":
    unittest.main()
