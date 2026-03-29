<template>
  <div class="beatmap-img-container">
    <!-- <h3>谱面图片</h3> -->
    <template v-if="url">
      <el-image :src="url" fit="fill" class="cover-image">
        <template #placeholder>
          <div class="placeholder">图片加载中...</div>
        </template>
        <template #error>
          <div class="placeholder">图片加载失败</div>
        </template>
      </el-image>
    </template>
    <div v-else class="placeholder">请选择一个谱面以显示其封面图片</div>
  </div>
</template>

<script lang="ts" setup>
import { computed } from "vue";
import { useCollectionsStore } from "../store/useCollectionsStore";
const collectionsStore = useCollectionsStore();
const url = computed(() =>
  collectionsStore.selectedBeatmap
    ? `https://assets.ppy.sh/beatmaps/${collectionsStore.selectedBeatmap.beatmapSetId}/covers/cover@2x.jpg`
    : null
);
</script>

<style scoped>
.beatmap-img-container {
  height: 100%;
  min-height: 0;
  overflow: hidden;
  padding: 8px 10px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.cover-image,
.el-image {
  border-radius: 8px;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.1);
  width: 100%;
  height: 100%;
  max-height: 100%;
  object-fit: contain;
}

.placeholder {
  width: 100%;
  height: 100%;
  min-height: 120px;
  max-height: 100%;
  background: #f5f7fa;
  color: #999;
  display: flex;
  align-items: center;
  justify-content: center;
  text-align: center;
  padding: 12px;
  border-radius: 8px;
}
</style>
