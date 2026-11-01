import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type ThemeMode = "system" | "light" | "dark";

let mode: ThemeMode = "system";
const media = window.matchMedia("(prefers-color-scheme: light)");

function apply() {
  const light = mode === "light" || (mode === "system" && media.matches);
  const root = document.documentElement;
  root.dataset.theme = light ? "light" : "dark";
  root.style.colorScheme = light ? "light" : "dark";
}

export async function initTheme() {
  try {
    const config = await invoke<{ theme?: ThemeMode }>("config_get");
    mode = config.theme ?? "system";
  } catch {
    mode = "system";
  }
  apply();

  media.addEventListener("change", () => {
    if (mode === "system") {
      apply();
    }
  });

  await listen<{ theme: ThemeMode }>("theme-changed", (event) => {
    mode = event.payload.theme;
    apply();
  });
}
