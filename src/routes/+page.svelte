<script lang="ts">
  import { commands, events, type GoogleDriveFileInfo, type PobVersion, type InstallProgress } from "@/bindings";
  import { onMount, onDestroy } from "svelte";
  import { Button } from "@/components/ui/button";
  import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
  import { Badge } from "@/components/ui/badge";
  import { Progress } from "@/components/ui/progress";
  import * as Alert from "@/components/ui/alert/index.js";
  import { Input } from "@/components/ui/input";
  import { Label } from "@/components/ui/label";

  let installedVersion = $state<PobVersion | null>(null);
  let latestVersion = $state<GoogleDriveFileInfo | null>(null);
  let isLoading = $state(false);
  let error = $state<string | null>(null);
  let installProgress = $state<InstallProgress | null>(null);

  let parseFileName = $state("");
  let parseResult = $state<string | null>(null);

  let unlistenInstallProgress: (() => void) | null = null;
  let unlistenCancelEvent: (() => void) | null = null;

  onMount(async () => {
    // 이벤트 리스너 설정
    unlistenInstallProgress = await events.installProgress.listen((event) => {
      installProgress = event.payload;

      // 설치 완료/실패/취소 시 설치된 버전 정보 새로고침
      if (event.payload.status === "completed" ||
          event.payload.status === "failed" ||
          event.payload.status === "cancelled") {
        setTimeout(checkInstalledVersion, 500);
      }
    });

    unlistenCancelEvent = await events.cancelEvent.listen(() => {
        console.log("취소 이벤트 수신됨");
    });

    // 초기 설치 버전 확인
    await checkInstalledVersion();
  });

  onDestroy(() => {
    if (unlistenInstallProgress) unlistenInstallProgress();
    if (unlistenCancelEvent) unlistenCancelEvent();
  });

  async function checkInstalledVersion() {
    isLoading = true;
    error = null;
    try {
      const result = await commands.installedPobInfo();
      if (result.status === "ok") {
        installedVersion = result.data;
      } else {
        error = `설치된 버전 확인 실패: ${JSON.stringify(result.error)}`;
      }
    } catch (e) {
      error = `오류: ${e}`;
    } finally {
      isLoading = false;
    }
  }

  async function checkLatestVersion(refresh = false) {
    isLoading = true;
    error = null;
    try {
      const result = await commands.fetchPob(refresh);
      if (result.status === "ok") {
        latestVersion = result.data;
      } else {
        error = `최신 버전 확인 실패: ${result.error}`;
      }
    } catch (e) {
      error = `오류: ${e}`;
    } finally {
      isLoading = false;
    }
  }

  async function install() {
    if (!latestVersion) {
      error = "먼저 최신 버전을 확인해주세요.";
      return;
    }

    isLoading = true;
    error = null;
    installProgress = null;
    try {
      const result = await commands.installPob(latestVersion);
      if (result.status === "error") {
        error = `설치 실패: ${JSON.stringify(result.error)}`;
        installProgress = null;
      } else {
        console.log(result.data);
      }
    } catch (e) {
      error = `오류: ${e}`;
      installProgress = null;
    } finally {
      isLoading = false;
    }
  }

  async function cancelInstall() {
    try {
      await commands.cancelInstallPob();
    } catch (e) {
      error = `취소 실패: ${e}`;
    }
  }

  async function uninstall() {
    if (!confirm("정말 PoB를 제거하시겠습니까?")) return;

    isLoading = true;
    error = null;
    try {
        const result = await commands.uninstallPob();
        if (result.status === "error") {
             error = `제거 실패: ${JSON.stringify(result.error)}`;
        } else {
             await checkInstalledVersion();
        }
    } catch (e) {
        error = `오류: ${e}`;
    } finally {
        isLoading = false;
    }
  }

  async function testParseVersion() {
      if (!parseFileName.trim()) {
          error = "파일 이름을 입력해주세요.";
          return;
      }

      isLoading = true;
      error = null;
      parseResult = null;
      try {
          const result = await commands.parseVersion(parseFileName);
          if (result.status === "ok") {
              parseResult = result.data;
          } else {
              error = `파싱 실패: ${JSON.stringify(result.error)}`;
          }
      } catch (e) {
          error = `오류: ${e}`;
      } finally {
          isLoading = false;
      }
  }

  function getPhaseText(phase: string): string {
    switch (phase) {
      case "downloading": return "다운로드 중";
      case "extracting": return "압축 해제 중";
      case "backingUp": return "백업 중";
      case "installing": return "설치 중";
      case "restoring": return "복구 중";
      case "finalizing": return "마무리 중";
      case "uninstalling": return "제거 중";
      default: return phase;
    }
  }

  function getStatusText(progress: InstallProgress): string {
    switch (progress.status) {
      case "started":
        return `시작됨${progress.total_size ? ` (총 크기: ${(progress.total_size / 1024 / 1024).toFixed(2)} MB)` : ''}`;
      case "inProgress": return `진행 중 (${progress.percent.toFixed(1)}%)`;
      case "completed": return "완료";
      case "failed": return `실패: ${progress.reason}`;
      case "cancelled": return "취소됨";
      default: return "알 수 없음";
    }
  }

  function getStatusBadgeVariant(status: string): "default" | "secondary" | "destructive" | "outline" {
    switch (status) {
      case "completed": return "default";
      case "failed": return "destructive";
      case "cancelled": return "secondary";
      default: return "outline";
    }
  }

  function execute() {
      commands.executePob();
  }
</script>

<main class="container mx-auto p-6 max-w-4xl">
  <h1 class="text-3xl font-bold mb-6 text-center">Path of Building 관리자</h1>

  <!-- {#if error} -->
    <Alert.Root variant="destructive" class="mb-4">
      <Alert.Description>{error}</Alert.Description>
    </Alert.Root>
  <!-- {/if} -->

  <div class="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
    <!-- 설치된 버전 카드 -->
    <Card>
      <CardHeader>
        <CardTitle>설치된 버전</CardTitle>
        <CardDescription>현재 시스템에 설치된 PoB 버전</CardDescription>
      </CardHeader>
      <CardContent>
        {#if installedVersion}
          <div class="space-y-2">
            <div class="flex items-center justify-between">
              <span class="text-sm text-muted-foreground">버전</span>
              <Badge>{installedVersion.version}</Badge>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-sm text-muted-foreground">설치일</span>
              <span class="text-sm">{new Date(installedVersion.installedAt).toLocaleString('ko-KR')}</span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-sm text-muted-foreground">파일 ID</span>
              <span class="text-xs font-mono">{installedVersion.fileId.slice(0, 10)}...</span>
            </div>
          </div>
        {:else}
          <p class="text-sm text-muted-foreground">설치된 버전이 없습니다.</p>
        {/if}
      </CardContent>
      <CardFooter class="flex gap-2">
        <Button onclick={checkInstalledVersion} disabled={isLoading} class="flex-1">
          {isLoading ? "확인 중..." : "버전 확인"}
        </Button>
        {#if installedVersion}
            <Button onclick={uninstall} disabled={isLoading} variant="destructive" class="flex-1">
                제거
            </Button>
            <Button onclick={execute} disabled={isLoading} variant="outline" class="flex-1">
                실행
            </Button>
        {/if}
      </CardFooter>
    </Card>

    <!-- 최신 버전 카드 -->
    <Card>
      <CardHeader>
        <CardTitle>최신 버전</CardTitle>
        <CardDescription>Google Drive에서 가져온 최신 버전</CardDescription>
      </CardHeader>
      <CardContent>
        {#if latestVersion}
          <div class="space-y-2">
            <div class="flex items-center justify-between">
              <span class="text-sm text-muted-foreground">이름</span>
              <span class="text-sm font-medium">{latestVersion.name}</span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-sm text-muted-foreground">타입</span>
              <Badge variant={latestVersion.isFolder ? "secondary" : "outline"}>
                {latestVersion.isFolder ? "폴더" : "파일"}
              </Badge>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-sm text-muted-foreground">ID</span>
              <span class="text-xs font-mono">{latestVersion.id.slice(0, 10)}...</span>
            </div>
          </div>
        {:else}
          <p class="text-sm text-muted-foreground">최신 버전을 확인하지 않았습니다.</p>
        {/if}
      </CardContent>
      <CardFooter class="flex gap-2">
        <Button onclick={() => checkLatestVersion(false)} disabled={isLoading} class="flex-1">
          {isLoading ? "확인 중..." : "최신 버전 확인"}
        </Button>
        <Button onclick={() => checkLatestVersion(true)} disabled={isLoading} variant="outline" class="flex-1">
          새로고침
        </Button>
      </CardFooter>
    </Card>
  </div>

  <!-- 설치 제어 카드 -->
  <Card class="mb-6">
    <CardHeader>
      <CardTitle>설치 관리</CardTitle>
      <CardDescription>Path of Building 설치 및 제어</CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
      {#if installProgress}
        <div class="space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-sm font-medium">진행 단계</span>
            <Badge>{getPhaseText(installProgress.phase)}</Badge>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-sm font-medium">상태</span>
            <Badge variant={getStatusBadgeVariant(installProgress.status)}>
              {getStatusText(installProgress)}
            </Badge>
          </div>
          {#if installProgress.status === "inProgress"}
            <div class="space-y-2">
              <Progress value={installProgress.percent} max={100} />
              <p class="text-xs text-muted-foreground text-center">
                {installProgress.percent.toFixed(1)}%
              </p>
            </div>
          {/if}
        </div>
      {:else}
        <p class="text-sm text-muted-foreground text-center">진행 중인 작업이 없습니다.</p>
      {/if}
    </CardContent>
    <CardFooter class="flex gap-2">
      <Button
        onclick={install}
        disabled={isLoading || !latestVersion || (installProgress !== null && installProgress.status === "inProgress")}
        class="flex-1"
      >
        설치
      </Button>
      <Button
        onclick={cancelInstall}
        disabled={!installProgress || installProgress.status !== "inProgress"}
        variant="destructive"
        class="flex-1"
      >
        설치 취소
      </Button>
    </CardFooter>
  </Card>

  <!-- 버전 파싱 테스트 카드 -->
  <Card>
      <CardHeader>
          <CardTitle>버전 파싱 테스트</CardTitle>
          <CardDescription>파일 이름에서 버전 정보 추출 테스트</CardDescription>
      </CardHeader>
      <CardContent>
          <div class="grid w-full items-center gap-4">
            <div class="flex flex-col space-y-1.5">
                <Label for="fileName">파일 이름</Label>
                <Input id="fileName" placeholder="예: PathOfBuilding-2.40.0.zip" bind:value={parseFileName} />
            </div>
            {#if parseResult}
                <div class="flex items-center gap-2 p-2 bg-muted rounded-md">
                    <span class="text-sm font-medium">추출된 버전:</span>
                    <Badge variant="outline">{parseResult}</Badge>
                </div>
            {/if}
          </div>
      </CardContent>
      <CardFooter>
          <Button onclick={testParseVersion} disabled={isLoading} class="w-full">
              파싱 테스트
          </Button>
      </CardFooter>
  </Card>
</main>

<style>
  :global(body) {
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  }
</style>
