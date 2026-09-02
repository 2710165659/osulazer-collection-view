<template>
  <div class="header-shell">
    <div class="header-main">
      <div class="file-selector">
        <span class="label">Realm 数据库</span>
        <span class="file-path" :title="appStore.realmPath">
          {{ appStore.realmPath || "未选择文件" }}
        </span>
      </div>
    </div>

    <div class="actions">
      <el-button plain type="primary" :disabled="appStore.loadingDatabase" @click="selectDatabase">
        浏览 Realm
      </el-button>
      <el-button
        type="success"
        plain
        :loading="appStore.loadingDatabase"
        :disabled="!appStore.realmPath"
        @click="loadDatabase"
      >
        加载
      </el-button>
      <el-button
        type="warning"
        plain
        :loading="appStore.exporting"
        :disabled="!canExportCurrent"
        @click="exportCurrentBeatmaps"
      >
        导出当前列表
      </el-button>
      <el-button
        type="warning"
        plain
        :loading="appStore.exporting"
        :disabled="!canExportAll"
        @click="exportAllBeatmaps"
      >
        导出所有 ZIP
      </el-button>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, save, message } from "@tauri-apps/plugin-dialog";

import { useAppStore } from "@/store/useAppStore";

const appStore = useAppStore();

/**
 * 判断当前收藏夹和模式是否存在可导出的可见字段。
 */
const canExportCurrent = computed(
  () =>
    appStore.loaded &&
    Boolean(appStore.selectedCollection) &&
    appStore.beatmapTotal > 0 &&
    appStore.visibleColumns.length > 0 &&
    !appStore.loadingDatabase
);

/**
 * 判断已加载数据是否允许生成四模式 ZIP。
 */
const canExportAll = computed(
  () =>
    appStore.loaded &&
    appStore.collectionCount > 0 &&
    appStore.visibleColumns.length > 0 &&
    !appStore.loadingDatabase
);

/**
 * 打开 Realm 文件对话框；取消选择时保留原路径。
 */
const selectDatabase = async (): Promise<void> => {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "Realm 数据库", extensions: ["realm"] }],
  });
  if (!selected || Array.isArray(selected)) return;
  try {
    await appStore.setRealmPath(selected);
  } catch (error) {
    await showError("保存数据库路径失败", error);
  }
};

/**
 * 让 Rust 校验并加载 Realm 数据库。
 */
const loadDatabase = async (): Promise<void> => {
  try {
    await appStore.loadDatabase();
  } catch (error) {
    await showError("加载数据库失败", error);
  }
};

/**
 * 将当前收藏夹、模式和服务端排序结果导出为 Excel。
 */
const exportCurrentBeatmaps = async (): Promise<void> => {
  const collection = appStore.selectedCollection;
  if (!collection) return;

  const filePath = await save({
    title: "导出当前谱面列表",
    defaultPath: `${safeFilename(collection.name)}_${appStore.selectedMode}.xlsx`,
    filters: [{ name: "Excel 文件", extensions: ["xlsx"] }],
  });
  if (!filePath) return;

  appStore.exporting = true;
  try {
    await invoke("export_current", {
      request: {
        outputPath: filePath,
        collectionId: collection.id,
        mode: appStore.selectedMode,
        columns: appStore.columns,
        sortColumn: appStore.sortColumn,
        descending: appStore.sortDescending,
      },
    });
    await message(`已导出到：${filePath}`, { title: "导出成功", kind: "info" });
  } catch (error) {
    await showError("导出当前列表失败", error);
  } finally {
    appStore.exporting = false;
  }
};

/**
 * 让 Rust 生成四个模式 Excel 并打包为 ZIP。
 */
const exportAllBeatmaps = async (): Promise<void> => {
  const filePath = await save({
    title: "导出所有模式",
    defaultPath: "all_modes_collections.zip",
    filters: [{ name: "ZIP 压缩包", extensions: ["zip"] }],
  });
  if (!filePath) return;

  appStore.exporting = true;
  try {
    await invoke("export_all_modes", {
      request: {
        outputPath: filePath,
        columns: appStore.columns,
      },
    });
    await message(`已导出到：${filePath}`, { title: "导出成功", kind: "info" });
  } catch (error) {
    await showError("导出所有模式失败", error);
  } finally {
    appStore.exporting = false;
  }
};

/**
 * 按 Windows 文件名规则清洗默认导出文件名。
 */
const safeFilename = (value: string): string =>
  value.replace(/[<>:"/\\|?*]/g, "_").trim() || "collection";

/**
 * 使用 Tauri 原生对话框显示未知错误。
 */
const showError = async (title: string, error: unknown): Promise<void> => {
  const text = error instanceof Error ? error.message : String(error);
  await message(text, { title, kind: "error" });
};
</script>

<style scoped>
.header-shell {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.header-main {
  min-width: 0;
  flex: 1;
  display: flex;
  align-items: center;
}

.file-selector {
  display: flex;
  align-items: center;
  gap: 10px;
  flex: 1;
  min-width: 0;
}

.label {
  color: #475569;
  font-weight: 700;
  white-space: nowrap;
}

.file-path {
  min-width: 0;
  flex: 1;
  padding: 8px 12px;
  overflow: hidden;
  border: 1px solid #dbe3ef;
  border-radius: 10px;
  color: #334155;
  background: #f8fafc;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
}

.actions {
  display: flex;
  align-items: center;
  flex-wrap: nowrap;
  justify-content: flex-end;
  gap: 8px;
}
</style>
