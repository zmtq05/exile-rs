<script lang="ts">
  import { commands } from "@/bindings";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { getVersion } from "@tauri-apps/api/app";
  import { toast } from "svelte-sonner";
  import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
  import { Button } from "@/components/ui/button";
  import { Switch } from "@/components/ui/switch";
  import { Label } from "@/components/ui/label";
  import { Settings, RefreshCw } from "@lucide/svelte";
  import {
    loadSettings,
    getSettings,
    setAutoCheckUpdate,
  } from "@/stores/settings.svelte";
  import {
    checkForUpdate,
    getUpdateState,
  } from "@/stores/updater.svelte";
  import UpdateDialog from "@/components/update-dialog.svelte";
    import { onMount } from "svelte";

  let installPath = $state<string | null>(null);
  let appVersion = $state<string | null>(null);
  let isLoading = $state(true);
  let updateDialogOpen = $state(false);

  const settings = getSettings();
  const updateState = getUpdateState();

  onMount(async () => {
    await Promise.all([fetchAppVersion(), loadSettings()]);
    isLoading = false;
  });

  async function fetchAppVersion() {
    try {
      appVersion = await getVersion();
    } catch (e) {
      console.error("Failed to get app version:", e);
    }
  }


  async function handleAutoCheckUpdateChange(checked: boolean) {
    await setAutoCheckUpdate(checked);
  }

  async function handleManualUpdateCheck() {
    const update = await checkForUpdate();
    if (update) {
      updateDialogOpen = true;
    } else if (!updateState.error) {
      toast.success("최신 버전입니다");
    } else {
      toast.error("업데이트 확인 실패", { description: updateState.error });
    }
  }
</script>

<UpdateDialog bind:open={updateDialogOpen} />

<main class="bg-background p-8">
  <div class="mx-auto max-w-xl space-y-6">
    <!-- Header -->
    <div class="flex items-center gap-2">
      <Settings class="h-6 w-6" />
      <h1 class="text-2xl font-bold">설정</h1>
    </div>

    <!-- Update Settings Card -->
    <Card>
      <CardHeader>
        <CardTitle class="text-base font-medium">업데이트</CardTitle>
      </CardHeader>
      <CardContent class="space-y-4">
        <div class="flex items-center justify-between">
          <Label for="auto-update" class="text-sm">
            시작 시 자동으로 업데이트 확인
          </Label>
          <Switch
            id="auto-update"
            checked={settings.autoCheckUpdate}
            onCheckedChange={handleAutoCheckUpdateChange}
            disabled={isLoading}
          />
        </div>
        <div class="flex items-center justify-between">
          <span class="text-sm text-muted-foreground">수동으로 확인</span>
          <Button
            variant="outline"
            size="sm"
            onclick={handleManualUpdateCheck}
            disabled={updateState.checking}
            class="gap-2"
          >
            <RefreshCw class="h-4 w-4 {updateState.checking ? 'animate-spin' : ''}" />
            {updateState.checking ? "확인 중..." : "업데이트 확인"}
          </Button>
        </div>
      </CardContent>
    </Card>

    <!-- App Info Card -->
    <Card>
      <CardHeader>
        <CardTitle class="text-base font-medium">앱 정보</CardTitle>
      </CardHeader>
      <CardContent>
        <div class="flex items-center justify-between">
          <span class="text-sm text-muted-foreground">버전</span>
          {#if isLoading}
            <span class="text-sm text-muted-foreground">불러오는 중...</span>
          {:else if appVersion}
            <span class="font-mono text-sm">v{appVersion}</span>
          {:else}
            <span class="text-sm text-muted-foreground">알 수 없음</span>
          {/if}
        </div>
      </CardContent>
    </Card>
  </div>
</main>
