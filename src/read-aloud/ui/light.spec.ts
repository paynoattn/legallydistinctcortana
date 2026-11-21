import { afterEach, describe, expect, it } from 'vitest';
import { lightOn, wordClicks } from './light.ts';

interface Box {
  readonly left: number;
  readonly top: number;
  readonly width: number;
  readonly height: number;
}

function place(element: HTMLElement, box: Box): void {
  Object.defineProperties(element, {
    offsetLeft: { configurable: true, value: box.left },
    offsetTop: { configurable: true, value: box.top },
    offsetWidth: { configurable: true, value: box.width },
    offsetHeight: { configurable: true, value: box.height },
  });
}

interface Page {
  readonly scroller: HTMLElement;
  readonly text: HTMLElement;
  readonly light: HTMLElement;
  readonly first: HTMLElement;
  readonly second: HTMLElement;
  readonly below: HTMLElement;
}

function found(selector: string): HTMLElement {
  return document.querySelector<HTMLElement>(selector) ?? document.body;
}

function nth(elements: readonly HTMLElement[], index: number): HTMLElement {
  const element = elements[index];
  if (element === undefined) {
    throw new Error('the page is incomplete');
  }
  return element;
}

function page(): Page {
  document.body.innerHTML = `
    <div class="reader-scroller">
      <div class="reader-text">
        <div class="reader-light"></div>
        <p><span data-sentence="0"><span class="word">One</span> <span class="word">two</span></span></p>
        <p><span data-sentence="1"><span class="word">Three</span></span></p>
      </div>
    </div>`;
  const scroller = found('.reader-scroller');
  const text = found('.reader-text');
  const light = found('.reader-light');
  const words = [...document.querySelectorAll<HTMLElement>('.word')];
  const first = nth(words, 0);
  const second = nth(words, 1);
  const below = nth(words, 2);
  Object.defineProperty(scroller, 'clientHeight', { configurable: true, value: 600 });
  place(text, { left: 0, top: 50, width: 680, height: 2000 });
  place(first, { left: 10, top: 101, width: 40, height: 30 });
  place(second, { left: 60, top: 101, width: 30, height: 30 });
  place(below, { left: 10, top: 901, width: 50, height: 30 });
  return { scroller, text, light, first, second, below };
}

afterEach(() => {
  document.body.innerHTML = '';
});

describe('lightOn', () => {
  it('puts the light behind the lit word with a little room around it', () => {
    const { light, first } = page();
    lightOn(false, 25)(first);
    expect(light.style.transform).toBe('translate(5px, 100px)');
    expect(light.style.width).toBe('50px');
    expect(light.style.height).toBe('32px');
  });

  it('glides along a line and jumps to a new line or a new text size', () => {
    const { light, first, second, below } = page();
    const onLit = lightOn(false, 25);
    onLit(first);
    expect(light.hasAttribute('data-jump')).toBe(true);
    onLit(second);
    expect(light.hasAttribute('data-jump')).toBe(false);
    onLit(below);
    expect(light.hasAttribute('data-jump')).toBe(true);
    lightOn(false, 28)(below);
    expect(light.hasAttribute('data-jump')).toBe(true);
  });

  it('keeps the lit line a third of the way down while following', () => {
    const { scroller, below } = page();
    lightOn(true, 25)(below);
    expect(scroller.scrollTop).toBe(50 + 900 - 200);
  });

  it('scrolls on the first word it lights, then only on a new line', () => {
    const { scroller, first, second } = page();
    const onLit = lightOn(true, 25);
    onLit(first);
    expect(scroller.scrollTop).toBe(0);
    scroller.scrollTop = 500;
    onLit(second);
    expect(scroller.scrollTop).toBe(500);
    lightOn(true, 25)(second);
    expect(scroller.scrollTop).toBe(0);
  });

  it('does not scroll while not following', () => {
    const { scroller, below } = page();
    scroller.scrollTop = 10;
    lightOn(false, 25)(below);
    expect(scroller.scrollTop).toBe(10);
  });

  it('does nothing for a word that is no longer lit or not in a text', () => {
    const { light } = page();
    const onLit = lightOn(true, 25);
    onLit(null);
    onLit(document.createElement('span'));
    expect(light.style.transform).toBe('');
  });

  it('does nothing for a text without a light or a scroller', () => {
    document.body.innerHTML = '<div class="reader-text"><span class="word">Alone</span></div>';
    const word = document.querySelector<HTMLElement>('.word');
    expect(() => {
      lightOn(true, 25)(word);
    }).not.toThrow();
  });
});

describe('wordClicks', () => {
  it('reports the sentence of a clicked word until it is detached', () => {
    const { text, below, first } = page();
    const clicked: number[] = [];
    const detach = wordClicks((index) => clicked.push(index))(text);
    below.click();
    first.click();
    text.click();
    detach?.();
    below.click();
    expect(clicked).toEqual([1, 0]);
  });

  it('attaches nothing without a text', () => {
    expect(wordClicks(() => undefined)(null)).toBeUndefined();
  });
});
