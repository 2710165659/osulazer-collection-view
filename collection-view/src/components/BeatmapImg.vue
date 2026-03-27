<template>
  <div>
    <h2 style="margin-bottom: 10px; color: #333; font-weight: 600;">谱面图片</h2>
    <el-image :src="url" fit="fill" v-show="url" style="width: 100%; height: 200px; border-radius: 8px;" />
    <div v-if="!url"
      style="width: 100%; height: 200px; display: flex; align-items: center; justify-content: center; background: #f0f0f0; border-radius: 8px; color: #999;">
      暂无图片
    </div>
  </div>
</template>

<script lang="ts" setup>
import { useCollectionsStore } from "../store/useCollectionsStore";
const collectionsStore = useCollectionsStore();
const url = collectionsStore.selectedBeatmap ? `https://assets.ppy.sh/beatmaps/${collectionsStore.selectedBeatmap.beatmapSetId}/covers/cover@2x.jpg` : null;
</script>

<style scoped></style>
