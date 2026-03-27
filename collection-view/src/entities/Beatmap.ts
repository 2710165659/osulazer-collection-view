// src/entities/Beatmap.ts
export class Beatmap {
  md5: string; // 谱面文件的 MD5 值，唯一标识一个谱面
  title: string; // 歌曲名称
  titleUnicode: string; // 歌曲名称（Unicode 版本）
  artist: string; // 艺术家
  artistUnicode: string; // 艺术家（Unicode 版本）
  beatmapId: number; // BID
  beatmapSetId: number; // SID
  starRating: number; // 难度星级
  circleSize: number; // CS
  overallDifficulty: number; // OD
  approachRate: number; // AR
  drainRate: number; // HP
  totalObjectCount: number; // 物件总数
  lengthMs: number; // 谱面长度（毫秒）
  bpm: number; // BPM
  statusInt: number; // rank 状态，0 = Graveyard, 1 = WIP, 2 = Pending, 3 = Ranked, 4 = Approved, 5 = Qualified, 6 = Loved
  difficultyName: string; // 难度名称
  mapper: string; // 谱师
  rulesetShortName: string; // 模式简称
  rulesetName: string; // 模式名称
  backgroundUrl: string; // 封面图片URL
  missing: boolean; // 是否缺失（在本地找不到对应的谱面文件）

  constructor(data: any) {
    this.md5 = data.md5;
    this.title = data.title;
    this.titleUnicode = data.titleUnicode;
    this.artist = data.artist;
    this.artistUnicode = data.artistUnicode;
    this.beatmapId = data.beatmapId;
    this.beatmapSetId = data.beatmapSetId;
    this.starRating = data.starRating;
    this.circleSize = data.circleSize;
    this.overallDifficulty = data.overallDifficulty;
    this.approachRate = data.approachRate;
    this.drainRate = data.drainRate;
    this.totalObjectCount = data.totalObjectCount;
    this.lengthMs = data.lengthMs;
    this.bpm = data.bpm;
    this.statusInt = data.statusInt;
    this.difficultyName = data.difficultyName;
    this.mapper = data.mapper;
    this.rulesetShortName = data.rulesetShortName;
    this.rulesetName = data.rulesetName;
    this.backgroundUrl = data.backgroundUrl;
    this.missing = data.missing;
  }
}