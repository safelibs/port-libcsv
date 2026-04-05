#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import textwrap
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable, Sequence

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_INVENTORY = ROOT / "downstream-apps.json"
DEFAULT_IMAGE_MANIFEST = ROOT / "target/downstream/image-manifest.json"
DEFAULT_IMAGE_TAG = "libcsv-downstream:dev"
DEFAULT_REPORT = ROOT / "target/downstream/smoke-results.json"
EXPECTED_VERIFIER_PHASES = [
    {"id": "check_downstream_matrix_smoke", "bounce_target": "impl_downstream_matrix_harness"},
    {"id": "check_downstream_matrix_senior", "bounce_target": "impl_downstream_matrix_harness"},
]


class HarnessError(RuntimeError):
    pass


@dataclass(frozen=True)
class App:
    data: dict[str, Any]

    @property
    def app_id(self) -> str:
        return self.data["id"]

    @property
    def display_name(self) -> str:
        return self.data["display_name"]

    @property
    def source(self) -> dict[str, Any]:
        return self.data["source"]

    @property
    def source_kind(self) -> str:
        return self.source["kind"]

    @property
    def build_dependencies(self) -> list[str]:
        return list(self.data.get("build_dependencies", []))

    @property
    def artifact_paths(self) -> dict[str, str]:
        return self.data["artifact_paths"]

    @property
    def runtime_probes(self) -> list[dict[str, Any]]:
        return list(self.data.get("runtime_probes", []))

    @property
    def script_path(self) -> Path:
        return ROOT / "downstream" / "apps" / f"{self.app_id}.sh"

    @property
    def distfile_path(self) -> Path:
        return ROOT / self.artifact_paths["distfile"]

    @property
    def source_dir(self) -> Path:
        return ROOT / self.artifact_paths["source_dir"]

    @property
    def build_dir(self) -> Path:
        return ROOT / self.artifact_paths["build_dir"]

    @property
    def install_root(self) -> Path:
        return ROOT / self.artifact_paths["install_root"]

    @property
    def log_dir(self) -> Path:
        return ROOT / self.artifact_paths["log_dir"]

    @property
    def in_image_install_root(self) -> str:
        return f"/opt/downstream/apps/{self.app_id}"


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def relpath(path: Path | str) -> str:
    value = Path(path)
    try:
        return str(value.relative_to(ROOT))
    except ValueError:
        return str(value)


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def run(
    cmd: Sequence[str],
    *,
    cwd: Path | None = None,
    env: dict[str, str] | None = None,
    capture_output: bool = False,
    check: bool = True,
    stdout: Any | None = None,
    stderr: Any | None = None,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        list(cmd),
        cwd=str(cwd) if cwd else None,
        env=env,
        check=check,
        capture_output=capture_output,
        text=True,
        stdout=stdout,
        stderr=stderr,
    )


def require_tool(name: str) -> None:
    if shutil.which(name) is None:
        raise HarnessError(f"required tool not found: {name}")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify_sha256(path: Path, expected: str, label: str) -> None:
    actual = sha256_file(path)
    if actual != expected:
        raise HarnessError(f"{label} checksum mismatch: expected {expected}, found {actual}")


def download(url: str, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(dir=destination.parent, delete=False) as tmp:
        tmp_path = Path(tmp.name)
    request = urllib.request.Request(url, headers={"User-Agent": "libcsv-downstream-matrix/1.0"})
    try:
        with urllib.request.urlopen(request) as response, tmp_path.open("wb") as handle:
            shutil.copyfileobj(response, handle)
        tmp_path.replace(destination)
    finally:
        if tmp_path.exists():
            tmp_path.unlink()


def copy_any(src: Path, dst: Path) -> None:
    if src.is_dir():
        shutil.copytree(src, dst, symlinks=True)
    else:
        shutil.copy2(src, dst)


def copy_tree_contents(src: Path, dst: Path) -> None:
    dst.mkdir(parents=True, exist_ok=True)
    for child in src.iterdir():
        copy_any(child, dst / child.name)


def extract_git_archive(archive_path: Path, destination: Path) -> None:
    with tempfile.TemporaryDirectory(prefix="libcsv-downstream-git-") as tmpdir:
        tmp_root = Path(tmpdir)
        run(["tar", "-xzf", str(archive_path), "-C", str(tmp_root)], cwd=ROOT)
        entries = list(tmp_root.iterdir())
        if len(entries) == 1 and entries[0].is_dir():
            source_root = entries[0]
            shutil.copytree(source_root, destination, symlinks=True)
            return
        copy_tree_contents(tmp_root, destination)


def load_inventory(path: Path) -> tuple[dict[str, Any], list[App]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    apps = [App(app) for app in data["applications"]]
    return data, apps


def load_dependents() -> dict[str, Any]:
    return json.loads((ROOT / "dependents.json").read_text(encoding="utf-8"))


def select_apps(apps: Sequence[App], only: str | None) -> list[App]:
    if only is None:
        return list(apps)
    for app in apps:
        if app.app_id == only:
            return [app]
    raise HarnessError(f"unknown app id for --only: {only}")


def validate_inventory(path: Path) -> tuple[dict[str, Any], list[App]]:
    manifest, apps = load_inventory(path)
    dependents = load_dependents()
    errors: list[str] = []

    if manifest.get("schema_version") != 1:
        errors.append("schema_version must be 1")

    if manifest.get("verifier_phases") != EXPECTED_VERIFIER_PHASES:
        errors.append("verifier_phases does not match the required matrix verifier topology")

    target_count = manifest.get("selection_policy", {}).get("target_application_count")
    if target_count != 12:
        errors.append(f"selection_policy.target_application_count must be 12, found {target_count!r}")
    if len(apps) != 12:
        errors.append(f"applications must contain exactly 12 entries, found {len(apps)}")

    app_ids = [app.app_id for app in apps]
    if len(app_ids) != len(set(app_ids)):
        errors.append("application ids must be unique")

    dependent_ids = [entry["source_package"] for entry in dependents.get("dependents", [])]
    preserved_ids = manifest.get("selection_policy", {}).get("direct_ubuntu_dependents_preserved", [])
    if preserved_ids != dependent_ids:
        errors.append(
            f"direct_ubuntu_dependents_preserved must equal dependents.json order {dependent_ids}, found {preserved_ids}"
        )

    mapped_ids = [
        entry["mapped_application_id"]
        for entry in manifest.get("dependents_mapping", {}).get("entries", [])
        if entry.get("direct_ubuntu_dependent")
    ]
    if mapped_ids != dependent_ids:
        errors.append(f"dependents_mapping direct entries must equal dependents.json order {dependent_ids}, found {mapped_ids}")

    for app in apps:
        if not app.script_path.exists():
            errors.append(f"missing downstream app script: {relpath(app.script_path)}")
        if not app.runtime_probes:
            errors.append(f"{app.app_id} must define at least one runtime probe")
        if app.source_kind not in {"dsc", "git"}:
            errors.append(f"{app.app_id} has unsupported source kind {app.source_kind!r}")
        if app.source_kind == "dsc":
            checksum = app.source.get("checksum")
            if not isinstance(checksum, str) or not checksum.startswith("sha256:"):
                errors.append(f"{app.app_id} dsc sources must declare a sha256 checksum")
            for support in app.source.get("supporting_files", []):
                if "name" not in support or "sha256" not in support:
                    errors.append(f"{app.app_id} supporting_files entries must contain name and sha256")
        if not str(app.source_dir).endswith(app.app_id):
            errors.append(f"{app.app_id} source_dir must end with the application id")
        if not str(app.install_root).endswith(app.app_id):
            errors.append(f"{app.app_id} install_root must end with the application id")
        if not str(app.log_dir).endswith(app.app_id):
            errors.append(f"{app.app_id} log_dir must end with the application id")

    if errors:
        raise HarnessError("inventory validation failed:\n" + "\n".join(f"- {item}" for item in errors))

    return manifest, apps


def expected_libcsv_version() -> str:
    result = run(
        ["dpkg-parsechangelog", "-l", str(ROOT / "safe" / "debian" / "changelog"), "-S", "Version"],
        cwd=ROOT,
        capture_output=True,
    )
    return result.stdout.strip()


def host_multiarch() -> str:
    result = run(["dpkg-architecture", "-qDEB_HOST_MULTIARCH"], cwd=ROOT, capture_output=True)
    return result.stdout.strip()


def locate_package(pattern: str) -> Path | None:
    matches = sorted(ROOT.glob(pattern))
    return matches[0] if matches else None


def ensure_package_artifacts() -> dict[str, Any]:
    version = expected_libcsv_version()
    runtime_deb = locate_package(f"libcsv3_{version}_*.deb")
    dev_deb = locate_package(f"libcsv-dev_{version}_*.deb")

    reused = runtime_deb is not None and dev_deb is not None
    if not reused:
        require_tool("dpkg-buildpackage")
        run(["dpkg-buildpackage", "-us", "-uc", "-b"], cwd=ROOT / "safe")
        runtime_deb = locate_package(f"libcsv3_{version}_*.deb")
        dev_deb = locate_package(f"libcsv-dev_{version}_*.deb")

    if runtime_deb is None or dev_deb is None:
        raise HarnessError(f"failed to locate local libcsv Debian packages for version {version}")

    dbgsym = locate_package(f"libcsv3-dbgsym_{version}_*.ddeb")
    changes = locate_package(f"libcsv_{version}_*.changes")
    buildinfo = locate_package(f"libcsv_{version}_*.buildinfo")
    return {
        "version": version,
        "reused_existing_artifacts": reused,
        "runtime_deb": runtime_deb,
        "dev_deb": dev_deb,
        "dbgsym": dbgsym,
        "changes": changes,
        "buildinfo": buildinfo,
    }


def fetch_dsc_source(app: App) -> None:
    require_tool("dpkg-source")
    source = app.source
    distfile = app.distfile_path
    distfile.parent.mkdir(parents=True, exist_ok=True)

    checksum = source["checksum"].split(":", 1)[1]
    if distfile.exists():
        try:
            verify_sha256(distfile, checksum, relpath(distfile))
        except HarnessError:
            distfile.unlink()
    if not distfile.exists():
        download(source["locator"], distfile)
        verify_sha256(distfile, checksum, relpath(distfile))

    base_url = source["locator"].rsplit("/", 1)[0]
    for support in source.get("supporting_files", []):
        support_path = distfile.parent / support["name"]
        if support_path.exists():
            try:
                verify_sha256(support_path, support["sha256"], relpath(support_path))
            except HarnessError:
                support_path.unlink()
        if not support_path.exists():
            download(f"{base_url}/{support['name']}", support_path)
            verify_sha256(support_path, support["sha256"], relpath(support_path))

    sentinel_path = app.source_dir / ".downstream-source.json"
    sentinel = {
        "app_id": app.app_id,
        "kind": app.source_kind,
        "locator": source["locator"],
        "ref": source.get("ref"),
        "distfile": relpath(distfile),
    }
    if sentinel_path.exists():
        current = json.loads(sentinel_path.read_text(encoding="utf-8"))
        if current == sentinel and app.source_dir.exists():
            return

    if app.source_dir.exists():
        shutil.rmtree(app.source_dir)
    app.source_dir.parent.mkdir(parents=True, exist_ok=True)
    run(["dpkg-source", "-x", str(distfile), str(app.source_dir)], cwd=ROOT)
    write_json(sentinel_path, sentinel)


def derive_git_archive_url(locator: str, ref: str) -> str:
    if locator.endswith(".git"):
        locator = locator[:-4]
    return f"{locator}/archive/{ref}.tar.gz"


def fetch_git_source(app: App) -> None:
    require_tool("tar")
    source = app.source
    distfile = app.distfile_path
    distfile.parent.mkdir(parents=True, exist_ok=True)

    if not distfile.exists():
        download(derive_git_archive_url(source["locator"], source["ref"]), distfile)

    sentinel_path = app.source_dir / ".downstream-source.json"
    sentinel = {
        "app_id": app.app_id,
        "kind": app.source_kind,
        "locator": source["locator"],
        "ref": source["ref"],
        "distfile": relpath(distfile),
    }
    if sentinel_path.exists():
        current = json.loads(sentinel_path.read_text(encoding="utf-8"))
        if current == sentinel and app.source_dir.exists():
            return

    if app.source_dir.exists():
        shutil.rmtree(app.source_dir)
    app.source_dir.parent.mkdir(parents=True, exist_ok=True)
    extract_git_archive(distfile, app.source_dir)
    write_json(sentinel_path, sentinel)


def fetch_sources(path: Path, only: str | None = None) -> list[App]:
    _, apps = validate_inventory(path)
    selected = select_apps(apps, only)
    for app in selected:
        if app.source_kind == "dsc":
            fetch_dsc_source(app)
        elif app.source_kind == "git":
            fetch_git_source(app)
        else:
            raise HarnessError(f"unsupported source kind for {app.app_id}: {app.source_kind}")
    return selected


def image_stage_dir() -> Path:
    return ROOT / "target" / "downstream" / "image-build"


def prepare_image_context(packages: dict[str, Any]) -> Path:
    stage_dir = image_stage_dir()
    packages_dir = stage_dir / "packages"
    if packages_dir.exists():
        shutil.rmtree(packages_dir)
    packages_dir.mkdir(parents=True, exist_ok=True)

    for key in ("runtime_deb", "dev_deb"):
        src = packages[key]
        shutil.copy2(src, packages_dir / src.name)

    return packages_dir


def image_tag_with_suffix(image_tag: str, suffix: str) -> str:
    if ":" in image_tag:
        repository, tag = image_tag.rsplit(":", 1)
        return f"{repository}:{tag}-{suffix}"
    return f"{image_tag}:{suffix}"


def docker_build(target: str, tag: str) -> None:
    require_tool("docker")
    run(
        [
            "docker",
            "build",
            "--target",
            target,
            "-t",
            tag,
            "-f",
            str(ROOT / "docker" / "downstream-matrix.Dockerfile"),
            str(ROOT),
        ],
        cwd=ROOT,
    )


def build_apps_in_container(builder_tag: str, apps: Sequence[App]) -> None:
    uid = os.getuid()
    gid = os.getgid()
    repo_mount = f"{ROOT}:/work"
    script_lines = [
        "set -euo pipefail",
        "mkdir -p /tmp/downstream-home",
        "export DOWNSTREAM_REPO_ROOT=/work",
        "export DOWNSTREAM_TARGET_ROOT=/work/target/downstream",
    ]
    for app in apps:
        script_lines.append(f"/work/downstream/apps/{app.app_id}.sh build")

    run(
        [
            "docker",
            "run",
            "--rm",
            "--user",
            f"{uid}:{gid}",
            "-e",
            "HOME=/tmp/downstream-home",
            "-v",
            repo_mount,
            "-w",
            "/work",
            builder_tag,
            "bash",
            "-lc",
            "\n".join(script_lines),
        ],
        cwd=ROOT,
    )


def build_image(
    inventory_path: Path,
    image_tag: str,
    image_manifest_path: Path,
) -> dict[str, Any]:
    manifest, apps = validate_inventory(inventory_path)
    fetch_sources(inventory_path)

    packages = ensure_package_artifacts()
    multiarch = host_multiarch()
    prepare_image_context(packages)

    builder_tag = image_tag_with_suffix(image_tag, "builder")
    docker_build("with-local-libcsv", builder_tag)
    build_apps_in_container(builder_tag, apps)

    for app in apps:
        if not app.install_root.exists():
            raise HarnessError(f"missing staged install root after build: {relpath(app.install_root)}")

    docker_build("prepared", image_tag)

    image_manifest = {
        "schema_version": 1,
        "generated_at": utc_now(),
        "inventory": relpath(inventory_path),
        "image_tag": image_tag,
        "builder_image_tag": builder_tag,
        "package_version": packages["version"],
        "packages": {
            "reused_existing_artifacts": packages["reused_existing_artifacts"],
            "runtime_deb": relpath(packages["runtime_deb"]),
            "dev_deb": relpath(packages["dev_deb"]),
            "dbgsym": relpath(packages["dbgsym"]) if packages["dbgsym"] else None,
            "changes": relpath(packages["changes"]) if packages["changes"] else None,
            "buildinfo": relpath(packages["buildinfo"]) if packages["buildinfo"] else None,
        },
        "system_install_layout": {
            "header": "/usr/include/csv.h",
            "runtime_library": f"/usr/lib/{multiarch}/libcsv.so.3.0.2",
            "runtime_symlink": f"/usr/lib/{multiarch}/libcsv.so.3",
            "development_symlink": f"/usr/lib/{multiarch}/libcsv.so",
        },
        "applications": [
            {
                "id": app.app_id,
                "display_name": app.display_name,
                "host_source_dir": relpath(app.source_dir),
                "host_build_dir": relpath(app.build_dir),
                "host_install_root": relpath(app.install_root),
                "host_log_dir": relpath(app.log_dir),
                "app_script": relpath(app.script_path),
                "in_image_install_root": app.in_image_install_root,
            }
            for app in apps
        ],
        "selection_policy": manifest["selection_policy"],
    }
    write_json(image_manifest_path, image_manifest)
    return image_manifest


def docker_image_exists(tag: str) -> bool:
    result = subprocess.run(
        ["docker", "image", "inspect", tag],
        cwd=str(ROOT),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    return result.returncode == 0


def image_manifest_matches(
    image_manifest_path: Path,
    apps: Sequence[App],
    image_tag: str,
) -> bool:
    if not image_manifest_path.exists() or not docker_image_exists(image_tag):
        return False
    try:
        data = json.loads(image_manifest_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return False
    if data.get("image_tag") != image_tag:
        return False
    manifest_ids = [entry["id"] for entry in data.get("applications", [])]
    if manifest_ids != [app.app_id for app in apps]:
        return False
    runtime_deb = data.get("packages", {}).get("runtime_deb")
    dev_deb = data.get("packages", {}).get("dev_deb")
    if not runtime_deb or not dev_deb:
        return False
    if not (ROOT / runtime_deb).exists() or not (ROOT / dev_deb).exists():
        return False
    return True


def ensure_prepared_image(
    inventory_path: Path,
    image_tag: str,
    image_manifest_path: Path,
) -> dict[str, Any]:
    _, apps = validate_inventory(inventory_path)
    if image_manifest_matches(image_manifest_path, apps, image_tag):
        return json.loads(image_manifest_path.read_text(encoding="utf-8"))
    return build_image(inventory_path, image_tag, image_manifest_path)


def run_probe(image_tag: str, app: App) -> tuple[int, Path]:
    target_root = ROOT / "target" / "downstream"
    target_root.mkdir(parents=True, exist_ok=True)
    app.log_dir.mkdir(parents=True, exist_ok=True)
    driver_log = app.log_dir / "probe-driver.log"

    uid = os.getuid()
    gid = os.getgid()
    cmd = [
        "docker",
        "run",
        "--rm",
        "--user",
        f"{uid}:{gid}",
        "-e",
        "HOME=/tmp/downstream-home",
        "-e",
        "DOWNSTREAM_REPO_ROOT=/opt/downstream/harness",
        "-e",
        "DOWNSTREAM_HARNESS_ROOT=/opt/downstream/harness",
        "-e",
        "DOWNSTREAM_TARGET_ROOT=/mnt/downstream-target",
        "-e",
        "DOWNSTREAM_IMAGE_ROOT_BASE=/opt/downstream/apps",
        "-v",
        f"{target_root}:/mnt/downstream-target",
        image_tag,
        "bash",
        "-lc",
        f"mkdir -p /tmp/downstream-home && /opt/downstream/harness/downstream/apps/{app.app_id}.sh probe",
    ]
    with driver_log.open("w", encoding="utf-8") as handle:
        completed = subprocess.run(cmd, cwd=str(ROOT), stdout=handle, stderr=subprocess.STDOUT, check=False)
    return completed.returncode, driver_log


def build_report(
    inventory_path: Path,
    image_tag: str,
    image_manifest_path: Path,
    selected_apps: Sequence[App],
    keep_going: bool,
    results: list[dict[str, Any]],
) -> dict[str, Any]:
    passed = sum(1 for result in results if result["status"] == "passed")
    failed = sum(1 for result in results if result["status"] == "failed")
    skipped = sum(1 for result in results if result["status"] == "skipped")
    return {
        "schema_version": 1,
        "generated_at": utc_now(),
        "inventory": relpath(inventory_path),
        "image_tag": image_tag,
        "image_manifest": relpath(image_manifest_path),
        "keep_going": keep_going,
        "selected_app_ids": [app.app_id for app in selected_apps],
        "results": results,
        "summary": {
            "selected": len(selected_apps),
            "passed": passed,
            "failed": failed,
            "skipped": skipped,
        },
    }


def run_matrix(
    inventory_path: Path,
    image_tag: str,
    image_manifest_path: Path,
    report_path: Path,
    only: str | None,
    keep_going: bool,
) -> dict[str, Any]:
    _, apps = validate_inventory(inventory_path)
    selected_apps = select_apps(apps, only)
    ensure_prepared_image(inventory_path, image_tag, image_manifest_path)

    results: list[dict[str, Any]] = []
    failed = False

    try:
        for index, app in enumerate(selected_apps):
            started_at = datetime.now(timezone.utc)
            result: dict[str, Any] = {
                "app_id": app.app_id,
                "display_name": app.display_name,
                "status": "pending",
                "started_at": started_at.isoformat(timespec="seconds").replace("+00:00", "Z"),
                "host_log_dir": relpath(app.log_dir),
            }
            try:
                return_code, driver_log = run_probe(image_tag, app)
                result["probe_driver_log"] = relpath(driver_log)
                result["return_code"] = return_code
                if return_code != 0:
                    raise HarnessError(f"probe exited with status {return_code}")
                result["status"] = "passed"
            except Exception as exc:  # noqa: BLE001
                failed = True
                result["status"] = "failed"
                result["error"] = str(exc)
                if not keep_going:
                    results.append(finish_result(result, started_at))
                    for skipped_app in selected_apps[index + 1 :]:
                        results.append(
                            {
                                "app_id": skipped_app.app_id,
                                "display_name": skipped_app.display_name,
                                "status": "skipped",
                                "error": f"skipped after earlier failure in {app.app_id}",
                                "host_log_dir": relpath(skipped_app.log_dir),
                            }
                        )
                    break
            results.append(finish_result(result, started_at))
    finally:
        report = build_report(inventory_path, image_tag, image_manifest_path, selected_apps, keep_going, results)
        write_json(report_path, report)

    if failed:
        raise HarnessError(f"one or more downstream probes failed; see {relpath(report_path)}")
    return report


def finish_result(result: dict[str, Any], started_at: datetime) -> dict[str, Any]:
    finished_at = datetime.now(timezone.utc)
    result["finished_at"] = finished_at.isoformat(timespec="seconds").replace("+00:00", "Z")
    result["duration_seconds"] = round((finished_at - started_at).total_seconds(), 3)
    return result


def extract_findings_app_ids(payload: dict[str, Any]) -> tuple[set[str], set[str]]:
    findings_obj = payload.get("findings", payload)
    if isinstance(findings_obj, dict):
        findings = findings_obj.get("findings", [])
    elif isinstance(findings_obj, list):
        findings = findings_obj
    else:
        findings = []

    open_statuses = {"open", "known", "accepted", "triaged", "investigating", "pending"}
    closed_statuses = {"closed", "resolved", "fixed", "done", "pass", "passed"}

    accounted: set[str] = set()
    open_ids: set[str] = set()
    for finding in findings:
        if not isinstance(finding, dict):
            continue
        ids: list[str] = []
        if "app_id" in finding and isinstance(finding["app_id"], str):
            ids.append(finding["app_id"])
        if "app_ids" in finding and isinstance(finding["app_ids"], list):
            ids.extend(item for item in finding["app_ids"] if isinstance(item, str))
        if "applications" in finding and isinstance(finding["applications"], list):
            ids.extend(item for item in finding["applications"] if isinstance(item, str))
        if not ids:
            continue
        status = str(finding.get("status", "open")).strip().lower()
        accounted.update(ids)
        if status in open_statuses or status not in closed_statuses:
            open_ids.update(ids)
    return open_ids, accounted


def assert_report(
    inventory_path: Path,
    report_path: Path,
    findings_path: Path | None,
    allow_open: bool,
    require_all_passed: bool,
) -> None:
    _, apps = validate_inventory(inventory_path)
    expected_ids = {app.app_id for app in apps}
    report = json.loads(report_path.read_text(encoding="utf-8"))
    results = report.get("results", [])
    if not isinstance(results, list) or not results:
        raise HarnessError("report must contain a non-empty results list")

    result_ids = [entry["app_id"] for entry in results if "app_id" in entry]
    if len(result_ids) != len(set(result_ids)):
        raise HarnessError("report contains duplicate app ids")
    unknown = set(result_ids) - expected_ids
    if unknown:
        raise HarnessError(f"report contains unknown app ids: {sorted(unknown)}")

    selected_ids = report.get("selected_app_ids", result_ids)
    if sorted(result_ids) != sorted(selected_ids):
        raise HarnessError("report selected_app_ids does not match results")

    failed_ids = sorted(entry["app_id"] for entry in results if entry.get("status") != "passed")
    if require_all_passed and failed_ids:
        raise HarnessError(f"report contains non-passing applications: {failed_ids}")

    if allow_open:
        if findings_path is None:
            raise HarnessError("--allow-open requires --findings")
        findings = json.loads(findings_path.read_text(encoding="utf-8"))
        open_ids, _ = extract_findings_app_ids(findings)
        missing = [app_id for app_id in failed_ids if app_id not in open_ids]
        if missing:
            raise HarnessError(f"non-passing applications are not accounted for as open findings: {missing}")

    if not require_all_passed and not allow_open and failed_ids:
        raise HarnessError(f"report contains non-passing applications: {failed_ids}")


def print_plan(inventory_path: Path) -> None:
    manifest, apps = validate_inventory(inventory_path)
    print(f"Inventory: {relpath(inventory_path)}")
    print(f"Applications: {len(apps)}")
    print(f"Image Tag: {DEFAULT_IMAGE_TAG}")
    print(f"Manifest Verifier Phases: {manifest['verifier_phases']}")
    print("Applications:")
    for app in apps:
        distfile_state = "present" if app.distfile_path.exists() else "missing"
        source_state = "present" if app.source_dir.exists() else "missing"
        install_state = "present" if app.install_root.exists() else "missing"
        print(
            f"  - {app.app_id}: source={app.source_kind} "
            f"distfile={relpath(app.distfile_path)} ({distfile_state}) "
            f"source_dir={relpath(app.source_dir)} ({source_state}) "
            f"install_root={relpath(app.install_root)} ({install_state})"
        )


def list_apps(inventory_path: Path) -> None:
    _, apps = validate_inventory(inventory_path)
    for app in apps:
        print(app.app_id)


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Structured downstream matrix harness for libcsv.")
    subparsers = parser.add_subparsers(dest="command", required=True)

    validate_parser = subparsers.add_parser("validate", help="Validate downstream-apps.json and local script layout.")
    validate_parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)

    fetch_parser = subparsers.add_parser("fetch-sources", help="Fetch and unpack pinned downstream sources.")
    fetch_parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)
    fetch_parser.add_argument("--only", type=str, default=None)

    build_image_parser = subparsers.add_parser("build-image", help="Build the prepared downstream matrix image.")
    build_image_parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)
    build_image_parser.add_argument("--image-tag", type=str, default=DEFAULT_IMAGE_TAG)
    build_image_parser.add_argument("--image-manifest", type=Path, default=DEFAULT_IMAGE_MANIFEST)

    plan_parser = subparsers.add_parser("plan", help="Print the current matrix plan and cache state.")
    plan_parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)

    run_parser = subparsers.add_parser("run", help="Run downstream probes and emit a machine-readable report.")
    run_parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)
    run_parser.add_argument("--image-tag", type=str, default=DEFAULT_IMAGE_TAG)
    run_parser.add_argument("--image-manifest", type=Path, default=DEFAULT_IMAGE_MANIFEST)
    run_parser.add_argument("--report", type=Path, default=DEFAULT_REPORT)
    run_parser.add_argument("--only", type=str, default=None)
    run_parser.add_argument("--keep-going", action="store_true")

    assert_parser = subparsers.add_parser("assert-report", help="Assert the downstream report against expectations.")
    assert_parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)
    assert_parser.add_argument("--report", type=Path, required=True)
    assert_parser.add_argument("--findings", type=Path, default=None)
    assert_parser.add_argument("--allow-open", action="store_true")
    assert_parser.add_argument("--require-all-passed", action="store_true")

    list_parser = subparsers.add_parser("list-apps", help="List application ids in inventory order.")
    list_parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)

    return parser.parse_args(argv)


def main(argv: Sequence[str]) -> int:
    args = parse_args(argv)
    try:
        if args.command == "validate":
            validate_inventory(args.inventory)
            print(f"validated {relpath(args.inventory)}")
            return 0
        if args.command == "fetch-sources":
            apps = fetch_sources(args.inventory, args.only)
            print(f"fetched {len(apps)} application source tree(s)")
            return 0
        if args.command == "build-image":
            manifest = build_image(args.inventory, args.image_tag, args.image_manifest)
            print(f"built image {manifest['image_tag']}")
            return 0
        if args.command == "plan":
            print_plan(args.inventory)
            return 0
        if args.command == "run":
            report = run_matrix(
                args.inventory,
                args.image_tag,
                args.image_manifest,
                args.report,
                args.only,
                args.keep_going,
            )
            print(f"wrote report to {relpath(args.report)}")
            return 0 if report["summary"]["failed"] == 0 else 1
        if args.command == "assert-report":
            assert_report(
                args.inventory,
                args.report,
                args.findings,
                args.allow_open,
                args.require_all_passed,
            )
            print(f"asserted {relpath(args.report)}")
            return 0
        if args.command == "list-apps":
            list_apps(args.inventory)
            return 0
        raise HarnessError(f"unsupported command {args.command!r}")
    except HarnessError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    except subprocess.CalledProcessError as exc:
        command = " ".join(exc.cmd) if isinstance(exc.cmd, list) else str(exc.cmd)
        print(f"error: command failed ({exc.returncode}): {command}", file=sys.stderr)
        return exc.returncode or 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
