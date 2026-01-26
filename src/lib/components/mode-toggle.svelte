<script lang="ts">
	import { toggleMode } from "mode-watcher";
	import { Button } from "@/components/ui/button";
	import MoonIcon from "@lucide/svelte/icons/moon";
	import SunIcon from "@lucide/svelte/icons/sun";
</script>

<Button onclick={toggleMode} variant="ghost" size="icon" class="size-7">
	<SunIcon class="size-4 scale-100 rotate-0 transition-all dark:scale-0 dark:-rotate-90" />
	<MoonIcon class="absolute size-4 scale-0 rotate-90 transition-all dark:scale-100 dark:rotate-0" />
	<span class="sr-only">테마 전환</span>
</Button>
