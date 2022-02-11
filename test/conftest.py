import tempfile
import subprocess
import pathlib


import pytest


@pytest.fixture()
def repo():
    with tempfile.TemporaryDirectory("watchit_tests_") as tempdir:
        tempdir = pathlib.Path(tempdir)
        subprocess.run(["git", "init"], cwd=tempdir)
        (tempdir / ".gitignore").write_text("/ignored_directory\n*.ignored_file\n")
        (tempdir / "existing_file").write_text("Existing contents")
        yield tempdir
