<template>
  <div id="container"
    style="height: 100vh; display: flex; flex-direction: column; gap: 20px; background-color: #f5f5f5; padding: 20px; box-sizing: border-box;">
    <!-- 头部区域 -->
    <MyHeader style="flex: 1;" />
    <!-- 主体区域 -->
    <div class="main" style="flex: 9; display: flex; gap: 20px;">
      <div class="main-left" style="flex: 1; display: flex; flex-direction: column; gap: 20px;">
        <CollectionList style="flex: 2;" class="card" />
        <BeatmapImg style="flex: 1;" class="card" />
      </div>
      <div class="main-right" style="flex: 3;">
        <BeatmapList class="card" />
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

.card {
  background: white;
  border-radius: 8px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  padding: 20px;
}
</style>