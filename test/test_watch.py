import time
import subprocess
import os
from contextlib import contextmanager

import pytest


def test_new_file(repo):
    with run_watchit(repo) as proc:
        (repo / "new_file").write_text("New stuff")
        assert proc.wait(timeout=5) == 0


def test_modify_file(repo):
    with run_watchit(repo) as proc:
        (repo / "existing_file").write_text("New stuff")
        assert proc.wait(timeout=5) == 0


def test_delete_file(repo):
    with run_watchit(repo) as proc:
        (repo / "existing_file").unlink()
        assert proc.wait(timeout=5) == 0


def test_move_file(repo):
    with run_watchit(repo) as proc:
        (repo / "existing_file").rename(repo / "moved_file")
        assert proc.wait(timeout=5) == 0


def test_new_subdirectory_and_file(repo):
    with run_watchit(repo) as proc:
        (repo / "subdirectory").mkdir()
        with pytest.raises(subprocess.TimeoutExpired):
            proc.wait(timeout=3)
        (repo / "subdirectory" / "new_file").write_text("New stuff")
        assert proc.wait(timeout=5) == 0


def test_new_ignored_file(repo):
    with run_watchit(repo) as proc:
        (repo / "new_file.ignored_file").write_text("New stuff")
        with pytest.raises(subprocess.TimeoutExpired):
            proc.wait(timeout=3)


def test_new_ignored_subdirectory_and_file(repo):
    with run_watchit(repo) as proc:
        (repo / "ignored_directory").mkdir()
        with pytest.raises(subprocess.TimeoutExpired):
            proc.wait(timeout=3)
        (repo / "ignored_directory" / "new_file").write_text("New stuff")
        with pytest.raises(subprocess.TimeoutExpired):
            proc.wait(timeout=3)


@contextmanager
def run_watchit(repo):
    with subprocess.Popen(
        [os.path.abspath("target/debug/watchit"), "-v"], cwd=repo
    ) as proc:
        # TODO: This is bad
        time.sleep(1)

        try:
            yield proc
        finally:
            proc.kill()
