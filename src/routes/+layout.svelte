<script lang="ts">
	import './layout.css';
	import "../app.css";
	import { ModeWatcher } from "mode-watcher";
	import { Toaster } from "@/components/ui/sonner";
	import * as Sidebar from "@/components/ui/sidebar";
	import AppSidebar from "@/components/app-sidebar.svelte";
	import AppHeader from "@/components/app-header.svelte";

	let { children } = $props();
</script>

<ModeWatcher />
<Toaster richColors position="top-right" />

<Sidebar.Provider>
	<AppSidebar />
	<Sidebar.Inset>
		<AppHeader />
		<main class="flex-1 overflow-auto">
			{@render children?.()}
		</main>
	</Sidebar.Inset>
</Sidebar.Provider>
