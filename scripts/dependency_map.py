#!/usr/bin/env python3
"""Generate internal dependency map and violations report for /src."""
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re
import subprocess
import sys
from typing import Iterable

ROOT_DIR = Path(__file__).resolve().parents[1]
SRC_DIR = ROOT_DIR / "src"
OUTPUT_DIR = ROOT_DIR / "docs" / "architecture" / "dependencies"
DOT_PATH = OUTPUT_DIR / "dependency-map.dot"
PNG_PATH = OUTPUT_DIR / "dependency-map.png"
VIOLATIONS_PATH = OUTPUT_DIR / "dependency-violations.md"

LAYER_ORDER = {
    "stream": 0,
    "providers": 1,
    "types": 2,
    "utils": 3,
}

COLOR_OK = "#2e7d32"
COLOR_VIOLATION = "#c62828"
COLOR_NA = "#616161"

COMMENT_RE = re.compile(r"(?s)/\*.*?\*/|//.*?$", re.M)
USE_RE = re.compile(r"(?ms)^\s*(?:pub\s+)?use\s+[^;]+;")
IDENT_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


@dataclass
class EdgeEvidence:
    statements: set[str]
    files: set[Path]


@dataclass
class Edge:
    source: str
    target: str
    issue: str
    evidence: EdgeEvidence


def strip_comments(text: str) -> str:
    return re.sub(COMMENT_RE, "", text)


def normalize_use(stmt: str) -> str:
    stmt = stmt.strip()
    if stmt.startswith("pub "):
        stmt = stmt[4:]
    if stmt.startswith("use "):
        stmt = stmt[4:]
    stmt = stmt.rstrip(";").strip()
    stmt = re.sub(r"\s+as\s+[A-Za-z_][A-Za-z0-9_]*", "", stmt)
    return stmt


def compress(stmt: str) -> str:
    return " ".join(stmt.split())


def tokenize(text: str) -> list[str]:
    tokens: list[str] = []
    index = 0
    while index < len(text):
        char = text[index]
        if char.isspace():
            index += 1
            continue
        if text.startswith("::", index):
            tokens.append("::")
            index += 2
            continue
        if char in "{},":
            tokens.append(char)
            index += 1
            continue
        if char == "*":
            tokens.append("*")
            index += 1
            continue
        match = IDENT_RE.match(text[index:])
        if not match:
            index += 1
            continue
        tokens.append(match.group(0))
        index += len(match.group(0))
    return tokens


def parse_use_tree(tokens: list[str], prefix: list[str] | None = None, index: int = 0):
    if prefix is None:
        prefix = []
    results: list[list[str]] = []
    path: list[str] = []
    idx = index
    while idx < len(tokens):
        token = tokens[idx]
        if token == ",":
            if path:
                results.append(prefix + path)
                path = []
            idx += 1
            continue
        if token == "}":
            if path:
                results.append(prefix + path)
            return results, idx + 1
        if token == "{":
            nested_results, idx = parse_use_tree(tokens, prefix + path, idx + 1)
            results.extend(nested_results)
            path = []
            continue
        if token == "::":
            idx += 1
            continue
        path.append(token)
        idx += 1
    if path:
        results.append(prefix + path)
    return results, idx


def resolve_path(segments: list[str], current: list[str]) -> list[str] | None:
    if not segments:
        return None
    if segments[0] == "crate":
        return segments[1:]
    resolved = segments[:]
    base = current[:]
    while resolved and resolved[0] == "super":
        if base:
            base = base[:-1]
        resolved = resolved[1:]
    if resolved and resolved[0] == "self":
        resolved = resolved[1:]
    return base + resolved


def strip_special(segments: list[str]) -> list[str]:
    while segments and segments[-1] in ("*", "self"):
        segments = segments[:-1]
    return segments


def match_module(segments: list[str], modules: set[str]) -> str | None:
    for end in range(len(segments), 0, -1):
        candidate = "::".join(segments[:end])
        if candidate in modules:
            return candidate
    return None


def top_level(module: str) -> str:
    return module.split("::", 1)[0]


def dependency_issue(source: str, target: str) -> str:
    source_top = top_level(source)
    target_top = top_level(target)
    if source_top not in LAYER_ORDER or target_top not in LAYER_ORDER:
        return "n/a"
    if LAYER_ORDER[source_top] <= LAYER_ORDER[target_top]:
        return "ok"
    return "violation"


def collect_modules(src_dir: Path) -> dict[Path, str]:
    modules: dict[Path, str] = {}
    for file in src_dir.rglob("*.rs"):
        rel = file.relative_to(src_dir)
        if rel.name == "lib.rs":
            continue
        if rel.name == "mod.rs":
            parts = rel.parent.parts
        else:
            parts = rel.with_suffix("").parts
        module = "::".join(parts)
        modules[file] = module
    return modules


def extract_edges(modules: dict[Path, str]) -> dict[tuple[str, str], EdgeEvidence]:
    module_set = set(modules.values())
    edges: dict[tuple[str, str], EdgeEvidence] = {}
    for file, module in modules.items():
        text = strip_comments(file.read_text())
        use_stmts = USE_RE.findall(text)
        current_segments = module.split("::") if module else []
        for stmt in use_stmts:
            tree = normalize_use(stmt)
            tokens = tokenize(tree)
            paths, _ = parse_use_tree(tokens)
            for path in paths:
                resolved = resolve_path(path, current_segments)
                if not resolved:
                    continue
                resolved = strip_special(resolved)
                if not resolved:
                    continue
                target = match_module(resolved, module_set)
                if not target or target == module:
                    continue
                key = (module, target)
                if key not in edges:
                    edges[key] = EdgeEvidence(statements=set(), files=set())
                edges[key].statements.add(compress(stmt))
                edges[key].files.add(file.relative_to(ROOT_DIR))
    return edges


def build_dot(nodes: Iterable[str], edges: list[Edge]) -> str:
    lines = [
        "digraph dependencies {",
        "  rankdir=LR;",
        "  node [shape=box, fontname=\"Helvetica\", fontsize=10];",
        "  edge [fontname=\"Helvetica\", fontsize=9, arrowsize=0.7];",
    ]

    for node in sorted(nodes):
        lines.append(f"  \"{node}\";")

    for edge in edges:
        if edge.issue == "violation":
            color = COLOR_VIOLATION
            label = "violation"
            penwidth = 2.0
        elif edge.issue == "ok":
            color = COLOR_OK
            label = "ok"
            penwidth = 1.2
        else:
            color = COLOR_NA
            label = "n/a"
            penwidth = 1.0
        lines.append(
            f"  \"{edge.source}\" -> \"{edge.target}\" "
            f"[color=\"{color}\", label=\"{label}\", penwidth={penwidth}];"
        )

    lines.extend(
        [
            "  subgraph cluster_legend {",
            "    label=\"Legend\";",
            "    fontsize=10;",
            "    legend [shape=plaintext, label=<",
            "      <table border=\"0\" cellborder=\"1\" cellspacing=\"0\" cellpadding=\"4\">",
            f"        <tr><td><font color=\"{COLOR_OK}\">ok</font></td><td>dependency follows intended layer order</td></tr>",
            f"        <tr><td><font color=\"{COLOR_VIOLATION}\">violation</font></td><td>lower layer depends on higher (stream → providers → types → utils)</td></tr>",
            f"        <tr><td><font color=\"{COLOR_NA}\">n/a</font></td><td>outside layer set</td></tr>",
            "      </table>",
            "    >];",
            "  }",
            "}",
        ]
    )
    return "\n".join(lines) + "\n"


def write_violations(edges: list[Edge]) -> bool:
    violations = [edge for edge in edges if edge.issue == "violation"]
    lines = [
        "# Dependency Violations",
        "",
        "Rule: `stream → providers → types → utils`.",
        "",
    ]
    if not violations:
        lines.append("No violations found.")
        VIOLATIONS_PATH.write_text("\n".join(lines) + "\n")
        return False

    lines.extend([
        "| From | To | Files | Evidence |",
        "| --- | --- | --- | --- |",
    ])

    for edge in violations:
        files = ", ".join(f"`{path}`" for path in sorted(edge.evidence.files))
        evidence = " | ".join(f"`{stmt}`" for stmt in sorted(edge.evidence.statements))
        lines.append(
            f"| `{edge.source}` | `{edge.target}` | {files} | {evidence} |"
        )

    VIOLATIONS_PATH.write_text("\n".join(lines) + "\n")
    return True


def main() -> None:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    modules = collect_modules(SRC_DIR)
    module_set = set(modules.values())

    raw_edges = extract_edges(modules)
    edges: list[Edge] = []

    for (source, target), evidence in sorted(raw_edges.items()):
        issue = dependency_issue(source, target)
        edges.append(Edge(source=source, target=target, issue=issue, evidence=evidence))

    dot = build_dot(module_set, edges)
    DOT_PATH.write_text(dot)

    subprocess.run(
        ["dot", "-Tpng", str(DOT_PATH), "-o", str(PNG_PATH)],
        check=True,
    )

    has_violations = write_violations(edges)

    print(f"Wrote {DOT_PATH}")
    print(f"Wrote {PNG_PATH}")
    print(f"Wrote {VIOLATIONS_PATH}")

    if has_violations:
        print("Dependency violations found.")
        sys.exit(1)


if __name__ == "__main__":
    main()
