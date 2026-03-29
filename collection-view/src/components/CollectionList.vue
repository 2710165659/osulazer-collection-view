<template>
  <div class="collection-list-container">
    <div class="title">
      <div class="title-text">
        <h3>收藏夹列表</h3>
        <p>{{ collectionCountText }}</p>
      </div>
      <el-radio-group v-model="selectedRuleset" size="small">
        <el-radio-button label="osu">osu!</el-radio-button>
        <el-radio-button label="taiko">Taiko</el-radio-button>
        <el-radio-button label="catch">Catch</el-radio-button>
        <el-radio-button label="mania">Mania</el-radio-button>
      </el-radio-group>
    </div>

    <div class="table-wrapper">
      <el-table
        ref="tableRef"
        :data="tableData"
        :empty-text="emptyText"
        height="100%"
        border
        highlight-current-row
        row-key="id"
        @row-click="handleRowClick"
      >
        <el-table-column prop="name" label="收藏夹" min-width="190" show-overflow-tooltip />
        <el-table-column prop="total" label="总数" width="72" align="center" />
        <el-table-column prop="currentModeTotal" label="当前模式" width="88" align="center" />
        <el-table-column prop="lastModified" label="更新时间" min-width="120" show-overflow-tooltip />
      </el-table>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { computed, ref, watch } from "vue";
import { useCollectionsStore } from "@/store/useCollectionsStore";

const collectionsStore = useCollectionsStore();
const tableRef = ref();

const selectedRuleset = computed({
  get: () => collectionsStore.selectedRuleset ?? "osu",
  set: (value: string) => collectionsStore.setSelectedRuleset(value),
});

const tableData = computed(() => {
  return (
    collectionsStore.CollectionsList?.collections.map((collection) => ({
      id: collection.id,
      name: collection.name,
      total: collection.items.length,
      lastModified: collection.lastModified,
      currentModeTotal: collection.items.filter(
        (item) => item.rulesetShortName === collectionsStore.selectedRuleset
      ).length,
      raw: collection,
    })) ?? []
  );
});

const collectionCountText = computed(() => `共 ${tableData.value.length} 个收藏夹`);

const emptyText = computed(() => {
  return collectionsStore.CollectionsList ? "当前没有可展示的收藏夹" : "请先加载数据库";
});

const handleRowClick = (row: (typeof tableData.value)[number]) => {
  collectionsStore.setSelectedCollection(row.raw);
};

// 左侧收藏夹变化时同步高亮，避免用户在两个列表之间切换时状态丢失。
watch(
  () => collectionsStore.selectedCollection,
  (collection) => {
    const currentRow = tableData.value.find((item) => item.id === collection?.id);
    tableRef.value?.setCurrentRow(currentRow ?? undefined);
  },
  { immediate: true }
);

watch(
  [() => collectionsStore.CollectionsList, () => collectionsStore.selectedRuleset],
  () => {
    if (!collectionsStore.CollectionsList || !tableData.value.length) {
      collectionsStore.setSelectedCollection(null);
      tableRef.value?.setCurrentRow(undefined);
      return;
    }

    const currentCollection = collectionsStore.selectedCollection;
    if (currentCollection && tableData.value.some((item) => item.id === currentCollection.id)) {
      const row = tableData.value.find((item) => item.id === currentCollection.id);
      tableRef.value?.setCurrentRow(row ?? undefined);
      return;
    }

    const firstAvailable =
      tableData.value.find((item) => item.currentModeTotal > 0) ?? tableData.value[0];
    collectionsStore.setSelectedCollection(firstAvailable.raw);
    tableRef.value?.setCurrentRow(firstAvailable);
  },
  { immediate: true }
);
</script>

<style scoped>
.collection-list-container {
  height: 100%;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.title {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  min-width: 0;
}

.title-text {
  min-width: 0;
}

h3 {
  color: #111827;
  font-weight: 700;
  font-size: 15px;
  margin: 0 0 4px;
}

.title-text p {
  margin: 0;
  color: #6b7280;
  font-size: 12px;
}

.table-wrapper {
  flex: 1;
  min-height: 0;
  min-width: 0;
  max-height: 100%;
  width: 100%;
  max-width: 100%;
  overflow: auto;
}

:deep(.el-table) {
  --el-table-header-bg-color: #f8fafc;
  --el-table-row-hover-bg-color: #eff6ff;
  border-radius: 12px;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  font-size: 12px;
  margin: 0;
}

:deep(.el-table th.el-table__cell) {
  padding: 8px 0;
  color: #475569;
  font-weight: 700;
}

:deep(.el-table td.el-table__cell) {
  padding: 7px 0;
}
</style>
