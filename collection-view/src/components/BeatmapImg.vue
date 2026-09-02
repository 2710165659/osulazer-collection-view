<template>
  <div class="beatmap-img-container">
    <div v-if="coverUrl" class="cover-stage">
      <img class="cover-background" :src="coverUrl" alt="" aria-hidden="true" />
      <el-image :src="coverUrl" fit="contain" class="cover-image">
        <template #error>
          <div class="placeholder cover-error">图片解码失败</div>
        </template>
      </el-image>
    </div>
    <div v-else class="placeholder">
      <span v-if="loading">正在加载封面......</span>
      <span v-else-if="errorText">{{ errorText }}</span>
      <span v-else>请选择一个谱面以显示背景图</span>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { watch } from "vue";

import { useCover } from "@/composables/useCover";
import { useAppStore } from "@/store/useAppStore";

const appStore = useAppStore();
const { coverUrl, loading, errorText, clearCover, loadCover } = useCover();

/**
 * 当前谱面改变时加载对应封面，没有选择时恢复初始提示。
 */
watch(
  () => appStore.selectedBeatmap,
  async (beatmap) => {
    if (!beatmap) {
      clearCover(); // 没有选择谱面时显示初始引导，不把空选择误判为缺少 SID。
      return;
    }
    await loadCover(beatmap?.beatmapSetId ?? null, beatmap?.missing ?? false);
  },
  { immediate: true }
);
</script>

<style scoped>
.beatmap-img-container {
  position: relative;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  padding: 8px 10px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.cover-stage {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
  max-height: 100%;
  border-radius: 10px;
  background: #f8fafc;
  box-shadow: 0 6px 18px rgba(15, 23, 42, 0.1);
}

.cover-background {
  position: absolute;
  inset: -24px;
  width: calc(100% + 48px);
  height: calc(100% + 48px);
  object-fit: cover;
  filter: blur(20px) brightness(0.72) saturate(1.15);
  transform: scale(1.05);
}

.cover-image,
.cover-stage :deep(.el-image) {
  position: relative;
  z-index: 1;
  width: 100%;
  height: 100%;
}

.cover-stage :deep(.el-image__inner) {
  filter: drop-shadow(0 8px 16px rgba(15, 23, 42, 0.24));
}

.cover-error {
  position: relative;
  z-index: 2;
  background: rgba(248, 250, 252, 0.92);
}

.placeholder {
  width: 100%;
  height: 100%;
  min-height: 120px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 12px;
  border-radius: 10px;
  color: #64748b;
  background: #f8fafc;
  text-align: center;
}
</style>
