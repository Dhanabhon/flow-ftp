<script lang="ts">
  import { onMount } from 'svelte';
  import { confirmQuit, onQuitConfirm } from '$lib/ipc';
  import Modal from '$lib/components/ui/modal.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import { IconAlert } from '$lib/components/icons';

  let open = $state(false);
  let pending = $state(0);
  let connected = $state(0);

  function message(p: number, c: number): string {
    const transferPlural = p === 1 ? '' : 's';
    const serverPlural = c === 1 ? '' : 's';
    if (p > 0 && c > 0) {
      return `${p} transfer${transferPlural} are still running or queued, and you are connected to ${c} server${serverPlural}. Quitting stops the transfers and ends the sessions.`;
    }
    if (p > 0) {
      return `${p} transfer${transferPlural} are still running or queued. Quitting now stops them; resumable files keep their progress.`;
    }
    return `You are connected to ${c} server${serverPlural}. Quitting will end those sessions.`;
  }

  onMount(() => {
    let unlisten: (() => void) | null = null;
    let disposed = false;
    onQuitConfirm((info) => {
      pending = info.pending;
      connected = info.connected;
      open = true;
    }).then((un) => {
      if (disposed) un();
      else unlisten = un;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  });
</script>

<Modal bind:open title="Quit FlowFTP?" width="sm">
  <div class="flex items-start gap-3">
    <div class="grid h-10 w-10 shrink-0 place-items-center rounded-full bg-warning/15">
      <IconAlert size={20} class="text-warning" />
    </div>
    <p class="text-sm leading-relaxed text-fg-muted">{message(pending, connected)}</p>
  </div>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
    <Button variant="danger" onclick={() => confirmQuit()}>Quit</Button>
  {/snippet}
</Modal>
