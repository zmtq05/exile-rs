<script lang="ts">
  import {
    commands,
    events,
    type GoogleDriveFileInfo,
    type PobVersion,
    type InstallProgress,
    type ErrorKind,
  } from "@/bindings";
  import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
  import { toast } from "svelte-sonner";
  import { slide } from "svelte/transition";
  import { Button } from "@/components/ui/button";
  import { Card, CardContent } from "@/components/ui/card";
  import { Badge } from "@/components/ui/badge";
  import { Progress } from "@/components/ui/progress";
  import { Skeleton } from "@/components/ui/skeleton";
  import * as AlertDialog from "@/components/ui/alert-dialog/index.js";
  import { Alert } from "@/components/ui/alert";
  import * as AlertComp from "@/components/ui/alert";
  import {
    Play,
    Download,
    RefreshCw,
    FolderOpen,
    ExternalLink,
    Trash2,
    BookOpen,
    X,
    Link as LinkIcon,
    Coins,
    Hammer,
    Globe,
    AlertCircle,
    type Icon,
    CircleCheck,
    CircleAlert,
    ChartColumn,
    Database,
  } from "@lucide/svelte";

  // 상태 관리
  let showUninstallDialog = $state(false);
  let installedVersion = $state<PobVersion | null>(null);
  let latestVersion = $state<GoogleDriveFileInfo | null>(null);
  let latestVersionString = $state<string | null>(null);
  let installPath = $state<string | null>(null);
  let isInitialLoading = $state(true);
  let isFetchingLatest = $state(false);
  let installProgress = $state<InstallProgress | null>(null);
  let error = $state<{ kind: string; message?: string } | null>(null);

  // 파생 상태
  const isInstalled = $derived(installedVersion !== null);
  const isInstalling = $derived(
    installProgress !== null &&
      (installProgress.status === "started" ||
        installProgress.status === "inProgress"),
  );

  const appStatus = $derived<
    "idle" | "update_available" | "updating" | "not_installed"
  >(
    !installedVersion
      ? "not_installed"
      : isInstalling
        ? "updating"
        : latestVersionString &&
            installedVersion.version !== latestVersionString
          ? "update_available"
          : "idle",
  );

  const progress = $derived(
    installProgress?.status === "inProgress"
      ? installProgress.percent
      : installProgress?.status === "completed"
        ? 100
        : 0,
  );

  const versionInfo = $derived({
    installed: installedVersion
      ? `v${installedVersion.version}`
      : "v2026.01.12",
    latest: latestVersionString ? `v${latestVersionString}` : "v2026.01.13",
  });

  // UI 설정 상수
  type StatusConfig = {
    color: string;
    icon: typeof Icon;
    text: string;
    animate?: string;
  };

  const STATUS_CONFIGS: Record<typeof appStatus, StatusConfig> = {
    idle: {
      color: "bg-success/10 text-success border-success/20",
      icon: CircleCheck,
      text: "최신 버전",
    },
    update_available: {
      color: "bg-warning/10 text-warning border-warning/20",
      icon: CircleAlert,
      text: "업데이트 가능",
    },
    updating: {
      color: "bg-info/10 text-info border-info/20",
      icon: RefreshCw,
      text: "업데이트 중...",
      animate: "animate-spin",
    },
    not_installed: {
      color: "bg-slate-500/10 text-slate-400 border-slate-500/20",
      icon: Download,
      text: "미설치",
    },
  };

  const statusConfig = $derived(STATUS_CONFIGS[appStatus]);
  const StatusIcon = $derived(statusConfig.icon);

  const PHASE_TEXT: Record<string, string> = {
    preparing: "준비 중",
    downloading: "다운로드 중",
    extracting: "압축 해제 중",
    backingUp: "백업 중",
    moving: "이동 중",
    restoring: "복구 중",
    finalizing: "마무리 중",
    uninstalling: "제거 중",
  };

  const QUICK_LINKS = [
    {
      title: "PoE Ninja",
      desc: "시세 및 빌드 통계",
      icon: ChartColumn,
      color: "text-purple-400",
      bg: "bg-purple-500/10 border-purple-500/20",
      url: "https://poe.ninja",
    },
    {
      title: "Trade",
      desc: "공식 거래소",
      icon: Coins,
      color: "text-yellow-400",
      bg: "bg-yellow-500/10 border-yellow-500/20",
      url: "https://poe.game.daum.net/trade",
    },
    {
      title: "Craft of Exile",
      desc: "제작 시뮬레이터",
      icon: Hammer,
      color: "text-blue-400",
      bg: "bg-blue-500/10 border-blue-500/20",
      url: "https://www.craftofexile.com",
    },
    {
      title: "PoeDB",
      desc: "데이터베이스",
      icon: Database,
      color: "text-green-400",
      bg: "bg-green-500/10 border-green-500/20",
      url: "https://poedb.tw",
    },
  ] as const;

  // 초기화 및 이벤트 리스너 ($effect로 변경)
  $effect(() => {
    const init = async () => {
      const unlisten = await events.installProgress.listen((event) => {
        installProgress = event.payload;
      });

      await Promise.all([
        checkInstalledVersion(),
        checkLatestVersion(false),
        fetchInstallPath(),
      ]);

      isInitialLoading = false;
      return unlisten;
    };

    const cleanup = init();
    return () => {
      cleanup.then((unlisten) => unlisten());
    };
  });

  // 설치 진행 상태 변화 감지 ($effect로 side effect 분리)
  $effect(() => {
    if (!installProgress) return;

    const { status } = installProgress;
    if (status === "completed") {
      toast.success("설치 완료", {
        description: "Path of Building이 성공적으로 설치되었습니다.",
      });
      setTimeout(checkInstalledVersion, 500);
    } else if (status === "failed") {
      toast.error("설치 실패", { description: "설치 중 오류가 발생했습니다." });
      setTimeout(checkInstalledVersion, 500);
    } else if (status === "cancelled") {
      toast.info("설치 취소", { description: "사용자에 의해 취소되었습니다." });
      setTimeout(checkInstalledVersion, 500);
    }
  });

  // 유틸리티 함수
  function handleError(errorKind: ErrorKind, context: string) {
    if (errorKind.kind === "cancelled") return;
    error = { kind: errorKind.kind, message: errorKind.message };
    toast.error(context, { description: errorKind.message });
  }

  function getPhaseText(phase: string): string {
    return PHASE_TEXT[phase] ?? phase;
  }

  // API 호출 함수
  async function fetchInstallPath() {
    const result = await commands.getInstallPath();
    if (result.status === "ok") {
      installPath = result.data;
    }
  }

  async function checkInstalledVersion() {
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
          toast.success("새로고침 완료", {
            description: "최신 버전 정보를 가져왔습니다.",
          });
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
    error = null;
    try {
      const result = await commands.uninstallPob();
      if (result.status === "error") {
        handleError(result.error, "제거 실패");
      } else {
        await checkInstalledVersion();
        toast.success("제거 완료", {
          description: "Path of Building이 제거되었습니다.",
        });
      }
    } catch (e) {
      error = { kind: "unknown", message: `오류: ${e}` };
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
      await openUrl(
        "https://gall.dcinside.com/mgallery/board/view/?id=pathofexile&no=991032",
      );
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

  async function openExternalLink(url: string) {
    try {
      await openUrl(url);
    } catch (e) {
      toast.error("링크 열기 실패", { description: String(e) });
    }
  }
</script>

<div class="flex-1 p-4 md:p-6 overflow-y-auto flex justify-center items-start">
  <div class="w-full max-w-4xl mx-auto space-y-6">
    <!-- Error Banner -->
    {#if error && error.kind !== 'cancelled'}
      <Alert variant="destructive" class="relative">
        <AlertCircle class="h-4 w-4" />
        <AlertComp.AlertTitle>오류 발생</AlertComp.AlertTitle>
        <AlertComp.AlertDescription>
          {error.message || '알 수 없는 오류가 발생했습니다.'}
        </AlertComp.AlertDescription>
        <Button
          variant="ghost"
          size="sm"
          onclick={() => error = null}
          class="absolute top-2 right-2 h-6 w-6 p-0"
        >
          <X size={14} />
        </Button>
      </Alert>
    {/if}

    <!-- Hero Section: Status & Main Action -->
    {#if isInitialLoading}
      <!-- Loading Skeleton -->
      <Card class="relative border-border shadow-2xl">
        <CardContent class="p-8">
          <div class="space-y-6">
            <div class="flex flex-col md:flex-row justify-between items-start md:items-center gap-6">
              <div class="space-y-3 flex-1">
                <Skeleton class="h-8 w-48" />
                <Skeleton class="h-4 w-64" />
              </div>
              <div class="flex gap-3">
                <Skeleton class="h-11 w-24" />
                <Skeleton class="h-11 w-24" />
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    {:else}
    <div class="relative group">
      <!-- Background Glow -->
      <div
        class={`absolute -inset-0.5 bg-linear-to-r rounded-2xl blur opacity-30 transition duration-500
          ${appStatus === "update_available" ? "from-orange-500 to-amber-500" : "from-blue-500 to-indigo-500"}`}
      ></div>

      <Card class="relative border-border shadow-2xl">
        <CardContent class="p-8">
          <div
            class="flex flex-col md:flex-row justify-between items-start md:items-center gap-6"
          >
            <!-- Left: Status Info -->
            <div class="space-y-2">
              <div class="flex items-center gap-3">
                <h2 class="text-2xl font-bold text-foreground tracking-tight">
                  한글 POB
                </h2>
                <Badge
                  variant="outline"
                  class={`px-2.5 py-0.5 flex items-center gap-1.5 ${statusConfig.color}`}
                >
                  <StatusIcon size={12} class={statusConfig.animate || ""} />
                  {statusConfig.text}
                </Badge>
              </div>
              <div
                class="text-muted-foreground flex items-center gap-4 text-sm"
              >
                <span class="flex items-center gap-1.5">
                  <span class="w-2 h-2 rounded-full bg-muted-foreground/50"
                  ></span>
                  현재:
                  <span class="text-foreground font-mono"
                    >{versionInfo.installed}</span
                  >
                </span>
                {#if appStatus !== "not_installed"}
                  <span class="flex items-center gap-1.5">
                    <span
                      class={`w-2 h-2 rounded-full ${appStatus === "update_available" ? "bg-orange-500 animate-pulse" : "bg-green-500"}`}
                    ></span>
                    최신:
                    <span
                      class={`${appStatus === "update_available" ? "text-orange-400 font-bold" : "text-foreground"} font-mono`}
                      >{versionInfo.latest}</span
                    >
                  </span>
                {/if}
              </div>
            </div>

            <!-- Right: Primary Action -->
            <div class="flex flex-wrap items-center gap-3">
              {#if appStatus === "updating"}
                <Button
                  onclick={cancelInstall}
                  variant="outline"
                  class="px-6 py-3"
                >
                  <X size={18} /> 취소
                </Button>
              {:else if appStatus === "update_available"}
                <Button
                  onclick={install}
                  disabled={!latestVersion}
                  class="px-8 py-3 bg-linear-to-r from-orange-500 to-amber-600 hover:from-orange-400 hover:to-amber-500 text-white font-bold shadow-lg shadow-orange-500/20"
                >
                  <Download size={20} /> 업데이트
                </Button>
                <Button
                  onclick={execute}
                  disabled={!isInstalled}
                  class="px-8 py-3 bg-blue-600 hover:bg-blue-500 text-white font-bold shadow-lg shadow-blue-500/20"
                >
                  <Play size={20} fill="currentColor" /> 실행
                </Button>
              {:else if appStatus === "not_installed"}
                <Button
                  onclick={install}
                  disabled={!latestVersion}
                  class="px-8 py-3 bg-blue-600 hover:bg-blue-500 text-white font-bold shadow-lg shadow-blue-500/20"
                >
                  <Download size={20} /> 설치
                </Button>
              {:else}
                <Button
                  onclick={execute}
                  disabled={!isInstalled}
                  class="px-8 py-3 bg-blue-600 hover:bg-blue-500 text-white font-bold shadow-lg shadow-blue-500/20"
                >
                  <Play size={20} fill="currentColor" /> 실행
                </Button>
              {/if}
            </div>
          </div>

          <!-- Progress Bar Area -->
          {#if appStatus === "updating"}
            <div class="mt-8 space-y-2" transition:slide={{ duration: 300 }}>
              <div
                class="flex justify-between text-xs font-medium text-muted-foreground"
              >
                <span class="text-info">
                  {installProgress
                    ? getPhaseText(installProgress.phase)
                    : "다운로드 중..."}
                </span>
                <span>{progress.toFixed(0)}%</span>
              </div>
              <div class="relative">
                <Progress value={progress} max={100} class="h-2" />
                <!-- Shimmer effect -->
                <div class="absolute inset-0 overflow-hidden rounded-full">
                  <div class="h-full w-1/2 bg-gradient-to-r from-transparent via-white/10 to-transparent animate-shimmer"></div>
                </div>
              </div>
            </div>
          {/if}
        </CardContent>
      </Card>
    </div>
    {/if}

    <!-- Info Grid Section -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
      <!-- Details Card -->
      <Card class="border-border flex flex-col justify-between">
        <CardContent>
          <div class="flex items-center justify-between mb-6">
            <h3 class="font-semibold text-foreground flex items-center gap-2">
              <BookOpen size={18} class="text-muted-foreground" />
              관리
            </h3>
            <Button
              onclick={() => checkLatestVersion(true)}
              variant="ghost"
              size="sm"
              disabled={isFetchingLatest}
              class="text-xs"
            >
              {#if isFetchingLatest}
                <RefreshCw size={14} class="animate-spin mr-1" />
                확인 중...
              {:else}
                <RefreshCw size={14} class="mr-1" />
                업데이트 확인
              {/if}
            </Button>
          </div>

          <div class="space-y-3">
            <Button
              variant="outline"
              onclick={openInstallFolder}
              disabled={!isInstalled}
              class="w-full justify-start gap-2"
            >
              <FolderOpen size={16} /> 설치 폴더 열기
            </Button>

            <Button
              variant="outline"
              onclick={openSource}
              class="w-full justify-start gap-2"
            >
              <ExternalLink size={16} />
              <span class="truncate">POE1&2 한글 POB - 패스 오브 엑자일 갤러리</span>
            </Button>
          </div>

          <div
            class="pt-4 mt-6 border-t border-border flex items-center justify-end"
          >
            <Button
              onclick={() => (showUninstallDialog = true)}
              variant="ghost"
              size="sm"
              disabled={!isInstalled}
              class="text-xs text-destructive hover:text-destructive hover:bg-destructive/10"
            >
              <Trash2 size={14} /> 앱 제거
            </Button>
          </div>
        </CardContent>
      </Card>

      <!-- Quick Links Section -->
      <Card class="border-border flex flex-col">
        <CardContent>
          <h3
            class="font-semibold text-foreground mb-4 flex items-center gap-2"
          >
            <LinkIcon size={18} class="text-muted-foreground" />
            유용한 링크
          </h3>
          <div class="grid grid-cols-2 gap-3">
            {#each QUICK_LINKS as link (link.url)}
              {@const LinkIcon = link.icon}
              <button
                onclick={() => openExternalLink(link.url)}
                class={`flex flex-col items-start p-4 h-auto rounded-md ${link.bg} border transition-all hover:scale-[1.02] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2`}
              >
                <div class={`mb-2 ${link.color}`}>
                  <LinkIcon size={20} />
                </div>
                <div
                  class="font-semibold text-foreground text-sm truncate w-full text-left"
                >
                  {link.title}
                </div>
                <div class="text-xs text-muted-foreground truncate w-full text-left">
                  {link.desc}
                </div>
              </button>
            {/each}
          </div>
        </CardContent>
      </Card>
    </div>
  </div>
</div>

<!-- Uninstall Confirmation Dialog -->
<AlertDialog.Root bind:open={showUninstallDialog}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>PoB 제거</AlertDialog.Title>
      <AlertDialog.Description>
        제거하시겠습니까? 이 작업은 취소할 수 없습니다.
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel>취소</AlertDialog.Cancel>
      <AlertDialog.Action
        onclick={uninstall}
        class="bg-destructive text-white hover:bg-destructive/90"
      >
        제거
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
