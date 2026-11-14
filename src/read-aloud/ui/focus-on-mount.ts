export function focusOnMount(element: HTMLElement | null): void {
  element?.focus();
}

export function focusFirstMenuItem(): void {
  document.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
}
