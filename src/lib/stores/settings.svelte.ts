import { LazyStore } from "@tauri-apps/plugin-store";

export interface AppSettings {
  autoCheckUpdate: boolean;
}

const DEFAULT_SETTINGS: AppSettings = {
  autoCheckUpdate: true,
};

const store = new LazyStore("settings.json");
let settings = $state<AppSettings>({ ...DEFAULT_SETTINGS });

export async function loadSettings(): Promise<AppSettings> {
  const autoCheckUpdate = await store.get<boolean>("autoCheckUpdate");
  settings.autoCheckUpdate = autoCheckUpdate ?? DEFAULT_SETTINGS.autoCheckUpdate;
  return settings;
}

export async function setAutoCheckUpdate(value: boolean): Promise<void> {
  await store.set("autoCheckUpdate", value);
  await store.save();
  settings.autoCheckUpdate = value;
}

export function getSettings(): AppSettings {
  return settings;
}
