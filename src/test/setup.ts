import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach } from "vitest";

beforeEach(() => {
  localStorage.setItem("ferry.lang", JSON.stringify("tr"));
});

afterEach(() => {
  cleanup();
  localStorage.clear();
});

window.matchMedia ??= ((query: string) => ({
  matches: false,
  media: query,
  onchange: null,
  addEventListener: () => {},
  removeEventListener: () => {},
  addListener: () => {},
  removeListener: () => {},
  dispatchEvent: () => false,
})) as unknown as typeof window.matchMedia;

Element.prototype.scrollIntoView ??= function () {};
globalThis.CSS ??= { escape: (s: string) => s } as unknown as typeof CSS;
