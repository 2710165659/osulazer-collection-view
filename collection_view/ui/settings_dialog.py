from __future__ import annotations

import tkinter as tk
from tkinter import ttk

from .constants import COLUMN_DEFINITIONS


def open_settings_popup(app) -> None:
    """列设置弹窗从主控制器拆出来，便于单独维护。"""
    if app.settings_popup and app.settings_popup.winfo_exists():
        app.settings_popup.deiconify()
        app.settings_popup.lift()
        app.settings_popup.focus_force()
        position_settings_popup(app, app.settings_popup)
        return

    popup = tk.Toplevel(app.root)
    popup.title("列表设置")
    popup.transient(app.root)
    popup.resizable(False, False)
    popup.configure(padx=12, pady=12)
    popup.protocol("WM_DELETE_WINDOW", popup.destroy)
    app.settings_popup = popup
    app.icon_manager.apply(popup)

    ttk.Label(popup, text="谱面列表显示列", style="Section.TLabel").grid(row=0, column=0, columnspan=3, sticky="w")
    ttk.Label(popup, text="勾选后立即生效。", style="Muted.TLabel").grid(
        row=1,
        column=0,
        columnspan=3,
        sticky="w",
        pady=(2, 8),
    )

    for index, (key, header, _, _, _) in enumerate(COLUMN_DEFINITIONS):
        row = index // 3 + 2
        column = index % 3
        check = tk.Checkbutton(
            popup,
            text=header,
            variable=app.column_vars[key],
            command=app.on_column_setting_changed,
            anchor="w",
            padx=4,
            pady=2,
            selectcolor="white",
            font=("Segoe UI", 10),
        )
        check.grid(row=row, column=column, sticky="w", padx=(0, 18), pady=2)

    button_row = len(COLUMN_DEFINITIONS) // 3 + 3
    ttk.Button(popup, text="恢复默认列设置", command=app.reset_column_settings).grid(
        row=button_row,
        column=0,
        sticky="w",
        pady=(10, 0),
    )
    ttk.Button(popup, text="关闭", command=popup.destroy).grid(row=button_row, column=2, sticky="e", pady=(10, 0))

    position_settings_popup(app, popup)


def position_settings_popup(app, popup: tk.Toplevel) -> None:
    popup.update_idletasks()
    app.root.update_idletasks()
    button_x = app.view.settings_button.winfo_rootx()
    button_y = app.view.settings_button.winfo_rooty()
    button_height = app.view.settings_button.winfo_height()
    popup_width = popup.winfo_width()
    popup_height = popup.winfo_height()
    root_right = app.root.winfo_rootx() + app.root.winfo_width()
    root_bottom = app.root.winfo_rooty() + app.root.winfo_height()

    x = button_x + app.view.settings_button.winfo_width() - popup_width
    y = button_y + button_height + 6
    if x + popup_width > root_right - 12:
        x = root_right - popup_width - 12
    if y + popup_height > root_bottom - 12:
        y = button_y - popup_height - 6
    popup.geometry(f"+{max(x, 0)}+{max(y, 0)}")
