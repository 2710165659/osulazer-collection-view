import { defineStore } from 'pinia';

export const useBeatmapConfigStore = defineStore('beatmapConfig', {
  state: () => ({
      originConfig: [
        {key:"beatmapId", label:"BID", visible:true},
        {key:"titleUnicode", label:"歌曲名称（Unicode）", visible:true},
        {key:"artistUnicode", label:"艺术家（Unicode）", visible:false},
        {key:"starRating", label:"难度星级", visible:true},
        {key:"difficultyName", label:"难度名称", visible:true},
        {key:"bpm", label:"BPM", visible:true},
        {key:"statusInt", label:"rank状态", visible:true},
        {key:"mapper", label:"谱师", visible:true},
        {key:"title", label:"歌曲名称", visible:true},
        {key:"artist", label:"艺术家", visible:true},
        {key:"lengthMs", label:"谱面长度（ms）", visible:false},
        {key:"circleSize", label:"CS", visible:false},
        {key:"overallDifficulty", label:"OD", visible:false},
        {key:"approachRate", label:"AR", visible:false},
        {key:"drainRate", label:"HP", visible:false},
        {key:"totalObjectCount", label:"物件总数", visible:false},
        {key:"rulesetName", label:"模式", visible:false},
        {key:"rulesetShortName", label:"模式全称", visible:false},
        {key:"beatmapSetId", label:"SID", visible:false},
        {key:"backgroundUrl", label:"封面图片URL", visible:false},
        {key:"md5", label:"MD5", visible:false},
        {key:"missing", label:"是否缺失", visible:false}
      ] as {key:string, label:string, visible:boolean}[],

      appliedConfig: [] as {key:string, label:string, visible:boolean}[],
  }),
  getters: {
    getVisibleKeys: (state) => state.appliedConfig.filter(item => item.visible).map(item => item.key),
  },
  actions: {
    resetConfig() : void {
      this.appliedConfig = this.originConfig;
      localStorage.setItem('beatmapConfig', JSON.stringify(this.appliedConfig));
    },
    saveConfig(newConfig: {key:string, label:string, visible:boolean}[]) : void {
      this.appliedConfig = newConfig;
      localStorage.setItem('beatmapConfig', JSON.stringify(this.appliedConfig));
    },
    loadConfig() : void {
      const savedConfig = localStorage.getItem('beatmapConfig');
      if (savedConfig) {
        this.appliedConfig = JSON.parse(savedConfig);
      }
    }

  }
});