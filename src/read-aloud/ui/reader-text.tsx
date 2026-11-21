import { Fragment, type PointerEvent, type ReactElement } from 'react';
import type { ReaderStore } from '../application/reader-store.ts';
import { paragraphsOf, type Sentence, wordsOf } from '../domain/read-aloud-state.ts';
import { FOLLOW } from './icon-paths.ts';
import { Icon } from './icons.tsx';
import { lightOn, wordClicks } from './light.ts';

type SentenceState = 'read' | 'now' | 'ahead';

interface ReaderTextProps {
  readonly store: ReaderStore;
  readonly sentences: readonly Sentence[];
  readonly sentence: number;
  readonly word: number | null;
  readonly textSize: number;
  readonly following: boolean;
}

interface SentenceViewProps {
  readonly index: number;
  readonly text: string;
  readonly state: SentenceState;
  readonly lit: number | null;
  readonly onLit: (word: HTMLElement | null) => void;
}

interface ParagraphsProps {
  readonly sentences: readonly Sentence[];
  readonly sentence: number;
  readonly word: number | null;
  readonly onLit: (word: HTMLElement | null) => void;
}

function stateOf(index: number, current: number): SentenceState {
  if (index < current) {
    return 'read';
  }
  return index === current ? 'now' : 'ahead';
}

function SentenceView({ index, text, state, lit, onLit }: SentenceViewProps): ReactElement {
  return (
    <span className="sentence" data-sentence={index} data-state={state}>
      {wordsOf(text).map((word, position) => (
        <Fragment key={`${String(position)}:${word}`}>
          {position > 0 ? ' ' : null}
          <span
            className={position === lit ? 'word is-lit' : 'word'}
            ref={position === lit ? onLit : undefined}
          >
            {word}
          </span>
        </Fragment>
      ))}
    </span>
  );
}

function Paragraphs({ sentences, sentence, word, onLit }: ParagraphsProps): ReactElement {
  return (
    <>
      {paragraphsOf(sentences).map((paragraph) => (
        <p key={paragraph[0]?.index}>
          {paragraph.map(({ index, text }) => (
            <Fragment key={index}>
              <SentenceView
                index={index}
                text={text}
                state={stateOf(index, sentence)}
                lit={index === sentence ? word : null}
                onLit={onLit}
              />{' '}
            </Fragment>
          ))}
        </p>
      ))}
    </>
  );
}

function FollowButton({ store }: { readonly store: ReaderStore }): ReactElement {
  return (
    <button type="button" className="reader-follow" onClick={store.follow}>
      <Icon d={FOLLOW} lined />
      Follow the voice
    </button>
  );
}

function stopOnScrollbar(store: ReaderStore): (event: PointerEvent<HTMLDivElement>) => void {
  return (event) => {
    if (event.target === event.currentTarget) {
      store.stopFollowing();
    }
  };
}

export function ReaderText({
  store,
  sentences,
  sentence,
  word,
  textSize,
  following,
}: ReaderTextProps): ReactElement {
  return (
    <div className="reader-body">
      <div
        className="reader-scroller"
        onWheel={store.stopFollowing}
        onPointerDown={stopOnScrollbar(store)}
      >
        <div
          className="reader-text"
          style={{ fontSize: `${String(textSize)}px` }}
          aria-live="off"
          ref={wordClicks(store.readSentence)}
        >
          <div
            className={word === null ? 'reader-light is-off' : 'reader-light'}
            aria-hidden="true"
          />
          <Paragraphs
            sentences={sentences}
            sentence={sentence}
            word={word}
            onLit={lightOn(following, textSize)}
          />
        </div>
      </div>
      {following ? null : <FollowButton store={store} />}
    </div>
  );
}
