/**
 * 定义 Rust 返回给前端的谱面结构，可空字段与 Realm 提取器契约保持一致。
 */
export interface Beatmap {
  md5: string;
  title: string;
  titleUnicode: string;
  artist: string;
  artistUnicode: string;
  beatmapId: number | null;
  beatmapSetId: number | null;
  starRating: number | null;
  circleSize: number | null;
  overallDifficulty: number | null;
  approachRate: number | null;
  drainRate: number | null;
  totalObjectCount: number | null;
  lengthMs: number | null;
  bpm: number | null;
  statusInt: number | null;
  difficultyName: string;
  mapper: string;
  rulesetShortName: string;
  rulesetName: string;
  backgroundUrl: string;
  missing: boolean;
  mode: string;
}

/**
 * 定义 Rust 按歌曲名称、原语言艺术家和谱师聚合后的谱面组。
 */
export interface BeatmapGroup {
  key: string;
  items: Beatmap[];
}
