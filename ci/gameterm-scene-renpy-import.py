#!/usr/bin/env python3
"""Import a conservative Ren'Py script subset into a Scene Mode JSON file."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Import a small Ren'Py script subset into GameTerm Scene Mode JSON."
    )
    parser.add_argument("--source", required=True, help="Ren'Py .rpy source file")
    parser.add_argument("--output", required=True, help="Scene JSON output path")
    parser.add_argument(
        "--attribution",
        required=True,
        help="Attribution manifest output path",
    )
    parser.add_argument(
        "--asset-root",
        default=None,
        help="Optional Ren'Py game asset root; assets are recorded, not copied",
    )
    parser.add_argument(
        "--title",
        default="Ren'Py Demo Import",
        help="Generated Scene Mode title",
    )
    parser.add_argument(
        "--source-title",
        default="Ren'Py Demo",
        help="Original demo/source title metadata",
    )
    parser.add_argument(
        "--renpy-version",
        default="unknown",
        help="Ren'Py version used by the source material, if known",
    )
    return parser.parse_args()


def strip_comment(line: str) -> str:
    in_string = False
    escaped = False
    out = []
    for char in line:
        if escaped:
            out.append(char)
            escaped = False
            continue
        if char == "\\" and in_string:
            out.append(char)
            escaped = True
            continue
        if char == '"':
            in_string = not in_string
            out.append(char)
            continue
        if char == "#" and not in_string:
            break
        out.append(char)
    return "".join(out).rstrip()


def unquote(text: str) -> str:
    return bytes(text, "utf-8").decode("unicode_escape")


def state_value(raw: str) -> dict[str, object] | None:
    raw = raw.strip()
    if raw in {"True", "true"}:
        return {"Bool": True}
    if raw in {"False", "false"}:
        return {"Bool": False}
    if re.fullmatch(r"-?\d+", raw):
        return {"Number": int(raw)}
    quoted = re.fullmatch(r'"((?:[^"\\]|\\.)*)"', raw)
    if quoted:
        return {"Text": unquote(quoted.group(1))}
    return None


def set_variable(variables: dict[str, dict[str, object]], key: str, value: dict[str, object]) -> None:
    variables[key] = value


def parse_source(path: Path) -> tuple[list[dict[str, object]], list[dict[str, object]], dict[str, dict[str, object]], list[str], dict[str, int]]:
    dialogue: list[dict[str, object]] = []
    choices: list[dict[str, object]] = []
    variables: dict[str, dict[str, object]] = {}
    warnings: list[str] = []
    label_targets: dict[str, int] = {}
    current_label = "start"
    pending_choice: dict[str, object] | None = None

    for line_no, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        line = strip_comment(raw_line)
        stripped = line.strip()
        if not stripped:
            continue

        label = re.fullmatch(r"label\s+([A-Za-z_][A-Za-z0-9_]*)\s*:", stripped)
        if label:
            current_label = label.group(1)
            label_targets.setdefault(current_label, len(dialogue))
            pending_choice = None
            continue

        assignment = re.fullmatch(
            r"(?:default\s+|\$\s*)([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.+)",
            stripped,
        )
        if assignment:
            value = state_value(assignment.group(2))
            if value is None:
                warnings.append(
                    f"line {line_no}: unsupported assignment expression: {stripped}"
                )
            else:
                set_variable(variables, assignment.group(1), value)
            continue

        if stripped == "menu:" or re.fullmatch(r"menu\s+[A-Za-z_][A-Za-z0-9_]*\s*:", stripped):
            pending_choice = None
            continue

        choice = re.fullmatch(
            r'"((?:[^"\\]|\\.)*)"\s*(?:if\s+([A-Za-z_][A-Za-z0-9_]*))?\s*:',
            stripped,
        )
        if choice:
            pending_choice = {
                "label": unquote(choice.group(1)),
                "source_label": current_label,
                "guard": choice.group(2),
                "line": line_no,
            }
            continue

        jump = re.fullmatch(r"jump\s+([A-Za-z_][A-Za-z0-9_]*)", stripped)
        if jump:
            target_label = jump.group(1)
            if pending_choice is None:
                warnings.append(
                    f"line {line_no}: non-menu jump is recorded as source flow only: {target_label}"
                )
            else:
                pending_choice["target_label"] = target_label
                choices.append(pending_choice)
                pending_choice = None
            continue

        say = re.fullmatch(r'([A-Za-z_][A-Za-z0-9_]*)\s+"((?:[^"\\]|\\.)*)"', stripped)
        if say:
            dialogue.append(
                {
                    "speaker": say.group(1),
                    "text": unquote(say.group(2)),
                    "metadata": [["source_label", current_label], ["source_line", str(line_no)]],
                }
            )
            continue

        narration = re.fullmatch(r'"((?:[^"\\]|\\.)*)"', stripped)
        if narration:
            dialogue.append(
                {
                    "speaker": "Narrator",
                    "text": unquote(narration.group(1)),
                    "metadata": [["source_label", current_label], ["source_line", str(line_no)]],
                }
            )
            continue

        if stripped in {"return", "pass"}:
            continue

        warnings.append(f"line {line_no}: unsupported statement skipped: {stripped}")

    return dialogue, choices, variables, warnings, label_targets


def generated_choices(
    choices: list[dict[str, object]],
    label_targets: dict[str, int],
    warnings: list[str],
) -> list[dict[str, object]]:
    out = []
    for choice in choices:
        target_label = str(choice.get("target_label", "start"))
        target = label_targets.get(target_label)
        if target is None:
            warnings.append(
                f"line {choice['line']}: unknown jump target {target_label}; using first dialogue line"
            )
            target = 0
        generated = {
            "label": str(choice["label"]),
            "kind": {"AdvanceDialogue": {"target": target}},
            "policy": {
                "origin": "renpy_import",
                "risk": "state_change",
                "scope": "scene",
                "summary": f"Continue imported Ren'Py demo at label {target_label}",
            },
            "conditions": [],
        }
        guard = choice.get("guard")
        if guard:
            generated["conditions"].append(
                {"variable": str(guard), "equals": {"Bool": True}}
            )
        out.append(generated)
    return out


def scene_json(
    title: str,
    source_title: str,
    source_path: Path,
    dialogue: list[dict[str, object]],
    choices: list[dict[str, object]],
    variables: dict[str, dict[str, object]],
    warnings: list[str],
) -> dict[str, object]:
    scene_variables = [
        {"key": "source_engine", "value": {"Text": "renpy"}},
        {"key": "source_title", "value": {"Text": source_title}},
        {"key": "source_file", "value": {"Text": str(source_path)}},
    ]
    for key in sorted(variables):
        scene_variables.append({"key": key, "value": variables[key]})

    if warnings:
        scene_variables.append(
            {"key": "renpy_import_warnings", "value": {"Number": len(warnings)}}
        )

    if not dialogue:
        dialogue = [
            {
                "speaker": "Importer",
                "text": "No supported Ren'Py dialogue lines were found.",
                "metadata": [["source_engine", "renpy"]],
            }
        ]

    return {
        "title": title,
        "background": "workspace-map",
        "width": 16,
        "height": 9,
        "mode": {
            "mode_id": "renpy-demo",
            "label": "Ren'Py Demo",
            "description": "Imported Ren'Py subset demo",
            "scene_profile": "scene",
            "allowed_actions": ["Inspect", "AdvanceDialogue"],
        },
        "variables": scene_variables,
        "entities": [
            {
                "id": "renpy-source",
                "kind": "Project",
                "label": source_title,
                "position": {"x": 2, "y": 2},
                "sprite": "project_core",
                "metadata": [
                    ["source_engine", "renpy"],
                    ["source_file", str(source_path)],
                ],
            },
            {
                "id": "renpy-narrator",
                "kind": "Agent",
                "label": "Narrator",
                "position": {"x": 7, "y": 4},
                "sprite": "agent_idle",
                "metadata": [["source_engine", "renpy"]],
            },
            {
                "id": "renpy-importer",
                "kind": "Task",
                "label": "Import Check",
                "position": {"x": 12, "y": 5},
                "sprite": "task_tile",
                "metadata": [["warnings", str(len(warnings))]],
            },
        ],
        "dialogue_speaker": dialogue[0]["speaker"],
        "dialogue": dialogue[0]["text"],
        "dialogue_lines": dialogue,
        "choices": choices,
    }


def attribution_json(
    source_path: Path,
    asset_root: str | None,
    source_title: str,
    renpy_version: str,
    warnings: list[str],
) -> dict[str, object]:
    return {
        "source": "renpy-demo-subset",
        "source_title": source_title,
        "renpy_version": renpy_version,
        "source_path": str(source_path),
        "asset_root": asset_root,
        "license_url": "https://www.renpy.org/doc/html/license.html",
        "assets": [],
        "notes": [
            "This importer records asset provenance but does not copy assets.",
            "The checked-in fixture source is GameTerm-authored Ren'Py-shaped test content.",
            "When importing upstream Ren'Py demo/tutorial material, preserve upstream credits and license files before vendoring assets.",
        ],
        "warnings": warnings,
    }


def write_json(path: Path, value: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    source_path = Path(args.source)
    if not source_path.is_file():
        print(f"Ren'Py source file not found: {source_path}", file=sys.stderr)
        return 1

    dialogue, parsed_choices, variables, warnings, label_targets = parse_source(source_path)
    choices = generated_choices(parsed_choices, label_targets, warnings)
    scene = scene_json(
        args.title,
        args.source_title,
        source_path,
        dialogue,
        choices,
        variables,
        warnings,
    )
    attribution = attribution_json(
        source_path,
        args.asset_root,
        args.source_title,
        args.renpy_version,
        warnings,
    )

    write_json(Path(args.output), scene)
    write_json(Path(args.attribution), attribution)

    for warning in warnings:
        print(f"WARN: {warning}", file=sys.stderr)
    print(f"Wrote Scene Mode Ren'Py import: {args.output}")
    print(f"Wrote Scene Mode Ren'Py attribution: {args.attribution}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
