<script lang="ts">
	import { page } from "$app/stores";
	import * as Sidebar from "@/components/ui/sidebar";
	import BookOpenIcon from "@lucide/svelte/icons/book-open";
	import PackageIcon from "@lucide/svelte/icons/package";
	import SettingsIcon from "@lucide/svelte/icons/settings";
	import SwordsIcon from "@lucide/svelte/icons/swords";
	import TimerIcon from "@lucide/svelte/icons/timer";
	import type { Component } from "svelte";

	const menuItems: { title: string; url: string; icon: Component }[] = [
		{ title: "Path of Building", url: "/", icon: PackageIcon },
		{ title: "타이머", url: "/timer", icon: TimerIcon },
		{ title: "레벨링 가이드", url: "/guide", icon: BookOpenIcon },
		{ title: "설정", url: "/settings", icon: SettingsIcon },
	];
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
								<span class="truncate font-semibold">Exile Manager</span>
								<span class="truncate text-xs text-muted-foreground">Desktop App</span>
							</div>
						</a>
					{/snippet}
				</Sidebar.MenuButton>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Header>

	<Sidebar.Content>
		<Sidebar.Group>
			<Sidebar.GroupLabel>Navigation</Sidebar.GroupLabel>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					{#each menuItems as item (item.title)}
						<Sidebar.MenuItem>
							<Sidebar.MenuButton isActive={item.url === "/" ? $page.url.pathname === "/" : $page.url.pathname.startsWith(item.url)} tooltipContent={item.title}>
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

	<Sidebar.Footer />
	<Sidebar.Rail />
</Sidebar.Root>
