<script lang="ts">
  import * as AlertDialog from "@/components/ui/alert-dialog";
  import { Button } from "@/components/ui/button";
  import { Progress } from "@/components/ui/progress";
  import {
    getUpdateState,
    downloadAndInstall,
    resetUpdateState,
  } from "@/stores/updater.svelte";

  let {
    open = $bindable(false),
  }: {
    open?: boolean;
  } = $props();

  const updateState = getUpdateState();

  function handleLater() {
    open = false;
    resetUpdateState();
  }

  async function handleInstall() {
    await downloadAndInstall();
  }
</script>

<AlertDialog.Root bind:open>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>
        {#if updateState.downloading}
          업데이트 다운로드 중...
        {:else}
          업데이트 (v{updateState.update?.version})가 있습니다
        {/if}
      </AlertDialog.Title>
      <AlertDialog.Description>
        {#if updateState.downloading}
          <div class="space-y-2">
            <Progress value={updateState.progress} max={100} />
            <p class="text-sm text-muted-foreground text-center">
              {updateState.progress}%
            </p>
          </div>
        {:else if updateState.error}
          <p class="text-destructive">{updateState.error}</p>
        {:else}
          새로운 버전을 설치하면 앱이 다시 시작됩니다.
        {/if}
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      {#if !updateState.downloading}
        <AlertDialog.Cancel onclick={handleLater}>나중에</AlertDialog.Cancel>
        <Button onclick={handleInstall} disabled={updateState.downloading}>
          설치 및 다시 시작
        </Button>
      {/if}
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
