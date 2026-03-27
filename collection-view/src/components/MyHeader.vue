<template>
  <div class="header">
    <div class="file-selector">
      <span style="font-weight: 500; color: #555;">本地数据库文件路径：</span>
      <span class="file-path">{{ dbPath || "未选择文件" }}</span>
    </div>
    <div class="actions">
      <el-button plain @click="selectDatabase" type="primary">浏览</el-button>
      <el-button type="success" plain @click="loadDatabase">加载数据库</el-button>
      <el-button type="warning" plain @click="exportData">导出为 Excel</el-button>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { ref } from "vue";
import { open } from '@tauri-apps/plugin-dialog';
import { Command } from '@tauri-apps/plugin-shell';
import { exists, readFile } from '@tauri-apps/plugin-fs';
import { appDataDir, join } from '@tauri-apps/api/path';
import { useCollectionsStore } from "../store/useCollectionsStore";
import { message } from '@tauri-apps/plugin-dialog';

const collectionsStore = useCollectionsStore();
const dbPath = ref(localStorage.getItem("dbPath") || "");

const selectDatabase = async () => {
  const file = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "Realm Database", extensions: ["realm"] }],
  }) || "";
  dbPath.value = file;
  localStorage.setItem("dbPath", dbPath.value);
};

const loadDatabase = async () => {
  try {
    // 1. 获取输出路径
    const appDir = await appDataDir();
    const outputPath = await join(appDir, 'collections.json');

    // 2. 调用 Sidecar
    // Command.sidecar 参数对应 tauri.conf.json 中 externalBin 的路径
    const command = Command.sidecar('../../extractor/extractor', [dbPath.value, outputPath]);
    const result = await command.execute();

    // 3. 验证执行结果
    if (result.code !== 0) {
      await message(`数据提取失败: ${result.stderr}`, { title: 'Tauri', kind: 'error' });
      return;
    }

    // 4. 验证文件是否存在
    const fileExists = await exists(outputPath);
    if (!fileExists) {
      await message('输出文件未找到，请确保 Sidecar 正常工作', { title: 'Tauri', kind: 'error' });
      return;
    }

    // 5. 读取文件并更新 Store
    const contents = await readFile(outputPath);
    const json = JSON.parse(new TextDecoder('utf-8').decode(contents));
    collectionsStore.loadCollectionsList(json);

    await message('数据库加载成功！', { title: 'Tauri', kind: 'info' });
  } catch (error) {
    await message(`加载数据库时发生错误: ${error instanceof Error ? error.message : String(error)}`, { title: 'Tauri', kind: 'error' });
  }
};

const exportData = async () => {
  console.log("导出数据为excel");
};
</script>

<style scoped>
/* 头部区域 */
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background-color: #ffffff;
  padding: 15px 20px;
  border-radius: 8px;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.05);
}

.file-selector {
  display: flex;
  align-items: center;
  gap: 10px;
}

.file-path {
  min-width: 300px;
  padding: 5px 10px;
  border: 1px solid #ccc;
  border-radius: 4px;
  color: #555;
  background-color: #fff;
  word-break: break-all;
}

.actions {
  display: flex;
  align-items: center;
  gap: 10px;
}
</style>
