export const NORMAL_SPEED = 1000;
const QUICKER = 1250;
const FAST = 1500;
const DOUBLE = 2000;
const SLOW = 750;
const SPEEDS: readonly number[] = [NORMAL_SPEED, QUICKER, FAST, DOUBLE, SLOW];
const PER_MILLE = 1000;

export function nextSpeed(speed: number): number {
  return SPEEDS[(SPEEDS.indexOf(speed) + 1) % SPEEDS.length] ?? NORMAL_SPEED;
}

export function speedLabel(speed: number): string {
  return `${String(speed / PER_MILLE)}×`;
}

export function storedSpeed(stored: string | null): number {
  const speed = Number(stored);
  return SPEEDS.includes(speed) ? speed : NORMAL_SPEED;
}
