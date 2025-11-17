<script lang="ts">
	import { type KatexOptions, renderToString } from 'katex';
	import type { Snippet } from 'svelte';

	let {
		latex,
		displayMode,
		options,
		children
	}: {
		latex?: string;
		displayMode?: boolean;
		options?: KatexOptions;
		children?: Snippet;
	} = $props();

	let slot: HTMLElement | undefined = $state(undefined);
	let slotContent = $state('');
	$effect(() => {
		slotContent = slot?.innerText ?? '';
	});
	const markup = $derived(latex ?? slotContent ?? '');
	const html = $derived(renderToString(markup, { displayMode, ...options }));
</script>

<!-- Display rendered math-->
{@html html}

<!--Hidden slot-->
{#if children}
	<span class="svelte-math-slot" bind:this={slot} style="display:none">{@render children()}</span>
{/if}