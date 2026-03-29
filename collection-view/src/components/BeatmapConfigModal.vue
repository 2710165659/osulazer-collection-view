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

        <el-input-tag
          v-model="visibleLabels"
          class="column-order-input"
          draggable
          readonly
          tag-effect="plain"
          placeholder="从下方点击列名，把它加入当前展示列"
        />
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
import { computed, ref, watch } from "vue";
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

const labelToKeyMap = computed(() => {
  return new Map(allColumns.value.map((item) => [item.label, item.key]));
});

const visibleLabels = computed<string[]>({
  get: () =>
    selectedKeys.value
      .map((key) => allColumns.value.find((item) => item.key === key)?.label)
      .filter((label): label is string => Boolean(label)),
  set: (labels) => {
    // InputTag 的拖拽和删除都会回写 labels，这里统一还原到真实 key。
    const nextKeys: string[] = [];
    labels.forEach((label) => {
      const key = labelToKeyMap.value.get(label);
      if (key && !nextKeys.includes(key)) {
        nextKeys.push(key);
      }
    });
    selectedKeys.value = nextKeys;
  },
});

const syncFromProps = () => {
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

const isSelected = (key: string) => selectedKeys.value.includes(key);

const toggleColumn = (key: string) => {
  if (isSelected(key)) {
    selectedKeys.value = selectedKeys.value.filter((item) => item !== key);
    return;
  }

  selectedKeys.value = [...selectedKeys.value, key];
};

const restoreDefaults = () => {
  allColumns.value = cloneBeatmapConfig(defaultColumns.value);
  selectedKeys.value = defaultColumns.value.filter((item) => item.visible).map((item) => item.key);
};

const closeDialog = () => {
  localVisible.value = false;
};

const handleClosed = () => {
  emit("update:modelValue", false);
};

const confirmSelect = () => {
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
