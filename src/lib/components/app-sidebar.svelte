<script lang="ts">
	import { page } from "$app/state";
	import * as Sidebar from "@/components/ui/sidebar";
	import PackageIcon from "@lucide/svelte/icons/package";
	import SettingsIcon from "@lucide/svelte/icons/settings";
	import SwordsIcon from "@lucide/svelte/icons/swords";
	import type { Component } from "svelte";
	import { getVersion } from "@tauri-apps/api/app";

	const menuItems: { title: string; url: string; icon: Component }[] = [
		{ title: "한글 POB", url: "/", icon: PackageIcon },
		{ title: "설정", url: "/settings", icon: SettingsIcon },
	];

	let appVersion = $state<string | null>(null);

	$effect(() => {
		getVersion().then((v) => {
			appVersion = v;
		});
	});
</script>

<Sidebar.Root collapsible="icon">
	<Sidebar.Header>
		<Sidebar.Menu>
			<Sidebar.MenuItem>
				<Sidebar.MenuButton size="lg" class="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground">
					{#snippet child({ props })}
						<a href="/" {...props}>
							<div class="bg-sidebar-primary text-sidebar-primary-foreground flex aspect-square size-8 items-center justify-center rounded-lg">
								<SwordsIcon class="size-4" />
							</div>
							<div class="grid flex-1 text-left text-sm leading-tight">
								<span class="truncate font-semibold">exile-rs</span>
								<!-- <span class="truncate text-xs text-muted-foreground">Desktop App</span> -->
							</div>
						</a>
					{/snippet}
				</Sidebar.MenuButton>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Header>

	<Sidebar.Content>
		<Sidebar.Group>
			<Sidebar.GroupLabel>메뉴</Sidebar.GroupLabel>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					{#each menuItems as item (item.title)}
						<Sidebar.MenuItem>
							<Sidebar.MenuButton isActive={item.url === "/" ? page.url.pathname === "/" : page.url.pathname.startsWith(item.url)} tooltipContent={item.title}>
								{#snippet child({ props })}
									<a href={item.url} {...props}>
										<item.icon />
										<span>{item.title}</span>
									</a>
								{/snippet}
							</Sidebar.MenuButton>
						</Sidebar.MenuItem>
					{/each}
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>
	</Sidebar.Content>

	<Sidebar.Footer class="p-2">
		<div class="flex items-center justify-center text-xs text-muted-foreground">
			<span class="group-data-[collapsible=icon]:hidden">v{appVersion ?? "..."}</span>
		</div>
	</Sidebar.Footer>
	<Sidebar.Rail />
</Sidebar.Root>
