from __future__ import annotations

import threading
import tkinter as tk
from pathlib import Path
from tkinter import ttk

from PIL import Image, ImageOps, ImageTk


def open_detail_popup(app, item) -> None:
    """谱面详情弹窗。"""
    if app.detail_popup and app.detail_popup.winfo_exists():
        app.detail_popup.destroy()

    popup = tk.Toplevel(app.root)
    popup.title("谱面详情")
    popup.transient(app.root)
    popup.configure(padx=12, pady=12, bg="#f8fafc")
    popup.minsize(860, 460)
    popup.protocol("WM_DELETE_WINDOW", popup.destroy)
    app.detail_popup = popup
    app.icon_manager.apply(popup)

    shell = ttk.Frame(popup)
    shell.pack(fill=tk.BOTH, expand=True)

    cover_frame = ttk.LabelFrame(shell, text="背景图", padding=6)
    cover_frame.pack(fill=tk.X)
    cover_label = ttk.Label(cover_frame, text="正在加载图片...", anchor=tk.CENTER, justify=tk.CENTER)
    cover_label.pack(fill=tk.BOTH, expand=True)

    info_frame = ttk.LabelFrame(shell, text="详细信息", padding=10)
    info_frame.pack(fill=tk.BOTH, expand=True, pady=(10, 0))
    info_frame.columnconfigure(0, weight=1)

    ttk.Label(
        info_frame,
        text=item.name_original or "-",
        style="PopupTitle.TLabel",
        wraplength=760,
        justify=tk.LEFT,
    ).grid(row=0, column=0, sticky="w", pady=(0, 10))

    detail_grid = ttk.Frame(info_frame)
    detail_grid.grid(row=1, column=0, sticky="nsew")
    for column in range(4):
        detail_grid.columnconfigure(column, weight=1, uniform="popup")

    fields = [
        ("名称", item.name or "-"),
        ("艺术家（原语言）", item.artist_unicode or item.artist or "-"),
        ("谱师", item.mapper or "-"),
        ("难度名", item.difficulty_name or "-"),
        ("模式", item.mode),
        ("状态", item.status_text or "-"),
        ("难度", item.star_rating_text or "-"),
        ("长度", item.length_text or "-"),
        ("BPM", item.bpm_text or "-"),
        ("Note数", item.note_count_text or "-"),
        ("BID", item.bid_text or "-"),
        ("SID", item.sid_text or "-"),
        ("CS", item.cs_text or "-"),
        ("AR", item.ar_text or "-"),
        ("OD", item.od_text or "-"),
        ("HP", item.hp_text or "-"),
        ("MD5", item.md5 or "-"),
    ]
    for index, (label, value) in enumerate(fields):
        row = index // 4
        column = index % 4
        columnspan = 4 if label == "MD5" else 1
        wraplength = 720 if label == "MD5" else 150
        field = ttk.Frame(detail_grid)
        field.grid(row=row, column=column, columnspan=columnspan, sticky="nsew", padx=(0, 10), pady=(0, 8))
        ttk.Label(field, text=label, style="PopupField.TLabel").pack(anchor=tk.W)
        ttk.Label(
            field,
            text=value,
            style="PopupValue.TLabel",
            wraplength=wraplength,
            justify=tk.LEFT,
        ).pack(anchor=tk.W, pady=(1, 0))

    popup.update_idletasks()
    popup_width = popup.winfo_width()
    popup_height = popup.winfo_height()
    x = app.root.winfo_rootx() + (app.root.winfo_width() - popup_width) // 2
    y = app.root.winfo_rooty() + (app.root.winfo_height() - popup_height) // 2 - 150
    popup.geometry(f"+{max(x, 0)}+{max(y, 0)}")

    if item.missing:
        cover_label.configure(text="该条目缺少本地谱面信息")
        return

    threading.Thread(
        target=_load_popup_cover,
        args=(app, item.beatmap_set_id, popup, cover_label),
        daemon=True,
    ).start()


def _load_popup_cover(app, beatmap_set_id: int | None, popup: tk.Toplevel, cover_label: ttk.Label) -> None:
    with app.cover_load_limiter:
        cover_path = app.cover_cache.get_cover_path(beatmap_set_id)
    app.root.after(0, lambda: _apply_popup_cover(popup, cover_label, cover_path))


def _apply_popup_cover(popup: tk.Toplevel, cover_label: ttk.Label, cover_path: Path | None) -> None:
    if not popup.winfo_exists():
        return
    if cover_path is None or not cover_path.exists():
        cover_label.configure(text="暂无可用图片")
        return

    image = Image.open(cover_path).convert("RGB")
    image = ImageOps.contain(image, (760, 240), Image.Resampling.LANCZOS)
    photo = ImageTk.PhotoImage(image)
    cover_label.configure(image=photo, text="")
    cover_label.image = photo
