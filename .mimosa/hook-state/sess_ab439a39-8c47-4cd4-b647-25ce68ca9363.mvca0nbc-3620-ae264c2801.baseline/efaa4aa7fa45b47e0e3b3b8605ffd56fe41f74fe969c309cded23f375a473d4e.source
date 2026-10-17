/**
 * Svelte action: trap Tab focus inside the node while it is mounted, focus
 * the first focusable element on mount, and restore focus to the previously
 * focused element on destroy. Modal dialogs and the command palette use it.
 */
export function focusTrap(node: HTMLElement) {
  const previouslyFocused = document.activeElement as HTMLElement | null;

  const focusables = (): HTMLElement[] =>
    Array.from(
      node.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      )
    ).filter((el) => !el.hasAttribute('disabled') && el.offsetParent !== null);

  // Initial focus: first focusable, else the node itself.
  const targets = focusables();
  (targets[0] ?? node).focus();

  function onKeydown(event: KeyboardEvent) {
    if (event.key !== 'Tab') return;
    const elements = focusables();
    if (elements.length === 0) return;
    const first = elements[0];
    const last = elements[elements.length - 1];
    const active = document.activeElement;

    if (event.shiftKey && active === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && active === last) {
      event.preventDefault();
      first.focus();
    }
  }

  node.addEventListener('keydown', onKeydown);

  return {
    destroy() {
      node.removeEventListener('keydown', onKeydown);
      previouslyFocused?.focus();
    }
  };
}
