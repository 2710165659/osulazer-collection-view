import type { Beatmap } from "@/entities/Beatmap";

export type BeatmapColumnConfig = {
  key: string;
  label: string;
  visible: boolean;
};

const statusTextMap: Record<number, string> = {
  0: "Graveyard",
  1: "WIP",
  2: "Pending",
  3: "Ranked",
  4: "Approved",
  5: "Qualified",
  6: "Loved",
};

export const rulesetOrder = ["osu", "taiko", "catch", "mania"] as const;

export const rulesetLabelMap: Record<(typeof rulesetOrder)[number], string> = {
  osu: "osu!",
  taiko: "Taiko",
  catch: "Catch",
  mania: "Mania",
};

export const cloneBeatmapConfig = (config: BeatmapColumnConfig[]): BeatmapColumnConfig[] =>
  config.map((item) => ({ ...item }));

export const getVisibleColumns = (config: BeatmapColumnConfig[]): BeatmapColumnConfig[] =>
  config.filter((item) => item.visible);

const formatLength = (lengthMs: number): string => {
  const totalSeconds = Math.max(0, Math.floor(lengthMs / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
};

// 统一处理表格展示和导出，避免两边格式不一致。
export const formatBeatmapValue = (beatmap: Beatmap, key: string): string | number => {
  const value = beatmap[key as keyof Beatmap];

  switch (key) {
    case "statusInt":
      return statusTextMap[Number(value)] ?? String(value ?? "");
    case "lengthMs":
      return formatLength(Number(value));
    case "starRating":
      return Number(value).toFixed(2);
    case "missing":
      return value ? "是" : "否";
    default:
      return value == null ? "" : String(value);
  }
};
