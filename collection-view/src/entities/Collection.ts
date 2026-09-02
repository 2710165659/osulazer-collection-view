/**
 * 定义左侧收藏夹表格使用的 Rust 汇总结构。
 */
export interface CollectionSummary {
  id: string;
  name: string;
  lastModified: string;
  totalCount: number;
  currentModeCount: number;
  missingCount: number;
}
