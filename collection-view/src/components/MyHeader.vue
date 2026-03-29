<template>
  <div class="header">
    <div class="header-main">
      <div class="file-selector">
        <span class="label">本地数据库文件</span>
        <span class="file-path">{{ dbPath || "未选择文件" }}</span>
      </div>

      <div class="data-overview">
        <span class="overview-pill">收藏夹 {{ collectionCount }}</span>
        <span class="overview-pill">当前谱面 {{ currentBeatmapCount }}</span>
        <span class="overview-pill">显示列 {{ visibleColumnCount }}</span>
      </div>
    </div>

    <div class="actions">
      <el-button plain @click="selectDatabase" type="primary">浏览</el-button>
      <el-button type="success" plain @click="loadDatabase">加载数据库</el-button>
      <el-button
        type="warning"
        plain
        :disabled="!canExportCurrent"
        @click="exportCurrentBeatmaps"
      >
        导出当前列表
      </el-button>
      <el-button
        type="warning"
        plain
        :disabled="!canExportAll"
        @click="exportAllBeatmaps"
      >
        按模式导出全部
      </el-button>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { computed, ref } from "vue";
import { appDataDir, join } from "@tauri-apps/api/path";
import { open, save, message } from "@tauri-apps/plugin-dialog";
import { exists, readFile } from "@tauri-apps/plugin-fs";
import { Command } from "@tauri-apps/plugin-shell";
import { useCollectionsStore } from "@/store/useCollectionsStore";
import { useBeatmapConfigStore } from "@/store/useBeatmapConfigStore";
import { createWorkbookForCollections, saveWorkbook } from "@/utils/excel";
import { getVisibleColumns, rulesetOrder } from "@/utils/beatmapColumns";

const collectionsStore = useCollectionsStore();
const beatmapConfigStore = useBeatmapConfigStore();

const dbPath = ref(localStorage.getItem("dbPath") || "");

beatmapConfigStore.loadConfig();

const visibleColumns = computed(() => getVisibleColumns(beatmapConfigStore.appliedConfig));
const collectionCount = computed(() => collectionsStore.CollectionsList?.collections.length ?? 0);
const currentBeatmapCount = computed(() => {
  return (
    collectionsStore.selectedCollection?.items.filter(
      (item) => item.rulesetShortName === collectionsStore.selectedRuleset
    ).length ?? 0
  );
});
const visibleColumnCount = computed(() => visibleColumns.value.length);

const canExportCurrent = computed(
  () =>
    Boolean(collectionsStore.selectedCollection) &&
    Boolean(collectionsStore.selectedRuleset) &&
    visibleColumns.value.length > 0
);

const canExportAll = computed(
  () => Boolean(collectionsStore.CollectionsList?.collections.length) && visibleColumns.value.length > 0
);

const selectDatabase = async () => {
  const file =
    (await open({
      multiple: false,
      directory: false,
      filters: [{ name: "Realm Database", extensions: ["realm"] }],
    })) || "";

  dbPath.value = file;
  localStorage.setItem("dbPath", dbPath.value);
};

const loadDatabase = async () => {
  try {
    const appDir = await appDataDir();
    const outputPath = await join(appDir, "collections.json");

    const command = Command.sidecar("../../extractor/extractor", [dbPath.value, outputPath]);
    const result = await command.execute();

    if (result.code !== 0) {
      await message(`数据提取失败: ${result.stderr}`, { title: "Tauri", kind: "error" });
      return;
    }

    const fileExists = await exists(outputPath);
    if (!fileExists) {
      await message("输出文件未找到，请确保 Sidecar 正常工作", { title: "Tauri", kind: "error" });
      return;
    }

    const contents = await readFile(outputPath);
    const json = JSON.parse(new TextDecoder("utf-8").decode(contents));
    collectionsStore.loadCollectionsList(json);

    // 首次加载完成后给页面一个稳定的默认模式，方便立即查看和导出。
    if (!collectionsStore.selectedRuleset) {
      collectionsStore.setSelectedRuleset("osu");
    }
  } catch (error) {
    await message(
      `加载数据库时发生错误: ${error instanceof Error ? error.message : String(error)}`,
      { title: "Tauri", kind: "error" }
    );
  }
};

const exportCurrentBeatmaps = async () => {
  if (!collectionsStore.selectedCollection || !collectionsStore.selectedRuleset) {
    await message("请先选择要导出的收藏夹和模式。", { title: "导出失败", kind: "warning" });
    return;
  }

  try {
    const selectedCollection = collectionsStore.selectedCollection;
    const ruleset = collectionsStore.selectedRuleset;
    const filePath = await save({
      title: "导出当前谱面列表",
      defaultPath: `${selectedCollection.name}-${ruleset}.xlsx`,
      filters: [{ name: "Excel", extensions: ["xlsx"] }],
    });

    if (!filePath) {
      return;
    }

    const workbook = await createWorkbookForCollections(
      [selectedCollection],
      visibleColumns.value,
      (collection) => collection.items.filter((item) => item.rulesetShortName === ruleset)
    );

    await saveWorkbook(filePath, workbook);
    await message("当前谱面列表导出成功。", { title: "导出完成", kind: "info" });
  } catch (error) {
    await message(
      `导出当前列表失败: ${error instanceof Error ? error.message : String(error)}`,
      { title: "导出失败", kind: "error" }
    );
  }
};

const exportAllBeatmaps = async () => {
  if (!collectionsStore.CollectionsList?.collections.length) {
    await message("请先加载数据库。", { title: "导出失败", kind: "warning" });
    return;
  }

  try {
    const selectedDirectory = await open({
      title: "选择导出目录",
      directory: true,
      multiple: false,
    });

    if (!selectedDirectory || Array.isArray(selectedDirectory)) {
      return;
    }

    // 每个模式导出一个工作簿，每个收藏夹占一个 sheet。
    for (const ruleset of rulesetOrder) {
      const workbook = await createWorkbookForCollections(
        collectionsStore.CollectionsList.collections,
        visibleColumns.value,
        (collection) => collection.items.filter((item) => item.rulesetShortName === ruleset)
      );

      const outputPath = await join(selectedDirectory, `collections-${ruleset}.xlsx`);
      await saveWorkbook(outputPath, workbook);
    }

    await message("全部模式导出成功，已生成 4 个 Excel 文件。", {
      title: "导出完成",
      kind: "info",
    });
  } catch (error) {
    await message(
      `导出全部数据失败: ${error instanceof Error ? error.message : String(error)}`,
      { title: "导出失败", kind: "error" }
    );
  }
};
</script>

<style scoped>
.header {
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
  flex-direction: column;
  gap: 10px;
}

.file-selector {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.label {
  color: #475569;
  font-weight: 600;
  white-space: nowrap;
}

.file-path {
  min-width: 0;
  flex: 1;
  padding: 8px 12px;
  border: 1px solid #dbe3ef;
  border-radius: 10px;
  color: #334155;
  background: #f8fafc;
  word-break: break-all;
  font-size: 12px;
}

.data-overview {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.overview-pill {
  padding: 5px 10px;
  border-radius: 999px;
  background: #eff6ff;
  color: #1d4ed8;
  font-size: 12px;
  font-weight: 600;
}

.actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
}
</style>
