from __future__ import annotations

import json
from pathlib import Path

from .constants import DEFAULT_COLUMN_ORDER, DEFAULT_COLUMN_VISIBILITY


class SettingsStore:
    """集中处理界面配置的读写。"""

    def __init__(self, path: Path) -> None:
        self.path = path

    def load_payload(self) -> dict[str, object]:
        if not self.path.exists():
            return {}
        try:
            payload = json.loads(self.path.read_text(encoding="utf-8"))
        except Exception:  # noqa: BLE001
            return {}
        return payload if isinstance(payload, dict) else {}

    def save_payload(self, payload: dict[str, object]) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")

    def load_column_visibility(self, payload: dict[str, object]) -> dict[str, bool]:
        defaults = dict(DEFAULT_COLUMN_VISIBILITY)
        stored = payload.get("visibleColumns", {})
        if isinstance(stored, dict):
            for key in defaults:
                if key in stored:
                    defaults[key] = bool(stored[key])
        return defaults

    def load_column_order(self, payload: dict[str, object]) -> list[str]:
        defaults = list(DEFAULT_COLUMN_ORDER)
        stored = payload.get("columnOrder", [])
        if not isinstance(stored, list):
            return defaults

        seen: set[str] = set()
        ordered: list[str] = []
        for key in stored:
            if key in defaults and key not in seen:
                ordered.append(key)
                seen.add(key)
        for key in defaults:
            if key not in seen:
                ordered.append(key)
        return ordered

    def load_realm_path(self, payload: dict[str, object]) -> str:
        value = payload.get("realmPath", "")
        return value if isinstance(value, str) else ""

    def build_payload(
        self,
        *,
        visible_columns: dict[str, bool],
        column_order: list[str],
        realm_path: str,
    ) -> dict[str, object]:
        return {
            "visibleColumns": visible_columns,
            "columnOrder": column_order,
            "realmPath": realm_path,
        }
