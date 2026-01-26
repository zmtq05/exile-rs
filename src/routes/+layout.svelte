<script lang="ts">
	import './layout.css';
	import "../app.css";
	import * as Sidebar from "@/components/ui/sidebar";
	import AppSidebar from "@/components/app-sidebar.svelte";

	let { children } = $props();
</script>

<Sidebar.Provider>
	<AppSidebar />
	<Sidebar.Inset>
		<main class="flex-1 overflow-auto">
			{@render children?.()}
		</main>
	</Sidebar.Inset>
</Sidebar.Provider>
