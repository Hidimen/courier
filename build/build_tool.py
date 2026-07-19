#!/usr/bin/env python3
"""Courier build tool — discovers builtins and generates integration code.

Usage:
  py build/build_tool.py [generate]   Scan builtins/ and regenerate all output files
  py build/build_tool.py check        Verify generated files are up to date (CI use)

Discovery:
  Scans builtins/<name>/Cargo.toml for [package.metadata.carrier]:
    kind  = "bootloader" | "enhancement"
    entry = "crate::boot_fn"    e.g. "test_boot::boot"

  - bootloader → full integration: dep + feature + CLI enum + match arm
  - enhancement → dep + feature + re-export only (no CLI)

Outputs (regenerated from scratch each run):
  builtin/Cargo.toml           — features + deps sections
  waybill/Cargo.toml           — features sections (bootloaders only)
  courier/Cargo.toml           — features sections (bootloaders only)
  builtin/src/generated.rs     — conditional re-exports for all builtins
  waybill/src/commands/generated/bootloader.rs — enum + execute for bootloaders
"""

from __future__ import annotations

import re
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import List


# ---------------------------------------------------------------------------
# Data
# ---------------------------------------------------------------------------

ROOT = Path(__file__).resolve().parent.parent


@dataclass
class BuiltinModule:
    """A discovered builtin module."""
    name: str       # directory name, e.g. "test_boot"
    kind: str       # "bootloader" | "enhancement"
    entry: str      # e.g. "test_boot::boot"
    path: Path      # path to the crate directory


# ---------------------------------------------------------------------------
# Discovery
# ---------------------------------------------------------------------------

def discover() -> List[BuiltinModule]:
    """Walk builtins/ and return valid modules, sorted by name."""
    builtins_dir = ROOT / "builtins"
    if not builtins_dir.is_dir():
        print("warning: builtins/ directory not found", file=sys.stderr)
        return []

    modules: List[BuiltinModule] = []
    for entry in sorted(builtins_dir.iterdir()):
        if not entry.is_dir():
            continue

        cargo = entry / "Cargo.toml"
        if not cargo.exists():
            continue

        try:
            with open(cargo, "rb") as f:
                data = tomllib.load(f)
        except Exception as exc:
            print(f"warning: failed to parse {cargo}: {exc}", file=sys.stderr)
            continue

        carrier = data.get("package", {}).get("metadata", {}).get("carrier", {})
        kind = carrier.get("kind", "")
        entry_fn = carrier.get("entry", "")

        if not kind or not entry_fn:
            # Not a carrier-managed module — skip
            continue

        if kind not in ("bootloader", "enhancement"):
            print(f"warning: {entry.name}: unknown kind '{kind}'", file=sys.stderr)
            continue

        modules.append(BuiltinModule(name=entry.name, kind=kind, entry=entry_fn, path=entry))

    return modules


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def to_pascal_case(name: str) -> str:
    """Convert snake_case to PascalCase (e.g. test_boot → TestBoot)."""
    return "".join(w.capitalize() for w in name.split("_"))


def replace_markers(text: str, marker: str, content: str) -> str:
    """Replace everything between AUTOGEN:<marker> START and END lines.

    ``content`` is the new inner text (without trailing newline — the marker
    lines themselves are preserved).  Raises ValueError if markers are missing.
    """
    start_tag = f"# --- AUTOGEN:{marker} START ---"
    end_tag = f"# --- AUTOGEN:{marker} END ---"
    pattern = rf"({re.escape(start_tag)}\n).*?(\n{re.escape(end_tag)})"
    replacement = rf"\1{content}\2"
    result, count = re.subn(pattern, replacement, text, count=1, flags=re.DOTALL)
    if count == 0:
        raise ValueError(
            f"marker pair {marker!r} not found "
            f"(expected {start_tag} ... {end_tag})"
        )
    return result


def write_if_changed(path: Path, content: str) -> bool:
    """Write *content* to *path* only if it would differ.  Returns True when written."""
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        current = path.read_text(encoding="utf-8")
        if current == content:
            return False
    path.write_text(content, encoding="utf-8")
    print(f"  GEN  {path.relative_to(ROOT)}")
    return True


# ---------------------------------------------------------------------------
# Generators — every function returns a *string* for its target file/section.
# ---------------------------------------------------------------------------

def gen_cargo_features(header: str, entries: List[str]) -> str:
    """Build the inner content for an ``AUTOGEN:FEATURES`` section.

    *header* is a one-line comment shown above the entries.
    *entries* are "key = value" lines to place between the markers.
    """
    if not entries:
        return ""
    lines = [f"# Generated by build/build_tool.py -- DO NOT EDIT"]
    lines.extend(entries)
    return "\n".join(lines)


def builtin_features_content(modules: List[BuiltinModule]) -> str:
    """Features for builtin/Cargo.toml — every builtin gets one."""
    if not modules:
        return ""
    entries = [f'{m.name} = ["dep:{m.name}"]' for m in modules]
    return gen_cargo_features("Builtin modules", entries)


def builtin_deps_content(modules: List[BuiltinModule]) -> str:
    """Dependencies for builtin/Cargo.toml — every builtin gets one."""
    if not modules:
        return ""
    entries = [
        f'{m.name} = {{ path = "../builtins/{m.name}", optional = true }}'
        for m in modules
    ]
    return gen_cargo_features("Builtin modules", entries)


def waybill_features_content(bootloaders: List[BuiltinModule]) -> str:
    """Features for waybill/Cargo.toml — only bootloaders."""
    if not bootloaders:
        return ""
    entries = [f'{m.name} = ["builtin/{m.name}"]' for m in bootloaders]
    return gen_cargo_features("Bootloader modules", entries)


def courier_features_content(bootloaders: List[BuiltinModule]) -> str:
    """Features for courier/Cargo.toml — only bootloaders."""
    if not bootloaders:
        return ""
    entries = [
        f'{m.name} = ["builtin/{m.name}", "waybill/{m.name}"]'
        for m in bootloaders
    ]
    return gen_cargo_features("Bootloader modules", entries)


def generated_rs(modules: List[BuiltinModule]) -> str:
    """builtin/src/generated.rs — conditional re-exports for every builtin."""
    lines = [
        "// @generated by build/build_tool.py -- DO NOT EDIT",
        "",
    ]
    if not modules:
        lines.append("// No builtins discovered")
        lines.append("")
        return "\n".join(lines)

    for i, m in enumerate(modules):
        if i > 0:
            lines.append("")
        lines.append(f'#[cfg(feature = "{m.name}")]')
        lines.append("#[doc(inline)]")
        lines.append(f"pub use {m.name};")
    lines.append("")
    return "\n".join(lines)


def bootloader_rs(bootloaders: List[BuiltinModule]) -> str:
    """waybill/src/commands/generated/bootloader.rs — enum + execute."""
    lines = [
        "// @generated by build/build_tool.py -- DO NOT EDIT",
        "",
    ]
    if not bootloaders:
        lines.extend([
            "// No bootloaders configured — provide a stub to keep clap/ValueEnum happy.",
            "#[derive(ValueEnum, Clone)]",
            "enum BootloaderList {}",
            "",
            "impl StartArgs {",
            "    pub async fn execute(&self) {",
            "        // No bootloaders available",
            "    }",
            "}",
        ])
        return "\n".join(lines) + "\n"

    # --- Enum ---
    lines.extend([
        "#[derive(ValueEnum, Clone)]",
        "enum BootloaderList {",
    ])
    for m in bootloaders:
        lines.append(f'    #[cfg(feature = "{m.name}")]')
        lines.append(f"    {to_pascal_case(m.name)},")
    lines.append("}")
    lines.append("")

    # --- execute() ---
    lines.extend([
        "impl StartArgs {",
        "    pub async fn execute(&self) {",
        "        match self.bootloader {",
    ])
    for m in bootloaders:
        lines.append(f'            #[cfg(feature = "{m.name}")]')
        variant = to_pascal_case(m.name)
        lines.append(
            f"            BootloaderList::{variant}"
            f" => builtin::{m.name}::boot().await,"
        )
    lines.append("        }")
    lines.append("    }")
    lines.append("}")

    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# File paths
# ---------------------------------------------------------------------------

CARGO_BUILTIN = ROOT / "builtin" / "Cargo.toml"
CARGO_WAYBILL = ROOT / "waybill" / "Cargo.toml"
CARGO_COURIER = ROOT / "courier" / "Cargo.toml"
GENERATED_BUILTIN = ROOT / "builtin" / "src" / "generated.rs"
GENERATED_BOOTLOADER = ROOT / "waybill" / "src" / "commands" / "generated" / "bootloader.rs"


# ---------------------------------------------------------------------------
# Actions
# ---------------------------------------------------------------------------

def regenerate_all(modules: List[BuiltinModule]) -> None:
    """Regenerate every output file from scratch."""
    bootloaders = [m for m in modules if m.kind == "bootloader"]

    count = 0

    # 1. builtin/Cargo.toml
    text = CARGO_BUILTIN.read_text(encoding="utf-8")
    text = replace_markers(text, "FEATURES", builtin_features_content(modules))
    text = replace_markers(text, "DEPS", builtin_deps_content(modules))
    if write_if_changed(CARGO_BUILTIN, text):
        count += 1

    # 2. waybill/Cargo.toml
    text = CARGO_WAYBILL.read_text(encoding="utf-8")
    text = replace_markers(text, "FEATURES", waybill_features_content(bootloaders))
    if write_if_changed(CARGO_WAYBILL, text):
        count += 1

    # 3. courier/Cargo.toml
    text = CARGO_COURIER.read_text(encoding="utf-8")
    text = replace_markers(text, "FEATURES", courier_features_content(bootloaders))
    if write_if_changed(CARGO_COURIER, text):
        count += 1

    # 4. builtin/src/generated.rs
    if write_if_changed(GENERATED_BUILTIN, generated_rs(modules)):
        count += 1

    # 5. waybill/src/commands/generated/bootloader.rs
    if write_if_changed(GENERATED_BOOTLOADER, bootloader_rs(bootloaders)):
        count += 1

    print(f"\n{count} file(s) updated, {len(modules)} module(s) discovered")


def check(modules: List[BuiltinModule]) -> bool:
    """Verify all generated files are up to date.  Returns True when clean."""
    bootloaders = [m for m in modules if m.kind == "bootloader"]

    ok = True

    def verify(path: Path, expected: str) -> None:
        nonlocal ok
        rel = path.relative_to(ROOT)
        if not path.exists():
            print(f"  MISSING  {rel}")
            ok = False
        elif path.read_text(encoding="utf-8") != expected:
            print(f"  OUTDATED {rel}")
            ok = False

    # builtin/Cargo.toml
    text = CARGO_BUILTIN.read_text(encoding="utf-8")
    expected = replace_markers(text, "FEATURES", builtin_features_content(modules))
    expected = replace_markers(expected, "DEPS", builtin_deps_content(modules))
    verify(CARGO_BUILTIN, expected)

    # waybill/Cargo.toml
    text = CARGO_WAYBILL.read_text(encoding="utf-8")
    expected = replace_markers(text, "FEATURES", waybill_features_content(bootloaders))
    verify(CARGO_WAYBILL, expected)

    # courier/Cargo.toml
    text = CARGO_COURIER.read_text(encoding="utf-8")
    expected = replace_markers(text, "FEATURES", courier_features_content(bootloaders))
    verify(CARGO_COURIER, expected)

    # builtin/src/generated.rs
    verify(GENERATED_BUILTIN, generated_rs(modules))

    # waybill/src/commands/generated/bootloader.rs
    verify(GENERATED_BOOTLOADER, bootloader_rs(bootloaders))

    return ok


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> None:
    modules = discover()
    do_check = len(sys.argv) > 1 and sys.argv[1] == "check"

    if do_check:
        if check(modules):
            print("All generated files are up to date.")
            sys.exit(0)
        else:
            print("Run `py build/build_tool.py` to regenerate.", file=sys.stderr)
            sys.exit(1)
    else:
        regenerate_all(modules)


if __name__ == "__main__":
    main()
