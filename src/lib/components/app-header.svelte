<script lang="ts">
	import { page } from "$app/stores";
	import * as Breadcrumb from "@/components/ui/breadcrumb";
	import * as Sidebar from "@/components/ui/sidebar";
	import { Separator } from "@/components/ui/separator";
	import ModeToggle from "./mode-toggle.svelte";

	const routeTitles: Record<string, string> = {
		"/": "Path of Building",
		"/timer": "타이머",
		"/guide": "레벨링 가이드",
		"/settings": "설정",
	};

	let currentTitle = $derived(routeTitles[$page.url.pathname] || "페이지");
</script>

<header class="flex h-14 shrink-0 items-center gap-2 border-b px-4">
	<Sidebar.Trigger class="-ml-1" />
	<Separator orientation="vertical" class="mr-2 h-4" />
	<Breadcrumb.Root>
		<Breadcrumb.List>
			<Breadcrumb.Item>
				<Breadcrumb.Link href="/">Exile Manager</Breadcrumb.Link>
			</Breadcrumb.Item>
			<Breadcrumb.Separator />
			<Breadcrumb.Item>
				<Breadcrumb.Page>{currentTitle}</Breadcrumb.Page>
			</Breadcrumb.Item>
		</Breadcrumb.List>
	</Breadcrumb.Root>
	<div class="ml-auto">
		<ModeToggle />
	</div>
</header>
