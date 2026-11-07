export interface Tile<T> {
  item: T;
  x: number;
  y: number;
  width: number;
  height: number;
}

function worst(areas: number[], side: number): number {
  const sum = areas.reduce((a, b) => a + b, 0);
  const max = Math.max(...areas);
  const min = Math.min(...areas);
  return Math.max((side * side * max) / (sum * sum), (sum * sum) / (side * side * min));
}

export function squarify<T extends { value: number }>(items: readonly T[], width: number, height: number): Tile<T>[] {
  const positive = items.filter((i) => i.value > 0);
  const total = positive.reduce((a, b) => a + b.value, 0);
  if (total <= 0 || width <= 0 || height <= 0) return [];
  const scale = (width * height) / total;
  const rest = [...positive].sort((a, b) => b.value - a.value).map((item) => ({ item, area: item.value * scale }));
  const tiles: Tile<T>[] = [];
  let [x, y, w, h] = [0, 0, width, height];
  let start = 0;
  while (start < rest.length) {
    const side = Math.min(w, h);
    let end = start + 1;
    let best = worst([rest[start].area], side);
    while (end < rest.length) {
      const next = worst(rest.slice(start, end + 1).map((r) => r.area), side);
      if (next > best) break;
      best = next;
      end++;
    }
    const row = rest.slice(start, end);
    const sum = row.reduce((a, r) => a + r.area, 0);
    if (w >= h) {
      const across = sum / h;
      let at = y;
      for (const r of row) {
        const size = r.area / across;
        tiles.push({ item: r.item, x, y: at, width: across, height: size });
        at += size;
      }
      x += across;
      w -= across;
    } else {
      const across = sum / w;
      let at = x;
      for (const r of row) {
        const size = r.area / across;
        tiles.push({ item: r.item, x: at, y, width: size, height: across });
        at += size;
      }
      y += across;
      h -= across;
    }
    start = end;
  }
  return tiles;
}
