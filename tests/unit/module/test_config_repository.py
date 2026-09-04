from __future__ import annotations

import copy
import os
import stat
import threading
from pathlib import Path

import pytest
from ruamel.yaml import YAML

from core.atomic_write import atomic_dump_yaml
from module.config.repository import ConfigConflictError, ConfigDeltaError, ConfigRepository


class FakeConfig:
    def __init__(self, path: Path, values: dict) -> None:
        self._lock = threading.RLock()
        self.config_path = path
        self.config = copy.deepcopy(values)
        self.yaml = YAML()
        self.save_calls = 0

    def save(self, *, instant: bool = False) -> None:
        del instant
        self.save_calls += 1
        atomic_dump_yaml(self.yaml, self.config_path, self.config)


def _make_values() -> dict:
    return {
        "last_auto_change": 1.0,
        "hard_mirror": False,
        "hard_mirror_chance": 0,
        "set_win_size": 1080,
        "teams": {
            "1": {"team_number": 1, "remark_name": "one"},
            "2": {"team_number": 2, "remark_name": "two"},
            "3": {"team_number": 3, "remark_name": "three"},
        },
        "teams_active_queue": [1, 2, 3],
        "teams_be_select": [True, True, True],
        "teams_order": [1, 2, 3],
        "teams_be_select_num": 3,
        "mirrorchyan_cdk": "secret-cdk",
        "nested": {"wxpusher_spt": "secret-token", "visible": 1},
    }


def _make_repository(tmp_path: Path) -> tuple[ConfigRepository, FakeConfig]:
    config = FakeConfig(tmp_path / "config.yaml", _make_values())
    config.save(instant=True)
    return ConfigRepository(config, temp_root=tmp_path / "runner-runs"), config


def _delta(manifest: dict, *, delta_id: str = "delta-1", **kwargs) -> dict:
    return {
        "type": "config.delta",
        "runId": manifest["runId"],
        "seq": kwargs.pop("seq", 1),
        "deltaId": delta_id,
        "baseRevision": manifest["baseRevision"],
        "baseConfigHash": manifest["baseConfigHash"],
        "changes": kwargs.pop("changes", {}),
        "operations": kwargs.pop("operations", []),
        **kwargs,
    }


def test_snapshot_is_normalized_hashed_and_secret_free_by_default(tmp_path: Path) -> None:
    repository, _config = _make_repository(tmp_path)

    safe = repository.snapshot()

    assert "mirrorchyan_cdk" not in safe
    assert "wxpusher_spt" not in safe["nested"]
    assert safe["nested"]["visible"] == 1
    assert repository.authoritative_snapshot()["mirrorchyan_cdk"] == "secret-cdk"
    assert len(repository.canonical_sha256) == 64
    assert repository.revision == 0
    assert repository.canonical_hash == repository.canonical_sha256


def test_create_and_cleanup_run_config_uses_private_directory_and_file(tmp_path: Path) -> None:
    repository, _config = _make_repository(tmp_path)

    manifest = repository.create_run_config("run/private", temp_root=tmp_path / "runs")
    config_path = Path(manifest["configPath"])
    run_directory = config_path.parent
    loaded = YAML(typ="safe").load(config_path.read_text(encoding="utf-8"))

    assert config_path.is_absolute()
    assert run_directory.parent == (tmp_path / "runs").resolve()
    assert "mirrorchyan_cdk" not in loaded
    assert "wxpusher_spt" not in loaded["nested"]
    if os.name != "nt":
        assert stat.S_IMODE(run_directory.stat().st_mode) == 0o700
        assert stat.S_IMODE(config_path.stat().st_mode) == 0o600
    assert repository.cleanup_run_config(manifest) is True
    assert not run_directory.exists()
    assert repository.cleanup_run_config(manifest) is False


def test_delta_whitelist_cas_and_duplicate_are_atomic(tmp_path: Path) -> None:
    repository, config = _make_repository(tmp_path)
    manifest = repository.create_run_config("run-delta")
    delta = _delta(
        manifest,
        changes={"hard_mirror": True, "set_win_size": 1440},
    )

    result = repository.apply_delta(delta)

    assert result["status"] == "applied"
    assert result["applied"] == ["hard_mirror", "set_win_size"]
    assert config.config["hard_mirror"] is True
    assert config.config["set_win_size"] == 1440
    assert repository.revision == 1
    assert config.save_calls == 2  # initial fixture write + one atomic delta write

    duplicate = repository.apply_delta(delta)
    assert duplicate["status"] == "duplicate"
    assert duplicate["duplicate"] is True
    assert repository.revision == 1
    assert config.save_calls == 2

    with pytest.raises(ConfigDeltaError, match="白名单"):
        repository.apply_delta(_delta(manifest, delta_id="delta-unknown", changes={"simulator": True}))


def test_external_file_change_uses_field_baseline_cas_and_preserves_user_value(tmp_path: Path) -> None:
    repository, config = _make_repository(tmp_path)
    manifest = repository.create_run_config("run-conflict")
    external = copy.deepcopy(config.config)
    external["hard_mirror"] = True
    atomic_dump_yaml(YAML(), config.config_path, external)

    result = repository.apply_delta(
        _delta(
            manifest,
            changes={"hard_mirror": False, "hard_mirror_chance": 5},
        )
    )

    assert result["status"] == "applied"
    assert "hard_mirror" in result["conflicts"]
    assert "hard_mirror_chance" in result["applied"]
    assert config.config["hard_mirror"] is True
    assert config.config["hard_mirror_chance"] == 5
    assert repository.revision == 2  # external edit + accepted CAS merge


def test_external_edit_with_memory_edit_is_rejected_without_overwrite(tmp_path: Path) -> None:
    repository, config = _make_repository(tmp_path)
    manifest = repository.create_run_config("run-race")
    original_chance = config.config["hard_mirror_chance"]
    original_window_size = config.config["set_win_size"]
    external = copy.deepcopy(config.config)
    config.config["hard_mirror"] = True
    external["hard_mirror_chance"] = 9
    atomic_dump_yaml(YAML(), config.config_path, external)

    with pytest.raises(ConfigConflictError, match="内存修改和外部文件修改"):
        repository.apply_delta(_delta(manifest, changes={"set_win_size": 1440}))
    # Neither the external user's value nor the rejected Runner delta may be
    # silently copied into the in-memory facade during a CAS conflict.
    assert config.config["hard_mirror"] is True
    assert config.config["hard_mirror_chance"] == original_chance
    assert config.config["set_win_size"] == original_window_size
    assert config.save_calls == 1
    on_disk = YAML(typ="safe").load(config.config_path.read_text(encoding="utf-8"))
    assert on_disk["hard_mirror"] is False
    assert on_disk["hard_mirror_chance"] == 9
    assert on_disk["set_win_size"] == original_window_size


def test_saved_legacy_config_edit_advances_revision_without_false_external_conflict(tmp_path: Path) -> None:
    repository, config = _make_repository(tmp_path)
    config.config["hard_mirror"] = True
    config.save(instant=True)

    assert repository.revision == 1
    assert repository.authoritative_snapshot()["hard_mirror"] is True


def test_rotate_team_queue_uses_stable_id_and_rebuilds_legacy_fields(tmp_path: Path) -> None:
    repository, config = _make_repository(tmp_path)
    manifest = repository.create_run_config("run-queue")

    result = repository.apply_delta(
        _delta(
            manifest,
            operations=[{"op": "rotateTeamQueue", "completedTeamId": "team-1"}],
        )
    )

    assert result["applied"] == ["rotateTeamQueue"]
    assert config.config["teams_active_queue"] == [2, 3, 1]
    assert config.config["teams_be_select"] == [True, True, True]
    assert config.config["teams_order"] == [3, 1, 2]
    assert config.config["teams_be_select_num"] == 3


def test_rotate_team_queue_skips_missing_or_user_reordered_queue(tmp_path: Path) -> None:
    repository, config = _make_repository(tmp_path)
    missing_manifest = repository.create_run_config("run-missing")
    missing = repository.apply_delta(
        _delta(
            missing_manifest,
            operations=[{"op": "rotateTeamQueue", "completedTeamId": "team-99"}],
        )
    )
    assert missing["status"] == "accepted"
    assert missing["conflicts"] == ["rotateTeamQueue"]
    assert config.config["teams_active_queue"] == [1, 2, 3]

    reordered_repository, reordered_config = _make_repository(tmp_path / "reordered")
    reordered_manifest = reordered_repository.create_run_config("run-reordered")
    external = copy.deepcopy(reordered_config.config)
    external["teams_active_queue"] = [3, 1, 2]
    atomic_dump_yaml(YAML(), reordered_config.config_path, external)
    reordered = reordered_repository.apply_delta(
        _delta(
            reordered_manifest,
            operations=[{"op": "rotateTeamQueue", "completedTeamId": "team-1"}],
        )
    )
    assert reordered["conflicts"] == ["rotateTeamQueue"]
    assert reordered_config.config["teams_active_queue"] == [3, 1, 2]
