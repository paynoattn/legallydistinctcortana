const PAD_X = 5;
const PAD_Y = 1;
const BOTH_SIDES = 2;
const FOLLOW_THIRDS = 3;

interface Lit {
  readonly word: HTMLElement;
  readonly text: HTMLElement;
  readonly light: HTMLElement;
}

function scrollToLine(text: HTMLElement, top: number): void {
  const scroller = text.parentElement;
  if (scroller !== null) {
    scroller.scrollTop = Math.max(0, text.offsetTop + top - scroller.clientHeight / FOLLOW_THIRDS);
  }
}

function litParts(word: HTMLElement): Lit | undefined {
  const text = word.closest<HTMLElement>('.reader-text');
  const light = text?.querySelector<HTMLElement>('.reader-light') ?? null;
  return text === null || light === null ? undefined : { word, text, light };
}

function cover({ word, light }: Lit, top: number): void {
  const bottom = word.offsetTop + word.offsetHeight + PAD_Y;
  light.style.setProperty('width', `${String(word.offsetWidth + BOTH_SIDES * PAD_X)}px`);
  light.style.setProperty('height', `${String(bottom - top)}px`);
  light.style.setProperty(
    'transform',
    `translate(${String(word.offsetLeft - PAD_X)}px, ${String(top)}px)`,
  );
}

export function lightOn(following: boolean, textSize: number): (word: HTMLElement | null) => void {
  let placed = false;
  return (word) => {
    const lit = word === null ? undefined : litParts(word);
    if (lit === undefined) {
      return;
    }
    const top = lit.word.offsetTop - PAD_Y;
    const line = `${String(top)}@${String(textSize)}`;
    const jump = lit.light.getAttribute('data-line') !== line;
    lit.light.toggleAttribute('data-jump', jump);
    lit.light.setAttribute('data-line', line);
    cover(lit, top);
    if (following && (jump || !placed)) {
      scrollToLine(lit.text, top);
    }
    placed = true;
  };
}

export function wordClicks(
  onSentence: (index: number) => void,
): (text: HTMLElement | null) => (() => void) | undefined {
  return (text) => {
    if (text === null) {
      return undefined;
    }
    const click = (event: MouseEvent): void => {
      const target =
        event.target instanceof Element ? event.target.closest('[data-sentence]') : null;
      const index = Number(target?.getAttribute('data-sentence') ?? Number.NaN);
      if (Number.isInteger(index)) {
        onSentence(index);
      }
    };
    text.addEventListener('click', click);
    return () => {
      text.removeEventListener('click', click);
    };
  };
}
