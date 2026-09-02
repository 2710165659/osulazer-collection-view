import type { Beatmap } from "@/entities/Beatmap";

/**
 * 定义可持久化的谱面表格列。
 */
export interface BeatmapColumnConfig {
  key: string;
  label: string;
  visible: boolean;
}

export const modeDefinitions = [
  { key: "all", label: "全部" },
  { key: "osu", label: "osu!" },
  { key: "taiko", label: "Taiko" },
  { key: "ctb", label: "Catch" }, // Rust 已将 Realm 的 fruits 归一为 ctb。
  { key: "mania", label: "Mania" },
  { key: "missing", label: "缺失" },
] as const;

export type ModeKey = (typeof modeDefinitions)[number]["key"];

export const modeLabelMap: Record<ModeKey, string> = Object.fromEntries(
  modeDefinitions.map((item) => [item.key, item.label])
) as Record<ModeKey, string>;

/**
 * 深拷贝列配置，避免设置弹窗直接修改 Pinia 中的已应用状态。
 */
export const cloneBeatmapConfig = (
  config: BeatmapColumnConfig[]
): BeatmapColumnConfig[] => config.map((item) => ({ ...item }));

/**
 * 按用户配置顺序返回可见列。
 */
export const getVisibleColumns = (
  config: BeatmapColumnConfig[]
): BeatmapColumnConfig[] => config.filter((item) => item.visible);

/**
 * 将谱面长度从毫秒转换为分秒格式。
 */
const formatLength = (lengthMs: number | null): string => {
  if (lengthMs == null || !Number.isFinite(lengthMs) || lengthMs < 0) {
    return "";
  }
  const totalSeconds = Math.round(lengthMs / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
};

/**
 * 使用与 Python 版本一致的精度格式化普通难度数值。
 */
const formatCompactNumber = (value: number | null): string => {
  if (value == null || !Number.isFinite(value)) {
    return "";
  }
  return value.toFixed(1).replace(/\.0$/, "");
};

/**
 * 按提取器真实状态码返回谱面状态文本。
 */
const formatStatus = (value: number | null): string => {
  const statusTextMap: Record<number, string> = {
    [-2]: "graveyard",
    [-1]: "wip",
    0: "pending",
    1: "ranked",
    2: "approved",
    3: "qualified",
    4: "loved",
  };
  return value == null ? "" : statusTextMap[value] ?? String(value);
};

/**
 * 优先使用 Unicode 艺术家和标题生成列表主名称。
 */
export const getBeatmapNameOriginal = (beatmap: Beatmap): string => {
  if (beatmap.missing) {
    return `[Missing] ${beatmap.md5}`;
  }
  const artist = beatmap.artistUnicode || beatmap.artist;
  const title = beatmap.titleUnicode || beatmap.title;
  return [artist, title].filter(Boolean).join(" - ") || beatmap.md5;
};

/**
 * 生成英文艺术家与标题组合名称。
 */
export const getBeatmapName = (beatmap: Beatmap): string => {
  if (beatmap.missing) {
    return `[Missing] ${beatmap.md5}`;
  }
  return [beatmap.artist, beatmap.title].filter(Boolean).join(" - ") || beatmap.md5;
};

/**
 * 统一处理表格、详情和导出预览中的字段格式。
 */
export const formatBeatmapValue = (
  beatmap: Beatmap,
  key: string
): string => {
  switch (key) {
    case "nameOriginal":
      return getBeatmapNameOriginal(beatmap);
    case "name":
      return getBeatmapName(beatmap);
    case "artistOriginal":
    case "artistUnicode":
      return beatmap.artistUnicode || beatmap.artist;
    case "statusInt":
      return formatStatus(beatmap.statusInt);
    case "lengthMs":
      return formatLength(beatmap.lengthMs);
    case "starRating":
      return beatmap.starRating == null ||
        !Number.isFinite(beatmap.starRating) ||
        beatmap.starRating < 0
        ? ""
        : beatmap.starRating.toFixed(2);
    case "totalObjectCount":
      return beatmap.totalObjectCount == null || beatmap.totalObjectCount < 0
        ? ""
        : String(beatmap.totalObjectCount);
    case "circleSize":
      return formatCompactNumber(beatmap.circleSize);
    case "overallDifficulty":
      return formatCompactNumber(beatmap.overallDifficulty);
    case "approachRate":
      return formatCompactNumber(beatmap.approachRate);
    case "drainRate":
      return formatCompactNumber(beatmap.drainRate);
    case "bpm":
      return formatCompactNumber(beatmap.bpm);
    case "mode":
      return beatmap.missing ? "missing" : beatmap.mode;
    case "missing":
      return beatmap.missing ? "是" : "否";
    default: {
      const value = beatmap[key as keyof Beatmap];
      return value == null ? "" : String(value);
    }
  }
};

/**
 * 按字段内容返回适合 Element Plus 表格的最小列宽。
 */
export const getBeatmapColumnWidth = (key: string): number => {
  const wideColumns = new Set([
    "nameOriginal",
    "name",
    "title",
    "titleUnicode",
    "artist",
    "artistOriginal",
    "artistUnicode",
    "difficultyName",
    "mapper",
    "md5",
  ]);
  const compactColumns = new Set([
    "beatmapId",
    "beatmapSetId",
    "starRating",
    "bpm",
    "circleSize",
    "overallDifficulty",
    "approachRate",
    "drainRate",
    "missing",
    "statusInt",
    "mode",
  ]);
  if (wideColumns.has(key)) return key === "md5" ? 260 : 190;
  if (compactColumns.has(key)) return 92;
  return 128;
};

/**
 * 将 ISO 时间压缩为 Python 版使用的“日期 时间”文本，不额外转换时区。
 */
export const formatDateTime = (value: string): string => {
  const normalized = value.trim();
  const match = normalized.match(/^(\d{4}-\d{2}-\d{2})[T ](\d{2}:\d{2}:\d{2})/);
  return match ? `${match[1]} ${match[2]}` : normalized;
};
