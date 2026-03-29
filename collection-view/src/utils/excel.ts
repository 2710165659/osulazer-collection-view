import { writeFile } from "@tauri-apps/plugin-fs";
import type { Collection } from "@/entities/Collection";
import type { Beatmap } from "@/entities/Beatmap";
import type { BeatmapColumnConfig } from "@/utils/beatmapColumns";
import { formatBeatmapValue } from "@/utils/beatmapColumns";
import type { WorkBook, WorkSheet } from "xlsx";

const INVALID_SHEET_NAME = /[\\/?*\[\]:]/g;

const sanitizeSheetName = (name: string, fallback: string): string => {
  const safeName = name.replace(INVALID_SHEET_NAME, "_").trim();
  return (safeName || fallback).slice(0, 31);
};

const toRows = (beatmaps: Beatmap[], columns: BeatmapColumnConfig[]): (string | number)[][] =>
  beatmaps.map((beatmap) => columns.map((column) => formatBeatmapValue(beatmap, column.key)));

const getColumnWidths = (
  headers: string[],
  rows: (string | number)[][]
): Array<{ wch: number }> => {
  return headers.map((header, index) => {
    const maxCellLength = rows.reduce((max, row) => {
      return Math.max(max, String(row[index] ?? "").length);
    }, header.length);

    return { wch: Math.min(Math.max(maxCellLength + 2, 10), 40) };
  });
};

const createWorksheet = async (
  beatmaps: Beatmap[],
  columns: BeatmapColumnConfig[]
): Promise<WorkSheet> => {
  const XLSX = await import("xlsx");
  const headers = columns.map((column) => column.label);
  const rows = toRows(beatmaps, columns);
  const worksheet = XLSX.utils.aoa_to_sheet([headers, ...rows]);

  worksheet["!cols"] = getColumnWidths(headers, rows);
  return worksheet;
};

export const createWorkbookForCollections = async (
  collections: Collection[],
  columns: BeatmapColumnConfig[],
  getBeatmaps: (collection: Collection) => Beatmap[]
) => {
  const XLSX = await import("xlsx");
  const workbook = XLSX.utils.book_new();

  for (const [index, collection] of collections.entries()) {
    const sheetName = sanitizeSheetName(collection.name, `Collection${index + 1}`);
    const worksheet = await createWorksheet(getBeatmaps(collection), columns);
    XLSX.utils.book_append_sheet(workbook, worksheet, sheetName, true);
  }

  return workbook;
};

export const saveWorkbook = async (filePath: string, workbook: WorkBook) => {
  const XLSX = await import("xlsx");
  // 使用 array 输出后交给 Tauri 写文件，兼容桌面端保存流程。
  const buffer = XLSX.write(workbook, { bookType: "xlsx", type: "array" }) as ArrayBuffer;
  await writeFile(filePath, new Uint8Array(buffer));
};
