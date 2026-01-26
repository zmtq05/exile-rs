<script lang="ts">
  import { commands, events, type GoogleDriveFileInfo, type PobVersion, type InstallProgress, type ErrorKind, type DownloadMode } from "@/bindings";
  import { onMount, onDestroy } from "svelte";
  import { slide } from "svelte/transition";
  import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
  import { toast } from "svelte-sonner";
  import { Button } from "@/components/ui/button";
  import { Card, CardContent, CardHeader, CardTitle, CardFooter } from "@/components/ui/card";
  import { Badge } from "@/components/ui/badge";
  import { Progress } from "@/components/ui/progress";
  import { Skeleton } from "@/components/ui/skeleton";
  import * as Alert from "@/components/ui/alert/index.js";
  import * as AlertDialog from "@/components/ui/alert-dialog/index.js";
  import { Package, Check, AlertCircle, RefreshCw, ExternalLink, Download, CheckCircle, AlertTriangle, FolderOpen, Settings } from "@lucide/svelte";

  let installedVersion = $state<PobVersion | null>(null);
  let latestVersion = $state<GoogleDriveFileInfo | null>(null);
  let latestVersionString = $state<string | null>(null);
  let installPath = $state<string | null>(null);
  let isInitialLoading = $state(true);
  let isLoading = $state(false);
  let isFetchingLatest = $state(false);
  let error = $state<{ kind: string; message?: string } | null>(null);
  let installProgress = $state<InstallProgress | null>(null);
  let showUninstallDialog = $state(false);
  let downloadMode = $state<DownloadMode>(
    (localStorage.getItem("downloadMode") as DownloadMode) || "auto"
  );

  function setDownloadMode(mode: DownloadMode) {
    downloadMode = mode;
    localStorage.setItem("downloadMode", mode);
  }

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

      // Handle completion/failure/cancel with toast
      if (event.payload.status === "completed") {
        toast.success("설치 완료", { description: "Path of Building이 성공적으로 설치되었습니다." });
        setTimeout(checkInstalledVersion, 500);
      } else if (event.payload.status === "failed") {
        toast.error("설치 실패", { description: "설치 중 오류가 발생했습니다." });
        setTimeout(checkInstalledVersion, 500);
      } else if (event.payload.status === "cancelled") {
        toast.info("설치 취소", { description: "사용자에 의해 취소되었습니다." });
        setTimeout(checkInstalledVersion, 500);
      }
    });

    unlistenCancelEvent = await events.cancelEvent.listen(() => {
      console.log("취소 이벤트 수신됨");
    });

    // Initial checks
    await Promise.all([
      checkInstalledVersion(),
      checkLatestVersion(false),
      fetchInstallPath()
    ]);
    isInitialLoading = false;
  });

  onDestroy(() => {
    if (unlistenInstallProgress) unlistenInstallProgress();
    if (unlistenCancelEvent) unlistenCancelEvent();
  });

  function handleError(errorKind: ErrorKind, context: string) {
    if (errorKind.kind === "cancelled") {
      return;
    }
    error = { kind: errorKind.kind, message: errorKind.message };
    toast.error(context, { description: errorKind.message });
  }

  async function fetchInstallPath() {
    const result = await commands.getInstallPath();
    if (result.status === "ok") {
      installPath = result.data;
    }
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
        const parseResult = await commands.parseVersion(result.data.name);
        if (parseResult.status === "ok") {
          latestVersionString = parseResult.data;
        }
        if (refresh) {
          toast.success("새로고침 완료", { description: "최신 버전 정보를 가져왔습니다." });
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
      const result = await commands.installPob(latestVersion, downloadMode);
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
        toast.success("제거 완료", { description: "Path of Building이 제거되었습니다." });
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
    } else {
      toast.success("실행", { description: "Path of Building을 시작합니다." });
    }
  }

  async function openSource() {
    try {
      await openUrl("https://gall.dcinside.com/mgallery/board/view/?id=pathofexile&no=991032");
    } catch (e) {
      error = { kind: "unknown", message: `브라우저 열기 실패: ${e}` };
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

    <!-- Loading Skeleton -->
    {#if isInitialLoading}
      <Card>
        <CardContent class="flex items-center gap-4 p-6">
          <Skeleton class="h-12 w-12 rounded-full" />
          <div class="space-y-2 flex-1">
            <Skeleton class="h-4 w-32" />
            <Skeleton class="h-3 w-48" />
          </div>
        </CardContent>
      </Card>
      <Card>
        <CardHeader class="pb-4">
          <Skeleton class="h-5 w-24" />
        </CardHeader>
        <CardContent class="space-y-4">
          <div class="flex items-center justify-between">
            <Skeleton class="h-4 w-16" />
            <Skeleton class="h-4 w-24" />
          </div>
          <div class="flex items-center justify-between">
            <Skeleton class="h-4 w-16" />
            <Skeleton class="h-4 w-24" />
          </div>
          <div class="flex items-center justify-between">
            <Skeleton class="h-4 w-16" />
            <Skeleton class="h-4 w-24" />
          </div>
        </CardContent>
        <CardFooter class="flex gap-2 justify-end">
          <Skeleton class="h-9 w-20" />
          <Skeleton class="h-9 w-20" />
        </CardFooter>
      </Card>
    {:else}
      <!-- Status Banner -->
      {#if isUpToDate}
        <Card class="bg-green-500/10 border-green-500/20">
          <CardContent class="flex items-center gap-4 p-6">
            <div class="rounded-full bg-green-500/20 p-3">
              <CheckCircle class="h-6 w-6 text-green-500" />
            </div>
            <div class="space-y-1">
              <h3 class="font-medium leading-none text-green-500">최신 버전 사용 중</h3>
              <p class="text-sm text-muted-foreground">Path of Building이 최신 상태입니다.</p>
            </div>
          </CardContent>
        </Card>
      {:else if hasUpdate}
        <Card class="bg-amber-500/10 border-amber-500/20">
          <CardContent class="flex items-center gap-4 p-6">
            <div class="rounded-full bg-amber-500/20 p-3">
              <AlertTriangle class="h-6 w-6 text-amber-500" />
            </div>
            <div class="space-y-1">
              <h3 class="font-medium leading-none text-amber-500">업데이트 가능</h3>
              <p class="text-sm text-muted-foreground">새로운 버전({latestVersionString})이 출시되었습니다.</p>
            </div>
          </CardContent>
        </Card>
      {:else if !isInstalled}
        <Card class="bg-muted/50">
          <CardContent class="flex items-center gap-4 p-6">
            <div class="rounded-full bg-background p-3">
              <Download class="h-6 w-6 text-muted-foreground" />
            </div>
            <div class="space-y-1">
              <h3 class="font-medium leading-none">설치 필요</h3>
              <p class="text-sm text-muted-foreground">Path of Building이 설치되지 않았습니다.</p>
            </div>
          </CardContent>
        </Card>
      {/if}

      <!-- Version Info Card -->
      <Card>
        <CardHeader class="pb-4">
          <div class="flex items-center justify-between">
            <CardTitle class="flex items-center gap-2 text-base font-medium">
              <Package class="h-4 w-4" />
              버전 정보
            </CardTitle>
            <Button
              variant="ghost"
              size="icon"
              onclick={() => checkLatestVersion(true)}
              disabled={isFetchingLatest}
              class="h-8 w-8"
            >
              <RefreshCw class={`h-4 w-4 ${isFetchingLatest ? 'animate-spin' : ''}`} />
              <span class="sr-only">새로고침</span>
            </Button>
          </div>
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
              {/if}
            </div>
          </div>

          <!-- Install Path Row (if installed) -->
          {#if isInstalled && installPath}
            <div class="flex items-center justify-between">
              <span class="text-sm text-muted-foreground">설치 경로</span>
              <Button
                variant="link"
                onclick={openInstallFolder}
                class="h-auto p-0 text-sm font-normal gap-1 max-w-[200px]"
              >
                <FolderOpen class="h-3 w-3 shrink-0" />
                <span class="truncate">{installPath.split('\\').pop()}</span>
              </Button>
            </div>
          {/if}

          <!-- Source Row -->
          <div class="flex items-center justify-between">
            <span class="text-sm text-muted-foreground">출처</span>
            <Button
              variant="link"
              onclick={openSource}
              class="h-auto p-0 text-sm font-normal gap-1"
            >
              <ExternalLink class="h-3 w-3" />
              패스 오브 엑자일 갤러리
            </Button>
          </div>

          <!-- Download Mode Row -->
          <div class="flex items-center justify-between">
            <span class="text-sm text-muted-foreground flex items-center gap-1">
              <Settings class="h-3 w-3" />
              다운로드 방식
            </span>
            <div class="flex gap-1">
              <Button
                size="sm"
                variant={downloadMode === "auto" ? "default" : "outline"}
                onclick={() => setDownloadMode("auto")}
                class="h-7 px-2 text-xs"
              >
                자동
              </Button>
              <Button
                size="sm"
                variant={downloadMode === "parallel" ? "default" : "outline"}
                onclick={() => setDownloadMode("parallel")}
                class="h-7 px-2 text-xs"
              >
                병렬
              </Button>
              <Button
                size="sm"
                variant={downloadMode === "single" ? "default" : "outline"}
                onclick={() => setDownloadMode("single")}
                class="h-7 px-2 text-xs"
              >
                단일
              </Button>
            </div>
          </div>
        </CardContent>
        <CardFooter class="flex gap-2 justify-end">
          {#if !isInstalled}
            <Button
              onclick={install}
              disabled={isLoading || !latestVersion || isInstalling}
              class="flex-1"
            >
              설치
            </Button>
          {:else if hasUpdate}
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
        </CardFooter>
      </Card>

      <!-- Progress Card (only visible during installation) -->
      {#if isInstalling}
        <div transition:slide={{ duration: 300, axis: 'y' }}>
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
        </div>
      {/if}
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
