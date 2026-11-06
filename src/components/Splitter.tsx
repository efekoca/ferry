import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";

const NARROW = "(max-width: 1000px)";
const MIN = 180;

function read(key: string): number | null {
  try {
    const value = Number(localStorage.getItem(key));
    return Number.isFinite(value) && value > 0 ? value : null;
  } catch {
    return null;
  }
}

function write(key: string, value: number) {
  try {
    localStorage.setItem(key, String(Math.round(value)));
  } catch {
    // storage may be unavailable
  }
}

export function Splitter({ id, side, label, narrowAt = NARROW }: { id: string; side: "start" | "end"; label: string; narrowAt?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const [narrow, setNarrow] = useState(() => window.matchMedia?.(narrowAt).matches ?? false);
  const [size, setSize] = useState<number | null>(null);
  const [percent, setPercent] = useState(50);
  const key = `ferry.split.${id}.${narrow ? "rows" : "cols"}`;

  useEffect(() => {
    const query = window.matchMedia?.(narrowAt);
    if (!query) return;
    const update = () => setNarrow(query.matches);
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, [narrowAt]);

  useEffect(() => setSize(read(key)), [key]);

  useEffect(() => {
    const parent = ref.current?.parentElement;
    if (!parent) return;
    const name = narrow ? "--split-row" : "--split-col";
    const apply = () => {
      parent.style.removeProperty("--split-row");
      parent.style.removeProperty("--split-col");
      const rect = parent.getBoundingClientRect();
      const total = narrow ? rect.height : rect.width;
      if (size !== null) parent.style.setProperty(name, `${total > 0 ? Math.max(MIN, Math.min(total - MIN, size)) : size}px`);
    };
    apply();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(apply);
    observer.observe(parent);
    return () => observer.disconnect();
  }, [size, narrow]);

  useLayoutEffect(() => {
    const parent = ref.current?.parentElement?.getBoundingClientRect();
    const total = parent ? (narrow ? parent.height : parent.width) : 0;
    if (total > 0) setPercent(Math.round((current() / total) * 100));
  });

  const clamp = (value: number) => {
    const rect = ref.current?.parentElement?.getBoundingClientRect();
    const total = rect ? (narrow ? rect.height : rect.width) : value + MIN;
    return Math.max(MIN, Math.min(total - MIN, value));
  };

  const current = () => {
    const pane = side === "start" ? ref.current?.previousElementSibling : ref.current?.nextElementSibling;
    const rect = pane?.getBoundingClientRect();
    return rect ? (narrow ? rect.height : rect.width) : MIN;
  };

  const set = (value: number) => {
    const next = clamp(value);
    setSize(next);
    write(key, next);
  };

  const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
    const parent = ref.current?.parentElement;
    if (!parent) return;
    e.preventDefault();
    e.currentTarget.setPointerCapture?.(e.pointerId);
    const move = (ev: globalThis.PointerEvent) => {
      const rect = parent.getBoundingClientRect();
      const offset = narrow
        ? side === "start"
          ? ev.clientY - rect.top
          : rect.bottom - ev.clientY
        : side === "start"
          ? ev.clientX - rect.left
          : rect.right - ev.clientX;
      set(offset);
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const grow = side === "start" ? ["ArrowRight", "ArrowDown"] : ["ArrowLeft", "ArrowUp"];
    const shrink = side === "start" ? ["ArrowLeft", "ArrowUp"] : ["ArrowRight", "ArrowDown"];
    const step = e.shiftKey ? 96 : 24;
    if (grow.includes(e.key)) set(current() + step);
    else if (shrink.includes(e.key)) set(current() - step);
    else if (e.key === "Home") set(MIN);
    else if (e.key === "End") set(Number.MAX_SAFE_INTEGER);
    else return;
    e.preventDefault();
  };

  return (
    <div
      ref={ref}
      className="splitter"
      role="separator"
      tabIndex={0}
      aria-label={label}
      aria-orientation={narrow ? "horizontal" : "vertical"}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={percent}
      onPointerDown={onPointerDown}
      onKeyDown={onKeyDown}
      onDoubleClick={() => {
        setSize(null);
        try {
          localStorage.removeItem(key);
        } catch {
          // storage may be unavailable
        }
      }}
    />
  );
}
