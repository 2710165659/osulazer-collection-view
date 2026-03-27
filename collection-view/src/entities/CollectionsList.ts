import { Collection } from './Collection';

export class CollectionsList {
  sourcePath: string;
  generatedAt: string;
  collections: Collection[];

  constructor(data: any) {
    this.sourcePath = data.sourcePath;
    this.generatedAt = data.generatedAt;
    // 将 collections 数组里的原始对象转成 Collection 实例
    this.collections = Array.isArray(data.collections)
      ? data.collections.map((col: any) => new Collection(col))
      : [];
  }
}