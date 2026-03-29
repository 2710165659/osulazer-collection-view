from __future__ import annotations

import ctypes
import tkinter as tk
from pathlib import Path


class WindowIconManager:
    """统一处理主窗口和弹窗图标。"""

    def __init__(self, logo_png_path: Path, logo_ico_path: Path) -> None:
        self.logo_png_path = logo_png_path
        self.logo_ico_path = logo_ico_path
        self.app_icon_photo: tk.PhotoImage | None = None
        self.icon_handles: list[int] = []

    def apply(self, window: tk.Misc) -> None:
        if self.logo_ico_path.exists():
            try:
                window.iconbitmap(str(self.logo_ico_path))
            except Exception:  # noqa: BLE001
                pass

        if self.logo_png_path.exists():
            try:
                if self.app_icon_photo is None:
                    self.app_icon_photo = tk.PhotoImage(file=str(self.logo_png_path))
                window.iconphoto(True, self.app_icon_photo)
            except Exception:  # noqa: BLE001
                pass

        if self.logo_ico_path.exists():
            window.after(0, lambda target=window: self._apply_native_icon(target))

    def _apply_native_icon(self, window: tk.Misc) -> None:
        if not hasattr(ctypes, "windll") or not window.winfo_exists():
            return
        try:
            hwnd = window.winfo_id()
            user32 = ctypes.windll.user32
            image_icon = 1
            lr_loadfromfile = 0x0010
            wm_seticon = 0x0080
            icon_small = 0
            icon_big = 1
            gclp_hicon = -14
            gclp_hiconsm = -34
            sm_cxicon = 11
            sm_cyicon = 12
            sm_cxsmicon = 49
            sm_cysmicon = 50

            large_icon = user32.LoadImageW(
                None,
                str(self.logo_ico_path),
                image_icon,
                user32.GetSystemMetrics(sm_cxicon),
                user32.GetSystemMetrics(sm_cyicon),
                lr_loadfromfile,
            )
            small_icon = user32.LoadImageW(
                None,
                str(self.logo_ico_path),
                image_icon,
                user32.GetSystemMetrics(sm_cxsmicon),
                user32.GetSystemMetrics(sm_cysmicon),
                lr_loadfromfile,
            )

            if large_icon:
                user32.SendMessageW(hwnd, wm_seticon, icon_big, large_icon)
                self._set_window_class_icon(hwnd, gclp_hicon, large_icon)
                self.icon_handles.append(int(large_icon))
            if small_icon:
                user32.SendMessageW(hwnd, wm_seticon, icon_small, small_icon)
                self._set_window_class_icon(hwnd, gclp_hiconsm, small_icon)
                self.icon_handles.append(int(small_icon))
        except Exception:  # noqa: BLE001
            pass

    def _set_window_class_icon(self, hwnd: int, index: int, icon_handle: int) -> None:
        if not hasattr(ctypes, "windll"):
            return
        try:
            user32 = ctypes.windll.user32
            if hasattr(user32, "SetClassLongPtrW"):
                user32.SetClassLongPtrW(hwnd, index, icon_handle)
            else:
                user32.SetClassLongW(hwnd, index, icon_handle)
        except Exception:  # noqa: BLE001
            pass
