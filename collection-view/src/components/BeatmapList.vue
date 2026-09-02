<template>
  <div class="beatmap-list-container">
    <div class="toolbar">
      <div class="title-group">
        <div>
          <h2>谱面列表</h2>
          <p>
            {{ appStore.selectedCollection?.name || "未选择收藏夹" }}
            <span class="divider">/</span>
            {{ modeLabelMap[appStore.selectedMode] }}
          </p>
        </div>
        <div class="stats">
          <span class="stat-pill">谱面 {{ appStore.beatmapTotal }}</span>
          <span class="stat-pill">分组 {{ appStore.beatmapGroupTotal }}</span>
          <span class="stat-pill">本页 {{ appStore.currentPageCount }}</span>
          <span class="stat-pill">显示列 {{ appStore.visibleColumns.length }}</span>
        </div>
      </div>

      <el-button type="primary" plain @click="openSettings">
        <el-icon><Setting /></el-icon>
        配置列
      </el-button>
    </div>

    <div class="table-wrapper">
      <el-table
        ref="tableRef"
        v-loading="appStore.loadingBeatmaps"
        :data="displayRows"
        :empty-text="emptyText"
        height="100%"
        border
        highlight-current-row
        row-key="rowKey"
        :row-class-name="getRowClassName"
        @row-click="handleRowClick"
        @row-dblclick="handleRowDoubleClick"
        @sort-change="handleSortChange"
      >
        <!-- 名称（原语言）列禁用溢出浮窗，其余列继续显示提示。 -->
        <el-table-column
          v-for="column in appStore.visibleColumns"
          :key="column.key"
          :prop="column.key"
          :label="column.label"
          :min-width="getBeatmapColumnWidth(column.key)"
          :sort-orders="['ascending', 'descending', null]"
          sortable="custom"
          :show-overflow-tooltip="column.key !== 'nameOriginal'"
        >
          <template #header>
            <span class="column-header-label">{{ column.label }}</span>
          </template>
          <template #default="{ row }">
            <div
              class="cell-content"
              :class="{
                'group-name-cell': isGroupRow(row) && column.key === groupControlColumnKey,
                'name-original-cell': column.key === 'nameOriginal',
              }"
            >
              <!-- 无展开按钮的行也保留相同宽度，确保名称文本左边界一致。 -->
              <button
                v-if="isGroupRow(row) && column.key === groupControlColumnKey && row.group.items.length > 1"
                type="button"
                class="group-toggle"
                :aria-label="row.expanded ? '折叠谱面组' : '展开谱面组'"
                @click.stop="toggleGroup(row.group.key)"
              >
                <el-icon><ArrowDown v-if="row.expanded" /><ArrowRight v-else /></el-icon>
              </button>
              <span
                v-else-if="column.key === groupControlColumnKey"
                class="group-toggle-placeholder"
                aria-hidden="true"
              />
              <span
                :class="{ 'group-value-slash': isGroupSlashValue(row, column.key) }"
              >{{ formatRowValue(row, column.key) || "-" }}</span>
            </div>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <div class="pagination-row">
      <span class="pagination-total">
        共 {{ appStore.beatmapGroupTotal }} 组（{{ appStore.beatmapTotal }} 张谱面）
      </span>
      <el-pagination
        background
        layout="sizes, prev, pager, next"
        :total="appStore.beatmapGroupTotal"
        :current-page="appStore.page"
        :page-size="appStore.pageSize"
        :page-sizes="pageSizes"
        small
        @current-change="handlePageChange"
        @size-change="handlePageSizeChange"
      />
    </div>

    <BeatmapConfigModal
      v-model="settingsVisible"
      :current-config="appStore.columns"
      :default-config="appStore.defaultColumns"
      @confirm="confirmConfig"
    />
    <BeatmapDetailDialog v-model="detailVisible" :beatmap="detailBeatmap" />
  </div>
</template>

<script lang="ts" setup>
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { ArrowDown, ArrowRight, Setting } from "@element-plus/icons-vue";
import { ElMessage } from "element-plus";

import BeatmapConfigModal from "@/components/BeatmapConfigModal.vue";
import BeatmapDetailDialog from "@/components/BeatmapDetailDialog.vue";
import { useRealtimeColumnResize } from "@/composables/useRealtimeColumnResize";
import type { Beatmap, BeatmapGroup } from "@/entities/Beatmap";
import { useAppStore } from "@/store/useAppStore";
import {
  formatBeatmapValue,
  getBeatmapColumnWidth,
  modeLabelMap,
  type BeatmapColumnConfig,
} from "@/utils/beatmapColumns";

interface SortChangePayload {
  prop: string;
  order: "ascending" | "descending" | null;
}

interface GroupRow {
  rowKey: string;
  type: "group";
  group: BeatmapGroup;
  expanded: boolean;
  groupSelected: boolean; // 当前展开分组的父行使用统一选中样式。
}

interface ItemRow {
  rowKey: string;
  type: "item";
  item: Beatmap;
  groupKey: string;
  groupSelected: boolean; // 当前展开分组的子行使用统一选中样式。
}

type DisplayRow = GroupRow | ItemRow;

const appStore = useAppStore();
const tableRef = ref();
const settingsVisible = ref(false);
const detailVisible = ref(false);
const detailBeatmap = ref<Beatmap | null>(null);
const expandedGroups = ref<Set<string>>(new Set());
const pageSizes = [10, 20, 50, 100, 200]; // 分页下拉只提供约定的五个选项。
const groupSlashKeys = new Set(["beatmapId", "md5", "difficultyName"]); // BID、MD5 和难度名多值时使用居中的反斜杠。

/**
 * 返回用于显示分组箭头的列，名称列隐藏时回退到第一列。
 */
const groupControlColumnKey = computed(
  () =>
    appStore.visibleColumns.some((column) => column.key === "nameOriginal")
      ? "nameOriginal"
      : appStore.visibleColumns[0]?.key ?? "nameOriginal"
);

/**
 * 根据数据库和收藏夹状态返回谱面表格的空内容提示。
 */
const emptyText = computed(() => {
  if (!appStore.loaded) return "请先加载数据库";
  if (!appStore.selectedCollection) return "当前模式没有收藏夹";
  return "当前筛选下没有谱面";
});

/**
 * 判断表格行是否为折叠组父行。
 */
const isGroupRow = (row: DisplayRow): row is GroupRow => row.type === "group";

/**
 * 返回当前选中谱面所属的分组，用于展开时同时高亮父行和全部子行。
 */
const selectedGroupKey = computed<string | null>(() => {
  const beatmap = appStore.selectedBeatmap;
  if (!beatmap) return null;
  return (
    appStore.beatmapGroups.find((group) =>
      group.items.some((item) => item.md5 === beatmap.md5)
    )?.key ?? null
  );
});

/**
 * 将当前页分组转换成默认折叠的父行和按需展开的子行。
 */
const displayRows = computed<DisplayRow[]>(() => {
  const rows: DisplayRow[] = [];
  const activeGroupKey = selectedGroupKey.value;
  for (const group of appStore.beatmapGroups) {
    const expanded = expandedGroups.value.has(group.key);
    const groupSelected = expanded && activeGroupKey === group.key; // 只有当前展开分组的父子行共同高亮。
    rows.push({
      rowKey: `group:${group.key}`,
      type: "group",
      group,
      expanded,
      groupSelected,
    });
    if (expanded && group.items.length > 1) {
      group.items.forEach((item) =>
        rows.push({
          rowKey: `item:${group.key}:${item.md5}`,
          type: "item",
          item,
          groupKey: group.key,
          groupSelected,
        })
      );
    }
  }
  return rows;
});

/**
 * 获取组内每张谱面的字段文本，并保留空值和原始大小写用于公共值判断。
 */
const groupTexts = (group: BeatmapGroup, key: string): string[] =>
  group.items.map((item) => formatBeatmapValue(item, key).trim());

/**
 * 按数值字段格式化组范围，空值不参与范围计算。
 */
const formatNumericRange = (group: BeatmapGroup, key: string): string => {
  const values = group.items
    .map((item) => {
      const raw = item[key as keyof Beatmap];
      return typeof raw === "number" && Number.isFinite(raw) ? raw : null;
    })
    .filter((value): value is number => value != null);
  if (!values.length) return "";
  const min = Math.min(...values);
  const max = Math.max(...values);
  return min === max
    ? formatBeatmapValue({ ...group.items[0], [key]: min } as Beatmap, key)
    : `${formatBeatmapValue({ ...group.items[0], [key]: min } as Beatmap, key)} ~ ${formatBeatmapValue({ ...group.items[0], [key]: max } as Beatmap, key)}`;
};

/**
 * 根据组内谱面汇总生成父行字段文本，不改变单谱面子行原始展示。
 */
const formatGroupValue = (group: BeatmapGroup, key: string): string => {
  const firstItem = group.items[0];
  if (!firstItem) return "";
  if (group.items.length <= 1) return formatBeatmapValue(firstItem, key);
  const values = groupTexts(group, key);
  const distinctValues = [...new Set(values)];
  if (groupSlashKeys.has(key)) {
    if (key === "difficultyName") return "\\";
    return distinctValues.length === 1
      ? distinctValues[0]
      : distinctValues.some(Boolean)
        ? "\\"
        : "";
  }
  if (
    [
      "starRating",
      "circleSize",
      "overallDifficulty",
      "approachRate",
      "drainRate",
      "bpm",
      "lengthMs",
      "totalObjectCount",
    ].includes(key)
  ) {
    return formatNumericRange(group, key);
  }
  if (key === "beatmapSetId") {
    return distinctValues.length === 1 && distinctValues[0] ? distinctValues[0] : "多个值";
  }
  if (key === "mode") {
    return distinctValues.length === 1 && distinctValues[0] ? distinctValues[0] : "多模式";
  }
  return values.length && values[0] && values.every((value) => value === values[0])
    ? values[0]
    : values.some(Boolean)
      ? "多值"
      : "";
};

/**
 * 判断组父行当前字段是否需要使用居中的反斜杠占位。
 */
const isGroupSlashValue = (row: DisplayRow, key: string): boolean =>
  isGroupRow(row) &&
  row.group.items.length > 1 &&
  groupSlashKeys.has(key) &&
  formatGroupValue(row.group, key) === "\\";

/**
 * 根据父行或子行返回表格单元格展示文本。
 */
const formatRowValue = (row: DisplayRow, key: string): string =>
  isGroupRow(row)
    ? formatGroupValue(row.group, key)
    : formatBeatmapValue(row.item, key);

/**
 * 切换指定谱面组的展开状态。
 */
const toggleGroup = (key: string): void => {
  const next = new Set(expandedGroups.value);
  tableRef.value?.setCurrentRow(undefined); // 折叠或展开前先清除原来单行的 Element Plus 选中状态。
  if (next.has(key)) {
    next.delete(key);
  } else {
    const group = appStore.beatmapGroups.find((item) => item.key === key);
    if (group?.items[0]) appStore.selectBeatmap(group.items[0]); // 通过箭头展开时也切换到该组的首张谱面。
    next.add(key);
  }
  expandedGroups.value = next;
  void nextTick(syncCurrentRowHighlight); // 行集合变化后立即把当前谱面高亮切换到父行或子行。
};

/**
 * 清空所有组的展开状态，保证新查询默认折叠。
 */
const resetExpandedGroups = (): void => {
  expandedGroups.value = new Set();
};

/**
 * 将谱面列表交互中的未知错误转换为界面消息。
 */
const showActionError = (title: string, error: unknown): void => {
  const text = error instanceof Error ? error.message : String(error);
  ElMessage.error(`${title}：${text}`);
};

/**
 * 打开列配置弹窗。
 */
const openSettings = (): void => {
  settingsVisible.value = true;
};

/**
 * 保存列配置到 Rust 设置文件。
 */
const confirmConfig = async (newConfig: BeatmapColumnConfig[]): Promise<void> => {
  try {
    await appStore.setColumns(newConfig);
  } catch (error) {
    showActionError("保存列配置失败", error);
  }
};

/**
 * 单击父行预览组内第一张谱面，单击子行预览对应谱面。
 */
const handleRowClick = (row: DisplayRow): void => {
  appStore.selectBeatmap(isGroupRow(row) ? row.group.items[0] ?? null : row.item);
};

/**
 * 双击父行展开或折叠，双击子行打开详细信息。
 */
const handleRowDoubleClick = (row: DisplayRow): void => {
  if (isGroupRow(row)) {
    if (row.group.items.length > 1) toggleGroup(row.group.key);
    else if (row.group.items[0]) {
      appStore.selectBeatmap(row.group.items[0]);
      detailBeatmap.value = row.group.items[0];
      detailVisible.value = true; // 单谱面组没有子行，双击继续打开原有详情弹窗。
    }
    return;
  }
  appStore.selectBeatmap(row.item);
  detailBeatmap.value = row.item;
  detailVisible.value = true;
};

/**
 * 把 Element Plus 表头三态排序交给 Rust 处理并折叠所有组。
 */
const handleSortChange = async ({ prop, order }: SortChangePayload): Promise<void> => {
  resetExpandedGroups();
  try {
    await appStore.applySort(order ? prop : null, order);
  } catch (error) {
    showActionError("排序谱面失败", error);
  }
};

/**
 * 切换服务端分页页码并恢复默认折叠状态。
 */
const handlePageChange = async (page: number): Promise<void> => {
  resetExpandedGroups();
  try {
    await appStore.setPage(page);
  } catch (error) {
    showActionError("切换分页失败", error);
  }
};

/**
 * 修改每页分组数量、持久化设置并恢复默认折叠状态。
 */
const handlePageSizeChange = async (pageSize: number): Promise<void> => {
  resetExpandedGroups();
  try {
    await appStore.setPageSize(pageSize);
  } catch (error) {
    showActionError("修改分页大小失败", error);
  }
};

/**
 * 为缺失谱面和分组父行设置独立行样式。
 */
const getRowClassName = ({ row }: { row: DisplayRow }): string => {
  const classNames = isGroupRow(row)
    ? ["row-group"]
    : row.item.missing
      ? ["row-missing", "row-child"]
      : ["row-child"];
  if (row.groupSelected) {
    classNames.push("row-group-selected"); // 展开当前分组时父行和全部子行共同显示选中状态。
  }
  return classNames.join(" ");
};

const { bindRealtimeResize } = useRealtimeColumnResize(tableRef, (property) =>
  getBeatmapColumnWidth(property) < 120 ? 60 : 100
); // 根据列的初始用途区分紧凑列和文本列最小宽度。

/**
 * 根据当前选择和展开状态，把表格高亮定位到对应父行或子行。
 */
const syncCurrentRowHighlight = (): void => {
  const beatmap = appStore.selectedBeatmap;
  const group = beatmap
    ? appStore.beatmapGroups.find((item) =>
        item.items.some((groupItem) => groupItem.md5 === beatmap.md5)
      )
    : undefined;
  const itemRow = beatmap
    ? displayRows.value.find(
        (item) => !isGroupRow(item) && item.item.md5 === beatmap.md5
      )
    : undefined;
  const groupRow = group
    ? displayRows.value.find(
        (item) => isGroupRow(item) && item.group.key === group.key
      )
    : undefined;
  const row =
    group && expandedGroups.value.has(group.key) ? groupRow : itemRow ?? groupRow; // 展开组统一将当前行锚定到父行。
  tableRef.value?.setCurrentRow(row ?? undefined);
};

/**
 * 谱面表格挂载后启用实时列宽拖拽。
 */
onMounted(() => {
  void bindRealtimeResize();
});

/**
 * 当前谱面改变时同步表格高亮，折叠组优先落到对应父行。
 */
watch(
  () => appStore.selectedBeatmap,
  syncCurrentRowHighlight,
  { immediate: true }
);

/**
 * 查询上下文改变后恢复默认折叠，并在表格重绘后确认拖拽监听已绑定。
 */
watch(
  () => [
    appStore.beatmapGroups,
    appStore.page,
    appStore.selectedCollection?.id,
    appStore.selectedMode,
    ...appStore.visibleColumns.map((column) => column.key),
  ],
  async () => {
    resetExpandedGroups();
    await nextTick();
    void bindRealtimeResize();
    syncCurrentRowHighlight();
  }
);

/**
 * 清空服务端排序时同步清除 Element Plus 表头排序图标。
 */
watch(
  () => appStore.sortColumn,
  (column) => {
    if (!column) tableRef.value?.clearSort();
  }
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

.table-wrapper {
  flex: 1;
  min-height: 0;
}

.pagination-row {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 12px;
  min-height: 22px;
}

.pagination-total {
  color: #606266;
  font-size: 12px;
  white-space: nowrap;
}

/* 表头文字占用剩余空间，排序按钮固定贴齐当前列右侧。 */
.column-header-label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cell-content {
  display: flex;
  align-items: center;
  min-width: 0;
  gap: 4px;
}

.name-original-cell {
  overflow: hidden;
  white-space: nowrap;
}

.name-original-cell > span:last-child {
  display: block;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.group-toggle {
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  padding: 0;
  border: 0;
  border-radius: 4px;
  color: #64748b;
  background: transparent;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.15s ease, background-color 0.15s ease;
}

.group-toggle-placeholder {
  display: inline-flex;
  flex: 0 0 auto;
  width: 18px;
  height: 18px;
}

/* 反斜杠占据单元格剩余宽度，确保多值占位始终居中。 */
.group-value-slash {
  flex: 1;
  text-align: center;
}

.group-name-cell:hover .group-toggle,
.group-name-cell:focus-within .group-toggle {
  opacity: 1;
}

.group-toggle:hover {
  background: #dbeafe;
  color: #2563eb;
}

:deep(.el-pagination--small) {
  --el-pagination-button-width: 22px;
  --el-pagination-button-height: 22px;
  --el-pagination-font-size: 12px;
}

:deep(.el-pagination--small .el-select__wrapper) {
  min-height: 22px;
}

:deep(.el-table) {
  --el-table-header-bg-color: #f8fafc;
  --el-table-row-hover-bg-color: #eff6ff;
  --el-table-current-row-bg-color: #eaf3ff; /* 所有选中行统一使用更浅的背景色。 */
  border-radius: 12px;
  font-size: 12px;
}

:deep(.el-table th.el-table__cell) {
  padding: 8px 0;
  color: #475569;
  font-weight: 700;
}

:deep(.el-table th.el-table__cell .cell) {
  display: flex;
  align-items: center;
  min-width: 0;
  gap: 6px;
}

:deep(.el-table th.el-table__cell .caret-wrapper) {
  flex: 0 0 auto;
  margin-left: auto;
}

:deep(.el-table td.el-table__cell) {
  padding: 7px 0;
}

/* 父行与子行共享折叠组的背景、字重和颜色。 */
:deep(.el-table .row-group),
:deep(.el-table .row-child) {
  --el-table-tr-bg-color: #f8fbff;
  font-weight: 600;
  color: #606266;
}

/* 当前展开分组的父行和子行共同使用统一的浅色选中背景。 */
:deep(.el-table .row-group-selected > td.el-table__cell) {
  background-color: #eaf3ff;
}

/* 只有展开组的顶部父行使用更明显的下边框，普通选中行保持原边框。 */
:deep(.el-table .row-group-selected.row-group > td.el-table__cell) {
  border-bottom-color: #b7cbe1;
}

:deep(.el-table .row-missing) {
  --el-table-tr-bg-color: #fff1f2;
  color: #9f1239;
}
</style>
