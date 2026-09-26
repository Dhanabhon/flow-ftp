<script lang="ts">
  import { theme } from '$lib/stores/theme.svelte';
  import { IconSun, IconMoon } from '$lib/components/icons';
  import Tooltip from './tooltip.svelte';
  import Button from './button.svelte';

  // Animate icon swap
  let spinning = $state(false);
  function onClick() {
    spinning = true;
    theme.toggle();
    setTimeout(() => (spinning = false), 250);
  }

  const isDark = $derived(theme.resolved === 'dark');
</script>

<Tooltip label={isDark ? 'Switch to Light' : 'Switch to Dark'} side="bottom">
  <Button
    variant="ghost"
    size="icon-sm"
    onclick={onClick}
    aria-label={isDark ? 'Switch to light theme' : 'Switch to dark theme'}
  >
    <span
      class="inline-flex transition-transform duration-200"
      class:rotate-90={spinning}
    >
      {#if isDark}
        <IconMoon size={15} />
      {:else}
        <IconSun size={15} />
      {/if}
    </span>
  </Button>
</Tooltip>
