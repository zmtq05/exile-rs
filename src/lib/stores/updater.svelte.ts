import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export interface UpdateState {
  checking: boolean;
  available: boolean;
  downloading: boolean;
  progress: number;
  update: Update | null;
  error: string | null;
}

let state = $state<UpdateState>({
  checking: false,
  available: false,
  downloading: false,
  progress: 0,
  update: null,
  error: null,
});

export function getUpdateState(): UpdateState {
  return state;
}

export function resetUpdateState(): void {
  state.checking = false;
  state.available = false;
  state.downloading = false;
  state.progress = 0;
  state.update = null;
  state.error = null;
}

export async function checkForUpdate(): Promise<Update | null> {
  state.checking = true;
  state.error = null;

  try {
    const update = await check();
    state.update = update;
    state.available = update !== null;
    return update;
  } catch (e) {
    state.error = e instanceof Error ? e.message : String(e);
    return null;
  } finally {
    state.checking = false;
  }
}

export async function downloadAndInstall(): Promise<void> {
  if (!state.update) return;

  state.downloading = true;
  state.progress = 0;
  state.error = null;

  try {
    let downloaded = 0;
    let contentLength = 0;

    await state.update.downloadAndInstall((event) => {
      switch (event.event) {
        case "Started":
          contentLength = event.data.contentLength ?? 0;
          break;
        case "Progress":
          downloaded += event.data.chunkLength;
          if (contentLength > 0) {
            state.progress = Math.round((downloaded / contentLength) * 100);
          }
          break;
        case "Finished":
          state.progress = 100;
          break;
      }
    });

    await relaunch();
  } catch (e) {
    state.error = e instanceof Error ? e.message : String(e);
    state.downloading = false;
  }
}
