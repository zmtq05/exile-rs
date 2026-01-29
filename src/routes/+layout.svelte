<script lang="ts">
	import './layout.css';
	import "../app.css";
	import { ModeWatcher } from "mode-watcher";
	import { Toaster } from "@/components/ui/sonner";
	import * as Sidebar from "@/components/ui/sidebar";
	import AppSidebar from "@/components/app-sidebar.svelte";
	import AppHeader from "@/components/app-header.svelte";
	import UpdateDialog from "@/components/update-dialog.svelte";
	import { onMount } from "svelte";
	import { loadSettings, getSettings } from "@/stores/settings.svelte";
	import { checkForUpdate, getUpdateState } from "@/stores/updater.svelte";

	let { children } = $props();
	let updateDialogOpen = $state(false);

	onMount(async () => {
		const settings = await loadSettings();
		if (settings.autoCheckUpdate) {
			const update = await checkForUpdate();
			if (update) {
				updateDialogOpen = true;
			}
		}
	});
</script>

<ModeWatcher />
<Toaster richColors position="top-right" />
<UpdateDialog bind:open={updateDialogOpen} />

<Sidebar.Provider>
	<AppSidebar />
	<Sidebar.Inset>
		<AppHeader />
		<main class="flex-1 overflow-auto">
			{@render children?.()}
		</main>
	</Sidebar.Inset>
</Sidebar.Provider>
