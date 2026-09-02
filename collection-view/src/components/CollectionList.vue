<template>
  <div class="collection-list-container">
    <div class="title">
      <div class="title-text">
        <h3>收藏夹列表</h3>
        <p>当前模式 {{ modeLabelMap[appStore.selectedMode] }}，共 {{ appStore.collections.length }} 个</p>
      </div>
      <el-select
        :model-value="appStore.selectedMode"
        size="small"
        class="mode-select"
        @change="handleModeChange"
      >
        <el-option
          v-for="mode in modeDefinitions"
          :key="mode.key"
          :label="mode.label"
          :value="mode.key"
        >
          <template #default>
            <div class="mode-option">
              <img
                v-if="modeIconPath(mode.key)"
                class="mode-icon"
                :src="modeIconPath(mode.key)"
                :alt="`${mode.label}图标`"
              />
              <span v-else class="mode-icon-placeholder" aria-hidden="true" />
              <span>{{ mode.label }}</span>
            </div>
          </template>
        </el-option>
        <template #label="{ label }">
          <div class="mode-option">
            <img
              v-if="modeIconPath(appStore.selectedMode)"
              class="mode-icon"
              :src="modeIconPath(appStore.selectedMode)"
              :alt="`${label}图标`"
            />
            <span v-else class="mode-icon-placeholder" aria-hidden="true" />
            <span>{{ label }}</span>
          </div>
        </template>
      </el-select>
    </div>

    <div class="table-wrapper">
      <el-table
        ref="tableRef"
        v-loading="appStore.loadingCollections"
        :data="appStore.collections"
        :empty-text="emptyText"
        height="100%"
        border
        highlight-current-row
        row-key="id"
        @row-click="handleRowClick"
      >
        <!-- 收藏夹列按需求缩小约 20%，更新时间只展示日期。 -->
        <el-table-column prop="name" label="收藏夹" min-width="142" show-overflow-tooltip />
        <el-table-column prop="totalCount" label="总数" width="64" align="center" />
        <el-table-column prop="currentModeCount" label="当前" width="64" align="center" />
        <el-table-column v-if="showMissingColumn" prop="missingCount" label="缺失" width="64" align="center" />
        <el-table-column prop="lastModified" label="更新时间" min-width="142" show-overflow-tooltip>
          <template #default="{ row }">
            {{ formatDate(row.lastModified) }}
          </template>
        </el-table-column>
      </el-table>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { ElMessage } from "element-plus";

import type { CollectionSummary } from "@/entities/Collection";
import { useRealtimeColumnResize } from "@/composables/useRealtimeColumnResize";
import { useAppStore } from "@/store/useAppStore";
import {
  formatDate,
  modeLabelMap,
  modeDefinitions,
  type ModeKey,
} from "@/utils/beatmapColumns";

const appStore = useAppStore();
const tableRef = ref();
const { bindRealtimeResize } = useRealtimeColumnResize(tableRef, (property) =>
  property === "name" || property === "lastModified" ? 100 : 60
); // 收藏夹名与更新时间按文本列限制，其余计数列保持紧凑。

const modeIconMap: Partial<Record<ModeKey, string>> = {
  osu: "/modes/osu.png",
  taiko: "/modes/taiko.png",
  ctb: "/modes/ctb.png",
  mania: "/modes/mania.png",
}; // 复用 Python 版本的四个实际模式图标资源，all 和 missing 保留文字显示。

/**
 * 返回模式选项对应的图标路径。
 */
const modeIconPath = (mode: ModeKey): string => modeIconMap[mode] ?? "";

/**
 * 收藏夹表格挂载后启用实时列宽拖拽。
 */
onMounted(() => {
  void bindRealtimeResize();
});

/**
 * 在全部和缺失模式下额外展示收藏夹缺失数量。
 */
const showMissingColumn = computed(() =>
  ["all", "missing"].includes(appStore.selectedMode)
);

/**
 * 根据数据库状态返回收藏夹表格的空内容提示。
 */
const emptyText = computed(() => {
  if (!appStore.loaded) return "请先加载数据库";
  return "当前模式没有可展示的收藏夹";
});

/**
 * 将收藏夹交互中的未知错误转换为界面消息。
 */
const showActionError = (title: string, error: unknown): void => {
  const text = error instanceof Error ? error.message : String(error);
  ElMessage.error(`${title}：${text}`);
};

/**
 * 切换模式并由 Rust 重新计算收藏夹汇总。
 */
const handleModeChange = async (mode: ModeKey): Promise<void> => {
  try {
    await appStore.setMode(mode);
  } catch (error) {
    showActionError("切换模式失败", error);
  }
};

/**
 * 选择收藏夹并获取第一页谱面。
 */
const handleRowClick = async (row: CollectionSummary): Promise<void> => {
  try {
    await appStore.selectCollection(row);
  } catch (error) {
    showActionError("加载收藏夹谱面失败", error);
  }
};

/**
 * 收藏夹选择改变时同步表格当前行高亮。
 */
watch(
  () => appStore.selectedCollection,
  (collection) => {
    const row = appStore.collections.find((item) => item.id === collection?.id);
    tableRef.value?.setCurrentRow(row);
  },
  { immediate: true }
);

/**
 * 模式切换导致缺失列重新挂载时恢复当前会话中的列宽。
 */
watch(showMissingColumn, async () => {
  await nextTick();
  void bindRealtimeResize();
});
</script>

<style scoped>
.collection-list-container {
  height: 100%;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.title {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
}

.title-text {
  min-width: 0;
}

h3 {
  margin: 0 0 4px;
  color: #111827;
  font-size: 15px;
}

.title-text p {
  margin: 0;
  color: #6b7280;
  font-size: 12px;
}

.mode-select {
  width: 108px;
}

.mode-option {
  display: flex;
  align-items: center;
  gap: 7px;
  min-width: 0;
}

.mode-icon,
.mode-icon-placeholder {
  display: inline-flex;
  flex: 0 0 19.2px;
  width: 19.2px;
  height: 19.2px; /* 模式图标相对原尺寸放大 20%，占位元素同步保持对齐。 */
  object-fit: contain;
}

.table-wrapper {
  flex: 1;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
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

:deep(.el-table__header-wrapper),
:deep(.el-table__body-wrapper),
:deep(.el-table__footer-wrapper),
:deep(.el-table .el-scrollbar__wrap) {
  overflow-x: hidden !important;
}

/* 收藏夹表格只保留纵向滚动，隐藏 Element Plus 单独渲染的横向滚动条。 */
:deep(.el-table .el-scrollbar__bar.is-horizontal) {
  display: none;
}
</style>
