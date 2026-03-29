from __future__ import annotations

import tkinter as tk

# 模式按钮的显示文本、导出模式值和对应图标文件。
MODE_DEFINITIONS = (
    ("osu", "osu", "osu.png"),
    ("taiko", "taiko", "taiko.png"),
    ("ctb", "ctb", "ctb.png"),
    ("mania", "mania", "mania.png"),
)

# 谱面表格的所有列定义集中放在这里，方便主界面、设置弹窗和导出复用。
COLUMN_DEFINITIONS = (
    ("name_original", "名称（原语言）", True, 320, tk.W),
    ("star_rating", "难度", True, 70, tk.CENTER),
    ("bid", "BID", True, 80, tk.CENTER),
    ("artist", "艺术家（原语言）", True, 180, tk.W),
    ("difficulty_name", "难度名", True, 180, tk.W),
    ("mapper", "谱师", True, 140, tk.W),
    ("mode", "模式", True, 70, tk.CENTER),
    ("sid", "SID", False, 80, tk.CENTER),
    ("cs", "CS", False, 60, tk.CENTER),
    ("od", "OD", False, 60, tk.CENTER),
    ("ar", "AR", False, 60, tk.CENTER),
    ("hp", "HP", False, 60, tk.CENTER),
    ("note_count", "Note数", False, 80, tk.CENTER),
    ("length", "长度", False, 80, tk.CENTER),
    ("bpm", "BPM", False, 80, tk.CENTER),
    ("status", "状态", False, 90, tk.CENTER),
    ("name", "名称", False, 320, tk.W),
    ("md5", "MD5", False, 280, tk.W),
)

DEFAULT_COLUMN_VISIBILITY = {key: default for key, _, default, _, _ in COLUMN_DEFINITIONS}
DEFAULT_COLUMN_ORDER = [key for key, _, _, _, _ in COLUMN_DEFINITIONS]
