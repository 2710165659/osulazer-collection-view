from __future__ import annotations

import json
import subprocess
from pathlib import Path

from .models import ExtractedData

REALM_FILENAME = "client.realm"
EXTRACTOR_EXE_NAME = "extractor.exe"
EXTRACTOR_DLL_NAME = "realm-wrappers.dll"


class RealmExtractorError(RuntimeError):
    """包装提取器阶段异常，方便 UI 统一展示。"""


def is_realm_file(path: Path | None) -> bool:
    """判断用户选择的路径是否是可用的 realm 文件。"""
    return bool(path and path.exists() and path.is_file() and path.suffix.lower() == ".realm")


class RealmExtractor:
    """调用已经编译好的 extractor.exe 并解析输出。"""

    def __init__(self, runtime_dir: Path, resource_dir: Path) -> None:
        self.runtime_dir = runtime_dir
        self.resource_dir = resource_dir
        self.extractor_dir = self.resource_dir / "extractor"
        self.exe_path = self.extractor_dir / EXTRACTOR_EXE_NAME
        self.runtime_dll_path = self.extractor_dir / EXTRACTOR_DLL_NAME
        self.output_path = runtime_dir / "extracted.json"

    def ensure_runtime(self) -> None:
        """执行前先检查 exe 和 dll 是否都存在。"""
        missing_files = [
            path.name
            for path in (self.exe_path, self.runtime_dll_path)
            if not path.exists()
        ]
        if missing_files:
            raise RealmExtractorError(f"提取器运行文件缺失：{', '.join(missing_files)}。")

    def extract(self, realm_path: Path) -> ExtractedData:
        """后台运行 extractor.exe，把生成的 JSON 恢复为数据模型。"""
        if not is_realm_file(realm_path):
            raise RealmExtractorError(f"无效的 realm 文件：{realm_path}")

        self.ensure_runtime()
        self.runtime_dir.mkdir(parents=True, exist_ok=True)

        result = subprocess.run(
            [str(self.exe_path), str(realm_path), str(self.output_path)],
            cwd=self.extractor_dir,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
        )
        if result.returncode != 0:
            raise RealmExtractorError(
                "提取器执行失败。\n"
                f"stdout:\n{result.stdout}\n"
                f"stderr:\n{result.stderr}"
            )

        try:
            payload = json.loads(self.output_path.read_text(encoding="utf-8"))
        except Exception as exc:  # noqa: BLE001
            raise RealmExtractorError(f"无法读取提取结果：{exc}") from exc

        return ExtractedData.from_dict(payload)
