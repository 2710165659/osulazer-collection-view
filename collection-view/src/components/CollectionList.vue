<template>
  <div style="flex: 1; display: flex; flex-direction: column; gap: 20px;">
    <div class="title" style="display: flex; align-items: center; justify-content: space-between;">
      <h2 style="margin: 0; color: #333;">收藏夹列表</h2>
      <el-radio-group v-model="collectionsStore.selectedRuleset" size="small">
        <el-radio-button label="osu">osu!</el-radio-button>
        <el-radio-button label="taiko">Taiko</el-radio-button>
        <el-radio-button label="catch">Catch</el-radio-button>
        <el-radio-button label="mania">Mania</el-radio-button>
      </el-radio-group>
    </div>
    <el-table :data="tableData" style="width: 100%;" stripe>
      <el-table-column prop="name" label="收藏夹" width="60%" />
      <el-table-column prop="total" label="谱面数量" width="20%" />
      <el-table-column prop="currentModeTotal" label="当前模式谱面数量" width="20%" />
    </el-table>
  </div>
</template>

<script lang="ts" setup>
import { computed } from "vue";
import { useCollectionsStore } from "../store/useCollectionsStore";

const collectionsStore = useCollectionsStore();
const tableData = computed(() => {
  return collectionsStore.CollectionsList?.collections.map((collection) => ({
    name: collection.name,
    total: collection.items.length,
    currentModeTotal: collection.items.filter(
      (item) => item.rulesetShortName === collectionsStore.selectedRuleset
    ).length,
  }));
});

</script>

<style scoped>
.title h2 {
  font-weight: 600;
}
</style>
