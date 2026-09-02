<template>
  <el-dialog
    v-model="localVisible"
    title="配置谱面列表列"
    width="760px"
    destroy-on-close
    @closed="handleClosed"
  >
    <div class="config-layout">
      <section class="panel">
        <div class="panel-header">
          <div>
            <h3>当前显示顺序</h3>
            <p>拖拽标签可以调整顺序，删除标签会隐藏该列。</p>
          </div>
          <el-button text type="primary" @click="restoreDefaults">恢复默认</el-button>
        </div>

        <div
          class="column-order-input"
          :class="{ 'is-empty': !selectedKeys.length }"
          role="list"
          ref="columnOrderListRef"
        >
          <span v-if="!selectedKeys.length" class="column-order-placeholder">
            点击下方列卡片添加显示列
          </span>
          <div
            v-for="(key, index) in selectedKeys"
            :key="key"
            class="column-order-item"
            :class="{
              'is-dragging': draggingKey === key,
              'is-drag-over': dragOverKey === key,
            }"
            role="listitem"
            :aria-grabbed="draggingKey === key"
            :data-column-order-key="key"
            @pointerdown="handleColumnPointerDown($event, index)"
          >
            <el-tag
              closable
              disable-transitions
              effect="plain"
              @close="removeColumn(key)"
            >
              {{ getColumnLabel(key) }}
            </el-tag>
          </div>
        </div>
      </section>

      <section class="panel">
        <div class="panel-header">
          <div>
            <h3>全部列</h3>
            <p>点击卡片可切换显示状态，蓝色表示当前显示。</p>
          </div>
          <span class="counter">{{ selectedKeys.length }} / {{ allColumns.length }}</span>
        </div>

        <div class="grid-container">
          <el-card
            v-for="item in allColumns"
            :key="item.key"
            :class="{ selected: isSelected(item.key) }"
            class="grid-item"
            shadow="never"
            @click="toggleColumn(item.key)"
          >
            <div class="content">
              <span class="label">{{ item.label }}</span>
              <span class="key">{{ item.key }}</span>
            </div>
            <div class="corner-icon" v-if="isSelected(item.key)">
              <el-icon>
                <Check />
              </el-icon>
            </div>
          </el-card>
        </div>
      </section>
    </div>

    <template #footer>
      <el-button @click="closeDialog">取消</el-button>
      <el-button type="primary" @click="confirmSelect">确认</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from "vue";
import { ElMessage } from "element-plus";
import { Check } from "@element-plus/icons-vue";
import type { BeatmapColumnConfig } from "@/utils/beatmapColumns";
import { cloneBeatmapConfig } from "@/utils/beatmapColumns";

const props = defineProps<{
  modelValue: boolean;
  currentConfig: BeatmapColumnConfig[];
  defaultConfig: BeatmapColumnConfig[];
}>();

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
  (event: "confirm", value: BeatmapColumnConfig[]): void;
}>();

const localVisible = ref(false);
const allColumns = ref<BeatmapColumnConfig[]>([]);
const defaultColumns = ref<BeatmapColumnConfig[]>([]);
const selectedKeys = ref<string[]>([]);
const columnOrderListRef = ref<HTMLElement | null>(null);
const draggingKey = ref<string | null>(null);
const dragOverKey = ref<string | null>(null);

const POINTER_DRAG_THRESHOLD = 4; // 移动超过四像素才进入拖拽，避免点击标签时误触排序。

interface ColumnPointerSession {
  key: string;
  pointerId: number;
  startX: number;
  startY: number;
  sourceElement: HTMLElement;
  active: boolean;
  targetKey: string | null;
  targetPosition: "before" | "after" | null;
}

let columnPointerSession: ColumnPointerSession | null = null;

/**
 * 根据字段键读取列标签，字段不存在时保留键本身以避免拖拽项消失。
 */
const getColumnLabel = (key: string): string => {
  return allColumns.value.find((item) => item.key === key)?.label ?? key;
};

/**
 * 从当前显示顺序中移除指定列，等同于 InputTag 的关闭标签操作。
 */
const removeColumn = (key: string): void => {
  selectedKeys.value = selectedKeys.value.filter((item) => item !== key);
};

/**
 * 记录标签按下位置，并在移动达到阈值后启动指针拖拽会话。
 */
const handleColumnPointerDown = (event: PointerEvent, index: number): void => {
  if (event.button !== 0) {
    return;
  }

  const target = event.target as HTMLElement | null;
  if (target?.closest("button")) {
    return; // 关闭按钮只负责隐藏列，不参与拖拽。
  }

  const key = selectedKeys.value[index];
  if (!key) {
    return;
  }

  const sourceElement = event.currentTarget as HTMLElement | null;
  if (!sourceElement) {
    return;
  }

  event.preventDefault(); // 禁止拖动文字选中，指针移动由业务排序逻辑接管。
  columnPointerSession = {
    key,
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY,
    sourceElement,
    active: false,
    targetKey: null,
    targetPosition: null,
  };
  sourceElement.setPointerCapture?.(event.pointerId); // 指针移出标签后仍能收到 pointerup。
  window.addEventListener("pointermove", handleColumnPointerMove);
  window.addEventListener("pointerup", handleColumnPointerUp);
  window.addEventListener("pointercancel", handleColumnPointerCancel);
};

/**
 * 根据指针坐标找到当前悬停的列标签。
 */
const getColumnItemAtPoint = (clientX: number, clientY: number): HTMLElement | null => {
  const list = columnOrderListRef.value;
  const element = document.elementFromPoint(clientX, clientY);
  const item = element?.closest<HTMLElement>("[data-column-order-key]") ?? null;
  return item && list?.contains(item) ? item : null;
};

/**
 * 更新指针拖拽的目标列和插入方向，并实时显示目标高亮。
 */
const handleColumnPointerMove = (event: PointerEvent): void => {
  const session = columnPointerSession;
  if (!session || event.pointerId !== session.pointerId) {
    return;
  }

  const distance = Math.hypot(
    event.clientX - session.startX,
    event.clientY - session.startY
  );
  if (!session.active && distance < POINTER_DRAG_THRESHOLD) {
    return;
  }

  if (!session.active) {
    session.active = true;
    draggingKey.value = session.key;
  }
  event.preventDefault();

  const target = getColumnItemAtPoint(event.clientX, event.clientY);
  const targetKey = target?.dataset.columnOrderKey ?? null;
  if (!target || !targetKey || targetKey === session.key) {
    session.targetKey = null;
    session.targetPosition = null;
    dragOverKey.value = null;
    return;
  }

  const bounds = target.getBoundingClientRect();
  const isAfter =
    event.clientY >= bounds.top + bounds.height / 2 ||
    (event.clientY >= bounds.top &&
      event.clientY <= bounds.bottom &&
      event.clientX >= bounds.left + bounds.width / 2);
  const position = isAfter ? "after" : "before";
  session.targetKey = targetKey;
  session.targetPosition = position;
  dragOverKey.value = targetKey;
};

/**
 * 按目标列前后位置移动源列，并立即更新当前草稿顺序。
 */
const reorderColumn = (
  sourceKey: string,
  targetKey: string,
  position: "before" | "after"
): void => {
  const sourceIndex = selectedKeys.value.indexOf(sourceKey);
  const targetIndex = selectedKeys.value.indexOf(targetKey);
  if (sourceIndex < 0 || targetIndex < 0 || sourceIndex === targetIndex) {
    return;
  }

  let insertionIndex = targetIndex + (position === "after" ? 1 : 0);
  if (sourceIndex < insertionIndex) {
    insertionIndex -= 1; // 移除源项后，目标索引需要向前校正一位。
  }

  const nextKeys = [...selectedKeys.value];
  const [movedKey] = nextKeys.splice(sourceIndex, 1);
  nextKeys.splice(Math.max(0, Math.min(insertionIndex, nextKeys.length)), 0, movedKey);
  selectedKeys.value = nextKeys;
};

/**
 * 清理指针拖拽监听与临时状态，避免弹窗关闭后残留全局事件。
 */
const clearColumnDragState = (): void => {
  const session = columnPointerSession;
  if (session?.sourceElement.hasPointerCapture?.(session.pointerId)) {
    session.sourceElement.releasePointerCapture(session.pointerId);
  }
  window.removeEventListener("pointermove", handleColumnPointerMove);
  window.removeEventListener("pointerup", handleColumnPointerUp);
  window.removeEventListener("pointercancel", handleColumnPointerCancel);
  columnPointerSession = null;
  draggingKey.value = null;
  dragOverKey.value = null;
};

/**
 * 松开指针后提交列顺序变更，并结束当前拖拽会话。
 */
const handleColumnPointerUp = (event: PointerEvent): void => {
  const session = columnPointerSession;
  if (!session || event.pointerId !== session.pointerId) {
    return;
  }

  if (session.active && session.targetKey && session.targetPosition) {
    reorderColumn(session.key, session.targetKey, session.targetPosition);
  }
  clearColumnDragState();
};

/**
 * 指针被系统取消时放弃本次拖拽，不改变原有列顺序。
 */
const handleColumnPointerCancel = (event: PointerEvent): void => {
  if (columnPointerSession?.pointerId === event.pointerId) {
    clearColumnDragState();
  }
};

onBeforeUnmount(clearColumnDragState);

/**
 * 每次打开弹窗时从已应用配置重建本地草稿。
 */
const syncFromProps = (): void => {
  allColumns.value = cloneBeatmapConfig(props.currentConfig);
  defaultColumns.value = cloneBeatmapConfig(props.defaultConfig);
  selectedKeys.value = props.currentConfig.filter((item) => item.visible).map((item) => item.key);
};

watch(
  () => props.modelValue,
  (value) => {
    localVisible.value = value;
    if (value) {
      syncFromProps();
    }
  },
  { immediate: true }
);

watch(localVisible, (value) => {
  if (value !== props.modelValue) {
    emit("update:modelValue", value);
  }
});

/**
 * 判断指定列当前是否可见。
 */
const isSelected = (key: string): boolean => selectedKeys.value.includes(key);

/**
 * 切换指定列的显示状态，并保持用户选择顺序。
 */
const toggleColumn = (key: string): void => {
  if (isSelected(key)) {
    selectedKeys.value = selectedKeys.value.filter((item) => item !== key);
    return;
  }

  selectedKeys.value = [...selectedKeys.value, key];
};

/**
 * 恢复由 Rust 后端提供的默认列和顺序。
 */
const restoreDefaults = (): void => {
  allColumns.value = cloneBeatmapConfig(defaultColumns.value);
  selectedKeys.value = defaultColumns.value.filter((item) => item.visible).map((item) => item.key);
};

/**
 * 取消本次列配置修改并关闭弹窗。
 */
const closeDialog = (): void => {
  localVisible.value = false;
};

/**
 * 弹窗动画结束后同步父组件的显示状态。
 */
const handleClosed = (): void => {
  emit("update:modelValue", false);
};

/**
 * 校验并提交新的列可见状态和排列顺序。
 */
const confirmSelect = (): void => {
  if (!selectedKeys.value.length) {
    ElMessage.warning("至少保留一列用于展示和导出。");
    return;
  }

  const selectedSet = new Set(selectedKeys.value);
  const orderedKeys = [
    ...selectedKeys.value,
    ...allColumns.value.map((item) => item.key).filter((key) => !selectedSet.has(key)),
  ];

  const configMap = new Map(allColumns.value.map((item) => [item.key, item]));
  const nextConfig = orderedKeys.map((key) => {
    const item = configMap.get(key)!;
    return {
      ...item,
      visible: selectedSet.has(key),
    };
  });

  emit("confirm", nextConfig);
  localVisible.value = false;
};
</script>

<style scoped>
.config-layout {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.panel {
  border: 1px solid #e5e7eb;
  border-radius: 14px;
  padding: 16px;
  background: linear-gradient(180deg, #ffffff 0%, #f8fafc 100%);
}

.panel-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}

.panel-header h3 {
  margin: 0 0 4px;
  font-size: 14px;
  color: #111827;
}

.panel-header p {
  margin: 0;
  color: #6b7280;
  font-size: 12px;
}

.column-order-input {
  width: 100%;
  min-height: 32px;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
  padding: 4px 8px;
  border: 1px solid #dcdfe6;
  border-radius: 4px;
  background: #fff;
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
}

.column-order-input:focus-within,
.column-order-input:hover {
  border-color: #c0c4cc;
}

.column-order-input.is-empty {
  color: #909399;
}

.column-order-placeholder {
  font-size: 13px;
  line-height: 24px;
}

.column-order-item {
  display: inline-flex;
  align-items: center;
  cursor: grab;
  touch-action: none;
  user-select: none;
  transition: opacity 0.15s ease, box-shadow 0.15s ease;
}

.column-order-item:active {
  cursor: grabbing;
}

.column-order-item.is-dragging {
  opacity: 0.45;
}

.column-order-item.is-drag-over {
  border-radius: 4px;
  box-shadow: 0 0 0 2px rgba(64, 158, 255, 0.35);
}

.counter {
  color: #2563eb;
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
}

.grid-container {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 10px;
  max-height: 320px;
  overflow-y: auto;
  padding-right: 2px;
}

.grid-item {
  position: relative;
  min-height: 82px;
  cursor: pointer;
  transition: all 0.2s ease;
  user-select: none;
  border: 1px solid #dbe3ef;
  background: #fff;
}

.grid-item:hover {
  transform: translateY(-1px);
  border-color: #93c5fd;
  box-shadow: 0 10px 18px rgba(37, 99, 235, 0.08);
}

.grid-item.selected {
  border-color: #3b82f6;
  background: linear-gradient(180deg, #eff6ff 0%, #dbeafe 100%);
}

.content {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.label {
  font-size: 13px;
  font-weight: 600;
  color: #0f172a;
}

.key {
  color: #64748b;
  font-size: 12px;
  word-break: break-all;
}

.corner-icon {
  position: absolute;
  right: 8px;
  bottom: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  color: #2563eb;
  background: rgba(255, 255, 255, 0.9);
  border-radius: 999px;
}
</style>
