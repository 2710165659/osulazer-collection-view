import { Beatmap } from '@/entities/Beatmap';
import { Collection } from '@/entities/Collection';
import { CollectionsList } from '@/entities/CollectionsList';
import { defineStore } from 'pinia';

export const useCollectionsStore = defineStore('collections', {
  state: () => ({
    // 当前选中的规则、收藏夹和谱面
    selectedRuleset: null as string | null,   // 规则短名称，如 "osu", "taiko", "catch", "mania"
    selectedCollection: null as Collection | null,
    selectedBeatmap: null as Beatmap | null,
    // 从 json 里加载的收藏夹列表数据
    CollectionsList: null as CollectionsList | null,
  }),
  getters: {
    getSelectedRuleset: (state) => state.selectedRuleset,
    getSelectedCollection: (state) => state.selectedCollection,
    getSelectedBeatmap: (state) => state.selectedBeatmap,
    getCollectionsList: (state) => state.CollectionsList,
  },
  actions: {
    setSelectedRuleset(ruleset: string | null): void {
      if(ruleset === this.selectedRuleset) return; // 如果选择的规则与当前相同，则不进行任何操作
      // 切换规则时，重置选中的谱面和收藏夹
      this.selectedRuleset = ruleset;
      this.selectedCollection = null;
      this.selectedBeatmap = null;
    },
    setSelectedCollection(collection: Collection | null): void {
      // 切换收藏夹时，重置选中的谱面
      if(collection === this.selectedCollection) return; // 如果选择的收藏夹与当前相同，则不进行任何操作
      this.selectedCollection = collection;
      this.selectedBeatmap = null;
    },
    setSelectedBeatmap(beatmap: Beatmap | null): void {
      if(beatmap === this.selectedBeatmap) return; // 如果选择的谱面与当前相同，则不进行任何操作
      this.selectedBeatmap = beatmap;
    },

    // 从 JSON 数据加载收藏夹列表
    loadCollectionsList(data: any): void {
      this.CollectionsList = new CollectionsList(data);
    }
  }
});