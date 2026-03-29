<template>
  <div class="beatmap-list-container">
    <div class="toolbar">
      <div class="title-group">
        <div>
          <h2>谱面列表</h2>
          <p>
            {{ currentCollectionName }}
            <span class="divider">/</span>
            {{ currentRulesetLabel }}
          </p>
        </div>
        <div class="stats">
          <span class="stat-pill">显示列 {{ visibleColumns.length }}</span>
          <span class="stat-pill">谱面数 {{ filteredBeatmaps.length }}</span>
        </div>
      </div>

      <div class="toolbar-actions">
        <el-button type="primary" plain @click="openSettings">
          <el-icon><Setting /></el-icon>
          配置列
        </el-button>
      </div>
    </div>

    <div class="table-wrapper">
      <el-table
        ref="tableRef"
        :data="filteredBeatmaps"
        :empty-text="emptyText"
        height="100%"
        border
        highlight-current-row
        row-key="md5"
        :row-class-name="getRowClassName"
        @row-click="handleRowClick"
      >
        <el-table-column
          v-for="column in visibleColumns"
          :key="column.key"
          :prop="column.key"
          :label="column.label"
          :min-width="getColumnWidth(column.key)"
          show-overflow-tooltip
        >
          <template #default="{ row }">
            <span>{{ formatBeatmapValue(row, column.key) }}</span>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <BeatmapConfigModal
      v-model="modalVisible"
      :current-config="beatmapConfigStore.appliedConfig"
      :default-config="beatmapConfigStore.originConfig"
      @confirm="confirmConfig"
    />
  </div>
</template>

<script lang="ts" setup>
import { computed, ref, watch } from "vue";
import { Setting } from "@element-plus/icons-vue";
import type { Beatmap } from "@/entities/Beatmap";
import BeatmapConfigModal from "@/components/BeatmapConfigModal.vue";
import { useCollectionsStore } from "@/store/useCollectionsStore";
import { useBeatmapConfigStore } from "@/store/useBeatmapConfigStore";
import {
  formatBeatmapValue,
  getVisibleColumns,
  rulesetLabelMap,
  type BeatmapColumnConfig,
} from "@/utils/beatmapColumns";

const collectionsStore = useCollectionsStore();
const beatmapConfigStore = useBeatmapConfigStore();

const tableRef = ref();
const modalVisible = ref(false);

beatmapConfigStore.loadConfig();

const visibleColumns = computed(() => getVisibleColumns(beatmapConfigStore.appliedConfig));

const currentCollectionName = computed(
  () => collectionsStore.selectedCollection?.name ?? "未选择收藏夹"
);

const currentRulesetLabel = computed(() => {
  const ruleset = collectionsStore.selectedRuleset as keyof typeof rulesetLabelMap | null;
  return ruleset ? rulesetLabelMap[ruleset] : "未选择模式";
});

const filteredBeatmaps = computed(() => {
  return (
    collectionsStore.selectedCollection?.items.filter(
      (item) => item.rulesetShortName === collectionsStore.selectedRuleset
    ) ?? []
  );
});

const emptyText = computed(() => {
  if (!collectionsStore.CollectionsList) {
    return "请先加载数据库";
  }
  if (!collectionsStore.selectedRuleset) {
    return "请先选择模式";
  }
  if (!collectionsStore.selectedCollection) {
    return "请先从左侧选择收藏夹";
  }
  return "当前筛选下没有谱面";
});

const getColumnWidth = (key: string) => {
  const wideColumns = new Set(["title", "titleUnicode", "artist", "artistUnicode", "difficultyName", "mapper"]);
  const compactColumns = new Set([
    "beatmapId",
    "beatmapSetId",
    "bpm",
    "circleSize",
    "overallDifficulty",
    "approachRate",
    "drainRate",
    "missing",
    "statusInt",
  ]);

  if (wideColumns.has(key)) return 180;
  if (compactColumns.has(key)) return 96;
  return 128;
};

const openSettings = () => {
  modalVisible.value = true;
};

const confirmConfig = (newConfig: BeatmapColumnConfig[]) => {
  beatmapConfigStore.saveConfig(newConfig);
};

const handleRowClick = (row: Beatmap) => {
  collectionsStore.setSelectedBeatmap(row);
};

const getRowClassName = ({ row }: { row: Beatmap }) => {
  return row.missing ? "row-missing" : "";
};

// 选中谱面变化时同步高亮表格行，保证图片预览和列表状态一致。
watch(
  () => collectionsStore.selectedBeatmap,
  (beatmap) => {
    tableRef.value?.setCurrentRow(beatmap ?? undefined);
  },
  { immediate: true }
);

watch(
  filteredBeatmaps,
  (rows) => {
    if (!rows.length) {
      collectionsStore.setSelectedBeatmap(null);
      tableRef.value?.setCurrentRow(undefined);
      return;
    }

    const currentBeatmap = collectionsStore.selectedBeatmap;
    if (currentBeatmap && rows.some((item) => item.md5 === currentBeatmap.md5)) {
      tableRef.value?.setCurrentRow(currentBeatmap);
      return;
    }

    collectionsStore.setSelectedBeatmap(rows[0]);
    tableRef.value?.setCurrentRow(rows[0]);
  },
  { immediate: true }
);
</script>

<style scoped>
.beatmap-list-container {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.toolbar {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.title-group {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.title-group h2 {
  margin: 0 0 4px;
  font-size: 18px;
  color: #111827;
}

.title-group p {
  margin: 0;
  color: #6b7280;
  font-size: 12px;
}

.divider {
  margin: 0 6px;
  color: #cbd5e1;
}

.stats {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.stat-pill {
  padding: 5px 10px;
  border-radius: 999px;
  background: #eff6ff;
  color: #1d4ed8;
  font-size: 12px;
  font-weight: 600;
}

.toolbar-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.table-wrapper {
  flex: 1;
  min-height: 0;
}

:deep(.el-table) {
  --el-table-header-bg-color: #f8fafc;
  --el-table-row-hover-bg-color: #eff6ff;
  border-radius: 12px;
  font-size: 12px;
}

:deep(.el-table th.el-table__cell) {
  padding: 8px 0;
  color: #475569;
  font-weight: 700;
}

:deep(.el-table td.el-table__cell) {
  padding: 7px 0;
}

:deep(.el-table .row-missing) {
  --el-table-tr-bg-color: #fff7ed;
}
</style>
