#!/usr/bin/env python3
"""Assign Wave applicants on StellarRoute issues (first applicant wins conflicts)."""
from __future__ import annotations

import json
import re
import subprocess
from dataclasses import dataclass

REPO = "StellarRoute/StellarRoute"
ISSUE_NUMBERS = [1686, 1687, 1688, 1689, 1690]


@dataclass
class Application:
    user: str
    created_at: str
    has_merged_pr: bool


def merged_pr_count(login: str) -> int:
    out = subprocess.check_output(
        [
            "gh",
            "search",
            "prs",
            "--author",
            login,
            "--repo",
            REPO,
            "--merged",
            "--limit",
            "1",
            "--json",
            "number",
        ],
        text=True,
    )
    try:
        return len(json.loads(out))
    except json.JSONDecodeError:
        return 0


def fetch_applications(issue: int) -> list[Application]:
    comments = json.loads(
        subprocess.check_output(
            ["gh", "api", f"repos/{REPO}/issues/{issue}/comments"],
            text=True,
        )
    )
    apps: list[Application] = []
    for c in comments:
        body = c.get("body", "")
        if "wave:application-id" not in body:
            continue
        m = re.search(r"@(\S+) has applied", body)
        user = m.group(1) if m else c["user"]["login"]
        apps.append(
            Application(
                user=user,
                created_at=c["created_at"],
                has_merged_pr=merged_pr_count(user) > 0,
            )
        )
    apps.sort(key=lambda a: a.created_at)
    return apps


def pick_winner(apps: list[Application]) -> Application | None:
    if not apps:
        return None
    contributors = [a for a in apps if a.has_merged_pr]
    if contributors:
        return contributors[0]
    return apps[0]


def main() -> None:
    for issue in ISSUE_NUMBERS:
        apps = fetch_applications(issue)
        if not apps:
            print(f"#{issue}: no applications")
            continue
        winner = pick_winner(apps)
        assert winner is not None
        subprocess.check_call(
            ["gh", "issue", "edit", str(issue), "-R", REPO, "--add-assignee", winner.user]
        )
        note = (
            f"✅ **Wave assignment:** @{winner.user} is assigned ("
            f"{'prior merged PR to this repo' if winner.has_merged_pr else 'first qualified application'}).\n\n"
            "Thanks for applying via Drips Wave — please open a draft PR within a few days so we can track progress."
        )
        subprocess.check_call(
            [
                "gh",
                "issue",
                "comment",
                str(issue),
                "-R",
                REPO,
                "--body",
                note,
            ]
        )
        for loser in apps:
            if loser.user == winner.user:
                continue
            subprocess.check_call(
                [
                    "gh",
                    "issue",
                    "comment",
                    str(issue),
                    "-R",
                    REPO,
                    "--body",
                    (
                        f"Hi @{loser.user} — this issue was assigned to @{winner.user} "
                        f"({'earlier application' if loser.created_at > winner.created_at else 'returning contributor'}). "
                        "You're welcome to apply to any other open **Stellar Wave** issue — we just posted #1691–#1740."
                    ),
                ]
            )
        print(f"#{issue}: assigned {winner.user}")


if __name__ == "__main__":
    main()