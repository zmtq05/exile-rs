<script lang="ts">
  import { commands } from "@/bindings";
  import { onMount } from "svelte";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { getVersion } from "@tauri-apps/api/app";
  import { toast } from "svelte-sonner";
  import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
  import { Button } from "@/components/ui/button";
  import { Settings, FolderOpen } from "@lucide/svelte";

  let installPath = $state<string | null>(null);
  let appVersion = $state<string | null>(null);
  let isLoading = $state(true);

  onMount(async () => {
    await Promise.all([fetchInstallPath(), fetchAppVersion()]);
    isLoading = false;
  });

  async function fetchInstallPath() {
    const result = await commands.getInstallPath();
    if (result.status === "ok") {
      installPath = result.data;
    }
  }

  async function fetchAppVersion() {
    try {
      appVersion = await getVersion();
    } catch (e) {
      console.error("Failed to get app version:", e);
    }
  }

  async function openInstallFolder() {
    if (installPath) {
      try {
        await revealItemInDir(installPath);
      } catch (e) {
        toast.error("폴더 열기 실패", { description: String(e) });
      }
    }
  }
</script>

<main class="bg-background p-8">
  <div class="mx-auto max-w-xl space-y-6">
    <!-- Header -->
    <div class="flex items-center gap-2">
      <Settings class="h-6 w-6" />
      <h1 class="text-2xl font-bold">설정</h1>
    </div>


    <!-- Install Path Card -->
    <!--
    <Card>
      <CardHeader>
        <CardTitle class="text-base font-medium">Path of Building 설치 경로</CardTitle>
      </CardHeader>
      <CardContent class="space-y-4">
        <div class="flex items-center justify-between">
          <span class="text-sm text-muted-foreground">경로</span>
          {#if isLoading}
            <span class="text-sm text-muted-foreground">불러오는 중...</span>
          {:else if installPath}
            <span class="font-mono text-sm truncate max-w-[300px]" title={installPath}>
              {installPath}
            </span>
          {:else}
            <span class="text-sm text-muted-foreground">설정되지 않음</span>
          {/if}
        </div>
        <div class="flex items-center justify-between">
          <Button
            variant="outline"
            size="sm"
            onclick={openInstallFolder}
            disabled={!installPath}
            class="gap-2"
          >
            <FolderOpen class="h-4 w-4" />
            폴더 열기
          </Button>
        </div>
        <p class="text-xs text-muted-foreground">
          설치 경로 변경 기능은 추후 지원 예정입니다.
        </p>
      </CardContent>
    </Card>
  -->
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
