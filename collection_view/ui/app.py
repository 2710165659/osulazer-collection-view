from __future__ import annotations

import ctypes
import sys
import threading
import tkinter as tk
from pathlib import Path
from tkinter import filedialog, messagebox

from PIL import Image, ImageOps, ImageTk

from ..cover_cache import CoverCache
from ..exporter import MODE_EXPORT_ORDER, export_all_modes_zip, export_current_view
from ..extractor import REALM_FILENAME, RealmExtractor, is_realm_file
from ..models import BeatmapEntry, CollectionInfo, ExtractedData
from .constants import COLUMN_DEFINITIONS, DEFAULT_COLUMN_ORDER, DEFAULT_COLUMN_VISIBILITY, MODE_DEFINITIONS
from .detail_popup import open_detail_popup
from .icon_manager import WindowIconManager
from .layout import MainView, build_main_view
from .settings_dialog import open_settings_popup
from .settings_store import SettingsStore


class CollectionViewApp:
    """主控制器负责管理状态、异步任务和界面刷新。"""

    def __init__(self, root: tk.Tk) -> None:
        self.root = root

        # 统一处理开发版和打包版的资源路径。
        source_dir = Path(__file__).resolve().parents[2]
        self.resource_dir = Path(getattr(sys, "_MEIPASS", source_dir)).resolve()
        self.base_dir = Path(sys.executable).resolve().parent if getattr(sys, "frozen", False) else source_dir
        self.runtime_dir = self.base_dir / "runtime"
        self.cover_dir = self.runtime_dir / "covers"
        self.mode_assets_dir = self.resource_dir / "assets" / "modes"
        self.icon_assets_dir = self.resource_dir / "assets" / "icons"

        self.settings_store = SettingsStore(self.runtime_dir / "ui_settings.json")
        self.settings_payload = self.settings_store.load_payload()

        self.extractor = RealmExtractor(self.runtime_dir, self.resource_dir)
        self.cover_cache = CoverCache(self.cover_dir)
        self.icon_manager = WindowIconManager(
            self.resource_dir / "assets" / "logo.png",
            self.resource_dir / "assets" / "logo.ico",
        )

        # 数据状态。
        self.data: ExtractedData | None = None
        self.collections: list[CollectionInfo] = []
        self.selected_collection: CollectionInfo | None = None
        self.selected_mode = "osu"
        self.selected_realm_path = self._load_initial_realm_path()
        self.displayed_items: list[BeatmapEntry] = []
        self.selected_item: BeatmapEntry | None = None

        # 控件与图片状态。
        self.mode_buttons: dict[str, tk.Button] = {}
        self.mode_photos: dict[str, ImageTk.PhotoImage] = {}
        self.settings_icon_photo: ImageTk.PhotoImage | None = None
        self.detail_popup: tk.Toplevel | None = None
        self.settings_popup: tk.Toplevel | None = None

        # 预览图异步状态。
        self.cover_load_limiter = threading.Semaphore(4)
        self.preview_photo: ImageTk.PhotoImage | None = None
        self.preview_cover_path: Path | None = None
        self.preview_request_id = 0
        self.preview_resize_job: str | None = None

        # 表格排序和拖拽状态。
        self.sort_column: str | None = None
        self.sort_descending = False
        self.dragged_heading_column: str | None = None
        self.dragged_heading_active = False
        self.drag_start_x = 0
        self.suppress_next_heading_sort = False

        # 用户列设置。
        self.column_visibility = self.settings_store.load_column_visibility(self.settings_payload)
        self.column_order = self.settings_store.load_column_order(self.settings_payload)
        self.column_vars = {
            key: tk.BooleanVar(value=self.column_visibility.get(key, default))
            for key, _, default, _, _ in COLUMN_DEFINITIONS
        }

        # 文本变量。
        self.status_var = tk.StringVar(value="")
        self.realm_path_var = tk.StringVar(value="")
        self.collection_summary_var = tk.StringVar(value="尚未加载数据。")
        self.item_summary_var = tk.StringVar(value="")

        self.view: MainView = build_main_view(self)
        self.collection_tree = self.view.collection_tree
        self.beatmap_tree = self.view.beatmap_tree
        self.preview_label = self.view.preview_label

        self.icon_manager.apply(self.root)
        self._load_mode_icons()
        self._load_settings_icon()
        self._refresh_realm_state()
        self._apply_mode_button_styles()
        self._apply_column_visibility()
        self._update_export_state()
        self.root.after(0, self._reset_main_pane_layout)

    def _load_initial_realm_path(self) -> Path | None:
        """优先恢复配置里的路径，最后再兜底到项目根目录的 client.realm。"""
        stored = self.settings_store.load_realm_path(self.settings_payload).strip()
        stored_path = Path(stored).resolve() if stored else None
        if is_realm_file(stored_path):
            return stored_path

        fallback = (self.base_dir / REALM_FILENAME).resolve()
        return fallback if is_realm_file(fallback) else None

    def _save_settings(self) -> None:
        """统一落盘列设置和上次使用的 realm 路径。"""
        payload = self.settings_store.build_payload(
            visible_columns={key: var.get() for key, var in self.column_vars.items()},
            column_order=list(self.column_order),
            realm_path=str(self.selected_realm_path) if self.selected_realm_path else "",
        )
        self.settings_payload = payload
        self.settings_store.save_payload(payload)

    def _refresh_realm_state(self) -> None:
        """刷新顶部的路径提示和加载按钮状态。"""
        if is_realm_file(self.selected_realm_path):
            self.status_var.set("已选择可用的 realm 文件，可以点击“加载”开始解析。")
            self.realm_path_var.set(f"当前数据库：{self.selected_realm_path}")
            self.view.load_button.configure(state=tk.NORMAL)
            return

        self.selected_realm_path = None
        self.status_var.set("请点击“浏览 Realm”选择 client.realm 或其他 .realm 文件。")
        self.realm_path_var.set("当前数据库：未选择")
        self.view.load_button.configure(state=tk.DISABLED)
        self._save_settings()

    def browse_realm_file(self) -> None:
        """通过文件对话框选择 realm 文件，并记住路径。"""
        initial_dir = str(self.selected_realm_path.parent) if self.selected_realm_path else str(self.base_dir)
        path = filedialog.askopenfilename(
            parent=self.root,
            title="选择 realm 数据库",
            filetypes=[("Realm 数据库", "*.realm")],
            initialdir=initial_dir,
        )
        if not path:
            return

        selected = Path(path).resolve()
        if not is_realm_file(selected):
            messagebox.showwarning("文件无效", "请选择一个存在的 .realm 文件。")
            return

        self.selected_realm_path = selected
        self._save_settings()
        self._refresh_realm_state()

    def _load_mode_icons(self) -> None:
        for label, _, filename in MODE_DEFINITIONS:
            image_path = self.mode_assets_dir / filename
            if not image_path.exists():
                continue
            image = Image.open(image_path).convert("RGBA")
            image = image.resize((18, 18), Image.Resampling.LANCZOS)
            photo = ImageTk.PhotoImage(image)
            self.mode_photos[label] = photo
            self.mode_buttons[label].configure(image=photo)

    def _load_settings_icon(self) -> None:
        image_path = self.icon_assets_dir / "settings.png"
        if not image_path.exists():
            return
        image = Image.open(image_path).convert("RGBA")
        image = image.resize((18, 18), Image.Resampling.LANCZOS)
        self.settings_icon_photo = ImageTk.PhotoImage(image)
        self.view.settings_button.configure(image=self.settings_icon_photo, text="")

    def _reset_main_pane_layout(self) -> None:
        total_width = self.view.main_panes.winfo_width()
        if total_width <= 0:
            self.root.after(50, self._reset_main_pane_layout)
            return

        requested = max(self.view.left_panel.winfo_reqwidth() + 18, 300)
        left_width = min(requested, max(total_width - 760, 300))
        self.view.main_panes.sashpos(0, left_width)

    def open_settings_popup(self) -> None:
        open_settings_popup(self)

    def on_column_setting_changed(self) -> None:
        self._save_settings()
        if self.sort_column and not self.column_vars[self.sort_column].get():
            self.sort_column = None
            self.sort_descending = False
        self._apply_column_visibility()
        self._refresh_beatmap_rows()

    def reset_column_settings(self) -> None:
        self.column_order = list(DEFAULT_COLUMN_ORDER)
        for key, var in self.column_vars.items():
            var.set(DEFAULT_COLUMN_VISIBILITY[key])
        self._save_settings()
        if self.sort_column and not self.column_vars[self.sort_column].get():
            self.sort_column = None
            self.sort_descending = False
        self._apply_column_visibility()
        self._refresh_beatmap_rows()

    def _visible_column_keys(self) -> list[str]:
        keys = [key for key in self.column_order if self.column_vars[key].get()]
        return keys or ["name_original"]

    def _apply_column_visibility(self) -> None:
        self.beatmap_tree.configure(displaycolumns=self._visible_column_keys())
        self._refresh_beatmap_headings()

    def _current_displayed_column_keys(self) -> list[str]:
        display_columns = self.beatmap_tree.cget("displaycolumns")
        if display_columns == "#all":
            return list(self.beatmap_tree.cget("columns"))
        return list(display_columns)

    def _update_load_state(self, is_loading: bool) -> None:
        if is_loading:
            self.view.browse_button.configure(state=tk.DISABLED)
            self.view.load_button.configure(state=tk.DISABLED)
            self.view.export_current_button.configure(state=tk.DISABLED)
            self.view.export_all_button.configure(state=tk.DISABLED)
            return

        self.view.browse_button.configure(state=tk.NORMAL)
        self._refresh_realm_state()
        self._update_export_state()

    def _update_export_state(self) -> None:
        has_current_items = bool(self.selected_collection and self.selected_collection.items_for_mode(self.selected_mode))
        has_any_data = bool(self.collections)
        self.view.export_current_button.configure(state=tk.NORMAL if has_current_items else tk.DISABLED)
        self.view.export_all_button.configure(state=tk.NORMAL if has_any_data else tk.DISABLED)

    def load_realm(self) -> None:
        if not is_realm_file(self.selected_realm_path):
            messagebox.showwarning("未选择数据库", "请先选择一个有效的 .realm 文件。")
            return

        realm_path = self.selected_realm_path
        self._update_load_state(is_loading=True)
        self.status_var.set("正在后台运行 extractor.exe 读取 realm，请稍候。")
        self.realm_path_var.set(f"当前数据库：{realm_path}")

        def worker() -> None:
            try:
                extracted = self.extractor.extract(realm_path)
            except Exception as exc:  # noqa: BLE001
                self.root.after(0, lambda message=str(exc): self._on_load_failed(message))
                return
            self.root.after(0, lambda data=extracted, path=realm_path: self._on_load_completed(data, path))

        threading.Thread(target=worker, daemon=True).start()

    def _on_load_failed(self, error_text: str) -> None:
        self._update_load_state(is_loading=False)
        messagebox.showerror("加载失败", error_text)

    def _on_load_completed(self, extracted: ExtractedData, realm_path: Path) -> None:
        self.data = extracted
        self.collections = extracted.collections
        self.selected_collection = None
        self.selected_realm_path = realm_path
        self._save_settings()
        self._update_load_state(is_loading=False)
        self.status_var.set(f"加载完成，当前读取：{realm_path.name}")
        self.realm_path_var.set(f"当前数据库：{realm_path}")
        self._refresh_collections()

    def set_mode(self, mode: str) -> None:
        self.selected_mode = mode
        self._apply_mode_button_styles()
        self._refresh_collections()

    def _apply_mode_button_styles(self) -> None:
        for mode, button in self.mode_buttons.items():
            selected = mode == self.selected_mode
            button.configure(
                bg="#dbeafe" if selected else "#f3f4f6",
                activebackground="#bfdbfe" if selected else "#e5e7eb",
                fg="#0f172a",
                relief=tk.SOLID,
                bd=2 if selected else 1,
            )

    def _collections_for_current_mode(self) -> list[CollectionInfo]:
        return [collection for collection in self.collections if collection.items_for_mode(self.selected_mode)]

    def _refresh_collections(self) -> None:
        visible_collections = self._collections_for_current_mode()
        self.collection_tree.delete(*self.collection_tree.get_children())

        for index, collection in enumerate(visible_collections):
            mode_items = collection.items_for_mode(self.selected_mode)
            self.collection_tree.insert("", tk.END, iid=f"collection-{index}", values=(collection.name, collection.total_count, len(mode_items)))

        self.collection_summary_var.set(
            f"当前模式: {self.selected_mode} | 收藏夹数: {len(visible_collections)} / {len(self.collections)}"
        )

        self.selected_collection = visible_collections[0] if visible_collections else None
        if self.selected_collection:
            self.collection_tree.selection_set("collection-0")
            self.collection_tree.focus("collection-0")
            self._refresh_beatmap_rows(select_first=True)
        else:
            self.selected_item = None
            self.displayed_items = []
            self._clear_preview()
            self._refresh_beatmap_rows()
        self._update_export_state()

    def on_collection_selected(self, _event=None) -> None:
        selected = self.collection_tree.selection()
        if not selected:
            return
        index = int(selected[0].split("-")[-1])
        visible_collections = self._collections_for_current_mode()
        if index < len(visible_collections):
            self.selected_collection = visible_collections[index]
            self.selected_item = None
            self._refresh_beatmap_rows(select_first=True)

    def _refresh_beatmap_rows(self, select_first: bool = False) -> None:
        self.beatmap_tree.delete(*self.beatmap_tree.get_children())

        if not self.selected_collection:
            self.displayed_items = []
            self.item_summary_var.set("")
            self._update_export_state()
            return

        source_items = self.selected_collection.items_for_mode(self.selected_mode)
        items = self._sorted_beatmap_items(source_items)
        self.displayed_items = items
        self.item_summary_var.set(
            f"收藏夹: {self.selected_collection.name} | 显示 {len(items)} 项 | 缺失 {sum(1 for item in items if item.missing)} 项"
        )

        if not items:
            self.selected_item = None
            self._clear_preview()
            self._update_export_state()
            return

        if select_first or self.selected_item not in items:
            self.selected_item = items[0]

        for index, item in enumerate(items):
            row_values = self._row_values_for_item(item)
            tags = ("missing",) if item.missing else ()
            self.beatmap_tree.insert(
                "",
                tk.END,
                iid=f"beatmap-{index}",
                values=[row_values[column] for column in self.beatmap_tree["columns"]],
                tags=tags,
            )
        self.beatmap_tree.tag_configure("missing", foreground="#9f1239")
        self._apply_column_visibility()

        if select_first:
            self.beatmap_tree.selection_set("beatmap-0")
            self.beatmap_tree.focus("beatmap-0")
        selected_index = items.index(self.selected_item) if self.selected_item in items else 0
        target_iid = f"beatmap-{selected_index}"
        self.beatmap_tree.selection_set(target_iid)
        self.beatmap_tree.focus(target_iid)
        self._center_beatmap_row(target_iid, selected_index, len(items))
        self._load_preview_for_item(self.selected_item)
        self._update_export_state()

    def _row_values_for_item(self, item: BeatmapEntry) -> dict[str, str]:
        return {
            "name_original": item.name_original,
            "star_rating": item.star_rating_text or "-",
            "bid": item.bid_text or "-",
            "sid": item.sid_text or "-",
            "difficulty_name": item.difficulty_name or "-",
            "mapper": item.mapper or "-",
            "mode": item.mode,
            "cs": item.cs_text or "-",
            "od": item.od_text or "-",
            "ar": item.ar_text or "-",
            "hp": item.hp_text or "-",
            "note_count": item.note_count_text or "-",
            "length": item.length_text or "-",
            "bpm": item.bpm_text or "-",
            "status": item.status_text or "-",
            "artist": item.artist_unicode or item.artist or "-",
            "name": item.name,
            "md5": item.md5 or "-",
        }

    def on_beatmap_selected(self, _event=None) -> None:
        if not self.selected_collection:
            return
        selected = self.beatmap_tree.selection()
        if not selected:
            return
        index = int(selected[0].split("-")[-1])
        if index < len(self.displayed_items):
            self.selected_item = self.displayed_items[index]
            self._load_preview_for_item(self.selected_item)

    def toggle_beatmap_sort(self, column: str) -> None:
        if self.suppress_next_heading_sort:
            self.suppress_next_heading_sort = False
            return
        if self.sort_column == column:
            if not self.sort_descending:
                self.sort_descending = True
            else:
                self.sort_column = None
                self.sort_descending = False
        else:
            self.sort_column = column
            self.sort_descending = False
        self._refresh_beatmap_headings()
        self._refresh_beatmap_rows()

    def _refresh_beatmap_headings(self) -> None:
        for key, header, _, _, _ in COLUMN_DEFINITIONS:
            text = header
            if key == self.sort_column:
                text = f"{header} {'(DESC)' if self.sort_descending else '(ASC)'}"
            self.beatmap_tree.heading(key, text=text, command=lambda value=key: self.toggle_beatmap_sort(value))

    def _sorted_beatmap_items(self, items: list[BeatmapEntry]) -> list[BeatmapEntry]:
        if not self.sort_column:
            return list(reversed(items))

        valued_items: list[tuple[object, BeatmapEntry]] = []
        empty_items: list[BeatmapEntry] = []
        for item in items:
            sort_value = self._sort_value_for_item(item, self.sort_column)
            if sort_value is None:
                empty_items.append(item)
                continue
            valued_items.append((sort_value, item))

        valued_items.sort(key=lambda pair: pair[0], reverse=self.sort_descending)
        return [item for _, item in valued_items] + empty_items

    def _sort_value_for_item(self, item: BeatmapEntry, column: str) -> object | None:
        numeric_fields = {
            "star_rating": item.star_rating,
            "bid": item.beatmap_id,
            "sid": item.beatmap_set_id,
            "cs": item.circle_size,
            "od": item.overall_difficulty,
            "ar": item.approach_rate,
            "hp": item.drain_rate,
            "note_count": item.total_object_count,
            "length": item.length_ms,
            "bpm": item.bpm,
            "status": item.status_int,
        }
        if column in numeric_fields:
            value = numeric_fields[column]
            return None if value is None else float(value)

        text_fields = {
            "name_original": item.name_original,
            "artist": item.artist_unicode or item.artist,
            "difficulty_name": item.difficulty_name,
            "mapper": item.mapper,
            "mode": item.mode,
            "name": item.name,
            "md5": item.md5,
        }
        value = text_fields.get(column)
        if value is None:
            return None
        normalized = value.strip().casefold()
        return normalized or None

    def on_beatmap_double_clicked(self, event=None) -> None:
        if event is not None:
            region = self.beatmap_tree.identify("region", event.x, event.y)
            if region != "cell":
                return
            row_id = self.beatmap_tree.identify_row(event.y)
            if not row_id:
                return
            index = int(row_id.split("-")[-1])
            if index < len(self.displayed_items):
                self.selected_item = self.displayed_items[index]
        if self.selected_item is not None:
            open_detail_popup(self, self.selected_item)

    def _center_beatmap_row(self, item_id: str, item_index: int, total_items: int) -> None:
        self.beatmap_tree.see(item_id)
        self.root.update_idletasks()
        bbox = self.beatmap_tree.bbox(item_id)
        if not bbox or total_items <= 1:
            return

        row_height = max(bbox[3], 1)
        visible_rows = max(self.beatmap_tree.winfo_height() // row_height, 1)
        max_first_index = max(total_items - visible_rows, 0)
        first_index = min(max(item_index - visible_rows // 2, 0), max_first_index)
        if max_first_index <= 0:
            self.beatmap_tree.yview_moveto(0)
            return
        self.beatmap_tree.yview_moveto(first_index / total_items)
        self.beatmap_tree.see(item_id)

    def on_beatmap_tree_button_press(self, event) -> None:
        if self.beatmap_tree.identify("region", event.x, event.y) != "heading":
            self.dragged_heading_column = None
            self.dragged_heading_active = False
            return
        self.dragged_heading_column = self._display_column_key_from_x(event.x)
        self.dragged_heading_active = False
        self.drag_start_x = event.x

    def on_beatmap_tree_drag(self, event) -> None:
        if self.dragged_heading_column is None:
            return
        if abs(event.x - self.drag_start_x) >= 8:
            self.dragged_heading_active = True

    def on_beatmap_tree_button_release(self, event) -> None:
        if self.dragged_heading_column is None:
            return
        source_column = self.dragged_heading_column
        target_column = self._display_column_key_from_x(event.x)
        was_dragging = self.dragged_heading_active
        self.dragged_heading_column = None
        self.dragged_heading_active = False
        if not was_dragging or not target_column or target_column == source_column:
            return
        self._reorder_visible_columns(source_column, target_column)
        self.suppress_next_heading_sort = True

    def _display_column_key_from_x(self, x: int) -> str | None:
        column_id = self.beatmap_tree.identify_column(x)
        if not column_id.startswith("#"):
            return None
        try:
            column_index = int(column_id[1:]) - 1
        except ValueError:
            return None
        visible_keys = self._visible_column_keys()
        if 0 <= column_index < len(visible_keys):
            return visible_keys[column_index]
        return None

    def _reorder_visible_columns(self, source_column: str, target_column: str) -> None:
        visible_keys = self._visible_column_keys()
        if source_column not in visible_keys or target_column not in visible_keys:
            return

        reordered_visible = list(visible_keys)
        source_index = reordered_visible.index(source_column)
        target_index = reordered_visible.index(target_column)
        reordered_visible.pop(source_index)
        if source_index < target_index:
            target_index -= 1
        reordered_visible.insert(target_index, source_column)

        hidden_keys = [key for key in self.column_order if key not in visible_keys]
        self.column_order = reordered_visible + hidden_keys
        self._save_settings()
        self._apply_column_visibility()

    def export_current_list(self) -> None:
        """导出当前选中收藏夹下、当前模式的可见谱面列表。"""
        if not self.selected_collection:
            messagebox.showinfo("无法导出", "请先选择一个收藏夹。")
            return

        items = list(self.displayed_items)
        if not items:
            messagebox.showinfo("无法导出", "当前没有可导出的谱面列表。")
            return

        visible_keys = self._current_displayed_column_keys()
        headers = [
            self.beatmap_tree.heading(key, option="text").replace(" (ASC)", "").replace(" (DESC)", "")
            for key in visible_keys
        ]
        rows = []
        for item in items:
            row_values = self._row_values_for_item(item)
            rows.append([row_values[key] for key in visible_keys])

        default_name = f"{self.selected_collection.name}_{self.selected_mode}.xlsx".replace("/", "_").replace("\\", "_")
        path = filedialog.asksaveasfilename(
            parent=self.root,
            title="导出当前谱面列表",
            defaultextension=".xlsx",
            filetypes=[("Excel 文件", "*.xlsx")],
            initialfile=default_name,
        )
        if not path:
            return

        try:
            export_current_view(Path(path), sheet_name=self.selected_collection.name, headers=headers, rows=rows)
        except Exception as exc:  # noqa: BLE001
            messagebox.showerror("导出失败", str(exc))
            return

        messagebox.showinfo("导出成功", f"已导出到：{path}")

    def _export_row_for_item(self, collection_name: str, item: BeatmapEntry, visible_keys: list[str]) -> list[str]:
        """把单个谱面转换成导出行，和当前列表共用同一套列映射。"""
        row_values = self._row_values_for_item(item)
        row_values["collection"] = collection_name
        row_values["missing"] = "Yes" if item.missing else "No"
        row_values["title"] = item.title or "-"
        row_values["background_url"] = item.background_url or "-"
        return [row_values.get(key, "-") for key in visible_keys]

    def export_all_modes(self) -> None:
        """导出所有收藏夹：四个模式各一个 Excel，再打成 zip。"""
        if not self.collections:
            messagebox.showinfo("无法导出", "请先加载数据。")
            return

        path = filedialog.asksaveasfilename(
            parent=self.root,
            title="导出所有模式",
            defaultextension=".zip",
            filetypes=[("ZIP 压缩包", "*.zip")],
            initialfile="all_modes_collections.zip",
        )
        if not path:
            return

        visible_keys = self._current_displayed_column_keys()
        headers = [
            self.beatmap_tree.heading(key, option="text").replace(" (ASC)", "").replace(" (DESC)", "")
            for key in visible_keys
        ]

        try:
            export_all_modes_zip(
                Path(path),
                self.collections,
                headers,
                lambda collection_name, item: self._export_row_for_item(collection_name, item, visible_keys),
            )
        except Exception as exc:  # noqa: BLE001
            messagebox.showerror("导出失败", str(exc))
            return

        messagebox.showinfo("导出成功", f"已导出 {'、'.join(MODE_EXPORT_ORDER)} 四个模式的压缩包：{path}")

    def _clear_preview(self, text: str = "选择谱面后在这里显示背景图") -> None:
        self.preview_request_id += 1
        self.preview_photo = None
        self.preview_cover_path = None
        self.preview_label.configure(image="", text=text)

    def on_preview_configure(self, _event=None) -> None:
        if self.preview_resize_job is not None:
            self.root.after_cancel(self.preview_resize_job)
        self.preview_resize_job = self.root.after(80, self._refresh_preview_layout)

    def _refresh_preview_layout(self) -> None:
        self.preview_resize_job = None
        if self.preview_cover_path and self.preview_cover_path.exists():
            self._render_preview_cover(self.preview_cover_path)

    def _load_preview_for_item(self, item: BeatmapEntry | None) -> None:
        request_id = self.preview_request_id + 1
        self.preview_request_id = request_id

        if item is None:
            self._clear_preview()
            return
        if item.missing:
            self.preview_photo = None
            self.preview_label.configure(image="", text="该条目缺少本地谱面信息")
            return

        self.preview_label.configure(image="", text="正在加载背景图...")

        def worker() -> None:
            with self.cover_load_limiter:
                cover_path = self.cover_cache.get_cover_path(item.beatmap_set_id)
            self.root.after(0, lambda: self._apply_preview_cover(request_id, cover_path))

        threading.Thread(target=worker, daemon=True).start()

    def _apply_preview_cover(self, request_id: int, cover_path: Path | None) -> None:
        if request_id != self.preview_request_id:
            return
        if cover_path is None or not cover_path.exists():
            self.preview_photo = None
            self.preview_cover_path = None
            self.preview_label.configure(image="", text="暂无可用图片")
            return

        self.preview_cover_path = cover_path
        self._render_preview_cover(cover_path)

    def _render_preview_cover(self, cover_path: Path) -> None:
        width = max(self.preview_label.winfo_width() - 12, 120)
        height = max(self.preview_label.winfo_height() - 12, 90)
        image = Image.open(cover_path).convert("RGB")
        image = ImageOps.contain(image, (width, height), Image.Resampling.LANCZOS)
        self.preview_photo = ImageTk.PhotoImage(image)
        self.preview_label.configure(image=self.preview_photo, text="")


def launch() -> None:
    try:
        ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID("osulazer-collection-view.app")
    except Exception:  # noqa: BLE001
        pass

    root = tk.Tk()
    CollectionViewApp(root)
    root.mainloop()
