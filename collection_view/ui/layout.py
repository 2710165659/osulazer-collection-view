from __future__ import annotations

import tkinter as tk
from dataclasses import dataclass
from tkinter import ttk

from .constants import COLUMN_DEFINITIONS, MODE_DEFINITIONS


@dataclass(slots=True)
class MainView:
    main_panes: ttk.Panedwindow
    left_panel: ttk.Frame
    load_button: ttk.Button
    browse_button: ttk.Button
    export_current_button: ttk.Button
    export_all_button: ttk.Button
    collection_tree: ttk.Treeview
    beatmap_tree: ttk.Treeview
    preview_label: ttk.Label
    settings_button: tk.Button


def configure_styles(root: tk.Tk) -> None:
    """统一定义 ttk 风格。"""
    style = ttk.Style(root)
    style.theme_use("clam")
    style.configure("Section.TLabel", font=("Segoe UI", 13, "bold"))
    style.configure("Muted.TLabel", foreground="#666666")
    style.configure("Status.TLabel", foreground="#1f4f82")
    style.configure("Path.TLabel", foreground="#334155")
    style.configure("PopupTitle.TLabel", font=("Segoe UI", 14, "bold"))
    style.configure("PopupField.TLabel", foreground="#64748b", font=("Segoe UI", 9))
    style.configure("PopupValue.TLabel", font=("Segoe UI", 10))
    style.configure("Treeview", font=("Segoe UI", 9), rowheight=22)
    style.configure("Treeview.Heading", font=("Segoe UI", 9, "bold"))


def build_main_view(app) -> MainView:
    """只负责搭建界面骨架，不直接操作业务数据。"""
    root = app.root
    root.title("osu! 收藏夹查看器")
    root.geometry("1540x940")
    root.minsize(1280, 800)
    configure_styles(root)

    container = ttk.Frame(root, padding=10)
    container.pack(fill=tk.BOTH, expand=True)

    top_bar = ttk.Frame(container)
    top_bar.pack(fill=tk.X, pady=(0, 8))

    info_panel = ttk.Frame(top_bar)
    info_panel.pack(side=tk.LEFT, fill=tk.X, expand=True)
    ttk.Label(
        info_panel,
        textvariable=app.status_var,
        style="Status.TLabel",
        wraplength=980,
        justify=tk.LEFT,
    ).pack(anchor=tk.W, fill=tk.X)
    ttk.Label(
        info_panel,
        textvariable=app.realm_path_var,
        style="Path.TLabel",
        wraplength=980,
        justify=tk.LEFT,
    ).pack(anchor=tk.W, fill=tk.X, pady=(4, 0))

    actions = ttk.Frame(top_bar)
    actions.pack(side=tk.RIGHT)
    browse_button = ttk.Button(actions, text="浏览 Realm", command=app.browse_realm_file)
    browse_button.pack(side=tk.LEFT, padx=(0, 8))
    load_button = ttk.Button(actions, text="加载", command=app.load_realm)
    load_button.pack(side=tk.LEFT)
    export_current_button = ttk.Button(actions, text="导出当前谱面列表", command=app.export_current_list)
    export_current_button.pack(side=tk.LEFT, padx=(8, 0))
    export_all_button = ttk.Button(actions, text="导出所有", command=app.export_all_modes)
    export_all_button.pack(side=tk.LEFT, padx=(8, 0))

    panes = ttk.Panedwindow(container, orient=tk.HORIZONTAL)
    panes.pack(fill=tk.BOTH, expand=True)

    left_panel = ttk.Frame(panes, padding=6, width=270)
    right_panel = ttk.Frame(panes, padding=6)
    panes.add(left_panel, weight=1)
    panes.add(right_panel, weight=6)

    collection_tree, preview_label = _build_collection_panel(app, left_panel)
    beatmap_tree, settings_button = _build_detail_panel(app, right_panel)

    return MainView(
        main_panes=panes,
        left_panel=left_panel,
        load_button=load_button,
        browse_button=browse_button,
        export_current_button=export_current_button,
        export_all_button=export_all_button,
        collection_tree=collection_tree,
        beatmap_tree=beatmap_tree,
        preview_label=preview_label,
        settings_button=settings_button,
    )


def _build_collection_panel(app, parent: ttk.Frame) -> tuple[ttk.Treeview, ttk.Label]:
    header_row = ttk.Frame(parent)
    header_row.pack(fill=tk.X)

    ttk.Label(header_row, text="收藏夹列表", style="Section.TLabel").pack(side=tk.LEFT, anchor=tk.W)

    mode_row = ttk.Frame(header_row)
    mode_row.pack(side=tk.LEFT, padx=(12, 0))
    for label, _, _ in MODE_DEFINITIONS:
        button = tk.Button(
            mode_row,
            text=label.upper() if label != "ctb" else "CTB",
            compound=tk.LEFT,
            relief=tk.SOLID,
            borderwidth=1,
            command=lambda value=label: app.set_mode(value),
            bg="#f8fafc",
            activebackground="#dbeafe",
            font=("Segoe UI", 9, "bold"),
            cursor="hand2",
            padx=7,
            pady=4,
        )
        button.pack(side=tk.LEFT, padx=(0, 6))
        app.mode_buttons[label] = button

    ttk.Label(parent, textvariable=app.collection_summary_var, style="Muted.TLabel").pack(anchor=tk.W, pady=(4, 6))

    tree_holder = ttk.Frame(parent, height=470)
    tree_holder.pack(fill=tk.X, expand=False)
    tree_holder.pack_propagate(False)

    tree_frame = ttk.Frame(tree_holder)
    tree_frame.pack(fill=tk.BOTH, expand=True)

    collection_tree = ttk.Treeview(tree_frame, columns=("name", "total", "visible"), show="headings", selectmode="browse")
    collection_tree.heading("name", text="收藏夹")
    collection_tree.heading("total", text="总数")
    collection_tree.heading("visible", text="当前模式")
    collection_tree.column("name", width=180, anchor=tk.W)
    collection_tree.column("total", width=48, anchor=tk.CENTER)
    collection_tree.column("visible", width=58, anchor=tk.CENTER)
    collection_tree.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
    collection_tree.bind("<<TreeviewSelect>>", app.on_collection_selected)

    scrollbar = ttk.Scrollbar(tree_frame, orient=tk.VERTICAL, command=collection_tree.yview)
    scrollbar.pack(side=tk.RIGHT, fill=tk.Y)
    collection_tree.configure(yscrollcommand=scrollbar.set)

    preview_frame = ttk.LabelFrame(parent, text="背景图展示", padding=6, height=150)
    preview_frame.pack(fill=tk.X, expand=False, pady=(8, 0))
    preview_frame.pack_propagate(False)
    preview_label = ttk.Label(
        preview_frame,
        text="选择谱面后在这里显示背景图",
        anchor=tk.CENTER,
        justify=tk.CENTER,
        wraplength=220,
    )
    preview_label.pack(fill=tk.BOTH, expand=True)
    preview_label.bind("<Configure>", app.on_preview_configure)
    return collection_tree, preview_label


def _build_detail_panel(app, parent: ttk.Frame) -> tuple[ttk.Treeview, tk.Button]:
    title_row = ttk.Frame(parent)
    title_row.pack(fill=tk.X, pady=(0, 6))
    ttk.Label(title_row, text="谱面列表", style="Section.TLabel").pack(side=tk.LEFT)

    settings_button = tk.Button(
        title_row,
        text="设置",
        font=("Segoe UI", 9, "bold"),
        relief=tk.FLAT,
        borderwidth=0,
        cursor="hand2",
        command=app.open_settings_popup,
        bg="#eff6ff",
        activebackground="#dbeafe",
        padx=7,
        pady=4,
    )
    settings_button.pack(side=tk.LEFT, padx=(8, 0))

    ttk.Label(parent, textvariable=app.item_summary_var, style="Muted.TLabel").pack(anchor=tk.W, pady=(0, 8))

    table_frame = ttk.Frame(parent)
    table_frame.pack(fill=tk.BOTH, expand=True)

    all_columns = [key for key, *_ in COLUMN_DEFINITIONS]
    beatmap_tree = ttk.Treeview(table_frame, columns=all_columns, show="headings", selectmode="browse")
    for key, header, _, width, anchor in COLUMN_DEFINITIONS:
        beatmap_tree.heading(key, text=header, command=lambda value=key: app.toggle_beatmap_sort(value))
        beatmap_tree.column(key, width=width, anchor=anchor)
    beatmap_tree.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
    beatmap_tree.bind("<<TreeviewSelect>>", app.on_beatmap_selected)
    beatmap_tree.bind("<Double-1>", app.on_beatmap_double_clicked)
    beatmap_tree.bind("<ButtonPress-1>", app.on_beatmap_tree_button_press, add="+")
    beatmap_tree.bind("<B1-Motion>", app.on_beatmap_tree_drag, add="+")
    beatmap_tree.bind("<ButtonRelease-1>", app.on_beatmap_tree_button_release, add="+")

    scrollbar = tk.Scrollbar(
        table_frame,
        orient=tk.VERTICAL,
        command=beatmap_tree.yview,
        width=14,
        relief=tk.FLAT,
        activebackground="#cbd5e1",
        bg="#e5e7eb",
        troughcolor="#f8fafc",
        highlightthickness=0,
        bd=0,
    )
    scrollbar.place(relx=1.0, x=-2, y=2, relheight=1.0, height=-4, anchor="ne")
    beatmap_tree.configure(yscrollcommand=scrollbar.set)
    return beatmap_tree, settings_button
