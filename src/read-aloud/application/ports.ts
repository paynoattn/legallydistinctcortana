import type { MoodId } from '../domain/mood.ts';
import type { ReadingStatus, Selection } from '../domain/read-aloud-state.ts';

export interface ReadAloudGateway {
  onSelection(listener: (selection: Selection) => void): void;
  onStatus(listener: (status: ReadingStatus) => void): void;
  read(mood: MoodId): Promise<void>;
  stop(): Promise<void>;
  dismiss(): Promise<void>;
}
