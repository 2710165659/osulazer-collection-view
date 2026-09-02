import { invoke } from "@tauri-apps/api/core";
import { defineStore } from "pinia";

import type { Beatmap, BeatmapGroup } from "@/entities/Beatmap";
import type { CollectionSummary } from "@/entities/Collection";
import {
  cloneBeatmapConfig,
  getVisibleColumns,
  type BeatmapColumnConfig,
  type ModeKey,
} from "@/utils/beatmapColumns";

interface AppSettings {
  realmPath: string;
  selectedMode: ModeKey;
  columns: BeatmapColumnConfig[];
  pageSize: number;
}

interface LoadResult {
  sourcePath: string;
  generatedAt: string;
  collectionCount: number;
}

interface BeatmapPage {
  beatmapTotal: number;
  groupTotal: number;
  groups: BeatmapGroup[];
}

let settingsWriteQueue: Promise<void> = Promise.resolve(); // 设置写入按前端调用顺序串行，确保最后一次修改最终落盘。
const validPageSizes = new Set([10, 20, 50, 100, 200]); // 前端只接受界面公开的五个分页值。

/**
 * 将未知错误统一转换为可展示文本。
 */
const errorMessage = (error: unknown): string =>
  error instanceof Error ? error.message : String(error);

/**
 * 集中管理数据库、设置、筛选、排序、分页和当前选择。
 */
export const useAppStore = defineStore("app", {
  state: () => ({
    initialized: false,
    loaded: false,
    loadingDatabase: false,
    loadingCollections: false,
    loadingBeatmaps: false,
    exporting: false,
    realmPath: "",
    sourcePath: "",
    generatedAt: "",
    collectionCount: 0,
    selectedMode: "osu" as ModeKey,
    collections: [] as CollectionSummary[],
    selectedCollection: null as CollectionSummary | null,
    beatmaps: [] as Beatmap[], // 保留当前页原始谱面，供选中、详情和兼容逻辑使用。
    beatmapGroups: [] as BeatmapGroup[],
    beatmapTotal: 0,
    beatmapGroupTotal: 0,
    selectedBeatmap: null as Beatmap | null,
    page: 1,
    pageSize: 20,
    sortColumn: null as string | null,
    sortDescending: false,
    columns: [] as BeatmapColumnConfig[],
    defaultColumns: [] as BeatmapColumnConfig[],
    collectionRequestId: 0,
    beatmapRequestId: 0,
    lastError: "",
  }),

  getters: {
    /**
     * 返回当前可见列，并保留用户配置的顺序。
     */
    visibleColumns: (state): BeatmapColumnConfig[] =>
      getVisibleColumns(state.columns),

    /**
     * 返回当前页面中的谱面组数量。
     */
    currentPageCount: (state): number => state.beatmapGroups.length,
  },

  actions: {
    /**
     * 从 Rust 设置文件初始化数据库路径、模式、列配置和分页大小。
     */
    async initialize(): Promise<void> {
      if (this.initialized) return;
      try {
        const [settings, defaults] = await Promise.all([
          invoke<AppSettings>("load_settings"),
          invoke<BeatmapColumnConfig[]>("get_default_columns"),
        ]);
        this.realmPath = settings.realmPath;
        this.selectedMode = settings.selectedMode;
        this.columns = cloneBeatmapConfig(settings.columns);
        this.defaultColumns = cloneBeatmapConfig(defaults);
        this.pageSize = validPageSizes.has(settings.pageSize) ? settings.pageSize : 20; // 后端异常返回同样回退，避免分页控件出现未公开值。
      } catch (error) {
        this.lastError = `读取应用设置失败：${errorMessage(error)}`;
      } finally {
        this.initialized = true;
      }
    },

    /**
     * 把当前完整设置交给 Rust 原子写入 JSON 文件。
     */
    async persistSettings(): Promise<void> {
      const settings: AppSettings = {
        realmPath: this.realmPath,
        selectedMode: this.selectedMode,
        columns: cloneBeatmapConfig(this.columns),
        pageSize: this.pageSize,
      };
      const writeTask = settingsWriteQueue
        .catch(() => undefined)
        .then(async () => {
          await invoke<AppSettings>("save_settings", { settings });
        });
      settingsWriteQueue = writeTask;
      await writeTask;
    },

    /**
     * 更新用户选择的 Realm 路径，清理旧 Rust 数据并立即持久化。
     */
    async setRealmPath(path: string): Promise<void> {
      const pathChanged = path !== this.realmPath;
      this.realmPath = path;
      if (this.loaded && path !== this.sourcePath) {
        this.loaded = false; // 路径改变后旧数据库结果不再代表当前选择。
        this.sourcePath = "";
        this.generatedAt = "";
        this.collectionCount = 0;
        this.collections = [];
        this.selectedCollection = null;
        this.beatmaps = [];
        this.beatmapGroups = [];
        this.beatmapTotal = 0;
        this.beatmapGroupTotal = 0;
        this.selectedBeatmap = null;
        this.collectionRequestId += 1;
        this.beatmapRequestId += 1;
      }
      if (pathChanged) {
        await invoke("clear_database"); // 前后端同时撤销旧数据库，避免新路径尚未加载时仍可访问旧状态。
      }
      await this.persistSettings();
    },

    /**
     * 调用 Rust 加载 Realm，随后刷新收藏夹和第一页谱面。
     */
    async loadDatabase(): Promise<void> {
      if (!this.realmPath.trim()) {
        throw new Error("请先选择一个 .realm 数据库文件。");
      }
      if (this.loadingDatabase) return;

      this.loadingDatabase = true;
      this.loaded = false; // 新数据库完成前禁止查询和导出旧数据。
      this.lastError = "";
      this.collectionRequestId += 1; // 新数据库加载开始时立即作废旧收藏夹请求。
      this.beatmapRequestId += 1; // 防止旧数据库分页结果在加载期间回写前端。
      this.collections = [];
      this.selectedCollection = null;
      this.beatmaps = [];
      this.beatmapGroups = [];
      this.beatmapTotal = 0;
      this.beatmapGroupTotal = 0;
      this.selectedBeatmap = null;
      try {
        const result = await invoke<LoadResult>("load_database", {
          realmPath: this.realmPath,
        });
        this.loaded = true;
        this.sourcePath = result.sourcePath;
        this.generatedAt = result.generatedAt;
        this.collectionCount = result.collectionCount;
        this.page = 1;
        this.sortColumn = null;
        this.sortDescending = false;
        await this.refreshCollections();
      } catch (error) {
        this.loaded = false;
        this.sourcePath = "";
        this.generatedAt = "";
        this.collectionCount = 0;
        this.collections = [];
        this.beatmaps = [];
        this.beatmapGroups = [];
        this.beatmapTotal = 0;
        this.beatmapGroupTotal = 0;
        this.selectedCollection = null;
        this.selectedBeatmap = null;
        this.collectionRequestId += 1; // 加载失败后再次作废可能仍在结束阶段的旧请求。
        this.beatmapRequestId += 1;
        this.lastError = errorMessage(error);
        throw error;
      } finally {
        this.loadingDatabase = false;
      }
    },

    /**
     * 切换模式并重新获取 Rust 汇总；All 和 missing 都是正式模式。
     */
    async setMode(mode: ModeKey): Promise<void> {
      if (mode === this.selectedMode) return;
      this.selectedMode = mode;
      this.page = 1;
      this.selectedCollection = null;
      this.selectedBeatmap = null;
      this.beatmapRequestId += 1; // 切换到空模式时也必须阻止旧谱面请求回写。
      this.beatmaps = [];
      this.beatmapGroups = [];
      this.beatmapTotal = 0;
      this.beatmapGroupTotal = 0;
      if (this.loaded) {
        await this.refreshCollections();
      }
      await this.persistSettings();
    },

    /**
     * 从 Rust 获取当前模式下非空收藏夹，并尽量保留原选择。
     */
    async refreshCollections(): Promise<void> {
      if (!this.loaded) return;
      const requestId = ++this.collectionRequestId;
      const previousId = this.selectedCollection?.id;
      this.loadingCollections = true;
      this.lastError = "";
      try {
        const rows = await invoke<CollectionSummary[]>("get_collection_summaries", {
          mode: this.selectedMode,
        });
        if (requestId !== this.collectionRequestId) return;

        this.collections = rows;
        this.selectedCollection =
          rows.find((item) => item.id === previousId) ?? rows[0] ?? null;
        this.page = 1;
      } catch (error) {
        if (requestId !== this.collectionRequestId) return;
        this.collections = [];
        this.selectedCollection = null;
        this.beatmaps = [];
        this.beatmapGroups = [];
        this.beatmapTotal = 0;
        this.beatmapGroupTotal = 0;
        this.selectedBeatmap = null;
        this.beatmapRequestId += 1; // 收藏夹请求失败后作废仍在运行的旧谱面请求。
        this.lastError = errorMessage(error);
        throw error;
      } finally {
        if (requestId === this.collectionRequestId) {
          this.loadingCollections = false;
        }
      }
      if (requestId === this.collectionRequestId) {
        await this.refreshBeatmaps(true);
      }
    },

    /**
     * 选择收藏夹并加载该收藏夹第一页谱面。
     */
    async selectCollection(collection: CollectionSummary): Promise<void> {
      if (collection.id === this.selectedCollection?.id) return;
      this.selectedCollection = collection;
      this.selectedBeatmap = null;
      this.beatmaps = [];
      this.beatmapGroups = [];
      this.beatmapTotal = 0;
      this.beatmapGroupTotal = 0;
      this.page = 1;
      await this.refreshBeatmaps(true);
    },

    /**
     * 从 Rust 获取当前筛选、排序和分页下的谱面数据。
     */
    async refreshBeatmaps(selectFirst = false): Promise<void> {
      const requestId = ++this.beatmapRequestId; // 即使没有收藏夹，也要用新编号作废旧分页请求。
      const collection = this.selectedCollection;
      if (!this.loaded || !collection) {
        this.beatmaps = [];
        this.beatmapGroups = [];
        this.beatmapTotal = 0;
        this.beatmapGroupTotal = 0;
        this.selectedBeatmap = null;
        this.loadingBeatmaps = false;
        return;
      }

      const previousMd5 = this.selectedBeatmap?.md5;
      this.loadingBeatmaps = true;
      this.lastError = "";
      try {
        const result = await invoke<BeatmapPage>("query_beatmaps", {
          query: {
            collectionId: collection.id,
            mode: this.selectedMode,
            sortColumn: this.sortColumn,
            descending: this.sortDescending,
            page: this.page,
            pageSize: this.pageSize,
          },
        });
        if (requestId !== this.beatmapRequestId) return;

        this.beatmapGroups = result.groups;
        this.beatmaps = result.groups.flatMap((group) => group.items);
        this.beatmapTotal = result.beatmapTotal;
        this.beatmapGroupTotal = result.groupTotal;
        const retained = !selectFirst
          ? this.beatmaps.find((item) => item.md5 === previousMd5)
          : undefined;
        this.selectedBeatmap = retained ?? this.beatmaps[0] ?? null;
      } catch (error) {
        if (requestId !== this.beatmapRequestId) return;
        this.beatmaps = [];
        this.beatmapGroups = [];
        this.beatmapTotal = 0;
        this.beatmapGroupTotal = 0;
        this.selectedBeatmap = null;
        this.lastError = errorMessage(error);
        throw error;
      } finally {
        if (requestId === this.beatmapRequestId) {
          this.loadingBeatmaps = false;
        }
      }
    },

    /**
     * 更新当前高亮谱面。
     */
    selectBeatmap(beatmap: Beatmap | null): void {
      this.selectedBeatmap = beatmap;
    },

    /**
     * 应用 Element Plus 的三态排序并让 Rust 重新排序。
     */
    async applySort(
      column: string | null,
      order: "ascending" | "descending" | null
    ): Promise<void> {
      const nextColumn = order ? column : null;
      const nextDescending = order === "descending";
      if (
        nextColumn === this.sortColumn &&
        nextDescending === this.sortDescending
      ) {
        return; // clearSort 可能再次触发同一事件，这里避免重复请求 Rust。
      }
      this.sortColumn = nextColumn;
      this.sortDescending = nextDescending;
      this.page = 1;
      await this.refreshBeatmaps(true);
    },

    /**
     * 切换分页并获取对应页数据。
     */
    async setPage(page: number): Promise<void> {
      if (page === this.page) return;
      this.page = page;
      await this.refreshBeatmaps(true);
    },

    /**
     * 修改每页数量、持久化设置并返回第一页。
     */
    async setPageSize(pageSize: number): Promise<void> {
      this.pageSize = validPageSizes.has(pageSize) ? pageSize : 20; // 非法调用统一回退到默认二十组。
      this.page = 1;
      await this.persistSettings();
      await this.refreshBeatmaps(true);
    },

    /**
     * 保存新的列显示顺序和可见状态。
     */
    async setColumns(columns: BeatmapColumnConfig[]): Promise<void> {
      this.columns = cloneBeatmapConfig(columns);
      const sortColumnIsVisible = this.columns.some(
        (column) => column.key === this.sortColumn && column.visible
      );
      if (this.sortColumn && !sortColumnIsVisible) {
        this.sortColumn = null; // 隐藏当前排序列时恢复默认顺序，保持列表状态和表头一致。
        this.sortDescending = false;
        this.page = 1;
        await this.refreshBeatmaps(true);
      }
      await this.persistSettings();
    },
  },
});
