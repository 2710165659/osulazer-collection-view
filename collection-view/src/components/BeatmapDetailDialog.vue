<template>
  <el-dialog
    :model-value="modelValue"
    title="谱面详情"
    width="900px"
    destroy-on-close
    @update:model-value="emit('update:modelValue', $event)"
  >
    <template v-if="beatmap">
      <div class="detail-cover">
        <el-image v-if="coverUrl" :src="coverUrl" fit="contain" />
        <div v-else class="cover-placeholder">
          {{ loading ? "正在加载背景图..." : errorText || "暂无可用图片" }}
        </div>
      </div>

      <h2>{{ getBeatmapNameOriginal(beatmap) }}</h2>
      <div class="detail-grid">
        <div v-for="field in detailFields" :key="field.label" class="detail-field">
          <span class="field-label">{{ field.label }}</span>
          <span class="field-value">{{ field.value || "-" }}</span>
        </div>
      </div>
      <div class="md5-field">
        <span class="field-label">MD5</span>
        <span class="field-value">{{ beatmap.md5 || "-" }}</span>
      </div>
    </template>
  </el-dialog>
</template>

<script lang="ts" setup>
import { computed, watch } from "vue";

import { useCover } from "@/composables/useCover";
import type { Beatmap } from "@/entities/Beatmap";
import {
  formatBeatmapValue,
  getBeatmapNameOriginal,
} from "@/utils/beatmapColumns";

const props = defineProps<{
  modelValue: boolean;
  beatmap: Beatmap | null;
}>();

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
}>();

const { coverUrl, loading, errorText, loadCover } = useCover();

/**
 * 将当前谱面转换为详情弹窗使用的标签和值列表。
 */
const detailFields = computed(() => {
  if (!props.beatmap) return [];
  return [
    { label: "名称", value: formatBeatmapValue(props.beatmap, "name") },
    { label: "艺术家（原语言）", value: formatBeatmapValue(props.beatmap, "artistOriginal") },
    { label: "谱师", value: props.beatmap.mapper },
    { label: "难度名", value: props.beatmap.difficultyName },
    { label: "模式", value: formatBeatmapValue(props.beatmap, "mode") },
    { label: "状态", value: formatBeatmapValue(props.beatmap, "statusInt") },
    { label: "难度", value: formatBeatmapValue(props.beatmap, "starRating") },
    { label: "长度", value: formatBeatmapValue(props.beatmap, "lengthMs") },
    { label: "BPM", value: formatBeatmapValue(props.beatmap, "bpm") },
    { label: "Note数", value: formatBeatmapValue(props.beatmap, "totalObjectCount") },
    { label: "BID", value: formatBeatmapValue(props.beatmap, "beatmapId") },
    { label: "SID", value: formatBeatmapValue(props.beatmap, "beatmapSetId") },
    { label: "CS", value: formatBeatmapValue(props.beatmap, "circleSize") },
    { label: "AR", value: formatBeatmapValue(props.beatmap, "approachRate") },
    { label: "OD", value: formatBeatmapValue(props.beatmap, "overallDifficulty") },
    { label: "HP", value: formatBeatmapValue(props.beatmap, "drainRate") },
  ];
});

watch(
  [() => props.modelValue, () => props.beatmap],
  async ([visible, beatmap]) => {
    if (!visible || !beatmap) return;
    await loadCover(beatmap.beatmapSetId, beatmap.missing);
  },
  { immediate: true }
);
</script>

<style scoped>
.detail-cover {
  height: 250px;
  margin-bottom: 16px;
  overflow: hidden;
  border-radius: 12px;
  background: #f8fafc;
}

.detail-cover :deep(.el-image) {
  width: 100%;
  height: 100%;
}

.cover-placeholder {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #64748b;
}

h2 {
  margin: 0 0 16px;
  color: #0f172a;
  font-size: 20px;
}

.detail-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 14px;
}

.detail-field,
.md5-field {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.md5-field {
  margin-top: 16px;
}

.field-label {
  color: #64748b;
  font-size: 12px;
}

.field-value {
  overflow-wrap: anywhere;
  color: #0f172a;
  font-size: 14px;
}
</style>
