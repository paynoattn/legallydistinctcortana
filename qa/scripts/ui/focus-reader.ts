import { chromium } from 'playwright';
import {
  choosesWhilePaused,
  escapeStopsAndCloses,
  keepsReadingWithoutFocus,
  readsANewCapture,
  remembersSizeAndMood,
  sizesTheText,
  stopsFollowingOnScroll,
} from './reader/control-steps.ts';
import {
  finishesAndReadsAgain,
  nothingSelected,
  reducedMotionJumps,
  screenReaderShape,
  voiceServerDown,
} from './reader/edge-steps.ts';
import { manual, openReader, selectionOf, step } from './reader/harness.ts';
import { article } from './reader/reader-page.ts';
import {
  changesMood,
  changesVoice,
  clickReadsFromSentence,
  lightsTheSpokenWord,
  movesBySentence,
  opensAndReads,
  pausesAndResumes,
} from './reader/reading-steps.ts';

const browser = await chromium.launch();

try {
  const page = await openReader(browser, { selection: selectionOf(article()) });
  await step(1, () => opensAndReads(page));
  await step(2, () => lightsTheSpokenWord(page));
  await step(3, () => pausesAndResumes(page));
  await step(4, () => movesBySentence(page));
  await step(5, () => clickReadsFromSentence(page));
  await step(6, () => changesMood(page));
  await step(7, () => changesVoice(page));
  await step(8, () => choosesWhilePaused(page));
  await step(9, () => sizesTheText(page));
  await step(10, () => stopsFollowingOnScroll(page));
  await step(11, () => keepsReadingWithoutFocus(page));
  await step(12, () => readsANewCapture(page));
  await step(13, () => escapeStopsAndCloses(page));
  await step(14, () => remembersSizeAndMood(page));
  await page.context().close();
  await step(15, () => finishesAndReadsAgain(browser));
  await step(16, () => reducedMotionJumps(browser));
  manual(17, 'placing the window on the monitor under the pointer is done by the desktop shell');
  await step(18, () => nothingSelected(browser));
  await step(19, () => voiceServerDown(browser));
  await step(20, () => screenReaderShape(browser));
} finally {
  await browser.close();
}
