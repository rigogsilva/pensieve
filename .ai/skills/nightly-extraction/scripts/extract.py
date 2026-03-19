#!/usr/bin/env python3
"""
Session transcript parser for nightly memory extraction.

Discovers Claude Code and Codex CLI session JSONL files, parses them into
clean conversation text (stripping tool results, system prompts, etc.),
and outputs the results for Claude to analyze.

This script does NOT call any AI API — Claude (running the skill) does
the analysis and saves memories via pensieve CLI.
"""

import argparse
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

WATERMARK_PATH = Path.home() / ".pensieve" / "extraction_watermark"
CLAUDE_PROJECTS_DIR = Path.home() / ".claude" / "projects"
CODEX_SESSIONS_DIR = Path.home() / ".codex" / "sessions"

# Skip sessions with fewer than this many user turns
MIN_USER_TURNS = 5
# Skip sessions with less than this much conversation text (bytes)
MIN_TEXT_BYTES = 5000


# ---------------------------------------------------------------------------
# Watermark
# ---------------------------------------------------------------------------


def read_watermark() -> datetime:
    """Read the last extraction timestamp. Returns epoch if no watermark."""
    if WATERMARK_PATH.exists():
        text = WATERMARK_PATH.read_text().strip()
        try:
            return datetime.fromisoformat(text)
        except ValueError:
            pass
    return datetime(2000, 1, 1, tzinfo=timezone.utc)


def write_watermark(ts: datetime) -> None:
    """Write the current timestamp as the watermark."""
    WATERMARK_PATH.parent.mkdir(parents=True, exist_ok=True)
    WATERMARK_PATH.write_text(ts.isoformat())


# ---------------------------------------------------------------------------
# Session discovery
# ---------------------------------------------------------------------------


def discover_claude_sessions(since: datetime) -> list[dict]:
    """Find Claude Code session JSONL files modified since the watermark."""
    sessions = []
    if not CLAUDE_PROJECTS_DIR.exists():
        return sessions

    for project_dir in CLAUDE_PROJECTS_DIR.iterdir():
        if not project_dir.is_dir():
            continue
        project_name = infer_project_from_claude_path(project_dir.name)
        for jsonl_file in project_dir.glob("*.jsonl"):
            mtime = datetime.fromtimestamp(jsonl_file.stat().st_mtime, tz=timezone.utc)
            if mtime > since:
                sessions.append({
                    "path": str(jsonl_file),
                    "project": project_name,
                    "source": "claude-code",
                    "modified": mtime.isoformat(),
                })
    return sessions


def discover_codex_sessions(since: datetime) -> list[dict]:
    """Find Codex CLI session JSONL files modified since the watermark."""
    sessions = []
    if not CODEX_SESSIONS_DIR.exists():
        return sessions

    for jsonl_file in CODEX_SESSIONS_DIR.rglob("rollout-*.jsonl"):
        mtime = datetime.fromtimestamp(jsonl_file.stat().st_mtime, tz=timezone.utc)
        if mtime > since:
            sessions.append({
                "path": str(jsonl_file),
                "project": None,
                "source": "codex",
                "modified": mtime.isoformat(),
            })
    return sessions


def infer_project_from_claude_path(encoded_path: str) -> str | None:
    """e.g. '-Users-rigo-Documents-Projects-pensieve' -> 'pensieve'"""
    parts = encoded_path.strip("-").split("-")
    return parts[-1].lower() if parts else None


def infer_project_from_cwd(cwd: str) -> str | None:
    """Extract project name from a working directory path."""
    return Path(cwd).name.lower() if cwd else None


# ---------------------------------------------------------------------------
# Session parsing
# ---------------------------------------------------------------------------


def parse_session(path: str, source: str) -> dict:
    """Parse a session JSONL file into clean conversation text."""
    if source == "claude-code":
        return parse_claude_session(path)
    return parse_codex_session(path)


def parse_claude_session(path: str) -> dict:
    """Parse Claude Code session JSONL."""
    turns = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue

            msg_type = obj.get("type")
            if msg_type not in ("user", "assistant"):
                continue

            message = obj.get("message", {})
            role = message.get("role", msg_type)
            content = message.get("content", "")
            text = _extract_text(content)
            if text:
                turns.append({"role": role, "text": text})

    return {"turns": turns, "project_override": None}


def parse_codex_session(path: str) -> dict:
    """Parse Codex CLI session JSONL."""
    turns = []
    project = None

    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue

            if obj.get("type") == "session_meta":
                cwd = obj.get("payload", {}).get("cwd", "")
                if cwd:
                    project = infer_project_from_cwd(cwd)

            elif obj.get("type") == "response_item":
                payload = obj.get("payload", {})
                role = payload.get("role", "")
                if role not in ("user", "assistant"):
                    continue
                content = payload.get("content", [])
                text = _extract_text(content)
                if text:
                    turns.append({"role": role, "text": text})

    return {"turns": turns, "project_override": project}


def _extract_text(content) -> str:
    """Extract clean text from a message content field (string or array)."""
    if isinstance(content, str):
        if "<system-reminder>" in content or "<command-name>" in content:
            return ""
        return content.strip()

    if isinstance(content, list):
        texts = []
        for block in content:
            if not isinstance(block, dict):
                continue
            block_type = block.get("type", "")
            if block_type in ("text", "input_text"):
                text = block.get("text", "")
                if "<system-reminder>" in text or "<command-name>" in text:
                    continue
                texts.append(text)
            # Skip tool_use, tool_result, images, etc.
        return " ".join(texts).strip()

    return ""


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main():
    parser = argparse.ArgumentParser(
        description="Discover and parse agent session transcripts for memory extraction"
    )
    parser.add_argument("--since", type=str, help="Override watermark (YYYY-MM-DD or ISO datetime)")
    parser.add_argument("--limit", type=int, help="Process at most N sessions")
    parser.add_argument("--list", action="store_true", help="List sessions without parsing")
    parser.add_argument("--parse", type=str, help="Parse a single session file and output transcript")
    parser.add_argument("--update-watermark", action="store_true", help="Update watermark to now")
    args = parser.parse_args()

    # Single file parse mode
    if args.parse:
        path = args.parse
        source = "codex" if "rollout-" in path else "claude-code"
        result = parse_session(path, source)
        turns = result["turns"]
        user_turns = sum(1 for t in turns if t["role"] == "user")

        # Output as formatted transcript
        for turn in turns:
            role = turn["role"].upper()
            text = turn["text"]
            # Truncate very long turns for readability
            if len(text) > 2000:
                text = text[:2000] + "\n... [truncated]"
            print(f"{role}: {text}\n")

        print(f"---\nTurns: {len(turns)} ({user_turns} user, {len(turns) - user_turns} assistant)", file=sys.stderr)
        print(f"Size: {sum(len(t['text']) for t in turns)} bytes", file=sys.stderr)
        if result["project_override"]:
            print(f"Project (from session): {result['project_override']}", file=sys.stderr)
        return

    # Update watermark mode
    if args.update_watermark:
        write_watermark(datetime.now(timezone.utc))
        print(f"Watermark updated to: {datetime.now(timezone.utc).isoformat()}")
        return

    # Determine since timestamp
    if args.since:
        try:
            since = datetime.fromisoformat(args.since)
            if since.tzinfo is None:
                since = since.replace(tzinfo=timezone.utc)
        except ValueError:
            print(f"Error: Invalid --since format: {args.since}", file=sys.stderr)
            sys.exit(1)
    else:
        since = read_watermark()

    # Discover sessions
    sessions = discover_claude_sessions(since) + discover_codex_sessions(since)
    sessions.sort(key=lambda s: s["modified"])

    if args.limit:
        sessions = sessions[:args.limit]

    if args.list:
        # List mode — just show what's available
        for s in sessions:
            # Quick parse to get turn count
            result = parse_session(s["path"], s["source"])
            turns = result["turns"]
            user_turns = sum(1 for t in turns if t["role"] == "user")
            total_bytes = sum(len(t["text"]) for t in turns)
            project = result["project_override"] or s["project"] or "?"
            skip = user_turns < MIN_USER_TURNS or total_bytes < MIN_TEXT_BYTES
            status = "SKIP" if skip else "OK"
            print(f"[{status}] {s['source']:12s} | {project:20s} | {user_turns:3d} turns | {total_bytes:7d} bytes | {s['path']}")
        print(f"\nTotal: {len(sessions)} sessions, {sum(1 for s in sessions if True)} since {since.isoformat()}")
        return

    # Default: output JSON summary of all sessions for Claude to process
    output = {
        "since": since.isoformat(),
        "sessions": [],
    }
    for s in sessions:
        result = parse_session(s["path"], s["source"])
        turns = result["turns"]
        user_turns = sum(1 for t in turns if t["role"] == "user")
        total_bytes = sum(len(t["text"]) for t in turns)
        project = result["project_override"] or s["project"]
        skip = user_turns < MIN_USER_TURNS or total_bytes < MIN_TEXT_BYTES

        output["sessions"].append({
            "path": s["path"],
            "project": project,
            "source": s["source"],
            "modified": s["modified"],
            "user_turns": user_turns,
            "total_turns": len(turns),
            "total_bytes": total_bytes,
            "skip": skip,
            "skip_reason": "too_few_turns" if user_turns < MIN_USER_TURNS else "too_short" if total_bytes < MIN_TEXT_BYTES else None,
        })

    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
