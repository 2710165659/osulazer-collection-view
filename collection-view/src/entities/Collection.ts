// src/entities/Collection.ts
import { Beatmap } from './Beatmap';

export class Collection {
  id: string;
  name: string;
  lastModified: string;
  items: Beatmap[];

  constructor(data: any) {
    this.id = data.id;
    this.name = data.name;
    this.lastModified = data.lastModified;
    // 将 items 数组里的原始对象转成 Beatmap 实例
    this.items = Array.isArray(data.items) 
      ? data.items.map((item: any) => new Beatmap(item))
      : [];
  }
}