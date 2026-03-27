<template>
  <div>
    <div class="head">
      <h2>谱面列表</h2>
      <el-button type="primary" plain @click="openSettings">
        <el-icon>
          <Setting />
        </el-icon>
      </el-button>
      <el-table :data="tableData" style="width: 100%">
        <el-table-column v-for="key in shownKeys" :key="key" :prop="key" :label="getLabel(key)" />
      </el-table>
    </div>
    <!-- <BeatmapConfigModal :visible="modalVisible" @close="closeModal" @confirm="confirmConfig" /> -->
  </div>
</template>

<script lang="ts" setup>
import { Setting } from "@element-plus/icons-vue";
import { computed, ref } from "vue";
import { useCollectionsStore } from "../store/useCollectionsStore";
import { useBeatmapConfigStore } from "@/store/useBeatmapConfigStore";

const collectionsStore = useCollectionsStore();
const beatmapConfigStore = useBeatmapConfigStore();

const modalVisible = ref(false);

const openSettings = () => {
  modalVisible.value = true;
};

const closeModal = () => {
  modalVisible.value = false;
};

const confirmConfig = (newConfig: { key: string; label: string; visible: boolean }[]) => {
  beatmapConfigStore.saveConfig(newConfig);
  modalVisible.value = false;
};

beatmapConfigStore.loadConfig();
const shownKeys = computed(() => {
  return beatmapConfigStore.getVisibleKeys;
});

const getLabel = (key: string) => {
  const item = beatmapConfigStore.appliedConfig.find(c => c.key === key);
  return item ? item.label : key;
};

const tableData = computed(() => {
  return collectionsStore.selectedCollection?.items
    .filter((item) => item.rulesetShortName === collectionsStore.selectedRuleset)
    .map((item) => {
      const obj: any = {};
      beatmapConfigStore.appliedConfig.forEach(config => {
        obj[config.key] = (item as any)[config.key];
      });
      return obj;
    }) || [];
});

</script>

<style scoped>
.head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 20px;
}
</style>
