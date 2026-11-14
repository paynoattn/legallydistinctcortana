export function nextFocusIndex(key: string, current: number, count: number): number | undefined {
  if (count === 0) {
    return undefined;
  }
  const from = Math.max(current, 0);
  switch (key) {
    case 'ArrowDown':
      return (from + 1) % count;
    case 'ArrowUp':
      return (from - 1 + count) % count;
    case 'Home':
      return 0;
    case 'End':
      return count - 1;
    default:
      return undefined;
  }
}
