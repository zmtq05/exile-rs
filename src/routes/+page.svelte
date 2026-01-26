<script lang="ts">
  import { commands, events, type GoogleDriveFileInfo, type PobVersion, type InstallProgress, type ErrorKind } from "@/bindings";
  import { onMount, onDestroy } from "svelte";
  import { Button } from "@/components/ui/button";
  import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
  import { Badge } from "@/components/ui/badge";
  import { Progress } from "@/components/ui/progress";
  import * as Alert from "@/components/ui/alert/index.js";
  import * as AlertDialog from "@/components/ui/alert-dialog/index.js";
  import { Package, Check, AlertCircle, RefreshCw } from "@lucide/svelte";

  let installedVersion = $state<PobVersion | null>(null);
  let latestVersion = $state<GoogleDriveFileInfo | null>(null);
  let latestVersionString = $state<string | null>(null);
  let isLoading = $state(false);
  let isFetchingLatest = $state(false);
  let error = $state<{ kind: string; message?: string } | null>(null);
  let installProgress = $state<InstallProgress | null>(null);
  let showUninstallDialog = $state(false);

  let unlistenInstallProgress: (() => void) | null = null;
  let unlistenCancelEvent: (() => void) | null = null;

  // Derived states for UI logic
  let isInstalled = $derived(installedVersion !== null);
  let isInstalling = $derived(
    installProgress !== null && 
    (installProgress.status === "started" || installProgress.status === "inProgress")
  );
  let hasUpdate = $derived(
    isInstalled && 
    latestVersionString !== null && 
    installedVersion!.version !== latestVersionString
  );
  let isUpToDate = $derived(
    isInstalled && 
    latestVersionString !== null && 
    installedVersion!.version === latestVersionString
  );

  onMount(async () => {
    // Event listeners
    unlistenInstallProgress = await events.installProgress.listen((event) => {
      installProgress = event.payload;

      // Refresh installed version on completion/failure/cancel
      if (event.payload.status === "completed" ||
          event.payload.status === "failed" ||
          event.payload.status === "cancelled") {
        setTimeout(checkInstalledVersion, 500);
      }
    });

    unlistenCancelEvent = await events.cancelEvent.listen(() => {
      console.log("취소 이벤트 수신됨");
    });

    // Initial checks
    await checkInstalledVersion();
    await checkLatestVersion(false);
  });

  onDestroy(() => {
    if (unlistenInstallProgress) unlistenInstallProgress();
    if (unlistenCancelEvent) unlistenCancelEvent();
  });

  function handleError(errorKind: ErrorKind, context: string) {
    if (errorKind.kind === "cancelled") {
      // Silently ignore cancelled operations
      return;
    }
    
    // All non-cancelled variants have a message property
    error = { kind: errorKind.kind, message: errorKind.message };
  }

  async function checkInstalledVersion() {
    isLoading = true;
    error = null;
    try {
      const result = await commands.installedPobInfo();
      if (result.status === "ok") {
        installedVersion = result.data;
      } else {
        handleError(result.error, "설치된 버전 확인 실패");
      }
    } catch (e) {
      error = { kind: "unknown", message: `오류: ${e}` };
    } finally {
      isLoading = false;
    }
  }

  async function checkLatestVersion(refresh = false) {
    isFetchingLatest = true;
    error = null;
    try {
      const result = await commands.fetchPob(refresh);
      if (result.status === "ok") {
        latestVersion = result.data;
        // Parse version from filename
        const parseResult = await commands.parseVersion(result.data.name);
        if (parseResult.status === "ok") {
          latestVersionString = parseResult.data;
        }
      } else {
        handleError(result.error, "최신 버전 확인 실패");
      }
    } catch (e) {
      error = { kind: "unknown", message: `오류: ${e}` };
    } finally {
      isFetchingLatest = false;
    }
  }

  async function install() {
    if (!latestVersion) {
      error = { kind: "domain", message: "먼저 최신 버전을 확인해주세요." };
      return;
    }

    isLoading = true;
    error = null;
    installProgress = null;
    try {
      const result = await commands.installPob(latestVersion);
      if (result.status === "error") {
        handleError(result.error, "설치 실패");
        installProgress = null;
      }
    } catch (e) {
      error = { kind: "unknown", message: `오류: ${e}` };
      installProgress = null;
    } finally {
      isLoading = false;
    }
  }

  async function cancelInstall() {
    try {
      await commands.cancelInstallPob();
    } catch (e) {
      error = { kind: "unknown", message: `취소 실패: ${e}` };
    }
  }

  async function uninstall() {
    showUninstallDialog = false;
    isLoading = true;
    error = null;
    try {
      const result = await commands.uninstallPob();
      if (result.status === "error") {
        handleError(result.error, "제거 실패");
      } else {
        await checkInstalledVersion();
      }
    } catch (e) {
      error = { kind: "unknown", message: `오류: ${e}` };
    } finally {
      isLoading = false;
    }
  }

  async function execute() {
    const result = await commands.executePob();
    if (result.status === "error") {
      handleError(result.error, "실행 실패");
    }
  }

  function getPhaseText(phase: string): string {
    switch (phase) {
      case "downloading": return "다운로드 중";
      case "extracting": return "압축 해제 중";
      case "backingUp": return "백업 중";
      case "moving": return "이동 중";
      case "restoring": return "복구 중";
      case "finalizing": return "마무리 중";
      case "uninstalling": return "제거 중";
      default: return phase;
    }
  }
</script>

<main class="min-h-screen bg-background p-8">
  <div class="mx-auto max-w-xl space-y-6">
    <!-- Header -->
    <div class="text-center">
      <h1 class="text-2xl font-semibold tracking-tight">Path of Building</h1>
      <p class="text-sm text-muted-foreground">버전 관리</p>
    </div>

    <!-- Error Alert -->
    {#if error}
      <Alert.Root variant={error.kind === "conflict" ? "default" : "destructive"}>
        <AlertCircle class="h-4 w-4" />
        <Alert.Title>
          {#if error.kind === "conflict"}
            경고
          {:else}
            오류
          {/if}
        </Alert.Title>
        <Alert.Description>{error.message}</Alert.Description>
      </Alert.Root>
    {/if}

    <!-- Version Info Card -->
    <Card>
      <CardHeader class="pb-4">
        <CardTitle class="flex items-center gap-2 text-base font-medium">
          <Package class="h-4 w-4" />
          버전 정보
        </CardTitle>
      </CardHeader>
      <CardContent class="space-y-4">
        <!-- Installed Version Row -->
        <div class="flex items-center justify-between">
          <span class="text-sm text-muted-foreground">설치됨</span>
          <div class="flex items-center gap-2">
            {#if installedVersion}
              <span class="font-mono text-sm">v{installedVersion.version}</span>
              {#if isUpToDate}
                <Badge variant="secondary" class="gap-1">
                  <Check class="h-3 w-3" />
                  최신
                </Badge>
              {/if}
            {:else}
              <span class="text-sm text-muted-foreground">설치되지 않음</span>
            {/if}
          </div>
        </div>

        <!-- Install Date Row (if installed) -->
        {#if installedVersion}
          <div class="flex items-center justify-between">
            <span class="text-sm text-muted-foreground">설치일</span>
            <span class="text-sm">
              {new Date(installedVersion.installedAt).toLocaleDateString('ko-KR')}
            </span>
          </div>
        {/if}

        <!-- Latest Version Row -->
        <div class="flex items-center justify-between">
          <span class="text-sm text-muted-foreground">최신</span>
          <div class="flex items-center gap-2">
            {#if isFetchingLatest}
              <RefreshCw class="h-4 w-4 animate-spin text-muted-foreground" />
            {:else if latestVersionString}
              <span class="font-mono text-sm">v{latestVersionString}</span>
              {#if hasUpdate}
                <Badge variant="default">업데이트 가능</Badge>
              {/if}
            {:else}
              <span class="text-sm text-muted-foreground">확인 필요</span>
              <Button 
                variant="ghost" 
                size="sm" 
                onclick={() => checkLatestVersion(true)}
                disabled={isFetchingLatest}
                class="h-6 px-2"
              >
                <RefreshCw class="h-3 w-3" />
              </Button>
            {/if}
          </div>
        </div>

        <!-- Action Buttons -->
        <div class="flex gap-2 pt-2">
          {#if !isInstalled}
            <!-- Not installed: Show Install button only -->
            <Button 
              onclick={install} 
              disabled={isLoading || !latestVersion || isInstalling}
              class="flex-1"
            >
              설치
            </Button>
          {:else if hasUpdate}
            <!-- Installed with update available -->
            <Button 
              onclick={install} 
              disabled={isLoading || !latestVersion || isInstalling}
              class="flex-1"
            >
              업데이트
            </Button>
            <Button 
              onclick={execute} 
              disabled={isLoading || isInstalling}
              variant="outline"
              class="flex-1"
            >
              실행
            </Button>
            <Button 
              onclick={() => showUninstallDialog = true} 
              disabled={isLoading || isInstalling}
              variant="ghost"
              class="text-destructive hover:text-destructive"
            >
              제거
            </Button>
          {:else}
            <!-- Installed and up to date -->
            <Button 
              onclick={execute} 
              disabled={isLoading || isInstalling}
              class="flex-1"
            >
              실행
            </Button>
            <Button 
              onclick={() => showUninstallDialog = true} 
              disabled={isLoading || isInstalling}
              variant="ghost"
              class="text-destructive hover:text-destructive"
            >
              제거
            </Button>
          {/if}
        </div>
      </CardContent>
    </Card>

    <!-- Progress Card (only visible during installation) -->
    {#if isInstalling}
      <Card>
        <CardHeader class="pb-3">
          <CardTitle class="text-base font-medium">진행 상황</CardTitle>
        </CardHeader>
        <CardContent class="space-y-4">
          <div class="space-y-2">
            <div class="flex items-center justify-between text-sm">
              <span class="text-muted-foreground">
                {installProgress ? getPhaseText(installProgress.phase) : "준비 중"}
              </span>
              <span class="font-mono">
                {installProgress?.status === "inProgress" 
                  ? `${installProgress.percent.toFixed(0)}%` 
                  : "0%"}
              </span>
            </div>
            <Progress 
              value={installProgress?.status === "inProgress" ? installProgress.percent : 0} 
              max={100} 
            />
          </div>
          <Button 
            onclick={cancelInstall} 
            variant="outline" 
            class="w-full"
          >
            취소
          </Button>
        </CardContent>
      </Card>
    {/if}
  </div>
</main>

<!-- Uninstall Confirmation Dialog -->
<AlertDialog.Root bind:open={showUninstallDialog}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>PoB 제거</AlertDialog.Title>
      <AlertDialog.Description>
        Path of Building을 제거하시겠습니까? 이 작업은 취소할 수 없습니다.
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel>취소</AlertDialog.Cancel>
      <AlertDialog.Action onclick={uninstall} class="bg-destructive text-white hover:bg-destructive/90">
        제거
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
