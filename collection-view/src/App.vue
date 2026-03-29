<template>
  <div id="container">
    <!-- 头部区域 -->
    <MyHeader class="header card" />
    <!-- 主体区域 -->
    <div class="main">
      <div class="main-left">
        <CollectionList class="card collection-list" />
        <BeatmapImg class="card beatmap-img" />
      </div>
      <div class="main-right">
        <BeatmapList class="card beatmap-list" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import BeatmapList from "./components/BeatmapList.vue";
import CollectionList from "./components/CollectionList.vue";
import MyHeader from "./components/MyHeader.vue";
import BeatmapImg from "./components/BeatmapImg.vue";


</script>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
  font-family: "Microsoft YaHei", sans-serif;
}

#container {
  height: 100vh;
  overflow-y: hidden;
  display: flex;
  flex-direction: column;
  gap: 12px;
  background:
    radial-gradient(circle at top left, rgba(59, 130, 246, 0.12), transparent 26%),
    linear-gradient(180deg, #f8fafc 0%, #eef2f7 100%);
  padding: 10px;
}

.header {
  flex: 0 0 88px;
}

.main {
  flex: 1;
  display: flex;
  gap: 12px;
  min-height: 0;
  min-width: 0;
}

.main-left {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  min-height: 0;
  min-width: 0;
}

.main-right {
  flex: 2.3;
  min-height: 0;
  min-width: 0;
}

.collection-list {
  flex: 1.35;
  min-height: 0;
  min-width: 0;
}

.beatmap-img {
  flex: 0.9;
  min-height: 0;
  min-width: 0;
}

.card {
  background: rgba(255, 255, 255, 0.96);
  border-radius: 14px;
  box-shadow: 0 12px 28px rgba(15, 23, 42, 0.08);
  border: 1px solid rgba(226, 232, 240, 0.9);
  padding: 14px;
}
</style>
